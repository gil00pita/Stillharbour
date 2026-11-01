//! Separate, bounded derived-cache jobs. Never reconciles scan presence/history.
use super::*;

pub(super) struct ThumbnailJob {
    pub paused: Arc<super::super::job_control::PauseControl>,
    pub id: String,
    pub root_id: String,
    pub cancel: Arc<AtomicBool>,
}
impl Library {
    #[cfg(feature = "fixture-probes")]
    pub fn hold_initial_previews_for_probe(&self, hold: bool) {
        self.initial_previews_hold.store(hold, Ordering::Release);
    }
    pub fn generate_thumbnails(self: &Arc<Self>, root_id: String) -> Result<ScanJob> {
        // Serialize admission with destructive catalog/cache maintenance and scans.
        let _admission = self.scan_gate.try_start()?;
        self.ensure_work_admission()?;
        self.enqueue_thumbnails(root_id, true)
    }
    pub(super) fn enqueue_thumbnails(
        self: &Arc<Self>,
        root_id: String,
        explicit: bool,
    ) -> Result<ScanJob> {
        let _cache = self.cache_gate.read().map_err(|_| lock_error())?;
        if self.maintenance_pending.load(Ordering::Acquire) {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Cache maintenance is running.",
            ));
        }
        store::root(&self.catalog, root_id.clone())?;
        let mut jobs = self.thumbnail_jobs.lock().map_err(|_| lock_error())?;
        if let Some(job) = jobs.iter().find(|job| job.root_id == root_id) {
            return store::job(&self.catalog, job.id.clone());
        }
        if !explicit
            && let Some(initial) = store::initial_preparation_job(&self.catalog, root_id.clone())?
            && matches!(
                initial.state.as_str(),
                "canceled" | "interrupted" | "failed"
            )
        {
            return Ok(initial);
        }
        if jobs.len() >= 8 {
            return Err(AppError::new(
                ErrorCode::Busy,
                "The thumbnail job queue is full. Try again after a job finishes.",
            ));
        }
        let job = store::begin_thumbnail_job(&self.catalog, root_id.clone())?;
        let cancel = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(super::super::job_control::PauseControl::default());
        jobs.push(ThumbnailJob {
            paused: paused.clone(),
            id: job.id.clone(),
            root_id: root_id.clone(),
            cancel: cancel.clone(),
        });
        let owner = self.clone();
        let id = job.id.clone();
        let spawned = std::thread::Builder::new()
            .name("thumbnail-library-job".into())
            .spawn(move || {
                let outcome =
                    owner.run_thumbnail_job(&id, &root_id, cancel.clone(), paused.clone());
                let (state, message) = match outcome {
                    _ if cancel.load(Ordering::Relaxed) => (
                        "canceled",
                        "Thumbnail generation canceled; cached previews retained.".to_owned(),
                    ),
                    Ok(0) => (
                        "completed",
                        "Tiny previews prepared; clearer previews load as you browse.".to_owned(),
                    ),
                    Ok(_) => (
                        "partial",
                        "Tiny previews prepared with unavailable items. See details.".to_owned(),
                    ),
                    Err(error) => ("failed", error.message),
                };
                let _ =
                    store::thumbnail_job_state(&owner.catalog, id.clone(), state.into(), message);
                if let Ok(mut jobs) = owner.thumbnail_jobs.lock() {
                    jobs.retain(|job| job.id != id);
                }
            });
        if let Err(error) = spawned {
            jobs.retain(|active| active.id != job.id);
            store::thumbnail_job_state(
                &self.catalog,
                job.id.clone(),
                "failed".into(),
                "The thumbnail job could not start. Try again.".into(),
            )?;
            return Err(AppError::new(
                ErrorCode::WorkerFailed,
                "The thumbnail job could not start.",
            )
            .caused_by(error));
        }
        Ok(job)
    }
    fn run_thumbnail_job(
        &self,
        id: &str,
        root_id: &str,
        cancel: Arc<AtomicBool>,
        paused: Arc<super::super::job_control::PauseControl>,
    ) -> Result<u32> {
        let _permit = loop {
            super::super::job_control::boundary(&self.catalog, id, &paused, &cancel)?;
            if cancel.load(Ordering::Relaxed) {
                return Ok(0);
            }
            match self.thumbnail_job_gate.try_start() {
                Ok(permit) => break permit,
                Err(error) if error.code == ErrorCode::Busy => {
                    std::thread::sleep(Duration::from_millis(30))
                }
                Err(error) => return Err(error),
            }
        };
        store::thumbnail_job_state(
            &self.catalog,
            id.into(),
            "running".into(),
            "Preparing tiny previews".into(),
        )?;
        let (high, total) = store::thumbnail_inventory(&self.catalog, root_id.into())?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let (mut after, mut generated, mut cached, mut errors) = (0, 0, 0, 0);
        store::progress(
            &self.catalog,
            id.into(),
            total,
            0,
            0,
            0,
            "Preparing tiny previews".into(),
        )?;
        // A bounded, test-only latch holds real work before source admission.
        // It lets native checks inspect a known inventory without racing a fast decoder.
        #[cfg(feature = "fixture-probes")]
        if id.starts_with("thumbnail:initial:") {
            let held = std::time::Instant::now();
            while self.initial_previews_hold.load(Ordering::Acquire) {
                super::super::job_control::boundary(&self.catalog, id, &paused, &cancel)?;
                if cancel.load(Ordering::Relaxed) {
                    return Ok(0);
                }
                if held.elapsed() > Duration::from_secs(45) {
                    return Err(AppError::new(
                        ErrorCode::WorkerFailed,
                        "Fixture preview hold timed out.",
                    ));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        let mut work_time = Duration::ZERO;
        loop {
            super::super::job_control::boundary(&self.catalog, id, &paused, &cancel)?;
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let page = store::thumbnail_page(&self.catalog, root_id.into(), after, high)?;
            if page.is_empty() {
                break;
            }
            for (row, key) in page {
                super::super::job_control::boundary(&self.catalog, id, &paused, &cancel)?;
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let work_started = Instant::now();
                after = row;
                if self.existing_preview(&key)?.is_some() {
                    store::preview_state_for_edge(&self.catalog, key, None)?;
                    cached += 1;
                } else {
                    let result =
                        self.build_thumbnail(&key, &runtime, cancel.clone(), Class::Background);
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    match result {
                        Ok(()) => generated += 1,
                        Err(error) => {
                            errors += 1;
                            store::preview_state_for_edge(
                                &self.catalog,
                                key.clone(),
                                Some(error.message.clone()),
                            )?;
                            // No private paths in job errors; opaque IDs identify the failed asset.
                            store::issue(
                                &self.catalog,
                                id.into(),
                                key.asset_id.clone(),
                                format!("{:?}", error.code),
                                error.message,
                            )?;
                        }
                    }
                }
                work_time += work_started.elapsed();
                let processed = generated + cached + errors;
                let remaining_seconds = (work_time.as_secs_f64() / f64::from(processed)
                    * f64::from(total.saturating_sub(processed)))
                .ceil() as u64;
                store::progress(
                    &self.catalog,
                    id.into(),
                    total,
                    generated,
                    cached,
                    errors,
                    if processed < 3 {
                        "Preparing tiny previews; estimating remaining time…".into()
                    } else {
                        format!(
                            "Preparing tiny previews · about {} min remaining (estimate)",
                            remaining_seconds.div_ceil(60).max(1)
                        )
                    },
                )?;
            }
        }
        let processed = generated + cached + errors;
        if !cancel.load(Ordering::Relaxed) && processed != total {
            // A concurrent scan can change eligibility inside the high-water
            // window. Report partial coverage rather than inventing completion.
            errors += total.saturating_sub(processed);
            store::issue(
                &self.catalog,
                id.into(),
                "Catalog inventory".into(),
                "Conflict".into(),
                "Inventory changed during generation; generate again to cover changed items."
                    .into(),
            )?;
            store::progress(
                &self.catalog,
                id.into(),
                total.max(processed),
                generated,
                cached,
                errors,
                "Inventory changed during generation".into(),
            )?;
            return Err(AppError::new(
                ErrorCode::Conflict,
                "Inventory changed during preview preparation. Resume previews to cover remaining photos.",
            ));
        }
        Ok(errors)
    }
}

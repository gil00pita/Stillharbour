//! Separate, bounded derived-cache jobs. Never reconciles scan presence/history.
use super::*;

pub(super) struct ThumbnailJob {
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
        self.enqueue_thumbnails(root_id, true)
    }
    pub(super) fn enqueue_thumbnails(
        self: &Arc<Self>,
        root_id: String,
        explicit: bool,
    ) -> Result<ScanJob> {
        let _cache = self.cache_gate.read().map_err(|_| lock_error())?;
        if self.maintenance.load(Ordering::Acquire) {
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
        jobs.push(ThumbnailJob {
            id: job.id.clone(),
            root_id: root_id.clone(),
            cancel: cancel.clone(),
        });
        let owner = self.clone();
        let id = job.id.clone();
        let spawned = std::thread::Builder::new()
            .name("thumbnail-library-job".into())
            .spawn(move || {
                let outcome = owner.run_thumbnail_job(&id, &root_id, cancel.clone());
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
    fn run_thumbnail_job(&self, id: &str, root_id: &str, cancel: Arc<AtomicBool>) -> Result<u32> {
        let _permit = loop {
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
        loop {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let page = store::thumbnail_page(&self.catalog, root_id.into(), after, high)?;
            if page.is_empty() {
                break;
            }
            for (row, key) in page {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
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
                store::progress(
                    &self.catalog,
                    id.into(),
                    total,
                    generated,
                    cached,
                    errors,
                    "Preparing tiny previews; originals remain read-only".into(),
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

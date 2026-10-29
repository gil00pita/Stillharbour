use super::{
    filters::{CompiledFilter, invalid},
    model::*,
    native,
    scan::ScanContext,
    store, workers,
};
use crate::performance::{self, Endpoint, Metric, Span};
use crate::{
    catalog::Catalog,
    domain::{AppError, ErrorCode, Result},
    filesystem::{self, FileIdentity},
    jobs::{
        JobScheduler,
        resources::{Class, ResourceScheduler, Work},
    },
};
use std::{
    collections::VecDeque,
    fs::{self},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[path = "rendition_jobs.rs"]
mod rendition_jobs;

pub const THUMBNAIL_POLICY: &str = "p2-v3-tall-jpeg-srgb8-preview";
const CACHE_POLICIES: &[&str] = &[
    THUMBNAIL_POLICY,
    "p2-v2-bounded-pyramid-srgb8-preview",
    "p2-v1-file-backed-progressive-srgb8-preview",
    "m5-v2-oriented-native-srgb8-preview",
    "m1-v1-oriented-assumed-srgb-no-icc",
];

#[derive(Clone)]
pub struct LibraryOptions {
    pub data_directory: PathBuf,
    pub workers: workers::WorkerPaths,
}
struct ActiveScan {
    id: String,
    cancel: Arc<AtomicBool>,
}
struct ThumbnailWork {
    queued: Option<std::time::Instant>,
    key: ThumbnailKey,
    cancel: Arc<AtomicBool>,
}
struct Queue {
    pending: VecDeque<ThumbnailWork>,
    generation: u64,
    inflight: Vec<ThumbnailKey>,
    cancel: Arc<AtomicBool>,
    closed: bool,
}
struct ThumbnailQueue {
    state: Mutex<Queue>,
    changed: Condvar,
}
struct MaintenancePlan {
    preview: MaintenancePreview,
    revision: String,
    created: Instant,
}

pub struct Library {
    monitor: Mutex<Option<super::monitor::Monitor>>,
    cache_cleanup: super::cache_cleanup::CacheCleanup,
    cache_gate: RwLock<()>,
    maintenance: AtomicBool,
    maintenance_plan: Mutex<Option<MaintenancePlan>>,
    pub(crate) catalog: Arc<Catalog>,
    options: LibraryOptions,
    active: Mutex<Option<ActiveScan>>,
    thumbnail_jobs: Mutex<Vec<rendition_jobs::ThumbnailJob>>,
    thumbnail_job_gate: JobScheduler,
    scan_gate: JobScheduler,
    resources: ResourceScheduler,
    thumbnails: Arc<ThumbnailQueue>,
    recovery: Mutex<VecDeque<String>>,
}
// realpath preserves caller spelling on case-insensitive APFS. Compare
// ancestor objects as well as path components to reject nested case aliases.
fn ancestor_identity_matches(path: &Path, expected: &FileIdentity) -> bool {
    path.ancestors().any(|ancestor| {
        native::open_root(ancestor, None)
            .and_then(|file| filesystem::identity(&file))
            .is_ok_and(|actual| actual.volume == expected.volume && actual.file == expected.file)
    })
}
fn same_key(a: &ThumbnailKey, b: &ThumbnailKey) -> bool {
    a.asset_id == b.asset_id && a.revision == b.revision && a.edge == b.edge
}
fn lock_error() -> AppError {
    AppError::new(ErrorCode::WorkerFailed, "Library job state is unavailable.")
}

impl Library {
    pub(super) fn scan_idle(&self) -> bool {
        self.active.lock().is_ok_and(|active| active.is_none())
    }
    pub fn start_monitoring(self: &Arc<Self>) -> Result<()> {
        let mut state = self.monitor.lock().map_err(|_| lock_error())?;
        if state.is_none() {
            *state = Some(super::monitor::start(
                Arc::downgrade(self),
                self.options.data_directory.clone(),
            )?);
        }
        Ok(())
    }
    pub fn changes(&self, before: Option<String>, limit: u32) -> Result<LibraryChangePage> {
        super::changes::page(&self.catalog, before, limit)
    }

    pub fn browse_directory(
        &self,
        root_id: String,
        directory_id: Option<String>,
        name: String,
        cursor: Option<String>,
        limit: u32,
    ) -> Result<DirectoryPage> {
        super::directory::browse(&self.catalog, root_id, directory_id, name, cursor, limit)
    }
    pub fn root_exists(&self, id: String) -> Result<bool> {
        store::root_exists(&self.catalog, id)
    }
    pub fn prepare_maintenance(&self, target: MaintenanceTarget) -> Result<MaintenancePreview> {
        let _scan = self.scan_gate.try_start().map_err(|error| {
            AppError::new(
                ErrorCode::Busy,
                "Finish or cancel the active scan in Jobs, then review this action again.",
            )
            .caused_by(error)
        })?;
        if !self
            .thumbnail_jobs
            .lock()
            .map_err(|_| lock_error())?
            .is_empty()
        {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Finish or cancel thumbnail jobs in Jobs, then review this action again.",
            ));
        }
        let (preview, revision) = store::maintenance_preview(&self.catalog, target)?;
        *self.maintenance_plan.lock().map_err(|_| lock_error())? = Some(MaintenancePlan {
            preview: preview.clone(),
            revision,
            created: Instant::now(),
        });
        Ok(preview)
    }
    pub fn apply_maintenance(&self, id: String) -> Result<MaintenanceResult> {
        let _scan = self.scan_gate.try_start().map_err(|error| {
            AppError::new(
                ErrorCode::Busy,
                "Finish or cancel the active scan in Jobs, then review this action again.",
            )
            .caused_by(error)
        })?;
        if !self
            .thumbnail_jobs
            .lock()
            .map_err(|_| lock_error())?
            .is_empty()
        {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Finish or cancel thumbnail jobs in Jobs, then review this action again.",
            ));
        }
        if self.maintenance.swap(true, Ordering::AcqRel) {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Cache maintenance is already running.",
            ));
        }
        struct Reset<'a>(&'a AtomicBool);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _reset = Reset(&self.maintenance);
        let plan = {
            let mut stored = self.maintenance_plan.lock().map_err(|_| lock_error())?;
            if !stored.as_ref().is_some_and(|p| {
                p.preview.id == id && p.created.elapsed() < Duration::from_secs(300)
            }) {
                return Err(invalid("Review this action again before confirming."));
            }
            stored.take().ok_or_else(|| invalid("Missing review."))?
        };
        if store::revision(&self.catalog)? != plan.revision {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "The catalog changed. Review this action again.",
            ));
        }
        {
            let mut queue = self.thumbnails.state.lock().map_err(|_| lock_error())?;
            queue.cancel.store(true, Ordering::Relaxed);
            queue.pending.clear();
        }
        let _cache = self.cache_gate.write().map_err(|_| lock_error())?;
        let root = match plan.preview.target {
            MaintenanceTarget::Library { root_id } => Some(root_id),
            MaintenanceTarget::PreviewCache => None,
        };
        let result = (|| {
            let mut after = String::new();
            loop {
                let keys = store::cache_keys(&self.catalog, root.clone(), after.clone())?;
                if keys.is_empty() {
                    break;
                }
                for key in &keys {
                    self.cache_cleanup.remove(key)?;
                }
                after = keys
                    .last()
                    .cloned()
                    .ok_or_else(|| invalid("Missing cache cursor."))?;
            }
            self.cache_cleanup.sync()?;
            let result = store::finish_maintenance(&self.catalog, root.clone(), plan.revision)?;
            if let Some(id) = root {
                self.recovery
                    .lock()
                    .map_err(|_| lock_error())?
                    .retain(|value| value != &id);
            }
            Ok(result)
        })();
        self.thumbnails
            .state
            .lock()
            .map_err(|_| lock_error())?
            .cancel = Arc::new(AtomicBool::new(false));
        result
    }

    pub fn seek_asset(
        &self,
        query: AssetQuery,
        id: String,
        position: AssetSeekPosition,
        limit: u32,
    ) -> Result<AssetPage> {
        store::seek_asset(&self.catalog, query, id, position, limit)
    }

    pub fn validate_root_id(&self, id: String) -> Result<()> {
        store::root(&self.catalog, id)?;
        Ok(())
    }

    pub fn new(catalog: Arc<Catalog>, mut options: LibraryOptions) -> Result<Arc<Self>> {
        options.data_directory = options.data_directory.canonicalize()?;
        fs::create_dir_all(options.data_directory.join("thumbnails"))?;
        fs::create_dir_all(&options.workers.working_directory)?;
        let recovery = store::recover_jobs(&catalog)?;
        let library = Arc::new(Self {
            monitor: Mutex::new(None),
            cache_cleanup: super::cache_cleanup::CacheCleanup::open(&options.data_directory)?,
            cache_gate: RwLock::new(()),
            maintenance: AtomicBool::new(false),
            maintenance_plan: Mutex::new(None),
            catalog,
            options,
            active: Mutex::new(None),
            thumbnail_jobs: Mutex::new(Vec::new()),
            thumbnail_job_gate: JobScheduler::new(1)?,
            scan_gate: JobScheduler::new(1)?,
            resources: ResourceScheduler::default(),
            thumbnails: Arc::new(ThumbnailQueue {
                state: Mutex::new(Queue {
                    pending: VecDeque::new(),
                    generation: 0,
                    inflight: Vec::new(),
                    cancel: Arc::new(AtomicBool::new(false)),
                    closed: false,
                }),
                changed: Condvar::new(),
            }),
            recovery: Mutex::new(recovery.into()),
        });
        for index in 0..2 {
            let owner = Arc::downgrade(&library);
            let queue = library.thumbnails.clone();
            std::thread::Builder::new()
                .name(format!("thumbnail-{index}"))
                .spawn(move || {
                    let runtime = match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        Ok(r) => r,
                        Err(_) => return,
                    };
                    loop {
                        let task = {
                            let mut state = match queue.state.lock() {
                                Ok(s) => s,
                                Err(_) => return,
                            };
                            while state.pending.is_empty() && !state.closed {
                                state =
                                    match queue.changed.wait_timeout(state, Duration::from_secs(1))
                                    {
                                        Ok((s, _)) => s,
                                        Err(_) => return,
                                    };
                                if owner.strong_count() == 0 {
                                    return;
                                }
                            }
                            if state.closed {
                                return;
                            }
                            let task = state.pending.pop_front();
                            if let Some(task) = &task {
                                state.inflight.push(task.key.clone());
                            }
                            task
                        };
                        if let (Some(task), Some(library)) = (task, owner.upgrade()) {
                            performance::since(
                                Metric::ThumbnailQueueWait,
                                Endpoint::Worker,
                                task.queued,
                                0,
                            );
                            let result = library.build_thumbnail(
                                &task.key,
                                &runtime,
                                task.cancel.clone(),
                                Class::Interactive,
                            );
                            if let Err(error) = result
                                && !task.cancel.load(Ordering::Relaxed)
                            {
                                let _ = store::preview_state_for_edge(
                                    &library.catalog,
                                    task.key.clone(),
                                    Some(error.message),
                                );
                            }
                            if let Ok(mut state) = queue.state.lock() {
                                state.inflight.retain(|key| !same_key(key, &task.key));
                            }
                        }
                    }
                })?;
        }
        Ok(library)
    }

    pub fn register(
        self: &Arc<Self>,
        path: PathBuf,
        expected: Option<FileIdentity>,
        label: String,
        mode: RootMode,
        filter: ScanFilter,
    ) -> Result<ScanJob> {
        CompiledFilter::new(filter.clone())?;
        if label.len() > 128 || label.chars().any(char::is_control) {
            return Err(invalid(
                "Folder labels must be at most 128 bytes without control characters.",
            ));
        }
        let root = native::open_root(&path, expected.as_ref())?;
        let identity = filesystem::identity(&root)?;
        let volume = native::volume_token(&root)?;
        let path = path.canonicalize()?;
        let managed_identity =
            filesystem::identity(&native::open_root(&self.options.data_directory, None)?)?;
        if path.starts_with(&self.options.data_directory)
            || ancestor_identity_matches(&path, &managed_identity)
        {
            return Err(invalid(
                "Application-managed catalog and cache folders cannot be library roots.",
            ));
        }
        if path.components().any(|c| {
            c.as_os_str()
                .to_string_lossy()
                .to_ascii_lowercase()
                .ends_with(".photoslibrary")
        }) {
            return Err(invalid(
                "Export originals from Apple Photos before selecting an ordinary folder.",
            ));
        }
        self.check_overlap(&path, &identity, None)?;
        let label = if label.trim().is_empty() {
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        } else {
            label.trim().to_owned()
        };
        // Admission happens before persistence, so a busy scan cannot leave a
        // partly registered root and an ambiguous Save result.
        let permit = self.scan_gate.try_start()?;
        let id = store::add_root(
            &self.catalog,
            path,
            identity,
            volume,
            label,
            mode,
            filter.clone(),
        )?;
        self.start_with_permit(id, filter, permit)
    }

    fn check_overlap(
        &self,
        path: &Path,
        identity: &FileIdentity,
        except: Option<&str>,
    ) -> Result<()> {
        let roots = store::roots(&self.catalog)?;
        if except.is_none() && roots.len() >= 32 {
            return Err(invalid(
                "This version supports at most 32 nonoverlapping roots.",
            ));
        }
        for root in roots {
            if except == Some(root.view.id.as_str()) {
                continue;
            }
            if path.starts_with(&root.path)
                || root.path.starts_with(path)
                || (root.identity.volume == identity.volume && root.identity.file == identity.file)
                || ancestor_identity_matches(path, &root.identity)
                || ancestor_identity_matches(&root.path, identity)
            {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    format!(
                        "This folder overlaps the existing library folder ‘{}’. Use its folder filter instead.",
                        root.view.label
                    ),
                ));
            }
        }
        Ok(())
    }
    pub fn scan(self: &Arc<Self>, id: String, filter: ScanFilter) -> Result<ScanJob> {
        CompiledFilter::new(filter.clone())?;
        let permit = self.scan_gate.try_start()?;
        self.start_with_permit(id, filter, permit)
    }
    fn start_with_permit(
        self: &Arc<Self>,
        id: String,
        filter: ScanFilter,
        permit: crate::jobs::JobPermit,
    ) -> Result<ScanJob> {
        let mut root = store::root(&self.catalog, id.clone())?;
        let file = native::open_root(&root.path, Some(&root.identity))?;
        if native::volume_token(&file)? != root.volume {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "The selected volume changed. Relink before scanning.",
            ));
        }
        let compiled = CompiledFilter::new(filter.clone())?;
        #[cfg(unix)]
        let managed_identity =
            filesystem::identity(&native::open_root(&self.options.data_directory, None)?)?;
        let job = store::begin_job(&self.catalog, id, filter.clone())?;
        root.view.filter = filter;
        let cancel = permit.cancellation();
        *self.active.lock().map_err(|_| lock_error())? = Some(ActiveScan {
            id: job.id.clone(),
            cancel: cancel.clone(),
        });
        let owner = self.clone();
        let response = job.clone();
        let job_id = job.id.clone();
        let spawned = std::thread::Builder::new()
            .name("library-scan".into())
            .spawn(move || {
                let _permit = permit;
                let thumbnail_root = root.view.id.clone();
                let context = ScanContext {
                    catalog: owner.catalog.clone(),
                    root,
                    job,
                    filter: compiled,
                    paths: owner.options.workers.clone(),
                    #[cfg(unix)]
                    data_directory: owner.options.data_directory.clone(),
                    #[cfg(unix)]
                    data_identity: managed_identity,
                    canceled: cancel,
                    resources: owner.resources.clone(),
                    discovered: 0,
                    indexed: 0,
                    skipped: 0,
                    errors: 0,
                    batch: Vec::with_capacity(32),
                };
                if let Err(error) = context.run() {
                    let _ = store::finish_job(
                        &owner.catalog,
                        job_id.clone(),
                        "failed".into(),
                        error.message,
                    );
                }
                if let Ok(mut active) = owner.active.lock() {
                    *active = None;
                }
                // Enqueue only after the scan has published its inventory. A separate
                // bounded worker owns all rendition decoding and its own progress.
                if store::job(&owner.catalog, job_id.clone())
                    .is_ok_and(|job| matches!(job.state.as_str(), "completed" | "partial"))
                {
                    let _ = owner.enqueue_thumbnails(thumbnail_root);
                }
            });
        if let Err(error) = spawned {
            *self.active.lock().map_err(|_| lock_error())? = None;
            store::finish_job(
                &self.catalog,
                response.id.clone(),
                "failed".into(),
                "The scan worker could not start; retry scanning this saved folder.".into(),
            )?;
            return Err(AppError::new(
                ErrorCode::WorkerFailed,
                "The scan worker could not start. The folder remains saved.",
            )
            .caused_by(error));
        }
        Ok(response)
    }
    pub fn cancel(&self, id: String) -> Result<()> {
        if let Some(job) = self
            .thumbnail_jobs
            .lock()
            .map_err(|_| lock_error())?
            .iter()
            .find(|job| job.id == id)
        {
            job.cancel.store(true, Ordering::Relaxed);
            store::cancel_requested(&self.catalog, id)?;
            return Ok(());
        }

        if let Some(active) = self.active.lock().map_err(|_| lock_error())?.as_ref()
            && active.id == id
        {
            active.cancel.store(true, Ordering::Relaxed);
            store::cancel_requested(&self.catalog, id)?;
            return Ok(());
        }
        Err(invalid("This scan is no longer running."))
    }
    pub fn cancel_all(&self) {
        if let Ok(state) = self.monitor.lock()
            && let Some(monitor) = state.as_ref()
        {
            monitor.stop.store(true, Ordering::Relaxed);
        }
        if let Ok(jobs) = self.thumbnail_jobs.lock() {
            for job in jobs.iter() {
                job.cancel.store(true, Ordering::Relaxed);
            }
        }

        if let Ok(active) = self.active.lock()
            && let Some(active) = active.as_ref()
        {
            active.cancel.store(true, Ordering::Relaxed);
        }
        if let Ok(queue) = self.thumbnails.state.lock() {
            queue.cancel.store(true, Ordering::Relaxed);
        }
    }

    fn availability(root: &store::RootRecord) -> &'static str {
        match native::open_root(&root.path, Some(&root.identity))
            .and_then(|file| native::volume_token(&file))
        {
            Ok(volume) if volume == root.volume => "online",
            Ok(_) => "needs_relink",
            Err(error) if error.code == ErrorCode::Conflict => "needs_relink",
            Err(_) => match fs::symlink_metadata(&root.path) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => "offline",
                Err(_) => "inaccessible",
                Ok(m) if m.file_type().is_symlink() => "needs_relink",
                _ => "inaccessible",
            },
        }
    }
    pub fn status(self: &Arc<Self>) -> Result<LibraryStatus> {
        let _cache = self.cache_gate.read().map_err(|_| lock_error())?;
        let active_id = self
            .active
            .lock()
            .map_err(|_| lock_error())?
            .as_ref()
            .map(|a| a.id.clone());
        let jobs = store::jobs(&self.catalog)?;
        let running_root = active_id
            .as_ref()
            .and_then(|id| jobs.iter().find(|job| &job.id == id))
            .map(|job| job.root_id.as_str());
        for root in store::roots(&self.catalog)? {
            if running_root == Some(root.view.id.as_str()) {
                continue;
            }
            let availability = Self::availability(&root);
            if availability != root.view.availability {
                if availability == "online"
                    && matches!(root.view.availability.as_str(), "offline" | "inaccessible")
                {
                    let mut recovery = self.recovery.lock().map_err(|_| lock_error())?;
                    if !recovery.contains(&root.view.id) {
                        recovery.push_back(root.view.id.clone());
                    }
                }
                store::set_availability(&self.catalog, root.view.id, availability.into())?;
            }
        }
        if active_id.is_none() {
            let next = self.recovery.lock().map_err(|_| lock_error())?.pop_front();
            if let Some(id) = next {
                let root = store::root(&self.catalog, id.clone())?;
                if root.view.availability == "online"
                    && let Err(error) = self.scan(id.clone(), root.view.filter)
                {
                    self.recovery
                        .lock()
                        .map_err(|_| lock_error())?
                        .push_back(id);
                    if error.code != ErrorCode::Busy {
                        return Err(error);
                    }
                }
            }
        }
        let monitoring = self
            .monitor
            .lock()
            .map_err(|_| lock_error())?
            .as_ref()
            .map(|monitor| {
                monitor
                    .views
                    .lock()
                    .map(|views| views.clone())
                    .map_err(|_| lock_error())
            })
            .transpose()?;
        Ok(LibraryStatus {
            monitoring,
            roots: store::roots(&self.catalog)?
                .into_iter()
                .map(|r| r.view)
                .collect(),
            jobs: store::jobs(&self.catalog)?,
            revision: store::revision(&self.catalog)?,
            preview_generation: self
                .thumbnails
                .state
                .lock()
                .map_err(|_| lock_error())?
                .generation
                .to_string(),
            metadata_available: workers::helper_available(&self.options.workers),
            preview_available: self.options.workers.executable.is_file(),
        })
    }

    pub fn relink(
        self: &Arc<Self>,
        id: String,
        path: PathBuf,
        expected: Option<FileIdentity>,
    ) -> Result<ScanJob> {
        let root = store::root(&self.catalog, id.clone())?;
        let handle = native::open_root(&path, expected.as_ref())?;
        let identity = filesystem::identity(&handle)?;
        let volume = native::volume_token(&handle)?;
        let canonical = path.canonicalize()?;
        let managed_identity =
            filesystem::identity(&native::open_root(&self.options.data_directory, None)?)?;
        if canonical.starts_with(&self.options.data_directory)
            || ancestor_identity_matches(&canonical, &managed_identity)
        {
            return Err(invalid(
                "An application-managed folder cannot be linked as a library.",
            ));
        }
        self.check_overlap(&canonical, &identity, Some(&id))?;
        let samples = store::sample_assets(&self.catalog, id.clone())?;
        if samples.is_empty() && (volume != root.volume || identity.file != root.identity.file) {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "This root has no readable content samples to establish a new location. Restore its original folder or add a separate root.",
            ));
        }
        for (relative, digest) in samples {
            let file = native::open_asset(&canonical, &identity, &volume, &relative)?;
            if blake3::hash(&workers::read_input(file)?).to_hex().as_str() != digest {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "Relink samples do not match the saved library. Existing catalog entries were retained.",
                ));
            }
        }
        let permit = self.scan_gate.try_start()?;
        store::relink(&self.catalog, id.clone(), canonical, identity, volume)?;
        self.start_with_permit(id, root.view.filter, permit)
    }

    pub fn query(
        &self,
        query: AssetQuery,
        cursor: Option<String>,
        limit: u32,
    ) -> Result<AssetPage> {
        store::query(&self.catalog, query, cursor, limit)
    }
    pub fn detail(&self, id: String) -> Result<AssetDetail> {
        store::detail(&self.catalog, id)
    }
    pub fn neighbors(&self, query: AssetQuery, id: String) -> Result<AssetNeighbors> {
        store::neighbors(&self.catalog, query, id)
    }
    pub fn buckets(&self, query: AssetQuery) -> Result<Vec<DateBucket>> {
        store::buckets(&self.catalog, query)
    }
    pub fn freeze(&self, query: AssetQuery) -> Result<SelectionToken> {
        store::freeze(&self.catalog, query)
    }
    pub fn seek(&self, query: AssetQuery, year: String, limit: u32) -> Result<AssetPage> {
        store::seek(&self.catalog, query, year, limit)
    }
    pub fn seek_date(&self, query: AssetQuery, period: String, limit: u32) -> Result<AssetPage> {
        store::seek_date(&self.catalog, query, period, limit)
    }
    pub fn annotate(&self, selection: AssetSelection, patch: AnnotationPatch) -> Result<u32> {
        store::annotate(&self.catalog, selection, patch)
    }
    pub fn add_tags(&self, selection: AssetSelection, tags: Vec<String>) -> Result<u32> {
        store::add_tags(&self.catalog, selection, tags)
    }
    pub fn update_mode(&self, id: String, mode: RootMode) -> Result<()> {
        store::update_mode(&self.catalog, id, mode)
    }
    pub fn job_details(&self, id: String) -> Result<JobDetail> {
        Ok(JobDetail {
            job: store::job(&self.catalog, id.clone())?,
            issues: store::job_issues(&self.catalog, id)?,
        })
    }
    pub fn folders(&self, id: String) -> Result<Vec<FolderEntry>> {
        self.catalog.call(move|db|Ok(db.prepare("SELECT folder,count(*) FROM assets WHERE root_id=? AND is_media=1 AND presence<>'excluded' GROUP BY folder ORDER BY folder LIMIT 10000")?.query_map([id],|row|Ok(FolderEntry{path:row.get(0)?,count:row.get(1)?}))?.collect::<rusqlite::Result<_>>()?))
    }
    pub fn tags(&self) -> Result<Vec<String>> {
        self.catalog.call(|db|Ok(db.prepare("SELECT DISTINCT value FROM assets,json_each(assets.tags) WHERE is_media=1 ORDER BY value LIMIT 5000")?.query_map([],|row|row.get(0))?.collect::<rusqlite::Result<_>>()?))
    }

    pub fn filter_preview(&self, filter: ScanFilter, paths: Vec<String>) -> Result<Vec<String>> {
        if paths.len() > 32 || paths.iter().any(|p| p.len() > 2048) {
            return Err(invalid("Preview at most 32 bounded relative paths."));
        }
        let filter = CompiledFilter::new(filter)?;
        paths
            .into_iter()
            .map(|path| {
                let relative = Path::new(&path);
                if relative.is_absolute()
                    || relative
                        .components()
                        .any(|c| matches!(c, std::path::Component::ParentDir))
                {
                    return Err(invalid("Filter previews require root-relative paths."));
                }
                let mut reason = None;
                for parent in relative
                    .ancestors()
                    .skip(1)
                    .filter(|p| !p.as_os_str().is_empty())
                {
                    let hidden = parent
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with('.'));
                    reason = reason.or(filter.directory_reason(parent, hidden));
                }
                let hidden = relative
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with('.'));
                Ok(format!(
                    "{path}: {}",
                    reason
                        .or(filter.file_reason(relative, hidden, 0, None))
                        .unwrap_or("Included; size/date and OS attributes require a real file")
                ))
            })
            .collect()
    }

    fn cache_key(key: &ThumbnailKey) -> Result<String> {
        Self::cache_key_for_policy(key, THUMBNAIL_POLICY)
    }
    fn cache_key_for_policy(key: &ThumbnailKey, policy: &str) -> Result<String> {
        if uuid::Uuid::parse_str(&key.asset_id).is_err()
            || key.revision.len() != 64
            || !key.revision.bytes().all(|b| b.is_ascii_hexdigit())
            || !crate::rendition::supported_edge(key.edge)
        {
            return Err(invalid("Invalid thumbnail identity, revision or size."));
        }
        Ok(blake3::hash(
            format!("{policy}:{}:{}:{}", key.asset_id, key.revision, key.edge).as_bytes(),
        )
        .to_hex()
        .to_string())
    }
    fn cache_path(&self, key: &str) -> Result<PathBuf> {
        if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid("Invalid owned cache key."));
        }
        Ok(self
            .options
            .data_directory
            .join("thumbnails")
            .join(format!("{key}.png")))
    }
    fn indexed_preview(
        &self,
        key: &ThumbnailKey,
        found: &std::collections::HashSet<String>,
    ) -> Result<Option<String>> {
        for policy in CACHE_POLICIES {
            let hash = Self::cache_key_for_policy(key, policy)?;
            if found.contains(&hash) && self.cache_path(&hash)?.is_file() {
                return Ok(Some(hash));
            }
        }
        Ok(None)
    }
    fn existing_preview(&self, key: &ThumbnailKey) -> Result<Option<String>> {
        let candidates = CACHE_POLICIES
            .iter()
            .map(|policy| Self::cache_key_for_policy(key, policy))
            .collect::<Result<Vec<_>>>()?;
        let found = store::cache_lookup(&self.catalog, candidates)?;
        self.indexed_preview(key, &found)
    }
    pub fn thumbnails(
        &self,
        keys: Vec<ThumbnailKey>,
        generation: String,
    ) -> Result<Vec<ThumbnailResult>> {
        self.thumbnail_results(keys, generation, false)
    }
    /// Catalog/owned-cache status only: never advances/cancels the work queue,
    /// opens an original or schedules a worker, even for uncached online items.
    pub fn cached_thumbnails(
        &self,
        keys: Vec<ThumbnailKey>,
        generation: String,
    ) -> Result<Vec<ThumbnailResult>> {
        self.thumbnail_results(keys, generation, true)
    }
    fn thumbnail_results(
        &self,
        keys: Vec<ThumbnailKey>,
        generation: String,
        cache_only: bool,
    ) -> Result<Vec<ThumbnailResult>> {
        let _perf_span = Span::new(Metric::ThumbnailBatch, Endpoint::Worker);
        let _cache = self.cache_gate.read().map_err(|_| lock_error())?;
        if self.maintenance.load(Ordering::Acquire) {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Preview cache maintenance is running.",
            ));
        }
        if keys.len() > 64 || generation.len() > 20 {
            return Err(invalid("Request at most 64 visible thumbnails."));
        }
        let generation: u64 = generation
            .parse()
            .map_err(|_| invalid("Invalid preview generation."))?;
        if !cache_only
            && generation
                < self
                    .thumbnails
                    .state
                    .lock()
                    .map_err(|_| lock_error())?
                    .generation
        {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Preview request superseded.",
            ));
        }
        // One bounded cache-index snapshot and one lean asset/root projection,
        // rather than a catalog round trip for every policy/tier/asset.
        let mut candidates = Vec::new();
        for key in &keys {
            Self::cache_key(key)?;
            for edge in crate::rendition::EDGES
                .iter()
                .copied()
                .filter(|edge| *edge <= key.edge)
            {
                for policy in CACHE_POLICIES {
                    candidates.push(Self::cache_key_for_policy(
                        &ThumbnailKey {
                            edge,
                            ..key.clone()
                        },
                        policy,
                    )?);
                }
            }
        }
        let found = store::cache_lookup(&self.catalog, candidates)?;
        let statuses = store::preview_statuses(
            &self.catalog,
            keys.iter().map(|key| key.asset_id.clone()).collect(),
        )?;
        let mut output = Vec::with_capacity(keys.len());
        let mut pending = Vec::new();
        for key in keys {
            let cached = self.indexed_preview(&key, &found)?.is_some();
            let asset = statuses
                .get(&key.asset_id)
                .ok_or_else(|| invalid("The requested preview asset is unavailable."))?;
            let error = if asset.revision != key.revision {
                Some("The source revision changed; refresh this view.".into())
            } else if cached {
                None
            } else if asset.presence != "present"
                || !matches!(asset.root_availability.as_str(), "online" | "scanning")
            {
                Some("Reconnect the original to generate this preview.".into())
            } else if let Some(error) = asset.failure(key.edge) {
                Some(error)
            } else if asset.metadata_state == "placeholder" {
                Some("Download this original explicitly before generating a preview.".into())
            } else {
                None
            };
            let state = if error.is_some() {
                "unavailable"
            } else if cached {
                "ready"
            } else {
                "pending"
            };
            let mut fallback_edge = None;
            if asset.revision == key.revision {
                for edge in [1024, 512, 256, 32] {
                    if edge < key.edge
                        && self
                            .indexed_preview(
                                &ThumbnailKey {
                                    edge,
                                    ..key.clone()
                                },
                                &found,
                            )?
                            .is_some()
                    {
                        fallback_edge = Some(edge);
                        break;
                    }
                }
            }
            output.push(ThumbnailResult {
                fallback_edge,
                asset_id: key.asset_id.clone(),
                revision: key.revision.clone(),
                edge: key.edge,
                state: state.into(),
                error,
            });
            if state == "pending" {
                pending.push(key);
            }
        }
        if cache_only {
            return Ok(output);
        }
        let mut queue = self.thumbnails.state.lock().map_err(|_| lock_error())?;
        if generation < queue.generation {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Preview request superseded.",
            ));
        }
        if queue.generation != generation {
            queue.cancel.store(true, Ordering::Relaxed);
            queue.pending.clear();
            queue.cancel = Arc::new(AtomicBool::new(false));
            queue.generation = generation;
        }
        for key in pending {
            if queue.pending.len() >= 64 {
                break;
            }
            if !queue.inflight.iter().any(|active| same_key(active, &key))
                && !queue.pending.iter().any(|task| {
                    task.key.asset_id == key.asset_id
                        && task.key.revision == key.revision
                        && task.key.edge == key.edge
                })
            {
                let cancel = queue.cancel.clone();
                queue.pending.push_back(ThumbnailWork {
                    key,
                    cancel,
                    queued: performance::clock(),
                });
            }
        }
        self.thumbnails.changed.notify_all();
        Ok(output)
    }

    fn build_thumbnail(
        &self,
        key: &ThumbnailKey,
        runtime: &tokio::runtime::Runtime,
        cancel: Arc<AtomicBool>,
        class: Class,
    ) -> Result<()> {
        let _perf_span = Span::new(Metric::ThumbnailBuild, Endpoint::Worker);
        let _cache = self.cache_gate.read().map_err(|_| lock_error())?;
        if self.maintenance.load(Ordering::Acquire) || cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        if self.existing_preview(key)?.is_some() {
            return Ok(());
        }
        let (root_id, relative, revision) =
            store::asset_native(&self.catalog, key.asset_id.clone())?;
        if revision != key.revision {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "Preview revision changed.",
            ));
        }
        let root = store::root(&self.catalog, root_id)?;
        let file = native::open_asset(&root.path, &root.identity, &root.volume, &relative)?;
        let _lease = self.resources.acquire(
            filesystem::identity(&file)?.volume,
            class,
            Work::Rendition,
            &cancel,
        )?;
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        if native::revision(&file)? != revision {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "The original changed. Rescan the folder.",
            ));
        }
        let report = runtime.block_on(workers::thumbnail_file(
            &self.options.workers,
            file,
            key.edge,
            crate::media::PreviewFormat::from_extension(
                relative
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default(),
            ),
            cancel.clone(),
        ))?;
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        crate::rendition::validate(&report, key.edge)?;
        let hash = Self::cache_key(key)?;
        let path = self.cache_path(&hash)?;
        if !path.try_exists()? {
            let mut staged = tempfile::NamedTempFile::new_in(
                path.parent()
                    .ok_or_else(|| invalid("Missing cache directory."))?,
            )?;
            staged.write_all(&report.png)?;
            performance::record_at(
                Metric::Write,
                Endpoint::Cache,
                performance::file_volume(staged.as_file()),
                0,
                report.png.len() as u64,
            );
            staged.as_file().sync_all()?;
            match filesystem::publish_noreplace(staged.path(), &path) {
                Ok(()) => {}
                Err(error) if error.code == ErrorCode::Conflict => {}
                Err(error) => return Err(error),
            }
        }
        let existing = filesystem::open_readonly(&path)?;
        if existing.metadata()?.len() != report.png.len() as u64
            || workers::read_input_at(existing, Endpoint::Cache)? != report.png
        {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "An unexpected cache file was retained. Preview unavailable.",
            ));
        }
        // Discard the result if a writer replaced or changed the source while the child ran.
        let current = native::open_asset(&root.path, &root.identity, &root.volume, &relative)?;
        if native::revision(&current)? != revision {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "Source changed during preview generation. Rescan the folder.",
            ));
        }
        for removed in store::cache_add(
            &self.catalog,
            hash,
            key.asset_id.clone(),
            key.revision.clone(),
            key.edge,
            report.png.len() as i64,
        )? {
            // Only keys positively owned by the cache index are eligible. No
            // suffix-based cleanup of unknown or original files is performed.
            let obsolete = self.cache_path(&removed)?;
            match fs::remove_file(obsolete) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        if let Some((width, height)) = report.display_dimensions {
            store::preview_dimensions(&self.catalog, key.clone(), width, height)?;
        }
        store::preview_state_for_edge(&self.catalog, key.clone(), None)
    }
    pub fn cached_thumbnail(&self, key: ThumbnailKey, policy: &str) -> Result<Vec<u8>> {
        let _perf_span = Span::new(Metric::Read, Endpoint::Cache);
        let _cache = self.cache_gate.read().map_err(|_| lock_error())?;
        if policy != THUMBNAIL_POLICY {
            return Err(invalid("Unknown preview processing policy."));
        }
        let hash = self
            .existing_preview(&key)?
            .ok_or_else(|| AppError::new(ErrorCode::Io, "Preview is not cached."))?;
        let file = filesystem::open_readonly(&self.cache_path(&hash)?)?;
        if !file.metadata()?.is_file()
            || file.metadata()?.len() > crate::rendition::png_budget(key.edge)? as u64
        {
            return Err(invalid("Invalid cached preview."));
        }
        let volume = performance::file_volume(&file);
        let mut bytes = Vec::new();
        file.take(crate::rendition::png_budget(key.edge)? as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > crate::rendition::png_budget(key.edge)? {
            return Err(invalid("Invalid cached preview size."));
        }
        if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err(invalid("Invalid cached preview format."));
        }
        performance::record_at(Metric::Read, Endpoint::Cache, volume, 0, bytes.len() as u64);
        Ok(bytes)
    }
}
impl Drop for Library {
    fn drop(&mut self) {
        if let Ok(state) = self.monitor.lock()
            && let Some(monitor) = state.as_ref()
        {
            monitor.stop.store(true, Ordering::Relaxed);
        }
        if let Ok(mut queue) = self.thumbnails.state.lock() {
            queue.closed = true;
            queue.cancel.store(true, Ordering::Relaxed);
            queue.pending.clear();
            self.thumbnails.changed.notify_all();
        }
    }
}

#[cfg(test)]
mod maintenance_tests {
    use super::*;
    #[test]
    #[cfg(all(feature = "fixture-probes", target_os = "macos"))]
    fn cache_only_lookup_never_admits_or_cancels_source_work() {
        let data = tempfile::tempdir().unwrap();
        let catalog = Arc::new(Catalog::open(data.path().join("fixture.stillharbour")).unwrap());
        super::super::performance_probe::seed_catalog(&catalog, data.path(), 10_000).unwrap();
        // Explicit synthetic catalog observations, not an accessible source fixture.
        catalog
            .call(|db| {
                db.execute("UPDATE roots SET availability='online'", [])?;
                Ok(())
            })
            .unwrap();
        let library = Library::new(
            catalog,
            LibraryOptions {
                data_directory: data.path().to_owned(),
                workers: workers::WorkerPaths {
                    executable: data.path().join("must-not-start"),
                    exiftool: None,
                    working_directory: data.path().join("worker"),
                },
            },
        )
        .unwrap();
        let asset = library
            .query(AssetQuery::default(), None, 1)
            .unwrap()
            .assets
            .remove(0);
        let key = ThumbnailKey {
            asset_id: asset.id,
            revision: asset.revision,
            edge: 512,
        };
        library.thumbnails(vec![], "5".into()).unwrap();
        let cancel = library.thumbnails.state.lock().unwrap().cancel.clone();
        for generation in ["0", "999"] {
            let result = library
                .cached_thumbnails(vec![key.clone()], generation.into())
                .unwrap();
            assert_eq!(result[0].state, "pending");
        }
        let queue = library.thumbnails.state.lock().unwrap();
        assert_eq!(queue.generation, 5);
        assert!(queue.pending.is_empty() && queue.inflight.is_empty());
        assert!(!cancel.load(Ordering::Relaxed));
        drop(queue);
        assert!(library.thumbnails(vec![], "6".into()).is_ok());
        assert_eq!(
            library
                .cached_thumbnails(vec![key; 65], "7".into())
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
    #[test]
    fn scan_admission_blocks_maintenance_and_keeps_the_review_retryable() {
        let data = tempfile::tempdir().unwrap();
        let catalog = Arc::new(Catalog::open(data.path().join("fixture.stillharbour")).unwrap());
        let library = Library::new(
            catalog,
            LibraryOptions {
                data_directory: data.path().to_owned(),
                workers: workers::WorkerPaths {
                    executable: data.path().join("missing-worker"),
                    exiftool: None,
                    working_directory: data.path().join("worker"),
                },
            },
        )
        .unwrap();
        let unindexed = data.path().join("thumbnails/unindexed.png");
        std::fs::write(&unindexed, b"unindexed bytes preserved").unwrap();
        let preview = library
            .prepare_maintenance(MaintenanceTarget::PreviewCache)
            .unwrap();
        let permit = library.scan_gate.try_start().unwrap();
        assert_eq!(
            library
                .prepare_maintenance(MaintenanceTarget::PreviewCache)
                .unwrap_err()
                .code,
            ErrorCode::Busy
        );
        assert_eq!(
            library
                .apply_maintenance(preview.id.clone())
                .unwrap_err()
                .code,
            ErrorCode::Busy
        );
        drop(permit);
        #[cfg(unix)]
        library.apply_maintenance(preview.id).unwrap();
        #[cfg(not(unix))]
        {
            let revision = store::revision(&library.catalog).unwrap();
            assert_eq!(
                library.apply_maintenance(preview.id).unwrap_err().code,
                ErrorCode::Unsupported
            );
            assert_eq!(store::revision(&library.catalog).unwrap(), revision);
            // Failure releases admission and allows an explicit new review;
            // unverified native cleanup must remain unavailable on retry.
            let next = library
                .prepare_maintenance(MaintenanceTarget::PreviewCache)
                .unwrap();
            assert_eq!(
                library.apply_maintenance(next.id).unwrap_err().code,
                ErrorCode::Unsupported
            );
        }
        assert_eq!(
            std::fs::read(unindexed).unwrap(),
            b"unindexed bytes preserved"
        );
    }
    #[test]
    #[cfg(all(feature = "fixture-probes", target_os = "macos"))]
    fn queued_thumbnail_jobs_coalesce_cancel_and_block_maintenance_without_latching_it() {
        let data = tempfile::tempdir().unwrap();
        let catalog = Arc::new(Catalog::open(data.path().join("fixture.stillharbour")).unwrap());
        let fixture =
            super::super::performance_probe::seed_catalog(&catalog, data.path(), 10_000).unwrap();
        let root = fixture["rootIds"][0].as_str().unwrap().to_owned();
        let library = Library::new(
            catalog,
            LibraryOptions {
                data_directory: data.path().to_owned(),
                workers: workers::WorkerPaths {
                    executable: data.path().join("missing-worker"),
                    exiftool: None,
                    working_directory: data.path().join("worker"),
                },
            },
        )
        .unwrap();
        let review = library
            .prepare_maintenance(MaintenanceTarget::PreviewCache)
            .unwrap();
        let gate = library.thumbnail_job_gate.try_start().unwrap();
        let job = library.generate_thumbnails(root.clone()).unwrap();
        assert_eq!(library.generate_thumbnails(root).unwrap().id, job.id);
        assert_eq!(
            library
                .prepare_maintenance(MaintenanceTarget::PreviewCache)
                .unwrap_err()
                .code,
            ErrorCode::Busy
        );
        assert_eq!(
            library.apply_maintenance(review.id).unwrap_err().code,
            ErrorCode::Busy
        );
        assert!(!library.maintenance.load(Ordering::Acquire));
        library.cancel(job.id.clone()).unwrap();
        let started = Instant::now();
        while !library.thumbnail_jobs.lock().unwrap().is_empty() {
            assert!(started.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(
            store::job(&library.catalog, job.id).unwrap().state,
            "canceled"
        );
        drop(gate);
        assert!(
            library
                .prepare_maintenance(MaintenanceTarget::PreviewCache)
                .is_ok()
        );
    }
}

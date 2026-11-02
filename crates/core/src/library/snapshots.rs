//! Durable source-retaining snapshot workflow. Renderer authority is opaque IDs;
//! native selections, root handles, reviewed manifests and OS credentials stay here.
use super::{
    Library,
    inspection_inventory::{child, each},
    native, store,
};
use crate::backup::native_metadata::MetadataManifest;
use crate::{
    backup::{
        credentials::{CredentialPurpose, CredentialStore, Secret},
        helpers::{self, Helper, PinnedHelpers},
        snapshots::*,
    },
    catalog::Catalog,
    domain::{AppError, ErrorCode, Result},
    executor::snapshot_bridge::{self as bridge, FileSpec, Root},
    filesystem::{self, DirectoryChain, FileIdentity},
    jobs::operation_locks::{Access, Registry, Scope},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::File,
    path::{Component, Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
mod reconcile;

const MAX_FILES: usize = 10_000;
const MAX_RECORD: usize = 32 * 1024 * 1024;
const MAX_REVIEWS: usize = 32;
const TIMEOUT: Duration = Duration::from_secs(3600);
const REVIEW_AGE: Duration = Duration::from_secs(600);
const FIDELITY: &str = "Ordinary file bytes and the consistent catalog are verified separately from native metadata. Original or destination changes require fresh exact-version recovery with certified essential metadata and an independently verified physical failure domain.";
fn error(code: ErrorCode, message: &str) -> AppError {
    AppError::new(code, message)
}
fn invalid(message: &str) -> AppError {
    error(ErrorCode::InvalidInput, message)
}
fn conflict(message: &str) -> AppError {
    error(ErrorCode::Conflict, message)
}
fn unavailable(message: &str) -> AppError {
    error(ErrorCode::Unsupported, message)
}
fn id(value: &str) -> Result<()> {
    uuid::Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| invalid("Invalid snapshot application identifier."))
}
fn snapshot_id(value: &str) -> Result<()> {
    if value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(invalid("Select a concrete recorded snapshot identifier."))
    }
}
fn encode<T: Serialize>(value: &T) -> Result<String> {
    let text =
        serde_json::to_string(value).map_err(|_| invalid("Could not encode snapshot state."))?;
    if text.len() > MAX_RECORD {
        return Err(invalid("Snapshot state exceeds its bounded size."));
    }
    Ok(text)
}
fn decode<T: serde::de::DeserializeOwned>(text: String) -> Result<T> {
    if text.len() > MAX_RECORD {
        return Err(invalid("Stored snapshot state exceeds its bound."));
    }
    serde_json::from_str(&text)
        .map_err(|_| invalid("Stored snapshot state is invalid or newer than this build."))
}
fn now() -> String {
    store::now()
}
fn digest<T: Serialize>(value: &T) -> Result<String> {
    Ok(blake3::hash(encode(value)?.as_bytes()).to_hex().to_string())
}

/// Constructed only by the native shell's directory picker session.
pub struct NativeSnapshotSelection {
    pub id: String,
    pub path: PathBuf,
    pub identity: FileIdentity,
    pub volume: String,
}
pub struct NativeSnapshotCredentialTarget {
    pub needs_local_selection: bool,
    pub additional_purpose: Option<CredentialPurpose>,
    pub catalog_id: String,
    pub purpose: CredentialPurpose,
    pub reference: String,
    pub context_digest: Option<String>,
    pub label: String,
}
pub struct SnapshotKeyExportSource {
    pub secret: Secret,
    pub selection: NativeSnapshotSelection,
    pub label: String,
    pub journal_directory: PathBuf,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    path: Vec<u8>,
    identity: FileIdentity,
    volume: String,
}
impl Binding {
    fn capture(path: PathBuf, identity: FileIdentity, volume: String) -> Result<Self> {
        let binding = Self {
            path: native::encode_path(&path),
            identity,
            volume,
        };
        binding.open()?;
        Ok(binding)
    }
    fn path(&self) -> Result<PathBuf> {
        native::decode_path(&self.path)
    }
    fn open(&self) -> Result<File> {
        let file = native::open_root(&self.path()?, Some(&self.identity))?;
        if native::volume_token(&file)? != self.volume {
            return Err(conflict(
                "Snapshot destination/source volume changed. Reauthorize it before any action.",
            ));
        }
        Ok(file)
    }
    fn root(&self) -> Result<Root> {
        Ok(Root::existing(
            self.path()?,
            self.identity.clone(),
            self.volume.clone(),
        ))
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootBinding {
    id: String,
    binding: Binding,
    label: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileRecord {
    version: u32,
    view: SnapshotProfile,
    destination: SnapshotDestinationInput,
    local: Option<Binding>,
    roots: Vec<RootBinding>,
    repository_credential_ref: String,
    transport_credential_ref: Option<String>,
    namespace_id: String,
    supersedes: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileVersion {
    #[serde(default)]
    metadata: Option<MetadataManifest>,
    id: String,
    root_id: String,
    relative: Vec<u8>,
    identity: FileIdentity,
    revision: String,
    bytes: u64,
    digest: String,
    archive: String,
    display: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RootVersion {
    root: RootBinding,
    directories: Vec<(Vec<u8>, String)>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    platform: String,
    catalog_schema: u32,
    #[serde(default)]
    private_scope: bool,
    version: u32,
    catalog_id: String,
    catalog_revision: String,
    profile_digest: String,
    repository_id: Option<String>,
    roots: Vec<RootVersion>,
    files: Vec<FileVersion>,
    catalog_digest: Option<String>,
    catalog_bytes: u64,
}
impl Manifest {
    fn reviewed_digest(&self) -> Result<String> {
        let mut stable = self.clone();
        stable.catalog_digest = None;
        stable.catalog_bytes = 0;
        digest(&stable)
    }
    fn bytes(&self) -> u64 {
        self.files
            .iter()
            .map(|f| f.bytes)
            .fold(0, u64::saturating_add)
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepositoryBinding {
    namespace_id: String,
    local: Option<Binding>,
    repository_id: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    #[serde(default)]
    metadata_verified: bool,
    view: SnapshotRecord,
    manifest: Manifest,
    subfolder: String,
    repository_id: String,
    profile_namespace: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JobRecord {
    #[serde(default)]
    verified_output: Option<Binding>,
    #[serde(default)]
    metadata_verified: bool,
    #[serde(default)]
    private_scope: bool,
    review_id: String,
    view: SnapshotJob,
    profile: ProfileRecord,
    manifest: Manifest,
    work: Option<Binding>,
    restore: Option<Binding>,
    subfolder: Option<String>,
}
#[derive(Clone)]
struct Review {
    private_scope: bool,
    view: SnapshotReview,
    profile: ProfileRecord,
    manifest: Manifest,
    restore: Option<Binding>,
    record: Option<Record>,
    created: Instant,
}
struct Approval {
    review: String,
    digest: String,
    action: String,
    created: Instant,
}
struct Active {
    id: String,
    roots: Vec<String>,
    cancel: Arc<AtomicBool>,
}
struct State {
    terminal_failures: BTreeMap<String, SnapshotJob>,
    helpers: Option<PinnedHelpers>,
    authorized: BTreeSet<String>,
    reviews: BTreeMap<String, Review>,
    approvals: BTreeMap<String, Approval>,
    active: Option<Active>,
    timer_started: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScheduleRecord {
    view: SnapshotSchedule,
    profile_digest: String,
    manifest_digest: String,
    instance: String,
}
pub(crate) struct SnapshotWorkflow {
    started: chrono::DateTime<chrono::Utc>,
    catalog: Arc<Catalog>,
    data: Binding,
    state: Mutex<State>,
    stopped: AtomicBool,
}

pub(crate) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE backup_snapshot_profiles(id TEXT PRIMARY KEY,record TEXT NOT NULL CHECK(json_valid(record))); CREATE TABLE backup_snapshot_jobs(id TEXT PRIMARY KEY,profile_id TEXT NOT NULL,record TEXT NOT NULL CHECK(json_valid(record))); CREATE TABLE backup_snapshot_records(id TEXT PRIMARY KEY,profile_id TEXT NOT NULL,record TEXT NOT NULL CHECK(json_valid(record))); CREATE TABLE backup_snapshot_events(sequence INTEGER PRIMARY KEY,job_id TEXT NOT NULL,phase TEXT NOT NULL,evidence TEXT NOT NULL CHECK(json_valid(evidence))); CREATE TABLE backup_snapshot_schedules(profile_id TEXT PRIMARY KEY,record TEXT NOT NULL CHECK(json_valid(record))); CREATE INDEX backup_snapshot_jobs_profile ON backup_snapshot_jobs(profile_id,id); CREATE INDEX backup_snapshot_records_profile ON backup_snapshot_records(profile_id,id); CREATE INDEX backup_snapshot_events_job ON backup_snapshot_events(job_id,sequence); CREATE TRIGGER backup_profiles_no_update BEFORE UPDATE ON backup_snapshot_profiles BEGIN SELECT RAISE(ABORT,'Retain immutable snapshot profile'); END; CREATE TRIGGER backup_profiles_no_delete BEFORE DELETE ON backup_snapshot_profiles BEGIN SELECT RAISE(ABORT,'Retain snapshot profile'); END; CREATE TRIGGER backup_records_no_update BEFORE UPDATE ON backup_snapshot_records BEGIN SELECT RAISE(ABORT,'Retain snapshot record'); END; CREATE TRIGGER backup_records_no_delete BEFORE DELETE ON backup_snapshot_records BEGIN SELECT RAISE(ABORT,'Retain snapshot record'); END; CREATE TRIGGER backup_events_no_update BEFORE UPDATE ON backup_snapshot_events BEGIN SELECT RAISE(ABORT,'Retain snapshot journal'); END; CREATE TRIGGER backup_events_no_delete BEFORE DELETE ON backup_snapshot_events BEGIN SELECT RAISE(ABORT,'Retain snapshot journal'); END;")?;
    Ok(())
}
impl SnapshotWorkflow {
    fn finish_job(
        &self,
        job: &mut JobRecord,
        cancel: &AtomicBool,
        outcome: std::thread::Result<Result<()>>,
    ) -> Result<()> {
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                job.view.state = if cancel.load(Ordering::Acquire) {
                    "canceled_with_partial_copies"
                } else {
                    "needs_recovery"
                }
                .into();
                job.view.message = error.message;
            }
            Err(_) => {
                job.view.state = "needs_recovery".into();
                job.view.message =
                    "The inspection worker stopped unexpectedly; partial outputs are retained."
                        .into();
            }
        }
        job.view.finished_at = Some(now());
        let result = self.phase(job, "terminal");
        let mut state = self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?;
        if result.is_err() {
            job.view.state = "needs_recovery".into();
            job.view.message =
                "The durable terminal result could not be stored; no completion is certified."
                    .into();
            state
                .terminal_failures
                .insert(job.view.id.clone(), job.view.clone());
        }
        if state
            .active
            .as_ref()
            .is_some_and(|active| active.id == job.view.id)
        {
            state.active = None;
        }
        result
    }
    fn validate_restore_selection(
        &self,
        library: &Library,
        p: &ProfileRecord,
        record: &Record,
        selection: &NativeSnapshotSelection,
    ) -> Result<Binding> {
        id(&selection.id)?;
        let target = Binding::capture(
            selection.path.clone(),
            selection.identity.clone(),
            selection.volume.clone(),
        )?;
        let chain = DirectoryChain::open(&selection.path)?;
        let mut protected = vec![self.data.identity.clone()];
        protected.extend(
            store::roots(&library.catalog)?
                .into_iter()
                .map(|root| root.identity),
        );
        protected.extend(p.roots.iter().map(|root| root.binding.identity.clone()));
        protected.extend(
            record
                .manifest
                .roots
                .iter()
                .map(|root| root.root.binding.identity.clone()),
        );
        if let Some(repository) = self.repository(p)?.and_then(|repository| repository.local) {
            protected.push(repository.identity);
        }
        if protected.iter().any(|root| {
            chain
                .identities()
                .iter()
                .any(|ancestor| ancestor.volume == root.volume && ancestor.file == root.file)
        }) {
            return Err(conflict(
                "A protected restore parent cannot be inside a current/historical source, application data or repository, including native aliases.",
            ));
        }
        // Only the parent is picked. The final reviewed UUID child is created
        // exclusively; an ancestor parent cannot make that NEW empty object
        // contain an existing source. Missing sources are never treated empty.
        Ok(target)
    }
    fn secret_present(&self, p: &ProfileRecord, purpose: CredentialPurpose) -> bool {
        match purpose {
            CredentialPurpose::Repository => {
                CredentialStore::get(&p.view.catalog_id, purpose, &p.repository_credential_ref)
                    .is_ok()
            }
            CredentialPurpose::Transport => helpers::credential_context(&p.destination)
                .ok()
                .is_some_and(|(context, _)| {
                    p.transport_credential_ref
                        .as_deref()
                        .is_some_and(|reference| {
                            CredentialStore::get_bound(
                                &p.view.catalog_id,
                                purpose,
                                reference,
                                Some(&context),
                            )
                            .is_ok()
                        })
                }),
        }
    }
    fn refresh_manifest(
        &self,
        p: &ProfileRecord,
        frozen: &Manifest,
        private_scope: bool,
        cancel: &AtomicBool,
    ) -> Result<Manifest> {
        if !private_scope {
            return self.inventory(p, cancel);
        }
        let mut current = frozen.clone();
        current.catalog_revision = store::revision(&self.catalog)?;
        current.repository_id = self.repository(p)?.and_then(|r| r.repository_id);
        for root in &current.roots {
            root.root.binding.open()?;
        }
        for file in &mut current.files {
            let root = current
                .roots
                .iter()
                .find(|r| r.root.id == file.root_id)
                .ok_or_else(|| invalid("Private recovery scope lost its native root."))?;
            let relative = native::decode_path(&file.relative)?;
            let binding = &root.root.binding;
            let mut source = native::open_asset(
                &binding.path()?,
                &binding.identity,
                &binding.volume,
                &relative,
            )?;
            let identity = filesystem::identity(&source)?;
            let revision = native::revision(&source)?;
            let bytes = source.metadata()?.len();
            let digest = bridge::digest(&mut source, cancel)?;
            let reopened = native::open_asset(
                &binding.path()?,
                &binding.identity,
                &binding.volume,
                &relative,
            )?;
            if filesystem::identity(&reopened)? != identity
                || native::revision(&reopened)? != revision
            {
                return Err(conflict(
                    "An exact old destination version changed native membership during recovery review.",
                ));
            }
            file.identity = identity;
            file.revision = revision;
            file.bytes = bytes;
            file.digest = digest;
            file.metadata = metadata_for(&source)?;
        }
        Ok(current)
    }
    #[cfg(any(test, feature = "backup-workflow", feature = "fixture-probes"))]
    pub(crate) fn review_recovery_backup(
        &self,
        _library: &Library,
        profile_id: &str,
        files: Vec<crate::backup::sync::RecoveryFile>,
    ) -> Result<SnapshotReview> {
        let mut profile = self.profile(profile_id)?;
        self.authorized(&profile)?;
        self.registered_scope(&profile)?;
        if files.is_empty() || files.len() > MAX_FILES {
            return Err(invalid(
                "Exact destination recovery requires a nonempty bounded backend scope.",
            ));
        }
        let original_digest = digest(&profile)?;
        let mut roots: BTreeMap<String, RootBinding> = BTreeMap::new();
        let mut versions = Vec::new();
        let mut seen = BTreeSet::new();
        for file in files {
            id(&file.root_id)?;
            if file.catalog_id != profile.view.catalog_id
                || !seen.insert((file.root_id.clone(), file.relative_path.clone()))
            {
                return Err(conflict(
                    "The backend recovery scope has another catalog or duplicate path.",
                ));
            }
            let binding = Binding::capture(
                native::decode_path(&file.native_root)?,
                file.root_identity,
                file.root_volume,
            )?;
            if let Some(existing) = roots.get(&file.root_id) {
                if encode(&existing.binding)? != encode(&binding)? {
                    return Err(conflict(
                        "One recovery dataset identifier maps to different native roots.",
                    ));
                }
            } else {
                roots.insert(
                    file.root_id.clone(),
                    RootBinding {
                        id: file.root_id.clone(),
                        binding,
                        label: "Exact reviewed old destination".into(),
                    },
                );
            }
            let relative = native::decode_path(&file.relative_path)?;
            let root = &roots
                .get(&file.root_id)
                .ok_or_else(|| invalid("Missing backend recovery root."))?
                .binding;
            let source =
                native::open_asset(&root.path()?, &root.identity, &root.volume, &relative)?;
            let metadata = metadata_for(&source)?;
            if let Some(expected) = &file.essential_metadata_digest
                && metadata
                    .as_ref()
                    .map(MetadataManifest::digest)
                    .transpose()?
                    .as_ref()
                    != Some(expected)
            {
                return Err(conflict(
                    "The exact old destination essential metadata changed or is unsupported.",
                ));
            }
            let key = blake3::hash(&file.relative_path).to_hex().to_string();
            versions.push(FileVersion {
                metadata,
                id: format!("{}:{key}", file.root_id),
                root_id: file.root_id.clone(),
                relative: file.relative_path,
                identity: file.identity,
                revision: file.revision,
                bytes: file.bytes,
                digest: file.full_digest,
                archive: format!("roots/{}/{key}.bin", file.root_id),
                display: format!("Old destination/{}", relative.display()),
            });
        }
        if roots.len() > 16 {
            return Err(invalid("Too many exact recovery dataset roots."));
        }
        profile.roots = roots.into_values().collect();
        profile.view.root_ids = profile.roots.iter().map(|r| r.id.clone()).collect();
        let mut manifest = Manifest {
            private_scope: true,
            version: 1,
            catalog_id: profile.view.catalog_id.clone(),
            catalog_revision: store::revision(&self.catalog)?,
            catalog_schema: self.catalog.summary()?.schema,
            platform: std::env::consts::OS.into(),
            profile_digest: original_digest,
            repository_id: self.repository(&profile)?.and_then(|r| r.repository_id),
            roots: profile
                .roots
                .iter()
                .map(|root| RootVersion {
                    root: root.clone(),
                    directories: Vec::new(),
                })
                .collect(),
            files: versions,
            catalog_digest: None,
            catalog_bytes: 0,
        };
        manifest.files.sort_by(|a, b| a.archive.cmp(&b.archive));
        validate_manifest(&manifest, &profile)?;
        if self
            .refresh_manifest(&profile, &manifest, true, &AtomicBool::new(false))?
            .reviewed_digest()?
            != manifest.reviewed_digest()?
        {
            return Err(conflict(
                "The backend old destination versions changed before recovery review.",
            ));
        }
        let cap = self.capabilities();
        let mut view=SnapshotReview{items:review_items(&manifest),id:uuid::Uuid::new_v4().to_string(),profile_id:profile.view.id.clone(),action:"backup".into(),digest:manifest.reviewed_digest()?,files:manifest.files.len() as u32,bytes:manifest.bytes().to_string(),root_ids:profile.view.root_ids.clone(),catalog_included:true,source_display_paths:profile.roots.iter().map(|r|r.binding.path().map(|p|p.display().to_string())).collect::<Result<Vec<_>>>()?,source_retained:true,target_display:profile.view.destination_display.clone(),issues:vec!["This approval protects only the explicitly listed exact old destination files and companions derived from the selected comparison. It grants no future destination scope.".into(),FIDELITY.into()],execution_available:cap.backup_available,unavailable_reason:cap.unavailable_reason};
        view.issues.extend(review_issues(&manifest));
        self.remember_review(Review {
            private_scope: true,
            view: view.clone(),
            profile,
            manifest,
            restore: None,
            record: None,
            created: Instant::now(),
        })?;
        Ok(view)
    }
    pub(crate) fn new(catalog: Arc<Catalog>, data_directory: PathBuf) -> Result<Self> {
        let file = native::open_root(&data_directory, None)?;
        Ok(Self {
            started: chrono::Utc::now(),
            catalog,
            data: Binding::capture(
                data_directory,
                filesystem::identity(&file)?,
                native::volume_token(&file)?,
            )?,
            state: Mutex::new(State {
                terminal_failures: BTreeMap::new(),
                helpers: None,
                authorized: BTreeSet::new(),
                reviews: BTreeMap::new(),
                approvals: BTreeMap::new(),
                active: None,
                timer_started: false,
            }),
            stopped: AtomicBool::new(false),
        })
    }
    pub(crate) fn configure_helpers(&self, path: PathBuf) -> Result<()> {
        let helpers = PinnedHelpers::configure(path)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?;
        if self.stopped.load(Ordering::Acquire)
            || state.active.is_some()
            || state
                .helpers
                .as_ref()
                .is_some_and(|helpers| !helpers.idle())
        {
            return Err(error(
                ErrorCode::Busy,
                "Backup helper resources cannot change while stopping or in use.",
            ));
        }
        state.helpers = Some(helpers);
        Ok(())
    }
    fn helpers(&self) -> Result<PinnedHelpers> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(error(
                ErrorCode::Busy,
                "The snapshot helper service is stopping.",
            ));
        }
        self.state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?
            .helpers
            .clone()
            .ok_or_else(|| {
                unavailable("Pinned bundled backup helpers are unavailable in this build.")
            })
    }
    pub(crate) fn cancel_all(&self) {
        self.stopped.store(true, Ordering::Release);
        if let Ok(state) = self.state.lock()
            && let Some(helpers) = &state.helpers
        {
            helpers.cancel_all();
        }
        if let Ok(state) = self.state.lock()
            && let Some(active) = &state.active
        {
            active.cancel.store(true, Ordering::Release);
        }
    }
    pub(crate) fn root_busy(&self, root: &str) -> Result<bool> {
        Ok(self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?
            .active
            .as_ref()
            .is_some_and(|a| a.roots.iter().any(|r| r == root)))
    }
    pub(crate) fn idle(&self) -> Result<bool> {
        let state = match self.state.try_lock() {
            Ok(state) => state,
            Err(std::sync::TryLockError::WouldBlock) => return Ok(false),
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err(unavailable("Snapshot state is unavailable."));
            }
        };
        Ok(state.active.is_none() && state.helpers.as_ref().is_none_or(PinnedHelpers::idle))
    }
    pub(crate) fn cancel_root(&self, root: &str) -> Result<()> {
        let authorized = self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?
            .authorized
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let mut revoke = BTreeSet::new();
        for profile in authorized {
            if self
                .profile(&profile)?
                .view
                .root_ids
                .iter()
                .any(|id| id == root)
            {
                revoke.insert(profile);
            }
        }
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| unavailable("Snapshot state is unavailable."))?;
            state.authorized.retain(|id| !revoke.contains(id));
            state
                .reviews
                .retain(|_, review| !revoke.contains(&review.profile.view.id));
            let reviews = state.reviews.keys().cloned().collect::<BTreeSet<_>>();
            state
                .approvals
                .retain(|_, approval| reviews.contains(&approval.review));
        }
        if let Some(active) = &self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?
            .active
            && active.roots.iter().any(|r| r == root)
        {
            active.cancel.store(true, Ordering::Release);
        }
        Ok(())
    }
    fn capabilities(&self) -> SnapshotCapabilities {
        let helpers = self.helpers().is_ok();
        let enabled = cfg!(any(test, feature = "backup-workflow"));
        SnapshotCapabilities{helpers_available:helpers,credentials_available:CredentialStore::available(),backup_available:helpers&&enabled,restore_available:helpers&&enabled,schedule_available:helpers&&enabled,protocols:vec!["local".into(),"sftp".into(),"ftps".into(),"ftp".into(),"smb".into()],unavailable_reason:if !enabled{Some("Snapshot execution is disabled until native application recovery gates pass.".into())}else if !helpers{Some("Verified bundled restic/rclone resources are unavailable.".into())}else{None},helper_limits:"Original staging and byte comparison share the application-issued I/O budget. Helpers run serially with one transfer/checker, 64 KiB buffers, multi-thread transfers disabled and a 10 MiB/s network cap. Helper internal local reads, metadata and physical I/O are not capped.".into(),schedule_limits:"Exact reviewed manifests only, every 15–10080 elapsed minutes in UTC. Changed content/root/endpoint needs fresh review. Runs only while app/tray is alive; fully quit execution is unavailable.".into()}
    }
    fn require_execution(&self) -> Result<()> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(error(ErrorCode::Busy, "Snapshot service is stopping."));
        }
        if !self.capabilities().backup_available {
            return Err(unavailable(
                "Snapshot execution requires verified bundled helpers and the native workflow gate.",
            ));
        }
        Ok(())
    }
    fn event(&self, job: &str, phase: &str, evidence: serde_json::Value) -> Result<()> {
        let job = job.to_string();
        let phase = phase.to_string();
        let text = encode(&evidence)?;
        self.catalog.call(move |db| {
            db.execute(
                "INSERT INTO backup_snapshot_events(job_id,phase,evidence) VALUES(?1,?2,?3)",
                params![job, phase, text],
            )?;
            Ok(())
        })
    }
    fn profile(&self, value: &str) -> Result<ProfileRecord> {
        id(value)?;
        let key = value.to_string();
        let text=self.catalog.call(move |db|Ok(db.query_row("SELECT CASE WHEN length(CAST(record AS BLOB))<=262144 THEN record END FROM backup_snapshot_profiles WHERE id=?",[key],|r|r.get::<_,String>(0))?))?;
        let mut p: ProfileRecord = decode(text)?;
        if p.version != 1 || p.view.catalog_id != self.catalog.summary()?.id || p.view.id != value {
            return Err(conflict(
                "Snapshot profile belongs to another catalog or policy.",
            ));
        }
        helpers::validate_destination(&p.destination)?;
        id(&p.namespace_id)?;
        if p.roots.is_empty() || p.roots.len() > 16 {
            return Err(invalid("Invalid snapshot root scope."));
        }
        if matches!(p.destination, SnapshotDestinationInput::Local { .. }) != p.local.is_some() {
            return Err(invalid(
                "Stored snapshot protocol/native repository binding is inconsistent.",
            ));
        }
        let mut roots = BTreeSet::new();
        for root in &p.roots {
            id(&root.id)?;
            if !roots.insert(root.id.clone()) {
                return Err(invalid("Stored snapshot native roots repeat."));
            }
        }
        p.view.root_ids = roots.into_iter().collect();
        p.view.protocol = protocol(&p.destination).into();
        p.view.insecure_transport = matches!(p.destination, SnapshotDestinationInput::Ftp { .. });
        p.view.host_fingerprint = host_fingerprint(&p.destination);
        p.view.destination_display = match &p.local {
            Some(parent) => parent
                .path()?
                .join(repo_leaf(&p.view.catalog_id, &p.namespace_id))
                .display()
                .to_string(),
            None => remote_display(&p.destination, &p.namespace_id)?,
        };
        Ok(p)
    }
    fn lineage(&self, profile: &ProfileRecord) -> Result<Vec<String>> {
        let mut ids = Vec::new();
        let mut current = profile.clone();
        for _ in 0..32 {
            if ids.contains(&current.view.id) {
                return Err(invalid("Snapshot profile lineage contains a cycle."));
            }
            ids.push(current.view.id.clone());
            let Some(parent) = current.supersedes else {
                return Ok(ids);
            };
            let prior = self.profile(&parent)?;
            if prior.namespace_id != profile.namespace_id {
                return Err(conflict("Snapshot lineage changed repository scope."));
            }
            current = prior;
        }
        Err(invalid("Snapshot profile lineage exceeds its bound."))
    }
    fn repository(&self, p: &ProfileRecord) -> Result<Option<RepositoryBinding>> {
        let namespace = p.namespace_id.clone();
        let row=self.catalog.call(move |db|Ok(db.query_row("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=262144 THEN evidence END FROM backup_snapshot_events WHERE phase='repository_binding' AND CASE WHEN length(CAST(evidence AS BLOB))<=262144 THEN json_extract(evidence,'$.namespace_id') END=? ORDER BY sequence DESC LIMIT 1",[namespace],|r|r.get::<_,String>(0)).optional()?))?;
        row.map(decode).transpose()
    }
    fn profile_view(&self, p: &ProfileRecord) -> Result<SnapshotProfile> {
        let mut view = p.view.clone();
        view.repository_id = self.repository(p)?.and_then(|r| r.repository_id);
        view.credentials_present = self.secrets(p).is_ok();
        view.status = if !view.credentials_present {
            "credentials_unavailable"
        } else if !self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?
            .authorized
            .contains(&p.view.id)
        {
            "needs_reauthorization"
        } else if view.repository_id.is_some() {
            "ready"
        } else {
            "not_initialized"
        }
        .into();
        Ok(view)
    }
    fn secrets(&self, p: &ProfileRecord) -> Result<(Secret, Option<Secret>)> {
        let repository = CredentialStore::get(
            &p.view.catalog_id,
            CredentialPurpose::Repository,
            &p.repository_credential_ref,
        )?;
        let transport = if matches!(p.destination, SnapshotDestinationInput::Local { .. }) {
            None
        } else {
            let (scope, _) = helpers::credential_context(&p.destination)?;
            Some(CredentialStore::get_bound(
                &p.view.catalog_id,
                CredentialPurpose::Transport,
                p.transport_credential_ref
                    .as_deref()
                    .ok_or_else(|| invalid("Transport credential reference is required."))?,
                Some(&scope),
            )?)
        };
        Ok((repository, transport))
    }
    fn profiles(&self) -> Result<Vec<SnapshotProfile>> {
        let rows = self.catalog.call(|db| {
            Ok(db
                .prepare(
                    "SELECT CASE WHEN length(CAST(record AS BLOB))<=262144 THEN record END FROM backup_snapshot_profiles ORDER BY rowid DESC LIMIT 33",
                )?
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })?;
        if rows.len() > 32 {
            return Err(invalid(
                "Snapshot profile count exceeds the supported bound.",
            ));
        }
        rows.into_iter()
            .map(|row| {
                let p: ProfileRecord = decode(row)?;
                self.profile_view(&self.profile(&p.view.id)?)
            })
            .collect()
    }
    fn authorized(&self, p: &ProfileRecord) -> Result<()> {
        if !self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?
            .authorized
            .contains(&p.view.id)
        {
            return Err(conflict(
                "Reauthorize this snapshot profile before reading or writing its repository.",
            ));
        }
        if let Some(local) = &p.local {
            local.open()?;
        }
        Ok(())
    }
    fn registered_scope(&self, p: &ProfileRecord) -> Result<()> {
        for binding in &p.roots {
            let registered = store::root(&self.catalog, binding.id.clone())?;
            if native::encode_path(&registered.path) != binding.binding.path
                || registered.identity.volume != binding.binding.identity.volume
                || registered.identity.file != binding.binding.identity.file
                || registered.volume != binding.binding.volume
            {
                return Err(conflict(
                    "A snapshot source root was removed, relinked or changed identity. Create a fresh explicitly authorized profile.",
                ));
            }
        }
        Ok(())
    }
    fn validate_selection(
        &self,
        library: &Library,
        selection: &NativeSnapshotSelection,
    ) -> Result<Binding> {
        id(&selection.id)?;
        let binding = Binding::capture(
            selection.path.clone(),
            selection.identity.clone(),
            selection.volume.clone(),
        )?;
        let target = DirectoryChain::open(&selection.path)?;
        let mut scopes = vec![DirectoryChain::open(&self.data.path()?)?];
        for root in store::roots(&library.catalog)? {
            match DirectoryChain::open(&root.path) {
                Ok(chain) => scopes.push(chain),
                Err(_)
                    if target
                        .identities()
                        .iter()
                        .all(|id| id.volume != root.identity.volume) => {}
                Err(_) => {
                    return Err(conflict(
                        "An inaccessible source on this volume has unknown ancestry; choose a different verified volume for recovery output.",
                    ));
                }
            }
        }
        for scope in scopes {
            let a = target.identities();
            let b = scope.identities();
            let x = a
                .last()
                .ok_or_else(|| invalid("Invalid destination ancestry."))?;
            let y = b
                .last()
                .ok_or_else(|| invalid("Invalid source ancestry."))?;
            if a.iter().any(|v| v.volume == y.volume && v.file == y.file)
                || b.iter().any(|v| v.volume == x.volume && v.file == x.file)
            {
                return Err(conflict(
                    "Snapshot output cannot overlap any library root or application data.",
                ));
            }
        }
        Ok(binding)
    }
    // Keep typed IPC fields separate from the shell-only native selection;
    // combining them into a deserializable authority object would be misleading.
    #[allow(clippy::too_many_arguments)]
    fn profile_create(
        &self,
        library: &Library,
        label: String,
        root_ids: Vec<String>,
        repository_credential_ref: String,
        transport_credential_ref: Option<String>,
        destination: SnapshotDestinationInput,
        selection: Option<NativeSnapshotSelection>,
    ) -> Result<SnapshotProfile> {
        if label.trim().is_empty() || label.len() > 80 || label.chars().any(char::is_control) {
            return Err(invalid(
                "Choose a snapshot profile name of 1–80 visible characters.",
            ));
        }
        if self.profiles()?.len() >= 32 {
            return Err(invalid(
                "The 32-profile bound has been reached; old profiles/history are retained.",
            ));
        }
        helpers::validate_destination(&destination)?;
        id(&repository_credential_ref)?;
        let roots = store::roots(&library.catalog)?;
        let root_ids = if root_ids.is_empty() {
            vec![
                roots
                    .iter()
                    .find(|r| r.view.is_base)
                    .ok_or_else(|| invalid("Choose a base library root first."))?
                    .view
                    .id
                    .clone(),
            ]
        } else {
            root_ids
        };
        if root_ids.len() > 16 || root_ids.iter().collect::<BTreeSet<_>>().len() != root_ids.len() {
            return Err(invalid("Choose 1–16 distinct authorized roots."));
        }
        let mut bindings = Vec::new();
        for root_id in &root_ids {
            id(root_id)?;
            let root = roots
                .iter()
                .find(|r| r.view.id == *root_id)
                .ok_or_else(|| invalid("A selected source root is not registered."))?;
            let binding = Binding::capture(
                root.path.clone(),
                root.identity.clone(),
                root.volume.clone(),
            )?;
            bindings.push(RootBinding {
                id: root_id.clone(),
                binding,
                label: root.view.label.clone(),
            });
        }
        bindings.sort_by(|a, b| a.id.cmp(&b.id));
        let local = match &destination {
            SnapshotDestinationInput::Local { selection_id } => {
                let selected = selection.filter(|s| s.id == *selection_id).ok_or_else(|| {
                    invalid("Select the local repository parent in the native picker.")
                })?;
                Some(self.validate_selection(library, &selected)?)
            }
            _ => None,
        };
        let namespace_id = uuid::Uuid::new_v4().to_string();
        let profile_id = uuid::Uuid::new_v4().to_string();
        let protocol = protocol(&destination).to_string();
        let destination_display = if let Some(local) = &local {
            local
                .path()?
                .join(repo_leaf(&library.catalog.summary()?.id, &namespace_id))
                .to_string_lossy()
                .into_owned()
        } else {
            remote_display(&destination, &namespace_id)?
        };
        let view = SnapshotProfile {
            id: profile_id.clone(),
            label,
            catalog_id: library.catalog.summary()?.id,
            root_ids,
            protocol,
            destination_display,
            repository_id: None,
            created_at: now(),
            host_fingerprint: host_fingerprint(&destination),
            insecure_transport: matches!(destination, SnapshotDestinationInput::Ftp { .. }),
            credentials_present: true,
            status: "not_initialized".into(),
        };
        let p = ProfileRecord {
            version: 1,
            view: view.clone(),
            destination,
            local,
            roots: bindings,
            repository_credential_ref,
            transport_credential_ref,
            namespace_id,
            supersedes: None,
        };
        self.secrets(&p)?;
        let text = encode(&p)?;
        self.catalog.call(move |db| {
            db.execute(
                "INSERT INTO backup_snapshot_profiles VALUES(?1,?2)",
                params![profile_id, text],
            )?;
            Ok(())
        })?;
        self.state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?
            .authorized
            .insert(view.id.clone());
        Ok(view)
    }
}

#[cfg(test)]
mod application_tests {
    use super::*;
    use crate::library::{
        LibraryOptions, WorkerPaths,
        model::{RootMode, ScanFilter},
    };
    struct OwnedExternalRun {
        directory: Option<tempfile::TempDir>,
        path: PathBuf,
        identity: FileIdentity,
        volume: String,
        token: String,
    }
    impl OwnedExternalRun {
        fn optional() -> Option<Self> {
            let drive = std::env::var("STILLHARBOUR_M4_GENERATED_REPOSITORY_DRIVE").ok()?;
            assert!(
                cfg!(windows)
                    && drive.len() == 2
                    && drive.as_bytes()[0].is_ascii_alphabetic()
                    && drive.as_bytes()[1] == b':',
                "The generated repository fixture accepts only a native drive letter such as E:."
            );
            let token = uuid::Uuid::new_v4().to_string();
            let directory = tempfile::Builder::new()
                .prefix(&format!("stillharbour-generated-m4-{token}-"))
                .tempdir_in(format!("{drive}\\"))
                .unwrap();
            let path = directory.path().canonicalize().unwrap();
            let root = native::open_root(&path, None).unwrap();
            let identity = filesystem::identity(&root).unwrap();
            let volume = native::volume_token(&root).unwrap();
            let mut marker = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path.join(".stillharbour-generated-m4-owned"))
                .unwrap();
            std::io::Write::write_all(&mut marker, token.as_bytes()).unwrap();
            marker.sync_all().unwrap();
            Some(Self {
                directory: Some(directory),
                path,
                identity,
                volume,
                token,
            })
        }
        fn boundary(&self) -> bool {
            (|| -> Result<bool> {
                let root = native::open_root(&self.path, Some(&self.identity))?;
                if native::volume_token(&root)? != self.volume {
                    return Ok(false);
                }
                let file = native::open_asset(
                    &self.path,
                    &self.identity,
                    &self.volume,
                    Path::new(".stillharbour-generated-m4-owned"),
                )?;
                let mut marker = String::new();
                std::io::Read::read_to_string(&mut std::io::Read::take(file, 128), &mut marker)?;
                Ok(marker == self.token
                    && self
                        .path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .is_some_and(|name| {
                            name.starts_with(&format!("stillharbour-generated-m4-{}-", self.token))
                        })
                    && self
                        .path
                        .parent()
                        .is_some_and(|parent| parent.parent().is_none()))
            })()
            .unwrap_or(false)
        }
    }
    impl Drop for OwnedExternalRun {
        fn drop(&mut self) {
            let Some(directory) = self.directory.take() else {
                return;
            };
            if !self.boundary() {
                let _ = directory.keep();
                eprintln!(
                    "Generated M4 external fixture ownership changed; no cleanup was attempted."
                );
                return;
            }
            if directory.close().is_err() {
                eprintln!(
                    "Generated M4 external fixture cleanup failed; the generated directory is retained."
                );
            }
        }
    }
    struct Fixture {
        library: Arc<Library>,
        _run: tempfile::TempDir,
        external: Option<OwnedExternalRun>,
        source: PathBuf,
        backup: PathBuf,
        output: PathBuf,
        root: String,
    }
    impl Fixture {
        fn new() -> Self {
            let run = tempfile::tempdir().unwrap();
            let data = run.path().join("app");
            let source = run.path().join("photos");
            let backup = run.path().join("backup");
            let output = run.path().join("restore");
            for p in [&data, &source, &backup, &output] {
                std::fs::create_dir(p).unwrap();
            }
            let data = data.canonicalize().unwrap();
            let source = source.canonicalize().unwrap();
            let external = OwnedExternalRun::optional();
            let backup = external
                .as_ref()
                .map(|run| run.path.clone())
                .unwrap_or_else(|| backup.canonicalize().unwrap());
            let output = output.canonicalize().unwrap();
            std::fs::create_dir(source.join("hidden-source")).unwrap();
            std::fs::write(source.join("original.txt"), b"generated preserved original").unwrap();
            std::fs::write(
                source.join("hidden-source").join("companion.xmp"),
                b"generated sidecar outside gallery filter",
            )
            .unwrap();
            let catalog = Arc::new(Catalog::open(data.join("Generated.stillharbour")).unwrap());
            let library = Library::new(
                catalog,
                LibraryOptions {
                    data_directory: data.clone(),
                    workers: WorkerPaths {
                        executable: data.join("missing-worker"),
                        exiftool: None,
                        working_directory: data.join("worker"),
                    },
                },
            )
            .unwrap();
            let file = native::open_root(&source, None).unwrap();
            let root = store::add_root(
                &library.catalog,
                source.clone(),
                filesystem::identity(&file).unwrap(),
                native::volume_token(&file).unwrap(),
                "Generated source".into(),
                RootMode::Referenced,
                ScanFilter::default(),
            )
            .unwrap();
            Self {
                _run: run,
                library,
                external,
                source,
                backup,
                output,
                root,
            }
        }
        fn selection(path: &Path) -> NativeSnapshotSelection {
            let path = path.canonicalize().unwrap();
            let file = native::open_root(&path, None).unwrap();
            NativeSnapshotSelection {
                id: uuid::Uuid::new_v4().to_string(),
                path,
                identity: filesystem::identity(&file).unwrap(),
                volume: native::volume_token(&file).unwrap(),
            }
        }
        fn profile_without_vault(&self) -> ProfileRecord {
            let binding = Self::selection(&self.source);
            ProfileRecord {
                version: 1,
                view: SnapshotProfile {
                    id: uuid::Uuid::new_v4().to_string(),
                    label: "Generated profile".into(),
                    catalog_id: self.library.catalog.summary().unwrap().id,
                    root_ids: vec![self.root.clone()],
                    protocol: "local".into(),
                    destination_display: "Generated backup".into(),
                    repository_id: None,
                    created_at: now(),
                    host_fingerprint: None,
                    insecure_transport: false,
                    credentials_present: false,
                    status: "needs_reauthorization".into(),
                },
                destination: SnapshotDestinationInput::Local {
                    selection_id: uuid::Uuid::new_v4().to_string(),
                },
                local: Some({
                    let selection = Self::selection(&self.backup);
                    Binding::capture(selection.path, selection.identity, selection.volume).unwrap()
                }),
                roots: vec![RootBinding {
                    id: self.root.clone(),
                    binding: Binding::capture(binding.path, binding.identity, binding.volume)
                        .unwrap(),
                    label: "Generated source".into(),
                }],
                repository_credential_ref: uuid::Uuid::new_v4().to_string(),
                transport_credential_ref: None,
                namespace_id: uuid::Uuid::new_v4().to_string(),
                supersedes: None,
            }
        }
    }
    #[test]
    fn source_containing_live_catalog_and_owned_workspace_is_rejected_without_omission() {
        let f = Fixture::new();
        let mut p = f.profile_without_vault();
        let selected = Fixture::selection(f._run.path());
        p.roots[0].binding =
            Binding::capture(selected.path, selected.identity, selected.volume).unwrap();
        let error = f
            .library
            .snapshots
            .inventory(&p, &AtomicBool::new(false))
            .err()
            .unwrap();
        assert!(error.message.contains("active catalog"));
        assert_eq!(
            std::fs::read(f.source.join("original.txt")).unwrap(),
            b"generated preserved original"
        );
        assert!(f.library.catalog.native_path().is_file());
    }
    #[test]
    fn whole_root_inventory_ignores_gallery_filter_and_detects_exact_version_changes() {
        let f = Fixture::new();
        let p = f.profile_without_vault();
        let first = f
            .library
            .snapshots
            .inventory(&p, &AtomicBool::new(false))
            .unwrap();
        assert_eq!(first.files.len(), 2);
        assert!(
            first
                .files
                .iter()
                .any(|file| file.display.ends_with("companion.xmp"))
        );
        assert!(
            first
                .files
                .iter()
                .any(|file| file.display.ends_with("original.txt"))
        );
        let old = first.reviewed_digest().unwrap();
        std::fs::write(
            f.source.join("original.txt"),
            b"generated changed exact version",
        )
        .unwrap();
        assert_ne!(
            old,
            f.library
                .snapshots
                .inventory(&p, &AtomicBool::new(false))
                .unwrap()
                .reviewed_digest()
                .unwrap()
        );
        let mut foreign = first;
        foreign.catalog_id = uuid::Uuid::new_v4().to_string();
        assert!(validate_manifest(&foreign, &p).is_err());
    }
    #[test]
    fn relink_revokes_saved_profile_review_and_one_use_approval_before_helpers() {
        let f = Fixture::new();
        let p = f.profile_without_vault();
        let key = p.view.id.clone();
        let json = encode(&p).unwrap();
        f.library
            .catalog
            .call(move |db| {
                db.execute(
                    "INSERT INTO backup_snapshot_profiles VALUES(?1,?2)",
                    params![key, json],
                )?;
                Ok(())
            })
            .unwrap();
        f.library
            .snapshots
            .state
            .lock()
            .unwrap()
            .authorized
            .insert(p.view.id.clone());
        let review = f.library.snapshots.review(p.clone()).unwrap();
        f.library
            .snapshots
            .state
            .lock()
            .unwrap()
            .reviews
            .get_mut(&review.id)
            .unwrap()
            .view
            .execution_available = true;
        let approval = f
            .library
            .snapshots
            .approve(&review.id, &review.digest, "backup")
            .unwrap();
        let replacement = f._run.path().join("relinked-generated");
        std::fs::create_dir(&replacement).unwrap();
        let native = native::open_root(&replacement, None).unwrap();
        store::relink(
            &f.library.catalog,
            f.root.clone(),
            replacement,
            filesystem::identity(&native).unwrap(),
            native::volume_token(&native).unwrap(),
        )
        .unwrap();
        assert!(f.library.snapshots.review(p.clone()).is_err());
        f.library.snapshots.cancel_root(&f.root).unwrap();
        assert!(
            f.library
                .snapshots
                .take_approval(&review.id, &approval.id, "backup")
                .is_err()
        );
        assert!(
            f.library
                .snapshots
                .approve(&review.id, &review.digest, "backup")
                .is_err()
        );
        assert!(f.source.join("original.txt").exists());
        assert!(f.library.snapshots.state.lock().unwrap().active.is_none());
    }
    #[test]
    fn prospective_restore_case_directory_and_unicode_conflicts_fail_before_output() {
        let f = Fixture::new();
        let p = f.profile_without_vault();
        let original = f
            .library
            .snapshots
            .inventory(&p, &AtomicBool::new(false))
            .unwrap();
        let mut aliases = original.clone();
        aliases.files[0].relative = native::encode_path(Path::new("Album"));
        aliases.files[1].relative = native::encode_path(Path::new("album/child.txt"));
        assert!(validate_manifest(&aliases, &p).is_err());
        let mut unicode = original;
        unicode.files[0].relative = native::encode_path(Path::new("é.txt"));
        unicode.files[1].relative = native::encode_path(Path::new("e\u{301}.TXT"));
        assert!(validate_manifest(&unicode, &p).is_err());
        assert_eq!(std::fs::read_dir(&f.output).unwrap().count(), 0);
    }
    #[test]
    fn restored_catalog_checks_frozen_schema_without_rewriting_older_file() {
        let f = Fixture::new();
        let path = f.output.join("Known-historical-header.stillharbour");
        f.library.catalog.snapshot(path.clone()).unwrap();
        let catalog = f.library.catalog.summary().unwrap().id;
        let older = crate::catalog::SCHEMA_VERSION - 1;
        let db = Connection::open(&path).unwrap();
        db.pragma_update(None, "user_version", older).unwrap();
        drop(db);
        let bytes = std::fs::read(&path).unwrap();
        verify_catalog(&path, &catalog, older).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(verify_catalog(&path, &catalog, crate::catalog::SCHEMA_VERSION).is_err());
        let db = Connection::open(&path).unwrap();
        let future = crate::catalog::SCHEMA_VERSION + 1;
        db.pragma_update(None, "user_version", future).unwrap();
        drop(db);
        assert!(verify_catalog(&path, &catalog, future).is_err());
    }
    #[test]
    fn append_only_evidence_and_imported_schedule_cannot_authorize_execution() {
        let f = Fixture::new();
        let p = f.profile_without_vault();
        let key = p.view.id.clone();
        let json = encode(&p).unwrap();
        f.library
            .catalog
            .call(move |db| {
                db.execute(
                    "INSERT INTO backup_snapshot_profiles VALUES(?1,?2)",
                    params![key, json],
                )?;
                Ok(())
            })
            .unwrap();
        assert!(
            !f.library
                .snapshots
                .profile_view(&p)
                .unwrap()
                .credentials_present
        );
        assert!(f.library.snapshots.review(p.clone()).is_err());
        let profile = p.view.id.clone();
        assert!(
            f.library
                .catalog
                .call(move |db| {
                    db.execute("DELETE FROM backup_snapshot_profiles WHERE id=?", [profile])?;
                    Ok(())
                })
                .is_err()
        );
        let record = ScheduleRecord {
            view: SnapshotSchedule {
                profile_id: p.view.id.clone(),
                interval_minutes: 15,
                timezone: "UTC".into(),
                dst_policy: "elapsed_interval".into(),
                enabled: true,
                catch_up: true,
                next_due: now(),
                last_run: None,
                fully_quit_supported: false,
                approved_digest: "a".repeat(64),
            },
            profile_digest: digest(&p).unwrap(),
            manifest_digest: "a".repeat(64),
            instance: "f".repeat(64),
        };
        f.library.snapshots.persist_schedule(&record).unwrap();
        assert!(
            !f.library
                .snapshots
                .schedule_get(&p.view.id)
                .unwrap()
                .unwrap()
                .enabled
        );
        assert!(
            !f.library
                .snapshots
                .schedule_disable(&p.view.id)
                .unwrap()
                .unwrap()
                .enabled
        );
    }
    #[test]
    fn malformed_restore_mapping_and_overlapping_native_output_fail_closed() {
        let f = Fixture::new();
        let p = f.profile_without_vault();
        let mut m = f
            .library
            .snapshots
            .inventory(&p, &AtomicBool::new(false))
            .unwrap();
        m.files[0].relative = native::encode_path(Path::new("../escape"));
        assert!(validate_manifest(&m, &p).is_err());
        assert!(
            f.library
                .snapshots
                .validate_selection(&f.library, &Fixture::selection(&f.source))
                .is_err()
        );
        assert!(
            f.library
                .snapshots
                .validate_selection(
                    &f.library,
                    &Fixture::selection(f.library.snapshots.data.path().unwrap().as_path())
                )
                .is_err()
        );
    }
    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    #[ignore = "Native generated OS consent revocation integration: invoke explicitly"]
    fn native_schedule_disable_revokes_even_a_replayed_prior_enabled_catalog_row() {
        let f = Fixture::new();
        let profile = f.profile_without_vault();
        let key = profile.view.id.clone();
        let text = encode(&profile).unwrap();
        f.library
            .catalog
            .call(move |db| {
                db.execute(
                    "INSERT INTO backup_snapshot_profiles VALUES(?1,?2)",
                    params![key, text],
                )?;
                Ok(())
            })
            .unwrap();
        let instance = f.library.snapshots.instance().unwrap();
        let record = ScheduleRecord {
            view: SnapshotSchedule {
                profile_id: profile.view.id.clone(),
                interval_minutes: 10080,
                timezone: "UTC".into(),
                dst_policy: "elapsed_interval".into(),
                enabled: true,
                catch_up: true,
                next_due: now(),
                last_run: None,
                fully_quit_supported: false,
                approved_digest: "a".repeat(64),
            },
            profile_digest: digest(&profile).unwrap(),
            manifest_digest: "a".repeat(64),
            instance: instance.clone(),
        };
        let policy = SnapshotWorkflow::schedule_policy(&record).unwrap();
        let catalog = f.library.catalog.summary().unwrap().id;
        struct Revoke {
            catalog: String,
            instance: String,
            profile: String,
        }
        impl Drop for Revoke {
            fn drop(&mut self) {
                let _ = crate::backup::credentials::schedule_revoke(
                    &self.catalog,
                    &self.instance,
                    &self.profile,
                );
            }
        }
        let _revoke = Revoke {
            catalog: catalog.clone(),
            instance: instance.clone(),
            profile: profile.view.id.clone(),
        };
        crate::backup::credentials::schedule_permit(
            &catalog,
            &instance,
            &profile.view.id,
            &policy,
            true,
        )
        .unwrap();
        f.library.snapshots.persist_schedule(&record).unwrap();
        assert!(
            f.library
                .snapshots
                .schedule_get(&profile.view.id)
                .unwrap()
                .unwrap()
                .enabled
        );
        f.library
            .snapshots
            .schedule_disable(&profile.view.id)
            .unwrap();
        assert!(
            !crate::backup::credentials::schedule_permit(
                &catalog,
                &instance,
                &profile.view.id,
                &policy,
                false
            )
            .unwrap()
        );
        f.library.snapshots.persist_schedule(&record).unwrap();
        assert!(
            !f.library
                .snapshots
                .schedule_get(&profile.view.id)
                .unwrap()
                .unwrap()
                .enabled
        );
        f.library
            .snapshots
            .schedule_disable(&profile.view.id)
            .unwrap();
        assert!(f.library.snapshots.jobs(None).unwrap().is_empty());
    }
    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    #[ignore = "Native OS credential + pinned helper integration: invoke explicitly on generated fixture"]
    fn native_local_roundtrip_full_bytes_catalog_history_wrong_key_and_late_occupant() {
        let f = Fixture::new();
        let platform = if cfg!(windows) {
            "windows-x86_64"
        } else {
            "macos-aarch64"
        };
        let helpers = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/backup-helpers")
            .join(platform)
            .canonicalize()
            .unwrap();
        f.library.configure_snapshot_helpers(helpers).unwrap();
        let catalog = f.library.catalog.summary().unwrap().id;
        let reference = CredentialStore::store(
            &catalog,
            CredentialPurpose::Repository,
            "generated-disposable-repository-secret",
        )
        .unwrap();
        struct Cleanup {
            catalog: String,
            reference: String,
        }
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = CredentialStore::delete_owned(
                    &self.catalog,
                    CredentialPurpose::Repository,
                    &self.reference,
                );
            }
        }
        let _credential = Cleanup {
            catalog: catalog.clone(),
            reference: reference.clone(),
        };
        if f.external.is_some() {
            assert!(
                crate::backup::failure_domains::independent_local(
                    &native::open_root(&f.source, None).unwrap(),
                    &native::open_root(&f.backup, None).unwrap()
                )
                .unwrap(),
                "The explicit generated repository drive is not certified as a separate physical device."
            );
        }
        let selection = Fixture::selection(&f.backup);
        let profile = f
            .library
            .snapshots
            .profile_create(
                &f.library,
                "Generated actual snapshot".into(),
                vec![f.root.clone()],
                reference,
                None,
                SnapshotDestinationInput::Local {
                    selection_id: selection.id.clone(),
                },
                Some(selection),
            )
            .unwrap();
        let review = f
            .library
            .snapshots
            .review(f.library.snapshots.profile(&profile.id).unwrap())
            .unwrap();
        assert_eq!(review.files, 2);
        let approval = f
            .library
            .snapshots
            .approve(&review.id, &review.digest, "backup")
            .unwrap();
        let frozen = f
            .library
            .snapshots
            .take_approval(&review.id, &approval.id, "backup")
            .unwrap();
        let job = f
            .library
            .snapshots
            .start(f.library.clone(), frozen, "backup")
            .unwrap();
        let first_backup_job_id = job.id.clone();
        let done = wait(&f.library, &job.id);
        assert_eq!(done.state, "completed_verified", "{}", done.message);
        assert!(
            done.included
                && done.structural_checked
                && done.data_read_verified
                && done.restored_compared
        );
        assert!(!done.independent_recovery_verified);
        assert_eq!(
            std::fs::read(f.source.join("original.txt")).unwrap(),
            b"generated preserved original"
        );
        let p = f.library.snapshots.profile(&profile.id).unwrap();
        let records = f.library.snapshots.records(&p).unwrap();
        assert_eq!(records.len(), 1);
        let key = records[0].id.clone();
        let first_snapshot_key = key.clone();
        assert!(f.library.snapshots.record(&p, &"f".repeat(64)).is_err());
        let wrong = CredentialStore::store(
            &catalog,
            CredentialPurpose::Repository,
            "generated-wrong-secret",
        )
        .unwrap();
        let _wrong = Cleanup {
            catalog: catalog.clone(),
            reference: wrong.clone(),
        };
        assert!(
            f.library
                .snapshot_profile_rebind_credential(
                    &profile.id,
                    CredentialPurpose::Repository,
                    &wrong,
                    Some(Fixture::selection(&f.backup)),
                    None
                )
                .is_err()
        );
        let review = f
            .library
            .snapshots
            .restore_review(&f.library, p, &key, Fixture::selection(&f.output))
            .unwrap();
        let leaf = format!("stillharbour-restored-{}", review.id);
        std::fs::create_dir(f.output.join(&leaf)).unwrap();
        std::fs::write(
            f.output.join(&leaf).join("late-occupant.txt"),
            b"generated occupant retained",
        )
        .unwrap();
        let approval = f
            .library
            .snapshots
            .approve(&review.id, &review.digest, "restore")
            .unwrap();
        let frozen = f
            .library
            .snapshots
            .take_approval(&review.id, &approval.id, "restore")
            .unwrap();
        let job = f
            .library
            .snapshots
            .start(f.library.clone(), frozen, "restore")
            .unwrap();
        let done = wait(&f.library, &job.id);
        assert_eq!(done.state, "needs_recovery");
        assert_eq!(
            std::fs::read(f.output.join(&leaf).join("late-occupant.txt")).unwrap(),
            b"generated occupant retained"
        );
        let review = f
            .library
            .snapshots
            .restore_review(
                &f.library,
                f.library.snapshots.profile(&profile.id).unwrap(),
                &key,
                Fixture::selection(&f.output),
            )
            .unwrap();
        let leaf = format!("stillharbour-restored-{}", review.id);
        let approval = f
            .library
            .snapshots
            .approve(&review.id, &review.digest, "restore")
            .unwrap();
        let frozen = f
            .library
            .snapshots
            .take_approval(&review.id, &approval.id, "restore")
            .unwrap();
        let job = f
            .library
            .snapshots
            .start(f.library.clone(), frozen, "restore")
            .unwrap();
        let done = wait(&f.library, &job.id);
        assert_eq!(done.state, "completed_verified", "{}", done.message);
        assert_eq!(
            std::fs::read(
                f.output
                    .join(&leaf)
                    .join("roots")
                    .join(&f.root)
                    .join("original.txt")
            )
            .unwrap(),
            b"generated preserved original"
        );
        assert_eq!(
            std::fs::read(
                f.output
                    .join(&leaf)
                    .join("roots")
                    .join(&f.root)
                    .join("hidden-source")
                    .join("companion.xmp")
            )
            .unwrap(),
            b"generated sidecar outside gallery filter"
        );
        verify_catalog(
            &f.output.join(leaf).join("Catalog.snapshot.stillharbour"),
            &catalog,
            crate::catalog::SCHEMA_VERSION,
        )
        .unwrap();
        // Explicit backend comparison admission protects a unique manually
        // edited OLD destination version absent from the normal source root.
        let old = f._run.path().join("old-destination");
        std::fs::create_dir(&old).unwrap();
        let old = old.canonicalize().unwrap();
        std::fs::create_dir(old.join("dataset")).unwrap();
        std::fs::write(
            old.join("dataset/manual.txt"),
            b"unique generated manually edited OLD destination",
        )
        .unwrap();
        std::fs::write(
            old.join("dataset/manual.xmp"),
            b"unique generated OLD companion",
        )
        .unwrap();
        let root = native::open_root(&old, None).unwrap();
        let root_identity = filesystem::identity(&root).unwrap();
        let root_volume = native::volume_token(&root).unwrap();
        let mut files = Vec::new();
        for relative in ["dataset/manual.txt", "dataset/manual.xmp"] {
            let mut handle =
                native::open_asset(&old, &root_identity, &root_volume, Path::new(relative))
                    .unwrap();
            files.push(crate::backup::sync::RecoveryFile {
                catalog_id: catalog.clone(),
                root_id: f.root.clone(),
                native_root: native::encode_path(&old),
                root_identity: root_identity.clone(),
                root_volume: root_volume.clone(),
                relative_path: native::encode_path(Path::new(relative)),
                identity: filesystem::identity(&handle).unwrap(),
                revision: native::revision(&handle).unwrap(),
                bytes: handle.metadata().unwrap().len(),
                full_digest: bridge::digest(&mut handle, &AtomicBool::new(false)).unwrap(),
                essential_metadata_digest: None,
            });
        }
        let review = f
            .library
            .snapshots
            .review_recovery_backup(&f.library, &profile.id, files.clone())
            .unwrap();
        assert!(
            review
                .source_display_paths
                .iter()
                .any(|p| p.contains("old-destination"))
        );
        let approval = f
            .library
            .snapshots
            .approve(&review.id, &review.digest, "backup")
            .unwrap();
        let frozen = f
            .library
            .snapshots
            .take_approval(&review.id, &approval.id, "backup")
            .unwrap();
        let job = f
            .library
            .snapshots
            .start(f.library.clone(), frozen, "backup")
            .unwrap();
        let done = wait(&f.library, &job.id);
        assert_eq!(done.state, "completed_verified", "{}", done.message);
        let key = done.snapshot_id.unwrap();
        assert!(f.library.snapshots.verify_recovery(&files, &key).is_err());
        let review = f
            .library
            .snapshots
            .restore_review(
                &f.library,
                f.library.snapshots.profile(&profile.id).unwrap(),
                &key,
                Fixture::selection(&f.output),
            )
            .unwrap();
        let leaf = format!("stillharbour-restored-{}", review.id);
        let approval = f
            .library
            .snapshots
            .approve(&review.id, &review.digest, "restore")
            .unwrap();
        let frozen = f
            .library
            .snapshots
            .take_approval(&review.id, &approval.id, "restore")
            .unwrap();
        let job = f
            .library
            .snapshots
            .start(f.library.clone(), frozen, "restore")
            .unwrap();
        let done = wait(&f.library, &job.id);
        assert_eq!(done.state, "completed_verified", "{}", done.message);
        assert_eq!(
            std::fs::read(
                f.output
                    .join(leaf)
                    .join("roots")
                    .join(&f.root)
                    .join("dataset/manual.txt")
            )
            .unwrap(),
            b"unique generated manually edited OLD destination"
        );
        assert_eq!(
            std::fs::read(old.join("dataset/manual.txt")).unwrap(),
            b"unique generated manually edited OLD destination"
        );
        assert_eq!(
            std::fs::read(f.source.join("original.txt")).unwrap(),
            b"generated preserved original"
        );
        // Fault fixture: the concrete upload exists but its durable job row was
        // left running. Recreate all workflow session state, reauthorize the
        // native repository, then perform only existing-snapshot inspection.
        let interrupted_id = first_backup_job_id.clone();
        let text = f
            .library
            .catalog
            .call(move |db| {
                Ok(db.query_row(
                    "SELECT record FROM backup_snapshot_jobs WHERE id=?",
                    [interrupted_id],
                    |r| r.get::<_, String>(0),
                )?)
            })
            .unwrap();
        let mut interrupted: JobRecord = decode(text).unwrap();
        interrupted.view.state = "running".into();
        interrupted.view.finished_at = None;
        let text = encode(&interrupted).unwrap();
        let interrupted_id = first_backup_job_id.clone();
        f.library
            .catalog
            .call(move |db| {
                db.execute(
                    "UPDATE backup_snapshot_jobs SET record=?1 WHERE id=?2",
                    params![text, interrupted_id],
                )?;
                Ok(())
            })
            .unwrap();
        let cold = Arc::new(
            SnapshotWorkflow::new(
                f.library.catalog.clone(),
                f.library.snapshots.data.path().unwrap(),
            )
            .unwrap(),
        );
        cold.configure_helpers(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../vendor/backup-helpers")
                .join(platform)
                .canonicalize()
                .unwrap(),
        )
        .unwrap();
        assert!(cold.state.lock().unwrap().authorized.is_empty());
        let selection = Fixture::selection(&f.backup);
        cold.dispatch(
            f.library.clone(),
            SnapshotRequest::ProfileOpen {
                profile_id: profile.id.clone(),
                selection_id: Some(selection.id.clone()),
            },
            Some(selection),
        )
        .unwrap();
        let before = cold
            .records(&cold.profile(&profile.id).unwrap())
            .unwrap()
            .len();
        let inspected = cold
            .reconcile(f.library.clone(), &profile.id, &first_backup_job_id, None)
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(240);
        let inspected = loop {
            let current = cold
                .jobs(None)
                .unwrap()
                .into_iter()
                .find(|j| j.id == inspected.id)
                .unwrap();
            if current.state == "completed_verified" && cold.idle().unwrap() {
                break current;
            }
            assert!(
                matches!(
                    current.state.as_str(),
                    "queued" | "running" | "completed_verified"
                ),
                "{}",
                current.message
            );
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(25));
        };
        assert_eq!(inspected.action, "reconcile");
        assert!(inspected.restored_compared && inspected.data_read_verified);
        assert_eq!(
            cold.records(&cold.profile(&profile.id).unwrap())
                .unwrap()
                .len(),
            before
        );
        drop(cold);
        // Entire original root is now absent. Missing old credentials and an
        // empty native session must still permit explicit key re-enrollment and
        // historical restore, while a new backup remains denied.
        f.library.snapshots.cancel_root(&f.root).unwrap();
        CredentialStore::delete_owned(
            &catalog,
            CredentialPurpose::Repository,
            &_credential.reference,
        )
        .unwrap();
        let renamed = f.source.with_file_name("offline-generated-original-root");
        std::fs::rename(&f.source, &renamed).unwrap();
        let replacement = CredentialStore::store(
            &catalog,
            CredentialPurpose::Repository,
            "generated-disposable-repository-secret",
        )
        .unwrap();
        let _replacement = Cleanup {
            catalog: catalog.clone(),
            reference: replacement.clone(),
        };
        let target = f
            .library
            .snapshot_credential_target(&profile.id, CredentialPurpose::Repository)
            .unwrap();
        assert!(target.needs_local_selection);
        let recovered = f
            .library
            .snapshot_profile_rebind_credential(
                &profile.id,
                CredentialPurpose::Repository,
                &replacement,
                Some(Fixture::selection(&f.backup)),
                None,
            )
            .unwrap();
        assert!(
            f.library
                .snapshots
                .review(f.library.snapshots.profile(&recovered.id).unwrap())
                .is_err()
        );
        assert!(
            f.library
                .snapshots
                .restore_review(
                    &f.library,
                    f.library.snapshots.profile(&recovered.id).unwrap(),
                    &first_snapshot_key,
                    Fixture::selection(&renamed)
                )
                .is_err()
        );
        let review = f
            .library
            .snapshots
            .restore_review(
                &f.library,
                f.library.snapshots.profile(&recovered.id).unwrap(),
                &first_snapshot_key,
                Fixture::selection(&f.output),
            )
            .unwrap();
        let leaf = format!("stillharbour-restored-{}", review.id);
        let approval = f
            .library
            .snapshots
            .approve(&review.id, &review.digest, "restore")
            .unwrap();
        let frozen = f
            .library
            .snapshots
            .take_approval(&review.id, &approval.id, "restore")
            .unwrap();
        let job = f
            .library
            .snapshots
            .start(f.library.clone(), frozen, "restore")
            .unwrap();
        let done = wait(&f.library, &job.id);
        assert_eq!(done.state, "completed_verified", "{}", done.message);
        assert!(!f.source.exists());
        assert_eq!(
            std::fs::read(renamed.join("original.txt")).unwrap(),
            b"generated preserved original"
        );
        assert_eq!(
            std::fs::read(
                f.output
                    .join(leaf)
                    .join("roots")
                    .join(&f.root)
                    .join("original.txt")
            )
            .unwrap(),
            b"generated preserved original"
        );
    }
    #[cfg(any(windows, target_os = "macos"))]
    fn wait(library: &Library, id: &str) -> SnapshotJob {
        let deadline = Instant::now() + Duration::from_secs(240);
        loop {
            let job = library
                .snapshots
                .jobs(None)
                .unwrap()
                .into_iter()
                .find(|j| j.id == id)
                .unwrap();
            if !matches!(
                job.state.as_str(),
                "queued" | "running" | "cancel_requested"
            ) && library
                .snapshots
                .state
                .lock()
                .unwrap()
                .active
                .as_ref()
                .is_none_or(|active| active.id != id)
            {
                return job;
            }
            assert!(
                Instant::now() < deadline,
                "Native generated snapshot job timed out: {} {}",
                job.state,
                job.phase
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

fn protocol(destination: &SnapshotDestinationInput) -> &'static str {
    match destination {
        SnapshotDestinationInput::Local { .. } => "local",
        SnapshotDestinationInput::Sftp { .. } => "sftp",
        SnapshotDestinationInput::Ftps { .. } => "ftps",
        SnapshotDestinationInput::Ftp { .. } => "ftp",
        SnapshotDestinationInput::Smb { .. } => "smb",
    }
}
fn repo_leaf(catalog: &str, namespace: &str) -> String {
    format!("stillharbour-backup-{catalog}-{namespace}")
}
fn remote_path(destination: &SnapshotDestinationInput, namespace: &str) -> Result<String> {
    let path = match destination {
        SnapshotDestinationInput::Sftp { remote_path, .. }
        | SnapshotDestinationInput::Ftps { remote_path, .. }
        | SnapshotDestinationInput::Ftp { remote_path, .. }
        | SnapshotDestinationInput::Smb { remote_path, .. } => remote_path,
        _ => return Err(invalid("Not a remote destination.")),
    };
    let leaf = format!("stillharbour-backup-{namespace}");
    let repository = if path.is_empty() {
        leaf
    } else {
        format!("{}/{leaf}", path.trim_end_matches('/'))
    };
    Ok(match destination {
        SnapshotDestinationInput::Smb { share, .. } => format!("{share}/{repository}"),
        _ => repository,
    })
}
fn remote_display(destination: &SnapshotDestinationInput, namespace: &str) -> Result<String> {
    let (scope, label) = helpers::credential_context(destination)?;
    let _ = scope;
    Ok(format!("{label}/{}", remote_path(destination, namespace)?))
}
fn host_fingerprint(destination: &SnapshotDestinationInput) -> Option<String> {
    if let SnapshotDestinationInput::Sftp { host_key, .. } = destination {
        Some(format!(
            "Reviewed key BLAKE3: {}",
            blake3::hash(host_key.as_bytes()).to_hex()
        ))
    } else {
        None
    }
}

impl Library {
    pub fn snapshot_credential_referenced(
        &self,
        purpose: CredentialPurpose,
        reference: &str,
    ) -> Result<bool> {
        id(reference)?;
        let catalog = self.catalog.summary()?.id;
        let rows=self.catalog.call(|db|Ok(db.prepare("SELECT CASE WHEN length(CAST(record AS BLOB))<=262144 THEN record END FROM backup_snapshot_profiles LIMIT 33")?.query_map([],|row|row.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?))?;
        if rows.len() > 32 {
            return Err(invalid(
                "Stored profile count exceeds its bound; credential cleanup was not authorized.",
            ));
        }
        for row in rows {
            let profile: ProfileRecord = decode(row)?;
            if profile.view.catalog_id != catalog {
                return Err(conflict(
                    "Stored profile catalog binding is invalid; no credential was removed.",
                ));
            }
            if match purpose {
                CredentialPurpose::Repository => profile.repository_credential_ref == reference,
                CredentialPurpose::Transport => {
                    profile.transport_credential_ref.as_deref() == Some(reference)
                }
            } {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub fn start_snapshot_schedules(self: &Arc<Self>) -> Result<()> {
        self.snapshots.tick(self.clone())
    }
    pub fn configure_snapshot_helpers(&self, path: PathBuf) -> Result<()> {
        self.snapshots.configure_helpers(path)
    }
    pub fn backup_snapshots(
        self: &Arc<Self>,
        request: SnapshotRequest,
        selection: Option<NativeSnapshotSelection>,
    ) -> Result<SnapshotReply> {
        self.snapshots.dispatch(self.clone(), request, selection)
    }
    pub fn snapshot_credential_target(
        &self,
        profile_id: &str,
        purpose: CredentialPurpose,
    ) -> Result<NativeSnapshotCredentialTarget> {
        let p = self.snapshots.profile(profile_id)?;
        let other = match purpose {
            CredentialPurpose::Repository => CredentialPurpose::Transport,
            CredentialPurpose::Transport => CredentialPurpose::Repository,
        };
        let additional_purpose = if (other == CredentialPurpose::Transport
            && matches!(p.destination, SnapshotDestinationInput::Local { .. }))
            || self.snapshots.secret_present(&p, other)
        {
            None
        } else {
            Some(other)
        };
        let (reference, context_digest, label) = match purpose {
            CredentialPurpose::Repository => (
                p.repository_credential_ref.clone(),
                None,
                format!("Repository key for {}", p.view.label),
            ),
            CredentialPurpose::Transport => {
                let (context, label) = helpers::credential_context(&p.destination)?;
                (
                    p.transport_credential_ref
                        .clone()
                        .ok_or_else(|| invalid("This profile has no transport credential."))?,
                    Some(context),
                    label,
                )
            }
        };
        Ok(NativeSnapshotCredentialTarget {
            needs_local_selection: p.local.is_some(),
            additional_purpose,
            catalog_id: p.view.catalog_id,
            purpose,
            reference,
            context_digest,
            label,
        })
    }
    pub fn snapshot_key_export_source(
        &self,
        profile_id: &str,
        selection: NativeSnapshotSelection,
    ) -> Result<SnapshotKeyExportSource> {
        let p = self.snapshots.profile(profile_id)?;
        self.snapshots.validate_selection(self, &selection)?;
        let chosen = DirectoryChain::open(&selection.path)?;
        for view in self.snapshots.profiles()? {
            let profile = self.snapshots.profile(&view.id)?;
            if let Some(repo) = self.snapshots.repository(&profile)?.and_then(|r| r.local) {
                let chain = DirectoryChain::open(&repo.path()?)?;
                ensure_disjoint(&chosen, &chain)?;
            }
        }
        let secret = CredentialStore::get(
            &p.view.catalog_id,
            CredentialPurpose::Repository,
            &p.repository_credential_ref,
        )?;
        Ok(SnapshotKeyExportSource {
            secret,
            selection,
            label: p.view.label,
            journal_directory: self.snapshots.data.path()?,
        })
    }
    pub fn snapshot_profile_rebind_credential(
        &self,
        profile_id: &str,
        purpose: CredentialPurpose,
        new_reference: &str,
        selection: Option<NativeSnapshotSelection>,
        other_new_reference: Option<&str>,
    ) -> Result<SnapshotProfile> {
        id(new_reference)?;
        let mut p = self.snapshots.profile(profile_id)?;
        if let Some(parent) = &p.local {
            let selected=selection.ok_or_else(||invalid("Select this profile's exact repository parent using the native picker before credential recovery."))?;
            id(&selected.id)?;
            let validated = Binding::capture(selected.path, selected.identity, selected.volume)?;
            if validated.path != parent.path
                || validated.identity.volume != parent.identity.volume
                || validated.identity.file != parent.identity.file
                || validated.volume != parent.volume
            {
                return Err(conflict(
                    "The chosen native parent is not the immutable repository parent.",
                ));
            }
        } else if selection.is_some() {
            return Err(invalid(
                "A remote profile cannot adopt a local repository selection.",
            ));
        }
        if self.snapshots.profiles()?.len() >= 32 {
            return Err(invalid(
                "The immutable profile bound has been reached; no profile/history was removed.",
            ));
        }
        match purpose {
            CredentialPurpose::Repository => p.repository_credential_ref = new_reference.into(),
            CredentialPurpose::Transport => p.transport_credential_ref = Some(new_reference.into()),
        }
        if let Some(other) = other_new_reference {
            id(other)?;
            match purpose {
                CredentialPurpose::Repository => p.transport_credential_ref = Some(other.into()),
                CredentialPurpose::Transport => p.repository_credential_ref = other.into(),
            }
        }
        self.snapshots.secrets(&p)?;
        // A new native secret must unlock the already bound concrete repository;
        // it cannot initialize or silently retarget a repository during rebind.
        let binding = self.snapshots.repository(&p)?.ok_or_else(|| {
            conflict("Open and validate the existing repository before changing credentials.")
        })?;
        if binding.repository_id.is_none() {
            return Err(conflict(
                "A concrete previously bound repository ID is required for credential recovery.",
            ));
        }
        self.snapshots
            .read_repository_id(&p, &binding, None, &AtomicBool::new(false))?;
        let old = p.view.id.clone();
        self.snapshots.schedule_disable(&old)?;
        p.view.id = uuid::Uuid::new_v4().to_string();
        p.supersedes = Some(old);
        p.view.created_at = now();
        let text = encode(&p)?;
        let key = p.view.id.clone();
        self.catalog.call(move |db| {
            db.execute(
                "INSERT INTO backup_snapshot_profiles VALUES(?1,?2)",
                params![key, text],
            )?;
            Ok(())
        })?;
        self.snapshots
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?
            .authorized
            .insert(p.view.id.clone());
        self.snapshots.profile_view(&p)
    }
}
fn ensure_disjoint(a: &DirectoryChain, b: &DirectoryChain) -> Result<()> {
    let x = a
        .identities()
        .last()
        .ok_or_else(|| invalid("Missing native directory identity."))?;
    let y = b
        .identities()
        .last()
        .ok_or_else(|| invalid("Missing native directory identity."))?;
    if a.identities()
        .iter()
        .any(|v| v.volume == y.volume && v.file == y.file)
        || b.identities()
            .iter()
            .any(|v| v.volume == x.volume && v.file == x.file)
    {
        return Err(conflict(
            "The selected output overlaps a source, repository or application directory.",
        ));
    }
    Ok(())
}
impl SnapshotWorkflow {
    fn inventory(&self, p: &ProfileRecord, cancel: &AtomicBool) -> Result<Manifest> {
        self.source_namespace_guard(p)?;
        let mut manifest = Manifest {
            private_scope: false,
            version: 1,
            catalog_id: p.view.catalog_id.clone(),
            catalog_revision: store::revision(&self.catalog)?,
            catalog_schema: self.catalog.summary()?.schema,
            platform: std::env::consts::OS.into(),
            profile_digest: digest(p)?,
            repository_id: self.repository(p)?.and_then(|r| r.repository_id),
            roots: Vec::new(),
            files: Vec::new(),
            catalog_digest: None,
            catalog_bytes: 0,
        };
        let mut names = InventoryBudget::default();
        for root in &p.roots {
            if root
                .binding
                .path()?
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.to_ascii_lowercase().ends_with(".photoslibrary"))
            {
                return Err(unavailable(
                    "Protected photo-library bundles require a dedicated consistent snapshot adapter.",
                ));
            }
            let handle = root.binding.open()?;
            let mut directories = Vec::new();
            inventory_directory(
                root,
                &handle,
                Path::new(""),
                0,
                &mut manifest.files,
                &mut directories,
                &mut names,
                cancel,
            )?;
            root.binding.open()?;
            manifest.roots.push(RootVersion {
                root: root.clone(),
                directories,
            });
        }
        manifest.files.sort_by(|a, b| a.archive.cmp(&b.archive));
        if store::revision(&self.catalog)? != manifest.catalog_revision {
            return Err(conflict(
                "The catalog changed while preparing the snapshot review.",
            ));
        }
        Ok(manifest)
    }
    fn source_namespace_guard(&self, p: &ProfileRecord) -> Result<()> {
        let data = DirectoryChain::open(&self.data.path()?)?;
        let catalog_parent = self
            .catalog
            .native_path()
            .parent()
            .ok_or_else(|| invalid("The active catalog has no native parent."))?;
        let catalog = DirectoryChain::open(catalog_parent)?;
        for source in &p.roots {
            let object = &source.binding.identity;
            if data
                .identities()
                .iter()
                .chain(catalog.identities())
                .any(|ancestor| ancestor.volume == object.volume && ancestor.file == object.file)
            {
                return Err(unavailable(
                    "This source contains the active catalog or the application's cache/journal workspace. Select a source folder outside that namespace; the catalog is protected separately through its consistent snapshot. No live catalog or unknown sidecar was silently omitted.",
                ));
            }
        }
        Ok(())
    }
    fn review(&self, p: ProfileRecord) -> Result<SnapshotReview> {
        self.authorized(&p)?;
        self.registered_scope(&p)?;
        let manifest = self.inventory(&p, &AtomicBool::new(false))?;
        let cap = self.capabilities();
        let view = SnapshotReview {
            items: review_items(&manifest),
            id: uuid::Uuid::new_v4().to_string(),
            profile_id: p.view.id.clone(),
            action: "backup".into(),
            digest: manifest.reviewed_digest()?,
            files: manifest.files.len() as u32,
            bytes: manifest.bytes().to_string(),
            root_ids: p.view.root_ids.clone(),
            catalog_included: true,
            source_display_paths: p
                .roots
                .iter()
                .map(|r| r.binding.path().map(|p| p.display().to_string()))
                .collect::<Result<Vec<_>>>()?,
            source_retained: true,
            target_display: p.view.destination_display.clone(),
            issues: review_issues(&manifest),
            execution_available: cap.backup_available,
            unavailable_reason: cap.unavailable_reason,
        };
        self.remember_review(Review {
            private_scope: false,
            view: view.clone(),
            profile: p,
            manifest,
            restore: None,
            record: None,
            created: Instant::now(),
        })?;
        Ok(view)
    }
    fn remember_review(&self, review: Review) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?;
        state
            .reviews
            .retain(|_, r| r.created.elapsed() < REVIEW_AGE);
        if state.reviews.len() >= MAX_REVIEWS {
            return Err(error(ErrorCode::Busy, "Too many pending snapshot reviews."));
        }
        state.reviews.insert(review.view.id.clone(), review);
        Ok(())
    }
    fn approve(&self, review_id: &str, value: &str, action: &str) -> Result<SnapshotApproval> {
        id(review_id)?;
        let state = self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?;
        let review = state
            .reviews
            .get(review_id)
            .cloned()
            .ok_or_else(|| conflict("Create a fresh snapshot review."))?;
        drop(state);
        self.authorized(&review.profile)?;
        if action == "backup" {
            self.registered_scope(&self.profile(&review.profile.view.id)?)?;
        }
        if review.created.elapsed() >= REVIEW_AGE
            || review.view.digest != value
            || review.view.action != action
            || !review.view.execution_available
        {
            return Err(conflict(
                "The snapshot review is stale, mismatched or unavailable.",
            ));
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?;
        if !state.authorized.contains(&review.profile.view.id)
            || !state.reviews.contains_key(review_id)
        {
            return Err(conflict("The snapshot root authorization was revoked."));
        }
        if state.approvals.len() >= MAX_REVIEWS {
            return Err(error(ErrorCode::Busy, "Too many pending approvals."));
        }
        let result = SnapshotApproval {
            id: uuid::Uuid::new_v4().to_string(),
            review_id: review_id.into(),
            digest: value.into(),
        };
        state.approvals.insert(
            result.id.clone(),
            Approval {
                review: review_id.into(),
                digest: value.into(),
                action: action.into(),
                created: Instant::now(),
            },
        );
        Ok(result)
    }
    fn take_approval(&self, review_id: &str, approval_id: &str, action: &str) -> Result<Review> {
        id(review_id)?;
        id(approval_id)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?;
        let approval = state
            .approvals
            .remove(approval_id)
            .ok_or_else(|| conflict("A fresh explicit snapshot approval is required."))?;
        let review = state
            .reviews
            .get(review_id)
            .ok_or_else(|| conflict("The snapshot review expired."))?;
        if approval.review != review_id
            || approval.digest != review.view.digest
            || approval.action != action
            || approval.created.elapsed() >= REVIEW_AGE
            || review.created.elapsed() >= REVIEW_AGE
        {
            return Err(conflict(
                "Snapshot approval does not match the current immutable review.",
            ));
        }
        Ok(review.clone())
    }
    fn records(&self, p: &ProfileRecord) -> Result<Vec<SnapshotRecord>> {
        let ids = self.lineage(p)?;
        let rows=self.catalog.call(move |db|{let placeholders=std::iter::repeat_n("?",ids.len()).collect::<Vec<_>>().join(",");let sql=format!("SELECT CASE WHEN length(CAST(record AS BLOB))<=33554432 AND length(json_extract(record,'$.view'))<=8192 THEN json_extract(record,'$.view') END, CASE WHEN length(CAST(record AS BLOB))<=33554432 THEN json_extract(record,'$.profile_namespace') END FROM backup_snapshot_records WHERE profile_id IN ({placeholders}) ORDER BY rowid DESC LIMIT 1001");Ok(db.prepare(&sql)?.query_map(rusqlite::params_from_iter(ids),|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?)})?;
        if rows.len() > 1000 {
            return Err(invalid(
                "Snapshot history exceeds the 1,000-record bound; no records were removed.",
            ));
        }
        rows.into_iter()
            .map(|(view, namespace)| {
                let mut r: SnapshotRecord = decode(view)?;
                if namespace != p.namespace_id {
                    return Err(conflict("Snapshot history namespace mismatch."));
                }
                r.profile_id = p.view.id.clone();
                Ok(r)
            })
            .collect()
    }
    fn record(&self, p: &ProfileRecord, key: &str) -> Result<Record> {
        snapshot_id(key)?;
        let ids = self.lineage(p)?;
        let key = key.to_string();
        let text=self.catalog.call(move|db|Ok(db.query_row("SELECT CASE WHEN length(CAST(record AS BLOB))<=33554432 THEN record END FROM backup_snapshot_records WHERE id=?",[key],|r|r.get::<_,String>(0)).optional()?))?.ok_or_else(||invalid("Select a snapshot recorded for this authorized profile."))?;
        let mut record: Record = decode(text)?;
        if !ids.contains(&record.view.profile_id) || record.profile_namespace != p.namespace_id {
            return Err(conflict(
                "Snapshot record belongs to another immutable profile lineage.",
            ));
        }
        record.view.profile_id = p.view.id.clone();
        Ok(record)
    }
    fn jobs(&self, profile: Option<&str>) -> Result<Vec<SnapshotJob>> {
        let ids = profile
            .map(|s| self.profile(s).and_then(|p| self.lineage(&p)))
            .transpose()?;
        let rows = self.catalog.call(move |db| {
            Ok(db
                .prepare("SELECT CASE WHEN length(CAST(record AS BLOB))<=33554432 AND length(json_extract(record,'$.view'))<=8192 THEN json_extract(record,'$.view') END FROM backup_snapshot_jobs ORDER BY rowid DESC LIMIT 1001")?
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })?;
        if rows.len() > 1000 {
            return Err(invalid("Snapshot job history exceeds its supported bound."));
        }
        let state = self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?;
        let active = state.active.as_ref().map(|a| a.id.clone());
        let terminal_failures = state.terminal_failures.clone();
        drop(state);
        rows.into_iter().map(decode::<SnapshotJob>).filter_map(|record| {
            let mut record=match record {Ok(record)=>record,Err(error)=>return Some(Err(error))};
            if ids.as_ref().is_some_and(|ids|!ids.contains(&record.profile_id)) {return None;}
            if let Some(failure)=terminal_failures.get(&record.id) {record=failure.clone();}
            if matches!(record.state.as_str(),"queued"|"running"|"cancel_requested") && active.as_deref()!=Some(&record.id) {
                record.state="needs_recovery".into();
                record.message="The prior process ended without a durable completion. Sources and partial outputs are retained; reconcile the concrete repository before retry.".into();
            }
            if let Some(profile)=profile {record.profile_id=profile.into();}
            Some(Ok(record))
        }).collect()
    }
    fn dispatch(
        self: &Arc<Self>,
        library: Arc<Library>,
        request: SnapshotRequest,
        selection: Option<NativeSnapshotSelection>,
    ) -> Result<SnapshotReply> {
        use SnapshotRequest::*;
        match request {
            Capabilities => Ok(SnapshotReply::Capabilities(self.capabilities())),
            Profiles => Ok(SnapshotReply::Profiles(self.profiles()?)),
            ProfileCreate {
                label,
                root_ids,
                repository_credential_ref,
                transport_credential_ref,
                destination,
            } => Ok(SnapshotReply::Profile(self.profile_create(
                &library,
                label,
                root_ids,
                repository_credential_ref,
                transport_credential_ref,
                destination,
                selection,
            )?)),
            ProfileOpen {
                profile_id,
                selection_id,
            } => {
                let p = self.profile(&profile_id)?;
                if let Some(local) = &p.local {
                    let selected = selection.ok_or_else(|| {
                        invalid(
                            "Select the existing repository parent again using the native picker.",
                        )
                    })?;
                    if selection_id.as_deref() != Some(&selected.id) {
                        return Err(invalid("The native selection expired."));
                    }
                    id(&selected.id)?;
                    let binding =
                        Binding::capture(selected.path, selected.identity, selected.volume)?;
                    if binding.path != local.path
                        || binding.identity.volume != local.identity.volume
                        || binding.identity.file != local.identity.file
                        || binding.volume != local.volume
                    {
                        return Err(conflict(
                            "The selected parent is not this profile's repository parent.",
                        ));
                    }
                }
                self.secrets(&p)?;
                if let Some(binding) = self.repository(&p)? {
                    self.read_repository_id(&p, &binding, None, &AtomicBool::new(false))?;
                }
                self.state
                    .lock()
                    .map_err(|_| unavailable("Snapshot state is unavailable."))?
                    .authorized
                    .insert(profile_id);
                Ok(SnapshotReply::Profile(self.profile_view(&p)?))
            }
            BackupReview { profile_id } => Ok(SnapshotReply::Review(
                self.review(self.profile(&profile_id)?)?,
            )),
            BackupApprove { review_id, digest } => Ok(SnapshotReply::Approval(
                self.approve(&review_id, &digest, "backup")?,
            )),
            RestoreApprove { review_id, digest } => Ok(SnapshotReply::Approval(
                self.approve(&review_id, &digest, "restore")?,
            )),
            BackupStart {
                review_id,
                approval_id,
            } => {
                let review = self.take_approval(&review_id, &approval_id, "backup")?;
                Ok(SnapshotReply::Job(self.start(library, review, "backup")?))
            }
            RestoreStart {
                review_id,
                approval_id,
            } => {
                let review = self.take_approval(&review_id, &approval_id, "restore")?;
                Ok(SnapshotReply::Job(self.start(library, review, "restore")?))
            }
            Jobs { profile_id } => Ok(SnapshotReply::Jobs(self.jobs(profile_id.as_deref())?)),
            Job { job_id } => {
                id(&job_id)?;
                Ok(SnapshotReply::Job(
                    self.jobs(None)?
                        .into_iter()
                        .find(|j| j.id == job_id)
                        .ok_or_else(|| invalid("Unknown snapshot job."))?,
                ))
            }
            Cancel { job_id } => {
                id(&job_id)?;
                let state = self
                    .state
                    .lock()
                    .map_err(|_| unavailable("Snapshot state is unavailable."))?;
                let active = state
                    .active
                    .as_ref()
                    .filter(|a| a.id == job_id)
                    .ok_or_else(|| conflict("This snapshot job is no longer active."))?;
                active.cancel.store(true, Ordering::Release);
                drop(state);
                let mut job = self
                    .jobs(None)?
                    .into_iter()
                    .find(|j| j.id == job_id)
                    .ok_or_else(|| invalid("Unknown snapshot job."))?;
                job.state = "cancel_requested".into();
                Ok(SnapshotReply::Job(job))
            }
            Reconcile {
                profile_id,
                job_id,
                snapshot_id,
            } => Ok(SnapshotReply::Job(self.reconcile(
                library,
                &profile_id,
                &job_id,
                snapshot_id,
            )?)),
            Snapshots { profile_id } => {
                let p = self.profile(&profile_id)?;
                Ok(SnapshotReply::Snapshots(self.records(&p)?))
            }
            Browse {
                profile_id,
                snapshot_id,
                offset,
            } => {
                let p = self.profile(&profile_id)?;
                let record = self.record(&p, &snapshot_id)?;
                let total = record.manifest.files.len() as u32;
                if offset > total {
                    return Err(invalid("Invalid snapshot page offset."));
                }
                let entries = record
                    .manifest
                    .files
                    .iter()
                    .skip(offset as usize)
                    .take(200)
                    .map(|f| SnapshotEntry {
                        id: f.id.clone(),
                        display_path: f.display.clone(),
                        kind: "file".into(),
                        bytes: f.bytes.to_string(),
                        digest: Some(f.digest.clone()),
                    })
                    .collect::<Vec<_>>();
                let end = offset + entries.len() as u32;
                Ok(SnapshotReply::Browse(SnapshotBrowse {
                    profile_id,
                    snapshot_id,
                    entries,
                    next_offset: (end < total).then_some(end),
                    total,
                }))
            }
            RestoreReview {
                profile_id,
                snapshot_id,
                selection_id,
            } => {
                let p = self.profile(&profile_id)?;
                let selected = selection.ok_or_else(|| {
                    invalid("Choose a new protected restore output using the native picker.")
                })?;
                if selected.id != selection_id {
                    return Err(invalid("The native restore selection expired."));
                }
                Ok(SnapshotReply::Review(self.restore_review(
                    &library,
                    p,
                    &snapshot_id,
                    selected,
                )?))
            }
            Verify {
                profile_id,
                snapshot_id,
            } => {
                let p = self.profile(&profile_id)?;
                self.authorized(&p)?;
                let record = self.record(&p, &snapshot_id)?;
                let view = SnapshotReview {
                    items: review_items(&record.manifest),
                    id: uuid::Uuid::new_v4().to_string(),
                    profile_id: p.view.id.clone(),
                    action: "verify".into(),
                    digest: record.manifest.reviewed_digest()?,
                    files: record.view.files,
                    bytes: record.view.bytes.clone(),
                    root_ids: p.view.root_ids.clone(),
                    catalog_included: true,
                    source_display_paths: Vec::new(),
                    source_retained: true,
                    target_display: "New app-owned verification output".into(),
                    issues: review_issues(&record.manifest),
                    execution_available: self.capabilities().restore_available,
                    unavailable_reason: None,
                };
                let review = Review {
                    private_scope: false,
                    view,
                    profile: p,
                    manifest: record.manifest.clone(),
                    restore: None,
                    record: Some(record),
                    created: Instant::now(),
                };
                Ok(SnapshotReply::Job(self.start(library, review, "verify")?))
            }
            ScheduleGet { profile_id } => {
                Ok(SnapshotReply::Schedule(self.schedule_get(&profile_id)?))
            }
            ScheduleDisable { profile_id } => {
                Ok(SnapshotReply::Schedule(self.schedule_disable(&profile_id)?))
            }
            ScheduleSave {
                profile_id,
                review_id,
                digest,
                interval_minutes,
                timezone,
                enabled,
                catch_up,
            } => Ok(SnapshotReply::Schedule(Some(self.schedule_save(
                &profile_id,
                &review_id,
                &digest,
                interval_minutes,
                &timezone,
                enabled,
                catch_up,
            )?))),
        }
    }
}
// The recursive walker carries independent bounded accumulators and a held
// parent handle. Its arguments are private and never supplied by IPC.
#[derive(Default)]
struct InventoryBudget {
    names: usize,
    entries: usize,
}
#[allow(clippy::too_many_arguments)]
fn inventory_directory(
    root: &RootBinding,
    parent: &File,
    relative: &Path,
    depth: usize,
    files: &mut Vec<FileVersion>,
    directories: &mut Vec<(Vec<u8>, String)>,
    names: &mut InventoryBudget,
    cancel: &AtomicBool,
) -> Result<()> {
    if depth > 64 {
        return Err(invalid("The snapshot root exceeds 64 directory levels."));
    }
    let revision = native::revision(parent)?;
    let mut entries = Vec::new();
    each(parent, |entry| {
        names.entries = names
            .entries
            .checked_add(1)
            .ok_or_else(|| invalid("Snapshot entry count exceeds its bound."))?;
        if names.entries > MAX_FILES {
            return Err(invalid("The snapshot scope exceeds 10,000 entries."));
        }
        entries.push(entry);
        Ok(())
    })?;
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    for entry in entries {
        if cancel.load(Ordering::Acquire) {
            return Err(error(ErrorCode::Busy, "Snapshot review canceled."));
        }
        names.names = names
            .names
            .checked_add(entry.name.as_encoded_bytes().len())
            .ok_or_else(|| invalid("Snapshot paths exceed their bound."))?;
        if names.names > 16 * 1024 * 1024 {
            return Err(invalid("Snapshot paths exceed their 16 MiB bound."));
        }
        let display=entry.name.to_str().ok_or_else(||unavailable("This snapshot contains a name that cannot be restored portably. Originals remain untouched."))?;
        if !crate::organization::portable_filename(display)
            || display.to_ascii_lowercase().ends_with(".photoslibrary")
        {
            return Err(unavailable(
                "A protected bundle or unsupported portable filename requires a dedicated snapshot adapter.",
            ));
        }
        if entry.blocked {
            return Err(unavailable(
                "A link, placeholder or special file prevents a complete snapshot. No partial scope is approved.",
            ));
        }
        let path = relative.join(&entry.name);
        let mut handle = child(parent, &entry.name, entry.directory)?;
        if entry.directory {
            inventory_directory(
                root,
                &handle,
                &path,
                depth + 1,
                files,
                directories,
                names,
                cancel,
            )?;
        } else {
            if files.len() >= MAX_FILES {
                return Err(invalid("The snapshot contains more than 10,000 files."));
            }
            let identity = filesystem::identity(&handle)?;
            if identity.links != 1 {
                return Err(unavailable(
                    "Multiply linked files require a dedicated snapshot policy.",
                ));
            }
            let revision = native::revision(&handle)?;
            let bytes = handle.metadata()?.len();
            let full = bridge::digest(&mut handle, cancel)?;
            let metadata = metadata_for(&handle)?;
            let current = child(parent, &entry.name, false)?;
            if filesystem::identity(&current)? != identity
                || native::revision(&current)? != revision
            {
                return Err(conflict(
                    "A source changed native membership during snapshot review.",
                ));
            }
            let key = blake3::hash(&native::encode_path(&path))
                .to_hex()
                .to_string();
            files.push(FileVersion {
                metadata,
                id: format!("{}:{key}", root.id),
                root_id: root.id.clone(),
                relative: native::encode_path(&path),
                identity,
                revision,
                bytes,
                digest: full,
                archive: format!("roots/{}/{key}.bin", root.id),
                display: format!("{}/{}", root.label, path.display()),
            });
        }
    }
    if native::revision(parent)? != revision {
        return Err(conflict(
            "A directory changed while preparing the whole-root snapshot.",
        ));
    }
    directories.push((native::encode_path(relative), revision));
    Ok(())
}

struct Context {
    helpers: PinnedHelpers,
    work: Root,
    args: Vec<OsString>,
    env: BTreeMap<OsString, OsString>,
}
impl Context {
    fn run(&self, arguments: &[&str], cancel: &AtomicBool) -> Result<Vec<u8>> {
        let mut args = self.args.clone();
        args.extend(arguments.iter().map(OsString::from));
        let result = self.helpers.run(
            Helper::Restic,
            &args,
            &self.env,
            &self.work.path,
            cancel,
            TIMEOUT,
        )?;
        if result.code != 0 {
            return Err(error(
                ErrorCode::WorkerFailed,
                "The backup helper did not complete successfully. Sources and partial outputs are retained; inspect the job before retry.",
            ));
        }
        Ok(result.stdout)
    }
}
impl SnapshotWorkflow {
    fn workspace(&self, job: &str) -> Result<Root> {
        self.data.open()?;
        let name = format!("snapshot-work-{}", uuid::Uuid::new_v4());
        self.event(job, "workspace_intent", serde_json::json!({"leaf":name}))?;
        let work = bridge::make_directory(&self.data.root()?, &name)?;
        self.event(job,"workspace_created",serde_json::json!({"path":native::encode_path(&work.path),"identity":work.identity,"volume":work.volume}))?;
        Ok(work)
    }
    fn context(
        &self,
        p: &ProfileRecord,
        binding: &RepositoryBinding,
        work: Option<&Root>,
        cancel: &AtomicBool,
    ) -> Result<Context> {
        let helpers = self.helpers()?;
        let work = match work {
            Some(root) => root.clone(),
            None => self.workspace(&uuid::Uuid::new_v4().to_string())?,
        };
        work.source()?;
        let (repository, transport) = self.secrets(p)?;
        let mut env = BTreeMap::new();
        for name in ["TMP", "TEMP", "TMPDIR"] {
            env.insert(name.into(), work.path.clone().into_os_string());
        }
        env.insert("RESTIC_PASSWORD".into(), repository.as_str()?.into());
        env.insert("RESTIC_READ_CONCURRENCY".into(), "1".into());
        env.insert("GOMAXPROCS".into(), "2".into());
        let repo = if let Some(local) = &binding.local {
            local.open()?;
            let expected = p
                .local
                .as_ref()
                .ok_or_else(|| invalid("A remote profile cannot adopt a local repository."))?
                .path()?
                .join(repo_leaf(&p.view.catalog_id, &p.namespace_id));
            if local.path()? != expected {
                return Err(conflict(
                    "Stored repository binding changed its immutable namespace.",
                ));
            }
            local.path()?.into_os_string()
        } else {
            if matches!(p.destination, SnapshotDestinationInput::Local { .. }) {
                return Err(conflict("The local repository binding is missing."));
            }
            let token = uuid::Uuid::new_v4().simple().to_string();
            let config_leaf = format!("rclone-{token}.conf");
            let config_path = work.path.join(&config_leaf);
            let mut config = String::from("[profile]\n");
            match &p.destination {
                SnapshotDestinationInput::Sftp{host,port,user,host_key,..}=>{let trust_leaf=format!("known-hosts-{token}");let trusted_host=if *port==22{host.clone()}else{format!("[{host}]:{port}")};bridge::write_new(&work,&trust_leaf,format!("{trusted_host} {host_key}\n").as_bytes())?;config.push_str(&format!("type = sftp\nhost = {host}\nport = {port}\nuser = {user}\nknown_hosts_file = {}\nshell_type = none\ndisable_hashcheck = true\n",work.path.join(trust_leaf).to_str().ok_or_else(||invalid("Unsupported trust-resource native path."))?));},
                SnapshotDestinationInput::Ftps{host,port,user,ca_certificate_pem,..}=>{config.push_str(&format!("type = ftp\nhost = {host}\nport = {port}\nuser = {user}\ntls = false\nexplicit_tls = true\nno_check_certificate = false\nconcurrency = 3\n"));if let Some(pem)=ca_certificate_pem {let leaf=format!("ftps-ca-{token}.pem");bridge::write_new(&work,&leaf,pem.as_bytes())?;env.insert("RCLONE_CA_CERT".into(),work.path.join(leaf).into_os_string());}},
                SnapshotDestinationInput::Ftp{host,port,user,insecure_acknowledged:true,..}=>config.push_str(&format!("type = ftp\nhost = {host}\nport = {port}\nuser = {user}\ntls = false\nexplicit_tls = false\nconcurrency = 3\n")),
                SnapshotDestinationInput::Smb{host,port,user,domain,..}=>config.push_str(&format!("type = smb\nhost = {host}\nport = {port}\nuser = {user}\ndomain = {domain}\n")),
                _=>return Err(invalid("Invalid reviewed transport configuration.")),
            }
            bridge::write_new(&work, &config_leaf, config.as_bytes())?;
            let transport =
                transport.ok_or_else(|| invalid("The transport credential is unavailable."))?;
            let obscured = helpers.run_input(
                Helper::Rclone,
                &["obscure".into(), "-".into()],
                &BTreeMap::new(),
                &work.path,
                cancel,
                Duration::from_secs(30),
                Some(transport.as_str()?.as_bytes()),
            )?;
            if obscured.code != 0 || obscured.stdout.len() > 4096 {
                return Err(unavailable(
                    "The private transport credential channel failed.",
                ));
            }
            let obscured = std::str::from_utf8(&obscured.stdout)
                .map_err(|_| unavailable("The private transport credential channel failed."))?
                .trim_end();
            if obscured.is_empty() || obscured.contains(['\r', '\n', '\0']) {
                return Err(unavailable(
                    "The private transport credential channel failed.",
                ));
            }
            env.insert("RCLONE_CONFIG".into(), config_path.into_os_string());
            env.insert("RCLONE_CONFIG_PROFILE_PASS".into(), obscured.into());
            env.insert("RCLONE_TRANSFERS".into(), "1".into());
            env.insert("RCLONE_CHECKERS".into(), "1".into());
            env.insert("RCLONE_BUFFER_SIZE".into(), "64k".into());
            env.insert("RCLONE_MULTI_THREAD_STREAMS".into(), "0".into());
            env.insert("RCLONE_BWLIMIT".into(), "10M".into());
            format!(
                "rclone:profile:{}",
                remote_path(&p.destination, &p.namespace_id)?
            )
            .into()
        };
        let mut args = vec![
            "--json".into(),
            "--repo".into(),
            repo,
            "--no-cache".into(),
            "--limit-upload".into(),
            "10240".into(),
            "--limit-download".into(),
            "10240".into(),
        ];
        if binding.local.is_none() {
            args.extend(["-o".into(), "rclone.connections=1".into()]);
            args.extend(["-o".into(),format!("rclone.program={}",helpers.rclone_program()?).into(),"-o".into(),"rclone.args=serve restic --stdio --transfers=1 --checkers=1 --buffer-size=64k --multi-thread-streams=0".into()]);
        } else {
            args.extend(["-o".into(), "local.connections=1".into()]);
        }
        Ok(Context {
            helpers,
            work,
            args,
            env,
        })
    }
    fn read_repository_id(
        &self,
        p: &ProfileRecord,
        binding: &RepositoryBinding,
        work: Option<&Root>,
        cancel: &AtomicBool,
    ) -> Result<String> {
        let context = self.context(p, binding, work, cancel)?;
        let bytes = context.run(&["cat", "config"], cancel)?;
        let value: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("The repository configuration is not valid structured output."))?;
        let actual = value
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| invalid("The repository configuration lacks an identifier."))?;
        snapshot_id(actual)?;
        if binding
            .repository_id
            .as_ref()
            .is_some_and(|id| id != actual)
        {
            return Err(conflict(
                "The repository identifier changed. No write or restore is authorized.",
            ));
        }
        Ok(actual.into())
    }
    fn restore_review(
        &self,
        library: &Library,
        p: ProfileRecord,
        key: &str,
        selection: NativeSnapshotSelection,
    ) -> Result<SnapshotReview> {
        self.authorized(&p)?;
        let record = self.record(&p, key)?;
        let target = self.validate_restore_selection(library, &p, &record, &selection)?;
        validate_manifest(&record.manifest, &p)?;
        let binding = self.repository(&p)?.ok_or_else(|| {
            conflict("This repository has not been initialized by the application.")
        })?;
        self.read_repository_id(&p, &binding, None, &AtomicBool::new(false))?;
        let cap = self.capabilities();
        let review_id = uuid::Uuid::new_v4().to_string();
        let leaf = format!("stillharbour-restored-{review_id}");
        let scope_digest = digest(&(record.manifest.reviewed_digest()?, &target, leaf.as_str()))?;
        let mut view=SnapshotReview{items:review_items(&record.manifest),id:review_id,profile_id:p.view.id.clone(),action:"restore".into(),digest:scope_digest,files:record.view.files,bytes:record.view.bytes.clone(),root_ids:record.view.root_ids.clone(),catalog_included:true,source_display_paths:Vec::new(),source_retained:true,target_display:target.path()?.join(leaf).display().to_string(),issues:vec![FIDELITY.into(),"The catalog is restored as a separate consistent file; roots and credentials require native reauthorization when it is opened.".into()],execution_available:cap.restore_available,unavailable_reason:cap.unavailable_reason};
        view.issues.extend(review_issues(&record.manifest));
        self.remember_review(Review {
            private_scope: false,
            view: view.clone(),
            profile: p,
            manifest: record.manifest.clone(),
            restore: Some(target),
            record: Some(record),
            created: Instant::now(),
        })?;
        Ok(view)
    }
}
fn validate_manifest(m: &Manifest, p: &ProfileRecord) -> Result<()> {
    use unicode_normalization::UnicodeNormalization;
    if m.version != 1
        || m.platform != std::env::consts::OS
        || !(1..=crate::catalog::SCHEMA_VERSION).contains(&m.catalog_schema)
        || m.catalog_id != p.view.catalog_id
        || m.files.len() > MAX_FILES
        || m.roots.len() > 16
    {
        return Err(invalid(
            "The snapshot manifest is incompatible or exceeds its scope bound.",
        ));
    }
    if !m.private_scope
        && (m.roots.len() != p.roots.len()
            || m.roots.iter().any(|r| {
                !p.roots.iter().any(|allowed| {
                    allowed.id == r.root.id
                        && encode(&allowed.binding).ok() == encode(&r.root.binding).ok()
                })
            }))
    {
        return Err(conflict(
            "A normal snapshot manifest changed its immutable registered source scope.",
        ));
    }
    let mut archives = BTreeSet::new();
    let roots = m
        .roots
        .iter()
        .map(|root| root.root.id.clone())
        .collect::<BTreeSet<_>>();
    if roots.len() != m.roots.len() {
        return Err(invalid("Snapshot native root identifiers repeat."));
    }
    for root in &roots {
        id(root)?;
    }
    let mut names = BTreeSet::new();
    let mut portable_names = BTreeSet::new();
    for file in &m.files {
        if !roots.contains(&file.root_id) {
            return Err(invalid(
                "A snapshot file has no recorded native dataset root.",
            ));
        }
        let relative = native::decode_path(&file.relative)?;
        let portable = relative
            .to_str()
            .ok_or_else(|| invalid("The restore filename cannot be represented on this platform."))?
            .nfc()
            .flat_map(char::to_lowercase)
            .collect::<String>();
        if !portable_names.insert((file.root_id.clone(), portable)) || file.identity.links != 1 {
            return Err(invalid(
                "Snapshot restore paths collide after portable normalization, or contain multiply linked files.",
            ));
        }
        if relative.components().next().is_none()
            || relative
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            || relative.components().any(|c| {
                c.as_os_str()
                    .to_str()
                    .is_none_or(|s| !crate::organization::portable_filename(s))
            })
        {
            return Err(invalid(
                "The snapshot contains an unsafe or unrepresentable restore path.",
            ));
        }
        if !archives.insert(file.archive.clone())
            || !names.insert((file.root_id.clone(), file.relative.clone()))
            || file.digest.len() != 64
            || !file.digest.bytes().all(|b| b.is_ascii_hexdigit())
            || !file
                .archive
                .starts_with(&format!("roots/{}/", file.root_id))
            || file.archive.contains("..")
            || file.archive.contains('\\')
        {
            return Err(invalid(
                "The snapshot manifest has duplicate or unsafe archive paths.",
            ));
        }
    }
    let mut mapped = m
        .files
        .iter()
        .map(|file| mapped_target(&file.root_id, &file.relative))
        .collect::<Result<Vec<_>>>()?;
    mapped.extend([
        "Catalog.snapshot.stillharbour".into(),
        "manifest.json".into(),
    ]);
    bridge::screen_targets(&mapped, &mapped_directories(m)?)?;
    Ok(())
}
fn mapped_target(root: &str, relative: &[u8]) -> Result<String> {
    let path = native::decode_path(relative)?;
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(invalid("Unsafe original native restore path."));
    }
    let parts = path
        .components()
        .map(|part| {
            part.as_os_str()
                .to_str()
                .ok_or_else(|| invalid("Unsupported native restore path."))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(if parts.is_empty() {
        format!("roots/{root}")
    } else {
        format!("roots/{root}/{}", parts.join("/"))
    })
}
fn mapped_directories(manifest: &Manifest) -> Result<Vec<String>> {
    let mut directories = BTreeSet::new();
    for root in &manifest.roots {
        directories.insert(format!("roots/{}", root.root.id));
        for (relative, _) in &root.directories {
            directories.insert(mapped_target(&root.root.id, relative)?);
        }
    }
    Ok(directories.into_iter().collect())
}
fn review_items(manifest: &Manifest) -> Vec<SnapshotReviewItem> {
    manifest
        .files
        .iter()
        .map(|file| SnapshotReviewItem {
            display_path: file.display.clone(),
            bytes: file.bytes.to_string(),
            full_digest: file.digest.clone(),
        })
        .collect()
}
fn review_issues(manifest: &Manifest) -> Vec<String> {
    let mut issues = vec![FIDELITY.into()];
    let descriptions = manifest
        .files
        .iter()
        .filter_map(|file| file.metadata.as_ref().map(MetadataManifest::description))
        .collect::<BTreeSet<_>>();
    issues.extend(descriptions.into_iter().map(str::to_string));
    let unsupported = manifest
        .files
        .iter()
        .filter(|file| file.metadata.is_none())
        .collect::<Vec<_>>();
    for file in unsupported.iter().take(16) {
        issues.push(format!("Native metadata is unsupported for {}: this version receives ordinary-byte evidence only.",file.display));
    }
    if unsupported.len() > 16 {
        issues.push(format!("{} additional files have unsupported native metadata; no strict recovery permission is available for them.",unsupported.len()-16));
    }
    issues
}
fn metadata_for(file: &File) -> Result<Option<MetadataManifest>> {
    match MetadataManifest::inspect(file) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.code == ErrorCode::Unsupported => Ok(None),
        Err(error) => Err(error),
    }
}

impl SnapshotWorkflow {
    fn save_job(&self, job: &JobRecord) -> Result<()> {
        let key = job.view.id.clone();
        let profile = job.view.profile_id.clone();
        let text = encode(job)?;
        self.catalog.call(move|db|{db.execute("INSERT INTO backup_snapshot_jobs(id,profile_id,record) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET record=excluded.record",params![key,profile,text])?;Ok(())})
    }
    fn phase(&self, job: &mut JobRecord, phase: &str) -> Result<()> {
        job.view.phase = phase.into();
        self.event(&job.view.id,phase,serde_json::json!({"snapshot_id":job.view.snapshot_id,"included":job.view.included,"structural_checked":job.view.structural_checked,"data_read_verified":job.view.data_read_verified,"restored_compared":job.view.restored_compared}))?;
        self.save_job(job)
    }
    fn start(
        self: &Arc<Self>,
        library: Arc<Library>,
        review: Review,
        action: &str,
    ) -> Result<SnapshotJob> {
        self.require_execution()?;
        self.authorized(&review.profile)?;
        if action == "backup" {
            self.registered_scope(&self.profile(&review.profile.view.id)?)?;
        }
        validate_manifest(&review.manifest, &review.profile)?;
        let mut scopes = Vec::new();
        if action == "backup" {
            if review.private_scope {
                for root in self.profile(&review.profile.view.id)?.roots {
                    root.binding.open()?;
                    scopes.push(Scope::new(
                        DirectoryChain::open(&root.binding.path()?)?,
                        Access::Read,
                    ));
                }
            }
            for root in &review.profile.roots {
                root.binding.open()?;
                scopes.push(Scope::new(
                    DirectoryChain::open(&root.binding.path()?)?,
                    Access::Read,
                ));
            }
            if self
                .refresh_manifest(
                    &review.profile,
                    &review.manifest,
                    review.private_scope,
                    &AtomicBool::new(false),
                )?
                .reviewed_digest()?
                != review.view.digest
            {
                return Err(conflict(
                    "The source scope changed after review; prepare and approve a fresh snapshot.",
                ));
            }
        }
        if let Some(local) = &review.profile.local {
            local.open()?;
            scopes.push(Scope::new(
                DirectoryChain::open(&local.path()?)?,
                Access::Write,
            ));
        }
        if let Some(output) = &review.restore {
            output.open()?;
            scopes.push(Scope::new(
                DirectoryChain::open(&output.path()?)?,
                Access::Write,
            ));
        }
        if scopes.is_empty() {
            scopes.push(Scope::new(
                DirectoryChain::open(&self.data.path()?)?,
                Access::Read,
            ));
        }
        let lease = Registry::shared().try_acquire(scopes)?;
        let job_id = uuid::Uuid::new_v4().to_string();
        let cancel = Arc::new(AtomicBool::new(false));
        let mut state = self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?;
        if state.active.is_some() {
            return Err(error(
                ErrorCode::Busy,
                "A snapshot/restore/verification job is already running.",
            ));
        }
        let view = SnapshotJob {
            id: job_id.clone(),
            profile_id: review.profile.view.id.clone(),
            action: action.into(),
            state: "queued".into(),
            phase: "review_validated".into(),
            files: review.manifest.files.len() as u32,
            bytes: review.manifest.bytes().to_string(),
            snapshot_id: review.record.as_ref().map(|r| r.view.id.clone()),
            message: "Exact reviewed source scope; originals are retained.".into(),
            started_at: now(),
            finished_at: None,
            included: false,
            structural_checked: false,
            data_read_verified: false,
            restored_compared: false,
            source_retained: true,
            independent_recovery_verified: false,
        };
        let mut job = JobRecord {
            verified_output: None,
            metadata_verified: false,
            private_scope: review.private_scope,
            review_id: review.view.id.clone(),
            view: view.clone(),
            profile: review.profile.clone(),
            manifest: review.manifest.clone(),
            work: None,
            restore: review.restore.clone(),
            subfolder: review.record.as_ref().map(|r| r.subfolder.clone()),
        };
        self.save_job(&job)?;
        self.event(&job_id,"approved_plan",serde_json::json!({"review_id":review.view.id,"digest":review.view.digest,"action":action,"manifest":review.manifest}))?;
        state.active = Some(Active {
            id: job_id.clone(),
            roots: review.profile.view.root_ids.clone(),
            cancel: cancel.clone(),
        });
        drop(state);
        let workflow = self.clone();
        let spawned=std::thread::Builder::new().name("snapshot-workflow".into()).spawn(move||{
            let _lease=lease;let _library=library;
            let outcome=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||workflow.execute(&mut job,&review,&cancel)));
            #[cfg(test)]
            if let Ok(Err(error))=&outcome {eprintln!("Generated snapshot fixture phase {}: {:?}",job.view.phase,error);}
            match outcome {Ok(Ok(()))=>{},Ok(Err(error))=>{job.view.state=if cancel.load(Ordering::Acquire){"canceled_with_partial_copies"}else{"needs_recovery"}.into();job.view.message=format!("{} Sources, partial outputs and repository locks are retained; the job did not earn complete recovery verification.",error.message);},Err(_)=>{job.view.state="needs_recovery".into();job.view.message="The snapshot worker stopped unexpectedly. Partial outputs and sources are retained.".into();}}
            job.view.finished_at=Some(now());
            let terminal=workflow.phase(&mut job,"terminal");
            if let Ok(mut state)=workflow.state.lock() {
                if terminal.is_err() {
                    job.view.state="needs_recovery".into();
                    job.view.message="The durable terminal journal write failed. No completion is certified; retain all partial outputs and inspect this job after catalog recovery.".into();
                    state.terminal_failures.insert(job.view.id.clone(),job.view.clone());
                }
                if state.active.as_ref().is_some_and(|a|a.id==job.view.id) {state.active=None;}
            }
        });
        if spawned.is_err() {
            let mut state = self
                .state
                .lock()
                .map_err(|_| unavailable("Snapshot state is unavailable."))?;
            state.active = None;
            return Err(error(
                ErrorCode::WorkerFailed,
                "The snapshot worker could not start; its durable queued record requires inspection.",
            ));
        }
        Ok(view)
    }
    fn repository_for_write(
        &self,
        job: &mut JobRecord,
        work: &Root,
        cancel: &AtomicBool,
    ) -> Result<RepositoryBinding> {
        let p = &job.profile;
        if let Some(binding) = self.repository(p)? {
            if binding.repository_id.is_none() {
                return Err(conflict(
                    "Repository initialization has no durable concrete identity. Its namespace and copies remain retained; no automatic adoption or backup replay is authorized.",
                ));
            }
            self.read_repository_id(p, &binding, Some(work), cancel)?;
            return Ok(binding);
        }
        let local = if let Some(parent) = &p.local {
            parent.open()?;
            let leaf = repo_leaf(&p.view.catalog_id, &p.namespace_id);
            self.event(
                &job.view.id,
                "repository_create_intent",
                serde_json::json!({"namespace_id":p.namespace_id,"leaf":leaf}),
            )?;
            let root = bridge::make_directory(&parent.root()?, &leaf)?;
            Some(Binding::capture(root.path, root.identity, root.volume)?)
        } else {
            None
        };
        let mut binding = RepositoryBinding {
            namespace_id: p.namespace_id.clone(),
            local,
            repository_id: None,
        };
        // Persist the newly owned native namespace before invoking init. An
        // uncertain init is reconciled by concrete config, never blind unlock.
        self.event(
            &job.view.id,
            "repository_binding",
            serde_json::to_value(&binding)
                .map_err(|_| invalid("Invalid native repository binding."))?,
        )?;
        let context = self.context(p, &binding, Some(work), cancel)?;
        context.run(&["init"], cancel)?;
        let repository_id = self.read_repository_id(p, &binding, Some(work), cancel)?;
        binding.repository_id = Some(repository_id.clone());
        job.manifest.repository_id = Some(repository_id);
        self.event(
            &job.view.id,
            "repository_binding",
            serde_json::to_value(&binding)
                .map_err(|_| invalid("Invalid native repository binding."))?,
        )?;
        Ok(binding)
    }
    fn execute(&self, job: &mut JobRecord, review: &Review, cancel: &AtomicBool) -> Result<()> {
        self.authorized(&review.profile)?;
        if job.view.action == "backup" {
            self.registered_scope(&self.profile(&review.profile.view.id)?)?;
        }
        job.view.state = "running".into();
        let work = self.workspace(&job.view.id)?;
        job.work = Some(Binding::capture(
            work.path.clone(),
            work.identity.clone(),
            work.volume.clone(),
        )?);
        self.phase(job, "staging")?;
        if job.view.action == "backup" {
            let payload_name = format!("payload-{}", job.view.id);
            self.event(
                &job.view.id,
                "payload_intent",
                serde_json::json!({"leaf":payload_name}),
            )?;
            let payload = bridge::make_directory(&work, &payload_name)?;
            let roots = bridge::make_directory(&payload, "roots")?;
            let journals = bridge::make_directory(&work, "journals")?;
            for root in &job.profile.roots {
                let target = bridge::make_directory(&roots, &root.id)?;
                let files = job
                    .manifest
                    .files
                    .iter()
                    .filter(|f| f.root_id == root.id)
                    .map(|f| FileSpec {
                        metadata: f.metadata.clone(),
                        relative: f.relative.clone(),
                        target: f.archive.rsplit('/').next().unwrap_or("").into(),
                        identity: f.identity.clone(),
                        revision: f.revision.clone(),
                        bytes: f.bytes,
                        digest: f.digest.clone(),
                    })
                    .collect::<Vec<_>>();
                bridge::copy(
                    &root.binding.root()?,
                    &target,
                    &files,
                    &[],
                    &journals,
                    cancel,
                    &|| {
                        root.binding.open()?;
                        Ok(())
                    },
                )?;
            }
            if self
                .refresh_manifest(&job.profile, &job.manifest, job.private_scope, cancel)?
                .reviewed_digest()?
                != review.manifest.reviewed_digest()?
            {
                return Err(conflict(
                    "Source scope changed before repository creation. Staged copies are retained; no snapshot upload is authorized.",
                ));
            }
            self.phase(job, "catalog_snapshot")?;
            let catalog_path = payload.path.join("Catalog.snapshot.stillharbour");
            self.catalog.snapshot(catalog_path.clone())?;
            verify_catalog(
                &catalog_path,
                &job.manifest.catalog_id,
                job.manifest.catalog_schema,
            )?;
            let mut catalog = filesystem::open_readonly(&catalog_path)?;
            job.manifest.catalog_bytes = catalog.metadata()?.len();
            job.manifest.catalog_digest = Some(bridge::digest(&mut catalog, cancel)?);
            let binding = self.repository_for_write(job, &work, cancel)?;
            let manifest = encode(&job.manifest)?;
            self.event(&job.view.id,"manifest_intent",serde_json::json!({"digest":blake3::hash(manifest.as_bytes()).to_hex().to_string()}))?;
            bridge::write_new(&payload, "manifest.json", manifest.as_bytes())?;
            self.phase(job, "snapshotting")?;
            let context = self.context(&job.profile, &binding, Some(&work), cancel)?;
            let tag = format!("stillharbour-job:{}", job.view.id);
            // A fixed ASCII child of the held working directory prevents
            // restic from collecting unrelated ancestor metadata, and keeps
            // Windows extended-length prefixes out of repository tree names.
            let output = context.run(&["backup", "--tag", &tag, "--", &payload_name], cancel)?;
            let mut key = None;
            for line in output
                .split(|b| *b == b'\n')
                .filter(|line| !line.is_empty())
            {
                let value: serde_json::Value = serde_json::from_slice(line).map_err(|_| {
                    invalid("The snapshot helper returned invalid structured progress.")
                })?;
                if value.get("message_type").and_then(|v| v.as_str()) == Some("summary") {
                    key = value
                        .get("snapshot_id")
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                }
            }
            let key = key.ok_or_else(|| {
                invalid("The snapshot helper did not return a concrete completed snapshot ID.")
            })?;
            snapshot_id(&key)?;
            job.view.snapshot_id = Some(key);
            job.view.included = true;
            self.phase(job, "snapshot_included")?;
            let subfolder = self.inspect_snapshot(
                &context,
                job.view.snapshot_id.as_deref().unwrap_or(""),
                &payload_name,
                &job.manifest,
                cancel,
            )?;
            job.subfolder = Some(subfolder);
            self.verify_job(job, &context, &work, cancel)?;
            let record = Record {
                metadata_verified: job.metadata_verified,
                view: SnapshotRecord {
                    id: job
                        .view
                        .snapshot_id
                        .clone()
                        .ok_or_else(|| invalid("Missing completed snapshot ID."))?,
                    profile_id: job.view.profile_id.clone(),
                    time: now(),
                    files: job.view.files,
                    bytes: job.view.bytes.clone(),
                    root_ids: job.profile.view.root_ids.clone(),
                    included: true,
                    structural_checked: job.view.structural_checked,
                    data_read_verified: job.view.data_read_verified,
                    restored_compared: job.view.restored_compared,
                    catalog_included: true,
                    source_retained: true,
                },
                manifest: job.manifest.clone(),
                subfolder: job
                    .subfolder
                    .clone()
                    .ok_or_else(|| invalid("Missing verified snapshot subtree."))?,
                repository_id: binding
                    .repository_id
                    .ok_or_else(|| invalid("Missing bound repository ID."))?,
                profile_namespace: job.profile.namespace_id.clone(),
            };
            let key = record.view.id.clone();
            let profile = record.view.profile_id.clone();
            let text = encode(&record)?;
            self.catalog.call(move |db| {
                db.execute(
                    "INSERT INTO backup_snapshot_records VALUES(?1,?2,?3)",
                    params![key, profile, text],
                )?;
                Ok(())
            })?;
            job.view.state = if self
                .refresh_manifest(&job.profile, &job.manifest, job.private_scope, cancel)
                .and_then(|mut m| {
                    m.repository_id = review.manifest.repository_id.clone();
                    m.reviewed_digest()
                })
                .ok()
                == Some(review.manifest.reviewed_digest()?)
            {
                "completed_verified"
            } else {
                "completed_with_unverified_items"
            }
            .into();
            job.view.message = format!(
                "Snapshot inclusion, repository structure, complete data read and restored ordinary file bytes verified. {FIDELITY}"
            );
        } else {
            let record = review
                .record
                .as_ref()
                .ok_or_else(|| invalid("Missing concrete restore record."))?;
            let binding = self
                .repository(&job.profile)?
                .ok_or_else(|| conflict("Missing bound repository."))?;
            self.read_repository_id(&job.profile, &binding, Some(&work), cancel)?;
            let context = self.context(&job.profile, &binding, Some(&work), cancel)?;
            let payload_name = record
                .subfolder
                .rsplit('/')
                .next()
                .ok_or_else(|| invalid("Invalid recorded subtree."))?;
            let subfolder = self.inspect_snapshot(
                &context,
                &record.view.id,
                payload_name,
                &record.manifest,
                cancel,
            )?;
            if subfolder != record.subfolder {
                return Err(conflict("The concrete snapshot subtree changed."));
            }
            job.view.included = true;
            self.verify_job(job, &context, &work, cancel)?;
            if job.view.action == "restore" {
                self.publish_restore(job, &work, cancel)?;
            }
            job.view.state = "completed_verified".into();
            job.view.message =
                format!("Concrete snapshot fully read, restored and compared. {FIDELITY}");
        }
        Ok(())
    }
}
fn verify_catalog(path: &Path, catalog_id: &str, expected_schema: u32) -> Result<()> {
    let held = filesystem::open_readonly(path)?;
    let identity = filesystem::identity(&held)?;
    let db = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    db.pragma_update(None, "query_only", true)?;
    let check: String = db.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    let app: i32 = db.query_row("PRAGMA application_id", [], |r| r.get(0))?;
    let schema: u32 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let actual: String = db.query_row(
        "SELECT uuid FROM catalog_identity WHERE singleton=1",
        [],
        |r| r.get(0),
    )?;
    if check != "ok"
        || app != 0x53544842
        || schema != expected_schema
        || !(1..=crate::catalog::SCHEMA_VERSION).contains(&schema)
        || actual != catalog_id
        || filesystem::identity(&filesystem::open_readonly(path)?)? != identity
    {
        return Err(conflict(
            "The restored catalog is not an independently consistent compatible snapshot of this catalog.",
        ));
    }
    Ok(())
}
fn expected_files(manifest: &Manifest) -> Result<BTreeMap<String, (u64, String)>> {
    let mut files = manifest
        .files
        .iter()
        .map(|f| (f.archive.clone(), (f.bytes, f.digest.clone())))
        .collect::<BTreeMap<_, _>>();
    let text = encode(manifest)?;
    files.insert(
        "manifest.json".into(),
        (
            text.len() as u64,
            blake3::hash(text.as_bytes()).to_hex().to_string(),
        ),
    );
    files.insert(
        "Catalog.snapshot.stillharbour".into(),
        (
            manifest.catalog_bytes,
            manifest
                .catalog_digest
                .clone()
                .ok_or_else(|| invalid("Snapshot catalog digest is missing."))?,
        ),
    );
    Ok(files)
}
impl SnapshotWorkflow {
    fn inspect_snapshot(
        &self,
        context: &Context,
        key: &str,
        payload_name: &str,
        manifest: &Manifest,
        cancel: &AtomicBool,
    ) -> Result<String> {
        snapshot_id(key)?;
        if !crate::organization::portable_filename(payload_name) {
            return Err(invalid("Invalid application snapshot namespace."));
        }
        let output = context.run(&["ls", key], cancel)?;
        let expected = expected_files(manifest)?;
        let mut nodes = Vec::new();
        let mut subfolder = None;
        for line in output.split(|b| *b == b'\n').filter(|l| !l.is_empty()) {
            let value: serde_json::Value = serde_json::from_slice(line)
                .map_err(|_| invalid("Repository tree output is not valid structured data."))?;
            let Some(path) = value.get("path").and_then(|v| v.as_str()) else {
                continue;
            };
            let kind = value
                .get("type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| invalid("Repository node lacks its type."))?;
            if !matches!(kind, "file" | "dir") {
                return Err(unavailable(
                    "A repository tree contains a link or special object; restore was not started.",
                ));
            }
            if path.rsplit('/').next() == Some(payload_name)
                && kind == "dir"
                && subfolder.replace(path.to_string()).is_some()
            {
                return Err(invalid("Repository tree has ambiguous snapshot payloads."));
            }
            nodes.push((
                path.to_string(),
                kind.to_string(),
                value.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
            ));
            if nodes.len() > MAX_FILES * 4 + 1024 {
                return Err(invalid("Repository tree exceeds its bounded size."));
            }
        }
        let folder = subfolder
            .ok_or_else(|| invalid("Concrete snapshot lacks the expected application payload."))?;
        if !folder.starts_with('/')
            || folder.split('/').any(|s| matches!(s, "." | ".."))
            || folder.contains('\\')
        {
            return Err(invalid("The repository subtree path is unsafe."));
        }
        let prefix = format!("{folder}/");
        let mut actual = BTreeSet::new();
        for (path, kind, size) in nodes {
            if let Some(relative) = path.strip_prefix(&prefix) {
                if relative
                    .split('/')
                    .any(|s| s.is_empty() || s == "." || s == "..")
                    || relative.contains('\\')
                {
                    return Err(invalid("Unsafe repository descendant path."));
                }
                if kind == "file" {
                    let item = expected.get(relative).ok_or_else(|| {
                        conflict("The snapshot tree contains an unexpected file.")
                    })?;
                    if item.0 != size || !actual.insert(relative.to_string()) {
                        return Err(conflict(
                            "Snapshot inclusion contains duplicate paths or wrong file sizes.",
                        ));
                    }
                } else if !expected
                    .keys()
                    .any(|file| file.starts_with(&format!("{relative}/")))
                    && !relative
                        .rsplit('/')
                        .next()
                        .is_some_and(|s| s.starts_with(".stillharbour-staging-"))
                {
                    return Err(conflict("The snapshot includes an unexpected directory."));
                }
            } else if kind == "file"
                || !(path == folder
                    || folder.starts_with(&format!("{}/", path.trim_end_matches('/'))))
            {
                return Err(conflict(
                    "Snapshot contains objects outside its reviewed payload/ancestors.",
                ));
            }
        }
        if actual.len() != expected.len() {
            return Err(conflict(
                "The concrete snapshot is missing reviewed files or the consistent catalog.",
            ));
        }
        Ok(folder)
    }
    fn verify_job(
        &self,
        job: &mut JobRecord,
        context: &Context,
        work: &Root,
        cancel: &AtomicBool,
    ) -> Result<()> {
        let key = job
            .view
            .snapshot_id
            .clone()
            .ok_or_else(|| invalid("Missing concrete snapshot."))?;
        snapshot_id(&key)?;
        self.phase(job, "structural_check")?;
        context.run(&["check"], cancel)?;
        job.view.structural_checked = true;
        self.phase(job, "data_read")?;
        context.run(&["check", "--read-data"], cancel)?;
        job.view.data_read_verified = true;
        self.phase(job, "restoring")?;
        let leaf = format!("verification-{}", uuid::Uuid::new_v4());
        self.event(
            &job.view.id,
            "verification_output_intent",
            serde_json::json!({"leaf":leaf}),
        )?;
        let output = bridge::make_directory(work, &leaf)?;
        let target = output
            .path
            .to_str()
            .ok_or_else(|| invalid("Unsupported native verification path."))?;
        let subtree = job
            .subfolder
            .clone()
            .ok_or_else(|| invalid("Missing validated subtree."))?;
        let source = format!("{key}:{subtree}");
        context.run(
            &[
                "restore",
                &source,
                "--target",
                target,
                "--verify",
                "--overwrite",
                "never",
            ],
            cancel,
        )?;
        self.phase(job, "comparing")?;
        compare_payload(&output, &job.manifest, cancel)?;
        verify_catalog(
            &output.path.join("Catalog.snapshot.stillharbour"),
            &job.manifest.catalog_id,
            job.manifest.catalog_schema,
        )?;
        let metadata_files = job
            .manifest
            .files
            .iter()
            .map(|file| FileSpec {
                metadata: file.metadata.clone(),
                relative: native::encode_path(Path::new(&file.archive)),
                target: file.archive.clone(),
                identity: file.identity.clone(),
                revision: file.revision.clone(),
                bytes: file.bytes,
                digest: file.digest.clone(),
            })
            .collect::<Vec<_>>();
        let journal_leaf = format!("verification-metadata-{}", uuid::Uuid::new_v4());
        self.event(&job.view.id,"metadata_output_intent",serde_json::json!({"leaf":journal_leaf,"output":native::encode_path(&output.path),"manifest_digest":digest(&job.manifest)?}))?;
        let journals = bridge::make_directory(work, &journal_leaf)?;
        bridge::restore_metadata(&output, &metadata_files, &journals, cancel)?;
        job.metadata_verified = job
            .manifest
            .files
            .iter()
            .all(|file| file.metadata.is_some());
        job.view.restored_compared = true;
        self.event(&job.view.id,"restore_comparison",serde_json::json!({"snapshot_id":key,"output":native::encode_path(&output.path),"identity":output.identity,"volume":output.volume,"manifest_digest":digest(&job.manifest)?,"essential_metadata_verified":false,"independent_failure_domain":false}))?;
        // This path is owned by this job and is not accepted from the renderer.
        job.subfolder = Some(subtree.clone());
        job.verified_output = Some(Binding::capture(
            output.path.clone(),
            output.identity.clone(),
            output.volume.clone(),
        )?);
        self.event(
            &job.view.id,
            "verified_output",
            serde_json::to_value(Binding::capture(
                output.path,
                output.identity,
                output.volume,
            )?)
            .map_err(|_| invalid("Invalid verified output binding."))?,
        )?;
        self.save_job(job)
    }
    fn publish_restore(&self, job: &mut JobRecord, work: &Root, cancel: &AtomicBool) -> Result<()> {
        job.view.restored_compared = false;
        self.phase(job, "publication_name_preflight")?;
        let target = job
            .restore
            .as_ref()
            .ok_or_else(|| invalid("Missing reviewed restore output."))?;
        target.open()?;
        let profile = self.profile(&job.view.profile_id)?;
        validate_manifest(&job.manifest, &profile)?;
        let job_id = job.view.id.clone();
        let text=self.catalog.call(move |db|Ok(db.query_row("SELECT CASE WHEN length(CAST(evidence AS BLOB))<=262144 THEN evidence END FROM backup_snapshot_events WHERE job_id=? AND phase='verified_output' ORDER BY sequence DESC LIMIT 1",[job_id],|r|r.get::<_,String>(0))?))?;
        let verified: Binding = decode(text)?;
        verified.open()?;
        // The restore review ID, not the worker UUID, fixes its output namespace.
        let plan_id = job.review_id.clone();
        let leaf = format!("stillharbour-restored-{plan_id}");
        self.event(
            &job.view.id,
            "restore_output_intent",
            serde_json::json!({"leaf":leaf,"parent":target}),
        )?;
        let output = bridge::make_directory(&target.root()?, &leaf)?;
        let journals =
            bridge::make_directory(work, &format!("restore-journals-{}", uuid::Uuid::new_v4()))?;
        let mut files = Vec::new();
        for file in &job.manifest.files {
            let relative = native::decode_path(&file.relative)?;
            let target = Path::new("roots").join(&file.root_id).join(relative);
            let target = target
                .components()
                .map(|c| {
                    c.as_os_str()
                        .to_str()
                        .ok_or_else(|| invalid("Unsupported native restore filename."))
                })
                .collect::<Result<Vec<_>>>()?
                .join("/");
            let source = Path::new(&file.archive);
            let handle = native::open_asset(
                &verified.path()?,
                &verified.identity,
                &verified.volume,
                source,
            )?;
            files.push(FileSpec {
                metadata: file.metadata.clone(),
                relative: native::encode_path(source),
                target,
                identity: filesystem::identity(&handle)?,
                revision: native::revision(&handle)?,
                bytes: file.bytes,
                digest: file.digest.clone(),
            });
        }
        for (name, (bytes, digest)) in expected_files(&job.manifest)?
            .into_iter()
            .filter(|(name, _)| !name.starts_with("roots/"))
        {
            let relative = Path::new(&name);
            let handle = native::open_asset(
                &verified.path()?,
                &verified.identity,
                &verified.volume,
                relative,
            )?;
            files.push(FileSpec {
                metadata: None,
                relative: native::encode_path(relative),
                target: name,
                identity: filesystem::identity(&handle)?,
                revision: native::revision(&handle)?,
                bytes,
                digest,
            });
        }
        bridge::copy(
            &verified.root()?,
            &output,
            &files,
            &mapped_directories(&job.manifest)?,
            &journals,
            cancel,
            &|| {
                target.open()?;
                verified.open()?;
                Ok(())
            },
        )?;
        verify_catalog(
            &output.path.join("Catalog.snapshot.stillharbour"),
            &job.manifest.catalog_id,
            job.manifest.catalog_schema,
        )?;
        job.view.restored_compared = true;
        self.event(&job.view.id,"restore_published",serde_json::json!({"path":native::encode_path(&output.path),"identity":output.identity,"volume":output.volume,"files":files.len(),"sources_retained":true}))?;
        Ok(())
    }
}
fn compare_payload(output: &Root, manifest: &Manifest, cancel: &AtomicBool) -> Result<()> {
    let expected = expected_files(manifest)?;
    let mut actual = BTreeMap::new();
    collect_output(&output.source()?, Path::new(""), 0, &mut actual, cancel)?;
    if expected != actual {
        return Err(conflict(
            "Full restored file bytes, sizes or filename scope differ from the exact snapshot manifest.",
        ));
    }
    Ok(())
}
fn collect_output(
    parent: &File,
    relative: &Path,
    depth: usize,
    files: &mut BTreeMap<String, (u64, String)>,
    cancel: &AtomicBool,
) -> Result<()> {
    if depth > 68 {
        return Err(invalid("Restore tree exceeds its directory bound."));
    }
    each(parent, |entry| {
        if entry.blocked {
            return Err(invalid(
                "A restored special object is outside the approved manifest.",
            ));
        }
        let path = relative.join(&entry.name);
        let mut file = child(parent, &entry.name, entry.directory)?;
        if entry.directory {
            collect_output(&file, &path, depth + 1, files, cancel)?;
        } else {
            if files.len() > MAX_FILES + 2 {
                return Err(invalid("Restore file count exceeds its bound."));
            }
            let name = path
                .components()
                .map(|c| {
                    c.as_os_str()
                        .to_str()
                        .ok_or_else(|| invalid("Invalid restored native path."))
                })
                .collect::<Result<Vec<_>>>()?
                .join("/");
            if files
                .insert(
                    name,
                    (file.metadata()?.len(), bridge::digest(&mut file, cancel)?),
                )
                .is_some()
            {
                return Err(invalid("Duplicate restored path."));
            }
        }
        Ok(())
    })
}
impl SnapshotWorkflow {
    fn instance(&self) -> Result<String> {
        self.catalog.call(|db| {
            let path = db.path().ok_or_else(|| {
                unavailable("In-memory catalogs cannot authorize recurring work.")
            })?;
            let mut options = std::fs::OpenOptions::new();
            options.read(true);
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                use windows_sys::Win32::Storage::FileSystem::{
                    FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
                };
                options
                    .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
                    .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
            }
            let file = options.open(path)?;
            let identity = filesystem::identity(&file)?;
            digest(&(
                native::encode_path(Path::new(path)),
                identity.volume,
                identity.file,
            ))
        })
    }
    fn schedule_record(&self, profile: &str) -> Result<Option<ScheduleRecord>> {
        id(profile)?;
        let key = profile.to_string();
        let text = self.catalog.call(move |db| {
            Ok(db
                .query_row(
                    "SELECT CASE WHEN length(CAST(record AS BLOB))<=8192 THEN record END FROM backup_snapshot_schedules WHERE profile_id=?",
                    [key],
                    |r| r.get::<_, String>(0),
                )
                .optional()?)
        })?;
        text.map(decode).transpose()
    }
    fn schedule_policy(record: &ScheduleRecord) -> Result<String> {
        let mut stable = record.clone();
        stable.view.next_due.clear();
        stable.view.last_run = None;
        digest(&stable)
    }
    fn persist_schedule(&self, record: &ScheduleRecord) -> Result<()> {
        let id = record.view.profile_id.clone();
        let text = encode(record)?;
        self.catalog.call(move|db|{db.execute("INSERT INTO backup_snapshot_schedules VALUES(?1,?2) ON CONFLICT(profile_id) DO UPDATE SET record=excluded.record",params![id,text])?;Ok(())})
    }
    fn schedule_get(&self, profile: &str) -> Result<Option<SnapshotSchedule>> {
        self.profile(profile)?;
        let Some(mut record) = self.schedule_record(profile)? else {
            return Ok(None);
        };
        let permitted = record.instance == self.instance()?
            && crate::backup::credentials::schedule_permit(
                &self.catalog.summary()?.id,
                &record.instance,
                profile,
                &Self::schedule_policy(&record)?,
                false,
            )
            .unwrap_or(false);
        if !permitted {
            record.view.enabled = false;
        }
        Ok(Some(record.view))
    }
    fn schedule_disable(&self, profile: &str) -> Result<Option<SnapshotSchedule>> {
        self.profile(profile)?;
        let Some(mut record) = self.schedule_record(profile)? else {
            return Ok(None);
        };
        record.view.enabled = false;
        let revoked = if CredentialStore::available() {
            crate::backup::credentials::schedule_revoke(
                &self.catalog.summary()?.id,
                &record.instance,
                profile,
            )
        } else {
            Ok(())
        };
        self.persist_schedule(&record)?;
        revoked?;
        Ok(Some(record.view))
    }
    #[allow(clippy::too_many_arguments)]
    fn schedule_save(
        &self,
        profile: &str,
        review_id: &str,
        value: &str,
        minutes: u32,
        timezone: &str,
        enabled: bool,
        catch_up: bool,
    ) -> Result<SnapshotSchedule> {
        self.require_execution()?;
        if !(15..=10080).contains(&minutes) || timezone != "UTC" {
            return Err(invalid(
                "Schedules require 15–10080 elapsed minutes and timezone UTC.",
            ));
        }
        let p = self.profile(profile)?;
        self.authorized(&p)?;
        let review = self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?
            .reviews
            .get(review_id)
            .cloned()
            .ok_or_else(|| {
                conflict(
                    "Create a current exact-scope backup review before enabling repeat consent.",
                )
            })?;
        if review.created.elapsed() >= REVIEW_AGE
            || review.view.profile_id != profile
            || review.view.action != "backup"
            || review.view.digest != value
            || self
                .inventory(&p, &AtomicBool::new(false))?
                .reviewed_digest()?
                != value
        {
            return Err(conflict(
                "Recurring consent requires the current exact reviewed source manifest and profile.",
            ));
        }
        let record = ScheduleRecord {
            view: SnapshotSchedule {
                profile_id: profile.into(),
                interval_minutes: minutes,
                timezone: timezone.into(),
                dst_policy: "elapsed_interval".into(),
                enabled,
                catch_up,
                next_due: (chrono::Utc::now() + chrono::Duration::minutes(i64::from(minutes)))
                    .to_rfc3339(),
                last_run: None,
                fully_quit_supported: false,
                approved_digest: value.into(),
            },
            profile_digest: digest(&p)?,
            manifest_digest: review.manifest.reviewed_digest()?,
            instance: self.instance()?,
        };
        // The OS write is the explicit exact-repeat consent; a catalog row alone
        // is never an execution grant. Failure leaves the schedule unenabled.
        crate::backup::credentials::schedule_permit(
            &p.view.catalog_id,
            &record.instance,
            profile,
            &Self::schedule_policy(&record)?,
            true,
        )?;
        self.persist_schedule(&record)?;
        Ok(record.view)
    }
    pub(crate) fn tick(self: &Arc<Self>, library: Arc<Library>) -> Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?;
        if state.timer_started || self.stopped.load(Ordering::Acquire) {
            return Ok(());
        }
        state.timer_started = true;
        drop(state);
        let weak = Arc::downgrade(self);
        let library = Arc::downgrade(&library);
        std::thread::Builder::new()
            .name("snapshot-schedule".into())
            .spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_secs(15));
                    let (Some(workflow), Some(library)) = (weak.upgrade(), library.upgrade())
                    else {
                        break;
                    };
                    if workflow.stopped.load(Ordering::Acquire) {
                        break;
                    }
                    let _ = workflow.run_due(library);
                }
            })
            .map_err(|_| {
                error(
                    ErrorCode::WorkerFailed,
                    "The running-app schedule timer could not start.",
                )
            })?;
        Ok(())
    }
    fn run_due(self: &Arc<Self>, library: Arc<Library>) -> Result<()> {
        if !self.capabilities().schedule_available {
            return Ok(());
        }
        if self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?
            .active
            .is_some()
        {
            return Ok(());
        }
        let rows = self.catalog.call(|db| {
            Ok(db
                .prepare("SELECT CASE WHEN length(CAST(record AS BLOB))<=8192 THEN record END FROM backup_snapshot_schedules LIMIT 33")?
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })?;
        if rows.len() > 32 {
            return Err(invalid("Recurring policy count exceeds its bound."));
        }
        for text in rows {
            let mut record: ScheduleRecord = decode(text)?;
            if !record.view.enabled
                || chrono::DateTime::parse_from_rfc3339(&record.view.next_due)
                    .map_err(|_| invalid("Invalid schedule due time."))?
                    > chrono::Utc::now()
            {
                continue;
            }
            let profile = self.profile(&record.view.profile_id)?;
            let consent = record.instance == self.instance()?
                && record.profile_digest == digest(&profile)?
                && crate::backup::credentials::schedule_permit(
                    &profile.view.catalog_id,
                    &record.instance,
                    &profile.view.id,
                    &Self::schedule_policy(&record)?,
                    false,
                )
                .unwrap_or(false);
            if !consent {
                record.view.enabled = false;
                let revoked = crate::backup::credentials::schedule_revoke(
                    &profile.view.catalog_id,
                    &record.instance,
                    &profile.view.id,
                );
                self.persist_schedule(&record)?;
                revoked?;
                continue;
            }
            if !record.view.catch_up
                && chrono::DateTime::parse_from_rfc3339(&record.view.next_due)
                    .map_err(|_| invalid("Invalid schedule due time."))?
                    < self.started
            {
                record.view.next_due = (chrono::Utc::now()
                    + chrono::Duration::minutes(i64::from(record.view.interval_minutes)))
                .to_rfc3339();
                self.persist_schedule(&record)?;
                continue;
            }
            // Native session/root reauthorization is still required after a
            // restart. An imported row never authorizes its own filesystem scope.
            if self.authorized(&profile).is_err() {
                continue;
            }
            let manifest = self.inventory(&profile, &AtomicBool::new(false));
            let exact = manifest
                .ok()
                .and_then(|m| m.reviewed_digest().ok())
                .as_deref()
                == Some(record.view.approved_digest.as_str());
            record.view.last_run = Some(now());
            record.view.next_due = (chrono::Utc::now()
                + chrono::Duration::minutes(i64::from(record.view.interval_minutes)))
            .to_rfc3339();
            if !exact {
                record.view.enabled = false;
                let revoked = crate::backup::credentials::schedule_revoke(
                    &profile.view.catalog_id,
                    &record.instance,
                    &profile.view.id,
                );
                self.persist_schedule(&record)?;
                revoked?;
                self.event(&uuid::Uuid::new_v4().to_string(),"schedule_needs_review",serde_json::json!({"profile_id":profile.view.id,"reason":"Source, availability, root or exact version changed; no recurring approval was created."}))?;
                continue;
            }
            self.persist_schedule(&record)?;
            let view = self.review(profile)?;
            let review = self
                .state
                .lock()
                .map_err(|_| unavailable("Snapshot state is unavailable."))?
                .reviews
                .get(&view.id)
                .cloned()
                .ok_or_else(|| conflict("Scheduled review disappeared."))?;
            if review.view.digest != record.view.approved_digest {
                return Err(conflict("Exact recurring scope changed before admission."));
            }
            self.start(library, review, "backup")?;
            return Ok(());
        }
        Ok(())
    }
    #[cfg(any(test, feature = "backup-workflow", feature = "fixture-probes"))]
    pub(crate) fn verify_recovery(
        &self,
        files: &[crate::backup::sync::RecoveryFile],
        evidence_id: &str,
    ) -> Result<crate::backup::sync::RecoveryProof> {
        snapshot_id(evidence_id)?;
        if files.is_empty() || files.len() > MAX_FILES {
            return Err(invalid(
                "Exact recovery requires a nonempty bounded backend scope.",
            ));
        }
        let key = evidence_id.to_string();
        let text = self
            .catalog
            .call(move |db| {
                Ok(db
                    .query_row(
                        "SELECT CASE WHEN length(CAST(record AS BLOB))<=33554432 THEN record END FROM backup_snapshot_records WHERE id=?",
                        [key],
                        |r| r.get::<_, String>(0),
                    )
                    .optional()?)
            })?
            .ok_or_else(|| {
                conflict("No concrete application snapshot records this recovery evidence.")
            })?;
        let record: Record = decode(text)?;
        if !record.view.included
            || !record.view.structural_checked
            || !record.view.data_read_verified
            || !record.view.restored_compared
        {
            return Err(conflict(
                "Snapshot evidence lacks full inclusion/read/restore byte proof.",
            ));
        }
        self.require_execution()?;
        let authorized = self
            .state
            .lock()
            .map_err(|_| unavailable("Snapshot state is unavailable."))?
            .authorized
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let mut profiles = Vec::new();
        for id in authorized {
            let profile = self.profile(&id)?;
            if profile.namespace_id == record.profile_namespace
                && self.lineage(&profile)?.contains(&record.view.profile_id)
            {
                profiles.push(profile);
            }
        }
        profiles.sort_by(|a, b| b.view.created_at.cmp(&a.view.created_at));
        let profile=profiles.into_iter().next().ok_or_else(||conflict("Explicitly open this repository's native profile session before requesting recovery proof."))?;
        self.authorized(&profile)?;
        validate_manifest(&record.manifest, &profile)?;
        let repository = self
            .repository(&profile)?
            .ok_or_else(|| conflict("Concrete recovery repository binding is missing."))?;
        if repository.repository_id.as_deref() != Some(&record.repository_id) {
            return Err(conflict(
                "The recovery record belongs to another repository identifier.",
            ));
        }
        let repository_root=repository.local.as_ref().ok_or_else(||unavailable("Remote failure-domain independence is unknown; loopback/disposable servers never certify an independent role."))?;
        let repository_handle = repository_root.open()?;
        let mut scope = Vec::new();
        let mut seen = BTreeSet::new();
        for file in files {
            if !seen.insert((&file.root_id, &file.relative_path)) {
                return Err(invalid(
                    "The exact recovery request repeats an object path.",
                ));
            }
            let root=record.manifest.roots.iter().find(|r|r.root.id==file.root_id&&r.root.binding.path==file.native_root&&r.root.binding.identity==file.root_identity&&r.root.binding.volume==file.root_volume).ok_or_else(||conflict("The evidence does not include this exact authorized dataset root; source-root backups cannot protect an unrelated old destination."))?;
            root.root.binding.open()?;
            let captured = record.manifest.files.iter().find(|f| {
                f.root_id == file.root_id
                    && f.relative == file.relative_path
                    && f.identity == file.identity
                    && f.revision == file.revision
                    && f.bytes == file.bytes
                    && f.digest == file.full_digest
            });
            if file.catalog_id != record.manifest.catalog_id || captured.is_none() {
                return Err(conflict(
                    "The recovery snapshot does not protect the exact requested old version and companions.",
                ));
            }
            let captured = captured.ok_or_else(|| conflict("Missing exact recovery file."))?;
            let metadata=captured.metadata.as_ref().ok_or_else(||unavailable("This exact snapshot version has byte-only evidence; native essential metadata is unavailable."))?;
            if file.essential_metadata_digest.as_deref() != Some(metadata.digest()?.as_str())
                || !metadata.certified_replacement()?
            {
                return Err(unavailable(
                    "Exact essential metadata, including native security/audit state, is not certified for this replacement scope.",
                ));
            }
            let relative = native::decode_path(&file.relative_path)?;
            let binding = &root.root.binding;
            let mut current = native::open_asset(
                &binding.path()?,
                &binding.identity,
                &binding.volume,
                &relative,
            )?;
            if !crate::backup::failure_domains::independent_local(&current, &repository_handle)? {
                return Err(unavailable(
                    "The exact source file and repository share a physical failure domain, or independence is unknown.",
                ));
            }
            if filesystem::identity(&current)? != file.identity
                || native::revision(&current)? != file.revision
                || current.metadata()?.len() != file.bytes
                || bridge::digest(&mut current, &AtomicBool::new(false))? != file.full_digest
            {
                return Err(conflict(
                    "The requested exact old version changed before fresh recovery verification.",
                ));
            }
            metadata.compare(&current)?;
            scope.push((
                &file.catalog_id,
                &file.root_id,
                &file.native_root,
                &file.root_identity,
                &file.root_volume,
                &file.relative_path,
                &file.identity,
                &file.revision,
                file.bytes,
                &file.full_digest,
                &file.essential_metadata_digest,
            ));
        }
        scope.sort_by(|a, b| (&a.1, &a.5).cmp(&(&b.1, &b.5)));
        let scope_digest = digest(&scope)?;
        let _lease = Registry::shared().try_acquire(vec![Scope::new(
            DirectoryChain::open(&repository_root.path()?)?,
            Access::Write,
        )])?;
        let job_id = uuid::Uuid::new_v4().to_string();
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| unavailable("Snapshot state is unavailable."))?;
            if state.active.is_some() {
                return Err(error(
                    ErrorCode::Busy,
                    "Another snapshot operation is active.",
                ));
            }
            state.active = Some(Active {
                id: job_id.clone(),
                roots: files.iter().map(|f| f.root_id.clone()).collect(),
                cancel: cancel.clone(),
            });
        }
        let _activity = RecoveryActivity {
            workflow: self,
            id: job_id.clone(),
        };
        let view=SnapshotJob{id:job_id,profile_id:profile.view.id.clone(),action:"verify".into(),state:"queued".into(),phase:"exact_recovery_review".into(),files:record.view.files,bytes:record.view.bytes.clone(),snapshot_id:Some(evidence_id.into()),message:"Fresh exact-version recovery verification; original and destination files remain unchanged.".into(),started_at:now(),finished_at:None,included:false,structural_checked:false,data_read_verified:false,restored_compared:false,source_retained:true,independent_recovery_verified:false};
        let review = Review {
            private_scope: false,
            view: SnapshotReview {
                items: review_items(&record.manifest),
                id: uuid::Uuid::new_v4().to_string(),
                profile_id: profile.view.id.clone(),
                action: "verify".into(),
                digest: record.manifest.reviewed_digest()?,
                files: record.view.files,
                bytes: record.view.bytes.clone(),
                root_ids: record.view.root_ids.clone(),
                catalog_included: true,
                source_display_paths: Vec::new(),
                source_retained: true,
                target_display: "Fresh owned exact-recovery verification output".into(),
                issues: Vec::new(),
                execution_available: true,
                unavailable_reason: None,
            },
            profile: profile.clone(),
            manifest: record.manifest.clone(),
            restore: None,
            record: Some(record.clone()),
            created: Instant::now(),
        };
        let mut job = JobRecord {
            verified_output: None,
            metadata_verified: false,
            private_scope: false,
            review_id: review.view.id.clone(),
            view,
            profile,
            manifest: record.manifest.clone(),
            work: None,
            restore: None,
            subfolder: Some(record.subfolder.clone()),
        };
        self.save_job(&job)?;
        self.event(
            &job.view.id,
            "exact_recovery_scope",
            serde_json::json!({"evidence_id":evidence_id,"scope_digest":scope_digest}),
        )?;
        let result = (|| {
            self.execute(&mut job, &review, &cancel)?;
            let output = job
                .verified_output
                .as_ref()
                .ok_or_else(|| conflict("Fresh recovery output was not recorded."))?;
            for file in files {
                let captured = record
                    .manifest
                    .files
                    .iter()
                    .find(|f| f.root_id == file.root_id && f.relative == file.relative_path)
                    .ok_or_else(|| conflict("Exact recovery scope disappeared."))?;
                let metadata = captured
                    .metadata
                    .as_ref()
                    .ok_or_else(|| conflict("Essential metadata proof disappeared."))?;
                let restored = native::open_asset(
                    &output.path()?,
                    &output.identity,
                    &output.volume,
                    Path::new(&captured.archive),
                )?;
                metadata.compare(&restored)?;
                let root = record
                    .manifest
                    .roots
                    .iter()
                    .find(|r| r.root.id == file.root_id)
                    .ok_or_else(|| conflict("Recovery root disappeared."))?;
                let current = native::open_asset(
                    &root.root.binding.path()?,
                    &root.root.binding.identity,
                    &root.root.binding.volume,
                    &native::decode_path(&file.relative_path)?,
                )?;
                if filesystem::identity(&current)? != file.identity
                    || native::revision(&current)? != file.revision
                {
                    return Err(conflict(
                        "The exact old version changed during recovery verification.",
                    ));
                }
                metadata.compare(&current)?;
                if !crate::backup::failure_domains::independent_local(
                    &current,
                    &repository_root.open()?,
                )? {
                    return Err(conflict(
                        "Physical recovery independence changed during verification.",
                    ));
                }
            }
            Ok(())
        })();
        job.view.finished_at = Some(now());
        if result.is_ok() {
            job.view.independent_recovery_verified = true;
        } else {
            job.view.state = "needs_recovery".into();
            job.view.message =
                "Fresh exact recovery verification failed; replacement is not authorized.".into();
        }
        self.phase(&mut job, "terminal")?;
        result?;
        Ok(crate::backup::sync::RecoveryProof {
            evidence_id: evidence_id.into(),
            scope_digest,
            independent_failure_domain: true,
            essential_metadata_verified: true,
        })
    }
}
#[cfg(any(test, feature = "backup-workflow", feature = "fixture-probes"))]
struct RecoveryActivity<'a> {
    workflow: &'a SnapshotWorkflow,
    id: String,
}
#[cfg(any(test, feature = "backup-workflow", feature = "fixture-probes"))]
impl Drop for RecoveryActivity<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.workflow.state.lock()
            && state
                .active
                .as_ref()
                .is_some_and(|active| active.id == self.id)
        {
            state.active = None;
        }
    }
}

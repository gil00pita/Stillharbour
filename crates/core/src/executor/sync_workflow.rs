//! Catalog-backed readable-copy jobs. Only approved backend-native scopes can
//! reach the shared executor; persisted records never reconstruct approvals.
use super::*;
use crate::{
    backup::sync::*,
    catalog::Catalog,
    library::{Library, native as source, store},
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Component, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

mod preparation;
mod retained;
pub(crate) use retained::RetainedRecoveryScope;
mod storage;
#[cfg(test)]
mod tests;

const MAX_RECORD: usize = 32 * 1024 * 1024;
const MAX_REVIEWS: usize = 64;
const RESERVE: u128 = 64 * 1024 * 1024;
const METADATA_WARNING: &str = "Only ordinary file bytes are verified by this copy adapter. Permissions, alternate streams, extended attributes and resource forks are not certified. Sources and existing destination versions remain retained; this is not exact full-fidelity or independent disaster protection.";

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Observed {
    pub path: Vec<u8>,
    pub identity: filesystem::FileIdentity,
    pub revision: String,
    pub bytes: u64,
    pub copy: bool,
}
pub(crate) type ReplacementVersions = Vec<(String, Observed, Observed)>;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Scope {
    pub run_id: String,
    pub digest: String,
    pub catalog_id: String,
    pub root_id: String,
    pub source: Vec<u8>,
    pub source_identity: filesystem::FileIdentity,
    pub source_volume: String,
    pub destination: Vec<u8>,
    pub destination_identity: filesystem::FileIdentity,
    pub destination_volume: String,
    pub namespace: Vec<u8>,
    pub sources: Vec<Observed>,
    pub destinations: Vec<Observed>,
    pub source_directories: Vec<(Vec<u8>, String)>,
    pub uncopied: u32,
    pub destination_directories: Vec<(Vec<u8>, filesystem::FileIdentity)>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    scope: Scope,
    view: SyncReview,
    parents: Vec<(String, Object)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    retained: Option<retained::RetainedBinding>,
}
enum Stage {
    Prepare,
    Copy(CopyPlan),
    Resume(CopyPlan, resume::Review),
    Retained,
}
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fault {
    BeforeRun,
    StageCreated,
    ProbeCreated,
    NamesProven,
    ProbeMoved,
    PlanBound,
    Copy(CopyFault),
    RetainedDirectoryCreated,
    RetainedOldIntent,
    RetainedOldMoved,
    RetainedOldRecorded,
    RetainedNewIntent,
    RetainedNewMoved,
    RetainedNewRecorded,
    RetainedCompleted,
    RetainedMetadataApplied,
    RetainedMetadataRecorded,
}
struct Review {
    binding: Binding,
    view: SyncReview,
    stage: Stage,
    created: Instant,
}
struct Active {
    id: String,
    root: String,
    cancel: Arc<AtomicBool>,
}
pub(crate) struct SyncWorkflow {
    catalog: Arc<Catalog>,
    data_directory: PathBuf,
    reviews: std::sync::Mutex<BTreeMap<String, Arc<Review>>>,
    grants: std::sync::Mutex<BTreeMap<String, String>>,
    approvals: approval::Approvals,
    active: std::sync::Mutex<Option<Active>>,
    stopping: AtomicBool,
    cancel: Arc<AtomicBool>,
    #[cfg(test)]
    fault: std::sync::Mutex<Option<Fault>>,
    #[cfg(test)]
    crash: AtomicBool,
    #[cfg(test)]
    recovery_proof: std::sync::Mutex<Option<RecoveryProof>>,
}
fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message)
}
fn now() -> String {
    store::now()
}
fn encode<T: Serialize>(value: &T) -> Result<String> {
    let text = serde_json::to_string(value)
        .map_err(|e| failed("Could not encode sync record.").caused_by(e))?;
    if text.len() > MAX_RECORD {
        return Err(invalid("The sync record exceeds its 32 MiB bound."));
    }
    Ok(text)
}
fn decode<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    if text.len() > MAX_RECORD {
        return Err(failed("Sync record exceeds its bound."));
    }
    serde_json::from_str(text).map_err(|e| failed("Invalid retained sync record.").caused_by(e))
}
fn relative(path: &Path) -> Result<String> {
    let parts = path
        .components()
        .map(|p| match p {
            Component::Normal(n) => n
                .to_str()
                .filter(|n| crate::organization::portable_filename(n))
                .map(String::from)
                .ok_or_else(|| invalid("A sync target name needs unsupported native preparation.")),
            _ => Err(invalid("Unsafe sync relative path.")),
        })
        .collect::<Result<Vec<_>>>()?;
    if parts.is_empty() {
        return Err(invalid("Empty sync relative path."));
    }
    Ok(parts.join("/"))
}
fn action_name(action: SyncAction) -> &'static str {
    match action {
        SyncAction::PrepareNames => "prepare_names",
        SyncAction::CopyFiles => "copy_files",
        SyncAction::ResumeFiles => "resume_files",
        SyncAction::ReplaceRetaining => "replace_retaining",
        SyncAction::RestoreRetained => "restore_retained",
    }
}
fn state_name(state: SyncState) -> &'static str {
    match state {
        SyncState::Queued => "queued",
        SyncState::Running => "running",
        SyncState::Paused => "paused",
        SyncState::CancelRequested => "cancel_requested",
        SyncState::NeedsReview => "needs_review",
        SyncState::NeedsRecovery => "needs_recovery",
        SyncState::CompletedVerified => "completed_verified",
        SyncState::CompletedPartial => "completed_partial",
        SyncState::Canceled => "canceled",
        SyncState::Failed => "failed",
    }
}
fn expected(review: &SyncReview, selection: &str) -> Result<approval::Review> {
    let action = match review.action {
        SyncAction::PrepareNames => approval::Action::PrepareSyncNames,
        SyncAction::CopyFiles => approval::Action::CopySyncFiles,
        SyncAction::ResumeFiles => approval::Action::ResumeSyncFiles,
        SyncAction::ReplaceRetaining => approval::Action::ReplaceRetaining,
        SyncAction::RestoreRetained => approval::Action::RestoreRetained,
    };
    approval::Review::new(
        action,
        review.id.clone(),
        review.digest.clone(),
        selection.into(),
    )
}

impl SyncWorkflow {
    #[cfg(test)]
    fn boundary(&self, point: Fault) -> Result<()> {
        if self
            .fault
            .lock()
            .map_err(|_| failed("Fault control unavailable."))?
            .as_ref()
            == Some(&point)
        {
            if self.crash.load(Ordering::Acquire) {
                std::process::exit(88);
            }
            return Err(failed(
                "Injected sync interruption before durable completion.",
            ));
        }
        Ok(())
    }
    fn copy_fault(&self) -> Option<CopyFault> {
        #[cfg(test)]
        {
            if let Ok(fault) = self.fault.lock()
                && let Some(Fault::Copy(point)) = *fault
            {
                return Some(point);
            }
        }
        None
    }
    pub(crate) fn cancel_all(&self) {
        self.stopping.store(true, Ordering::Release);
        self.cancel.store(true, Ordering::Release);
        if let Ok(active) = self.active.try_lock()
            && let Some(active) = active.as_ref()
        {
            active.cancel.store(true, Ordering::Release);
        }
        let _ = PacedIo::shared().set_paused(false);
        if let Ok(mut reviews) = self.reviews.lock() {
            reviews.clear();
        }
        if let Ok(mut grants) = self.grants.lock() {
            grants.clear();
        }
    }
    pub(crate) fn idle(&self) -> Result<bool> {
        match self.active.try_lock() {
            Ok(active) => Ok(active.is_none()),
            Err(std::sync::TryLockError::WouldBlock) => Ok(false),
            Err(std::sync::TryLockError::Poisoned(_)) => Err(failed("Sync worker unavailable.")),
        }
    }
    pub(crate) fn has_active_root(&self, root: &str) -> Result<bool> {
        Ok(self
            .active
            .lock()
            .map_err(|_| failed("Sync worker unavailable."))?
            .as_ref()
            .is_some_and(|a| a.root == root))
    }
    pub(crate) fn cancel_root(&self, root: &str) -> Result<()> {
        let active = self
            .active
            .lock()
            .map_err(|_| failed("Sync worker unavailable."))?;
        if let Some(active) = active.as_ref().filter(|a| a.root == root) {
            active.cancel.store(true, Ordering::Release);
            PacedIo::shared().set_paused(false)?;
        }
        let mut reviews = self
            .reviews
            .lock()
            .map_err(|_| failed("Sync reviews unavailable."))?;
        reviews.retain(|_, r| r.binding.scope.root_id != root);
        self.grants
            .lock()
            .map_err(|_| failed("Sync approvals unavailable."))?
            .retain(|_, id| reviews.contains_key(id));
        Ok(())
    }
    pub(crate) fn new(catalog: Arc<Catalog>, data_directory: PathBuf) -> Result<Self> {
        catalog.call(|db|{db.execute("UPDATE backup_sync_jobs SET state='needs_recovery',phase='reopened',ready_review_id=NULL,message='Application restarted. Inspect native state and approve a fresh recovery review before continuing.',updated_at=? WHERE state IN ('queued','running','paused','cancel_requested','needs_review')",[now()])?;Ok(())})?;
        Ok(Self {
            catalog,
            data_directory,
            reviews: Default::default(),
            grants: Default::default(),
            approvals: approval::Approvals::new(),
            active: Default::default(),
            stopping: AtomicBool::new(false),
            cancel: Arc::new(AtomicBool::new(false)),
            #[cfg(test)]
            fault: Default::default(),
            #[cfg(test)]
            crash: AtomicBool::new(false),
            #[cfg(test)]
            recovery_proof: Default::default(),
        })
    }
    pub(crate) fn capabilities() -> Result<SyncCapabilities> {
        Ok(SyncCapabilities{execution_available:cfg!(any(test,feature="backup-workflow"))&&cfg!(any(windows,target_os="macos")),retained_replacement_available:false,reason:"Source-retaining copy requires fresh scope/name/capacity review and separate approvals. Destination replacement remains gated on independently verified exact-version and essential-metadata recovery evidence.".into(),io_scope:"One shared application-issued byte budget includes comparison, copying and verification. Pause affects this shared budget. Metadata, opaque helpers, OS caches and physical traffic are outside this cap.".into(),bytes_per_second:PacedIo::shared().bytes_per_second()?.to_string()})
    }
    pub(crate) fn handle(
        self: &Arc<Self>,
        library: Arc<Library>,
        request: SyncRequest,
    ) -> Result<SyncReply> {
        match request {
            SyncRequest::ReviewRecoveryBackup {
                run_id,
                entry_id,
                profile_id,
            } => Ok(SyncReply::RecoveryBackup(
                library.backup_old_destination_review(run_id, entry_id, profile_id)?,
            )),
            SyncRequest::Capabilities {} => Ok(SyncReply::Capabilities(Self::capabilities()?)),
            SyncRequest::ReviewReplacement {
                run_id,
                entry_id,
                recovery_evidence_id,
            } => Ok(SyncReply::Review(self.review_replacement(
                &library,
                run_id,
                entry_id,
                recovery_evidence_id,
            )?)),
            SyncRequest::Versions { job_id } => Ok(SyncReply::Versions(
                self.retained_versions(&library, &job_id)?,
            )),
            SyncRequest::ReviewRestoreRetained { job_id } => Ok(SyncReply::Review(
                self.review_retained_restore(&library, &job_id)?,
            )),
            SyncRequest::ReviewAppend {
                run_id,
                keep_destination,
            } => Ok(SyncReply::Review(self.review_append(
                &library,
                run_id,
                keep_destination,
            )?)),
            SyncRequest::Review { review_id } => {
                Ok(SyncReply::Review(self.review(&review_id)?.view.clone()))
            }
            SyncRequest::Approve {
                review_id,
                acknowledge_uncopied,
                acknowledge_metadata_limits,
            } => Ok(SyncReply::Approval(self.approve(
                &library,
                review_id,
                acknowledge_uncopied,
                acknowledge_metadata_limits,
            )?)),
            SyncRequest::Start { approval_id } => {
                Ok(SyncReply::Job(self.start(library, approval_id)?))
            }
            SyncRequest::List {} => Ok(SyncReply::Jobs(self.list()?)),
            SyncRequest::Job { job_id } => Ok(SyncReply::Job(self.job(&job_id)?)),
            SyncRequest::Pause { job_id } => {
                self.control(&job_id, "pause")?;
                Ok(SyncReply::Job(self.job(&job_id)?))
            }
            SyncRequest::Resume { job_id } => {
                self.control(&job_id, "resume")?;
                Ok(SyncReply::Job(self.job(&job_id)?))
            }
            SyncRequest::Cancel { job_id } => {
                self.control(&job_id, "cancel")?;
                Ok(SyncReply::Job(self.job(&job_id)?))
            }
            SyncRequest::RecoveryReview { job_id } => {
                Ok(SyncReply::Review(self.recovery_review(&library, &job_id)?))
            }
            SyncRequest::SetLimit { bytes_per_second } => {
                let rate = bytes_per_second.parse::<u64>().map_err(|_| {
                    invalid("Enter a whole byte rate from 1024 through 1073741824.")
                })?;
                PacedIo::shared().set_rate(rate)?;
                Ok(SyncReply::Capabilities(Self::capabilities()?))
            }
        }
    }
    fn review(&self, id: &str) -> Result<Arc<Review>> {
        crate::operations::validate_id(id)?;
        self.reviews
            .lock()
            .map_err(|_| failed("Sync reviews are unavailable."))?
            .get(id)
            .filter(|r| r.created.elapsed() < Duration::from_secs(600))
            .cloned()
            .ok_or_else(|| {
                invalid(
                    "This review expired or belongs to another application session. Inspect again.",
                )
            })
    }
    fn retain_review(&self, review: Review) -> Result<SyncReview> {
        if self.stopping.load(Ordering::Acquire) {
            return Err(AppError::new(
                ErrorCode::Busy,
                "The sync service is stopping; new reviews are unavailable.",
            ));
        }
        let mut reviews = self
            .reviews
            .lock()
            .map_err(|_| failed("Sync reviews are unavailable."))?;
        reviews.retain(|_, r| r.created.elapsed() < Duration::from_secs(600));
        if reviews.len() >= MAX_REVIEWS {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Too many pending sync reviews. Wait for their ten-minute expiry.",
            ));
        }
        let view = review.view.clone();
        reviews.insert(view.id.clone(), Arc::new(review));
        Ok(view)
    }
    fn review_append(
        &self,
        library: &Library,
        run_id: String,
        keep: Vec<String>,
    ) -> Result<SyncReview> {
        if self
            .active
            .lock()
            .map_err(|_| failed("Sync worker unavailable."))?
            .is_some()
        {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Wait for the current sync job to stop before reviewing another.",
            ));
        }
        let plan = library.backup_append_review(run_id.clone(), keep.clone())?;
        let scope = library.backup_sync_scope(run_id, keep)?;
        if plan.copy_files == 0 {
            return Err(invalid(
                "No source-only files remain to append. Destination-only and differing versions stay retained.",
            ));
        }
        library.validate_backup_sync_scope(&scope)?;
        preparation::validate_targets(&scope)?;
        let destination = source::decode_path(&scope.destination)?;
        let root = output_chain(&destination)?;
        let namespace = relative(&source::decode_path(&scope.namespace)?)?;
        let parents = preparation::capture_parents(root.leaf(), &namespace)?;
        let required = u128::from(
            plan.copy_bytes
                .parse::<u64>()
                .map_err(|_| invalid("Copy scope is too large."))?,
        )
        .saturating_add(RESERVE)
        .saturating_add(u128::from(plan.copy_files) * 8192);
        let stats = fs2::statvfs(&destination)?;
        let enough = u128::from(stats.available_space()) >= required;
        let id = uuid::Uuid::new_v4().to_string();
        let mut view = SyncReview {
            id,
            job_id: None,
            run_id: scope.run_id.clone(),
            action: SyncAction::PrepareNames,
            digest: String::new(),
            source_root_id: scope.root_id.clone(),
            source_display_path: source::decode_path(&scope.source)?
                .to_string_lossy()
                .into_owned(),
            dataset_display_path: destination
                .join(source::decode_path(&scope.namespace)?)
                .to_string_lossy()
                .into_owned(),
            copy_files: plan.copy_files,
            copy_bytes: plan.copy_bytes,
            retained_destination_files: plan.retained_destination_files,
            uncopied_source_revisions: plan.uncopied_source_revisions,
            available_bytes: Some(stats.available_space().to_string()),
            required_bytes: required.to_string(),
            metadata_warning: METADATA_WARNING.into(),
            execution_available: Self::capabilities()?.execution_available && enough,
            reason: if enough {
                "Approval prepares exclusive names/stages only. Copying needs a second review and approval after native preparation.".into()
            } else {
                "Destination space is below the current estimate plus reserve. No space is reserved; remedy capacity and re-review.".into()
            },
            approval_lifetime_seconds: 600,
            items: plan
                .items
                .into_iter()
                .map(|i| SyncPlanItem {
                    entry_id: i.entry_id,
                    display_path: i.display_path,
                    source_bytes: i.source_bytes,
                    action: match i.action {
                        crate::backup::append::AppendAction::CopyNew => SyncItemAction::CopyNew,
                        crate::backup::append::AppendAction::KeepDestination => {
                            SyncItemAction::KeepDestination
                        }
                        crate::backup::append::AppendAction::LeaveUnchanged => {
                            SyncItemAction::LeaveUnchanged
                        }
                    },
                })
                .collect(),
        };
        view.digest = blake3::hash(encode(&(&scope, &parents, &view))?.as_bytes())
            .to_hex()
            .to_string();
        let binding = Binding {
            scope,
            view: view.clone(),
            parents,
            retained: None,
        };
        self.retain_review(Review {
            binding,
            view,
            stage: Stage::Prepare,
            created: Instant::now(),
        })
    }
    fn approve(
        &self,
        library: &Library,
        id: String,
        ack_uncopied: bool,
        ack_metadata: bool,
    ) -> Result<SyncApproval> {
        if self.stopping.load(Ordering::Acquire) {
            return Err(AppError::new(
                ErrorCode::Busy,
                "The sync service is stopping; new approvals are unavailable.",
            ));
        }
        let review = self.review(&id)?;
        if !review.view.execution_available
            || !ack_metadata
            || (review.view.uncopied_source_revisions > 0 && !ack_uncopied)
        {
            return Err(invalid(
                "Review scope, uncopied revisions and metadata limitations before approving this available action.",
            ));
        }
        self.validate_binding(library, &review.binding)?;
        self.capacity(&review.binding)?;
        if matches!(review.stage, Stage::Retained) {
            self.validate_retained_review(library, &review)?;
        }
        if let Stage::Copy(plan) | Stage::Resume(plan, _) = &review.stage {
            let id = review
                .view
                .job_id
                .as_deref()
                .ok_or_else(|| failed("Copy review has no durable job."))?;
            preparation::validate_plan(self, id, &review.binding, plan)?;
            let inspection = self.inspect_plan(library, id, &review.binding, plan)?;
            if matches!(&review.stage, Stage::Resume(_, _))
                && inspection.digest() != review.view.digest
            {
                return Err(failed(
                    "Recovery observations changed after review. Inspect and approve again.",
                ));
            }
        }
        if self.stopping.load(Ordering::Acquire) {
            return Err(AppError::new(
                ErrorCode::Busy,
                "The sync service is stopping; new approvals are unavailable.",
            ));
        }
        let token = self
            .approvals
            .issue(expected(&review.view, &review.binding.scope.run_id)?)?;
        self.grants
            .lock()
            .map_err(|_| failed("Sync approvals unavailable."))?
            .insert(token.clone(), id.clone());
        Ok(SyncApproval {
            id: token,
            review_id: id,
            action: review.view.action,
        })
    }
    fn capacity(&self, binding: &Binding) -> Result<()> {
        let target = source::decode_path(&binding.scope.destination)?;
        let available = u128::from(fs2::statvfs(&target)?.available_space());
        let required = binding
            .view
            .required_bytes
            .parse::<u128>()
            .map_err(|_| failed("Invalid frozen capacity estimate."))?;
        if available < required
            || u128::from(fs2::statvfs(&self.data_directory)?.available_space()) < RESERVE
        {
            return Err(AppError::new(
                ErrorCode::Io,
                "Current destination or journal capacity is below the conservative estimate. Retain all data and remedy capacity before a fresh approval.",
            ));
        }
        Ok(())
    }
    fn start(self: &Arc<Self>, library: Arc<Library>, token: String) -> Result<SyncJob> {
        if self.stopping.load(Ordering::Acquire) {
            return Err(AppError::new(
                ErrorCode::Busy,
                "The sync service is stopping; new actions are unavailable.",
            ));
        }
        let mut active = self
            .active
            .lock()
            .map_err(|_| failed("Sync worker unavailable."))?;
        if self.stopping.load(Ordering::Acquire) {
            return Err(AppError::new(
                ErrorCode::Busy,
                "The sync service is stopping; new actions are unavailable.",
            ));
        }
        if active.is_some() {
            return Err(AppError::new(
                ErrorCode::Busy,
                "One sync job is already active; no additional transfer is queued.",
            ));
        }
        let id = self
            .grants
            .lock()
            .map_err(|_| failed("Sync approvals unavailable."))?
            .remove(&token)
            .ok_or_else(|| invalid("Unknown or already consumed sync approval."))?;
        let review = self.review(&id)?;
        let proof = self.approvals.consume(
            &token,
            &expected(&review.view, &review.binding.scope.run_id)?,
        )?;
        self.validate_binding(&library, &review.binding)?;
        self.capacity(&review.binding)?;
        if matches!(review.stage, Stage::Retained) {
            self.validate_retained_review(&library, &review)?;
        }
        let job_id = review
            .view
            .job_id
            .clone()
            .or_else(|| {
                review
                    .binding
                    .retained
                    .as_ref()
                    .map(|r| r.operation_id.clone())
            })
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        if review.view.job_id.is_none() {
            self.create_job(&job_id, &review.binding)?;
        } else {
            self.update(
                &job_id,
                SyncState::Queued,
                "queued",
                review.view.action,
                None,
                "Approved action queued; native preconditions will be checked again before I/O.",
            )?;
        }
        let cancel = self.cancel.clone();
        cancel.store(self.stopping.load(Ordering::Acquire), Ordering::Release);
        *active = Some(Active {
            id: job_id.clone(),
            root: review.binding.scope.root_id.clone(),
            cancel: cancel.clone(),
        });
        let workflow = self.clone();
        let worker_id = job_id.clone();
        let worker_action = review.view.action;
        let spawn = std::thread::Builder::new()
            .name("stillharbour-sync".into())
            .spawn(move || {
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    workflow.run(&library, &worker_id, &review, proof, &cancel)
                }))
                .unwrap_or_else(|_| Err(failed("The sync worker stopped unexpectedly.")));
                if let Err(error) = outcome {
                    let _ = workflow.update(
                        &worker_id,
                        SyncState::NeedsRecovery,
                        "needs_recovery",
                        review.view.action,
                        None,
                        &format!(
                            "{} Retain all outputs and inspect recovery; do not blindly retry.",
                            error.message
                        ),
                    );
                }
                let _ = PacedIo::shared().set_paused(false);
                if let Ok(mut current) = workflow.active.lock() {
                    *current = None;
                }
            });
        if let Err(error) = spawn {
            *active = None;
            self.update(
                &job_id,
                SyncState::NeedsRecovery,
                "worker_unavailable",
                worker_action,
                None,
                "The worker could not start. Retain the immutable plan and inspect recovery.",
            )?;
            return Err(failed("Sync worker could not start.").caused_by(error));
        }
        drop(active);
        self.job(&job_id)
    }
    fn control(&self, id: &str, command: &str) -> Result<()> {
        crate::operations::validate_id(id)?;
        let active = self
            .active
            .lock()
            .map_err(|_| failed("Sync worker unavailable."))?;
        let job = active.as_ref().filter(|a| a.id == id).ok_or_else(|| {
            invalid("This job has no active worker. Inspect recovery instead of replaying it.")
        })?;
        let action = self.job(id)?.action;
        match command {
            "pause" => {
                PacedIo::shared().set_paused(true)?;
                self.update(id,SyncState::Paused,"paused_at_io_boundary",action,None,"Shared owned I/O paused. An already admitted syscall may finish; metadata and opaque helper I/O are outside this pause.")
            }
            "resume" => {
                PacedIo::shared().set_paused(false)?;
                self.update(
                    id,
                    SyncState::Running,
                    "running",
                    action,
                    None,
                    "Shared owned I/O resumed; native preconditions remain checked.",
                )
            }
            "cancel" => {
                job.cancel.store(true, Ordering::Release);
                PacedIo::shared().set_paused(false)?;
                self.update(id,SyncState::CancelRequested,"cancel_requested",action,None,"Cancellation requested. Existing sources, destinations and partial stages remain retained.")
            }
            _ => Err(invalid("Unknown sync control.")),
        }
    }
    fn run(
        &self,
        library: &Library,
        id: &str,
        review: &Review,
        proof: approval::Proof,
        cancel: &AtomicBool,
    ) -> Result<()> {
        if self.stopping.load(Ordering::Acquire) {
            return Err(AppError::new(
                ErrorCode::Busy,
                "The sync service is stopping; all outputs remain retained.",
            ));
        }
        preparation::canceled(cancel)?;
        proof.authorize(&expected(&review.view, &review.binding.scope.run_id)?)?;
        let binding = &review.binding;
        let src = source::decode_path(&binding.scope.source)?;
        let dst = source::decode_path(&binding.scope.destination)?;
        let lease = crate::jobs::operation_locks::Registry::shared().try_acquire(vec![
            crate::jobs::operation_locks::Scope::new(
                DirectoryChain::open(&src)?,
                crate::jobs::operation_locks::Access::Read,
            ),
            crate::jobs::operation_locks::Scope::new(
                output_chain(&dst)?,
                crate::jobs::operation_locks::Access::Write,
            ),
        ])?;
        let result = (|| -> Result<()> {
            self.validate_binding(library, binding)?;
            self.capacity(binding)?;
            preparation::validate_parents(&dst, &binding.parents)?;
            #[cfg(test)]
            self.boundary(Fault::BeforeRun)?;
            self.update(
                id,
                SyncState::Running,
                "validating",
                review.view.action,
                None,
                "Validating the exact source, destination and durable journal.",
            )?;
            match &review.stage {
                Stage::Retained => {
                    self.run_retained(library, id, binding, cancel)?;
                }
                Stage::Prepare => {
                    let plan = preparation::prepare(self, library, id, binding, cancel)?;
                    self.bind_plan(id, &plan)?;
                    #[cfg(test)]
                    self.boundary(Fault::PlanBound)?;
                    let view = self.copy_review(id, binding.clone(), plan)?;
                    self.update(id,SyncState::NeedsReview,"prepared",SyncAction::PrepareNames,Some(view.id),"Native names and separate staging are prepared. Review and separately approve copying; no source bytes have been published.")?;
                }
                Stage::Copy(plan) | Stage::Resume(plan, _) => {
                    preparation::validate_plan(self, id, binding, plan)?;
                    if matches!(&review.stage, Stage::Copy(_)) {
                        self.inspect_plan(library, id, binding, plan)?;
                    }
                    let a = source::open_root(&src, Some(&binding.scope.source_identity))?;
                    let b = output_chain(&dst)?;
                    let journal =
                        storage::SyncJournal::new(self.catalog.clone(), id.into(), plan.clone())?;
                    let validate = || {
                        library.validate_backup_sync_scope(&binding.scope)?;
                        preparation::validate_parents(&dst, &binding.parents)
                    };
                    let io = PacedIo::shared();
                    let outcome = match &review.stage {
                        Stage::Copy(_) => CopyContext {
                            plan,
                            journal: &journal,
                            source_root: &a,
                            destination: b.leaf(),
                            io: &io,
                            cancel,
                            validate_scope: &validate,
                        }
                        .execute(self.copy_fault())?,
                        Stage::Resume(_, inspection) => {
                            let (cursors, terminal, digest) = journal.history()?;
                            let inputs = CopyInputs {
                                source_root: &a,
                                destination: b.leaf(),
                                io: &io,
                                cancel,
                                validate_scope: &validate,
                            };
                            resume::Continuation {
                                plan,
                                journal: &journal,
                                cursors: &cursors,
                                terminal,
                                review: inspection,
                                history_digest: digest,
                                inputs: &inputs,
                                fault: self.copy_fault(),
                                progress: None,
                            }
                            .execute()?
                        }
                        Stage::Prepare | Stage::Retained => unreachable!(),
                    };
                    let state = if outcome == CopyOutcome::CompletedVerified {
                        if binding.view.uncopied_source_revisions == 0 {
                            SyncState::CompletedVerified
                        } else {
                            SyncState::CompletedPartial
                        }
                    } else {
                        SyncState::NeedsRecovery
                    };
                    let message = if outcome == CopyOutcome::CompletedVerified {
                        "Every selected append was read back and fully compared. Sources and destination-only/differing files remain retained. Metadata fidelity and independent recovery are not certified."
                    } else {
                        "Transfer stopped with retained stages/outputs. Inspect current objects and approve fresh recovery; no source cleanup occurred."
                    };
                    self.update(
                        id,
                        state,
                        "verification_complete",
                        review.view.action,
                        None,
                        message,
                    )?;
                }
            }
            Ok(())
        })();
        if let Err(error) = &result {
            self.update(id,SyncState::NeedsRecovery,"needs_recovery",review.view.action,None,&format!("{} All sources, history and partial outputs remain retained; inspect before retry.",error.message))?;
        }
        drop(lease);
        result
    }
    fn copy_review(&self, id: &str, binding: Binding, plan: CopyPlan) -> Result<SyncReview> {
        let mut view = binding.view.clone();
        view.id = uuid::Uuid::new_v4().to_string();
        view.job_id = Some(id.into());
        view.action = SyncAction::CopyFiles;
        view.digest = plan.digest.clone();
        view.reason="Copy and verify into the prepared native namespace. Existing destination files and all sources stay retained.".into();
        view.execution_available = Self::capabilities()?.execution_available;
        self.retain_review(Review {
            binding,
            view,
            stage: Stage::Copy(plan),
            created: Instant::now(),
        })
    }
    fn recovery_review(&self, library: &Library, id: &str) -> Result<SyncReview> {
        if self
            .active
            .lock()
            .map_err(|_| failed("Sync worker unavailable."))?
            .as_ref()
            .is_some_and(|a| a.id == id)
        {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Wait for this worker to stop before recovery inspection.",
            ));
        }
        let binding = self.binding(id)?;
        if binding.retained.is_some() {
            return self.retained_recovery_review(library, id, binding);
        }
        library.validate_backup_sync_scope(&binding.scope)?;
        let Some(plan) = self.plan(id)? else {
            return preparation::review_incomplete(self, library, id, binding);
        };
        preparation::validate_plan(self, id, &binding, &plan)?;
        let inspection = self.inspect_plan(library, id, &binding, &plan)?;
        let mut view = binding.view.clone();
        view.id = uuid::Uuid::new_v4().to_string();
        view.job_id = Some(id.into());
        view.action = SyncAction::ResumeFiles;
        view.digest = inspection.digest().into();
        view.reason="Fresh native recovery inspection found journal-bound stages/published objects. This new approval continues only the existing immutable stream; it never recreates a missing object or overwrites an occupant.".into();
        view.execution_available = Self::capabilities()?.execution_available;
        self.retain_review(Review {
            binding,
            view,
            stage: Stage::Resume(plan, inspection),
            created: Instant::now(),
        })
    }
    fn inspect_plan(
        &self,
        library: &Library,
        id: &str,
        binding: &Binding,
        plan: &CopyPlan,
    ) -> Result<resume::Review> {
        let src = source::decode_path(&binding.scope.source)?;
        let dst = source::decode_path(&binding.scope.destination)?;
        let a = source::open_root(&src, Some(&binding.scope.source_identity))?;
        let b = output_chain(&dst)?;
        let journal = storage::SyncJournal::new(self.catalog.clone(), id.into(), plan.clone())?;
        let (cursors, _, digest) = journal.history()?;
        let validate = || {
            library.validate_backup_sync_scope(&binding.scope)?;
            preparation::validate_parents(&dst, &binding.parents)
        };
        let io = PacedIo::shared();
        let cancel = AtomicBool::new(false);
        resume::Review::capture(
            plan,
            &cursors,
            digest,
            &CopyInputs {
                source_root: &a,
                destination: b.leaf(),
                io: &io,
                cancel: &cancel,
                validate_scope: &validate,
            },
        )
    }
}

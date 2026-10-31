//! Application workflow; native filesystem effects remain in the central
//! preparation/copy engine, after a separate exact session approval.
use super::{
    CopyInputs, CopyJournal, CopyOutcome, Object,
    approval::Approvals,
    catalog_journal::{CatalogJournal, namespace_journal::NamespaceJournal},
    failed,
    sources::Snapshot,
};
use crate::{
    catalog::Catalog,
    domain::{AppError, ErrorCode, Result},
    filesystem::FileIdentity,
    jobs::{JobScheduler, operation_locks::Access, paced_io::PacedIo},
    library::{Library, native, operation_jobs},
    operations::*,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

struct Session {
    namespace: Arc<NamespaceJournal>,
    selection: String,
    selected_path: PathBuf,
    ready: Mutex<Option<Arc<CatalogJournal>>>,
}
struct Active {
    id: String,
    root: String,
    session: String,
    cancel: Arc<AtomicBool>,
}
struct ActiveGuard {
    workflow: Arc<CopyWorkflow>,
    id: String,
}
impl Drop for ActiveGuard {
    fn drop(&mut self) {
        let _ = PacedIo::shared().set_paused(false);
        if let Ok(mut active) = self.workflow.active.lock()
            && active.as_ref().is_some_and(|active| active.id == self.id)
        {
            *active = None;
        }
    }
}
pub(crate) struct CopyWorkflow {
    catalog: Arc<Catalog>,
    data_directory: PathBuf,
    data_identity: Object,
    data_volume: String,
    sessions: Mutex<BTreeMap<String, Option<Arc<Session>>>>,
    approvals: Approvals,
    gate: JobScheduler,
    active: Mutex<Option<Active>>,
}
impl CopyWorkflow {
    pub(crate) fn job(&self, id: String) -> Result<CopyWorkflowJob> {
        let job = operation_jobs::load(&self.catalog, id.clone())?;
        let live = self
            .active
            .lock()
            .map_err(|_| failed("Copy job state is unavailable."))?
            .as_ref()
            .is_some_and(|active| active.id == id);
        if !live
            && matches!(
                job.state,
                CopyJobState::Queued
                    | CopyJobState::Running
                    | CopyJobState::PauseRequested
                    | CopyJobState::CancelRequested
            )
        {
            self.update(&id,CopyJobState::NeedsRecovery,job.verified_files,job.ready_review_id,"The worker stopped before committing a final job state. Inspect retained files and journal.",None)?;
            return operation_jobs::load(&self.catalog, id);
        }
        Ok(job)
    }
    pub(crate) fn inspect(
        &self,
        id: String,
        selection: String,
        path: PathBuf,
        identity: FileIdentity,
        volume: String,
    ) -> Result<CopyInspection> {
        let session = self.session(&id)?;
        Self::check_selection(&session, &selection, &path, &identity, &volume)?;
        let ready = session
            .ready
            .lock()
            .map_err(|_| failed("Ready copy state is unavailable."))?
            .clone()
            .ok_or_else(|| {
                failed("Preparation is incomplete; retain its namespace and journal.")
            })?;
        let scope = session
            .namespace
            .sources()
            .open_scope(&self.catalog, Access::Read)?;
        let root = super::native::read_directory(
            scope.directory(1).leaf(),
            session.namespace.namespace(),
        )?;
        if !ready.plan().destination.matches(&root)? {
            return Err(failed("Retained output namespace identity changed."));
        }
        let io = PacedIo::shared();
        let cancel = AtomicBool::new(false);
        let validate = || session.namespace.sources().validate_roots(&self.catalog);
        let states = ready.inspect(CopyInputs {
            source_root: scope.directory(0).leaf(),
            destination: &root,
            io: &io,
            cancel: &cancel,
            validate_scope: &validate,
        })?;
        let all = states
            .iter()
            .all(|state| *state == super::RecoveredItem::PublishedVerified);
        Ok(CopyInspection {
            review_id: id,
            states,
            all_published_bytes_verified: all,
        })
    }
    pub(crate) fn new(catalog: Arc<Catalog>, data_directory: PathBuf) -> Result<Arc<Self>> {
        let data = native::open_root(&data_directory, None)?;
        Ok(Arc::new(Self {
            catalog,
            data_directory,
            data_identity: Object::of(&data)?,
            data_volume: native::volume_token(&data)?,
            sessions: Mutex::new(BTreeMap::new()),
            approvals: Approvals::new(),
            gate: JobScheduler::new(1)?,
            active: Mutex::new(None),
        }))
    }
    fn session(&self, id: &str) -> Result<Arc<Session>> {
        crate::operations::validate_id(id)?;
        let entries = self
            .sessions
            .lock()
            .map_err(|_| failed("Copy review state is unavailable."))?;
        for (key, value) in entries.iter() {
            if let Some(session) = value
                && (key == id
                    || session
                        .ready
                        .lock()
                        .map_err(|_| failed("Ready copy state is unavailable."))?
                        .as_ref()
                        .is_some_and(|ready| ready.plan().id == id))
            {
                return Ok(session.clone());
            }
        }
        Err(failed(
            "This copy review belongs to an earlier session. Reopen and authorize its destination again.",
        ))
    }
    fn check_selection(
        session: &Session,
        id: &str,
        path: &Path,
        identity: &FileIdentity,
        volume: &str,
    ) -> Result<()> {
        let expected = &session.namespace.sources().destination;
        if session.selection != id
            || identity.volume != expected.volume
            || identity.file != expected.file
            || volume != session.namespace.sources().destination_volume
        {
            return Err(failed(
                "The native destination selection changed. Review the output folder again.",
            ));
        }
        let selected = native::open_root(path, Some(identity))?;
        if !expected.matches(&selected)? || native::volume_token(&selected)? != volume {
            return Err(failed(
                "The selected destination changed identity or volume.",
            ));
        }
        Ok(())
    }
    fn check_space(
        &self,
        session: &Session,
        assessment: &CopyAssessment,
        initial: bool,
    ) -> Result<()> {
        let data = native::open_root(&self.data_directory, None)?;
        let output = native::open_root(&session.selected_path, None)?;
        if !self.data_identity.matches(&data)?
            || native::volume_token(&data)? != self.data_volume
            || !session.namespace.sources().destination.matches(&output)?
            || native::volume_token(&output)? != session.namespace.sources().destination_volume
        {
            return Err(failed(
                "Output/application-journal storage identity changed.",
            ));
        }
        let destination_stats = fs2::statvfs(&session.selected_path)?;
        let journal_stats = fs2::statvfs(&self.data_directory)?;
        let output_required = if initial {
            assessment
                .capacity
                .estimated_peak_additional_bytes
                .parse::<u128>()
                .map_err(|_| failed("Invalid retained copy capacity estimate."))?
        } else {
            64 * 1024 * 1024
        };
        let journal_required: u128 = if initial {
            128 * 1024 * 1024
        } else {
            16 * 1024 * 1024
        };
        let shared = session.namespace.sources().destination_volume == self.data_volume;
        let output_required = output_required
            + if shared {
                journal_required.saturating_sub(if initial { 32 * 1024 * 1024 } else { 0 })
            } else {
                0
            };
        if assessment.capacity.status == CopyCapacityStatus::InsufficientForEstimate
            || u128::from(destination_stats.available_space()) < output_required
            || (!shared && u128::from(journal_stats.available_space()) < journal_required)
        {
            return Err(failed(
                "Output/application-journal free space is below the operation estimate/reserve. Existing files are retained.",
            ));
        }
        if !self
            .data_identity
            .matches(&native::open_root(&self.data_directory, None)?)?
            || !session
                .namespace
                .sources()
                .destination
                .matches(&native::open_root(&session.selected_path, None)?)?
        {
            return Err(failed("Storage changed during free-space observation."));
        }
        Ok(())
    }
    fn fresh(&self, library: &Library, session: &Session) -> Result<CopyAssessment> {
        let source = session.namespace.sources();
        let assessment = library.organization_copy_prepared_assessment(
            source.owner_id.clone(),
            source.manifest_id.clone(),
        )?;
        let current = Snapshot::build(
            &self.catalog,
            source.owner_id.clone(),
            source.manifest_id.clone(),
        )?;
        if serde_json::to_vec(&current).ok() != serde_json::to_vec(source).ok() {
            return Err(failed(
                "The reviewed native source scope changed. Build a fresh review.",
            ));
        }
        self.check_space(session, &assessment, true)?;
        Ok(assessment)
    }
    fn view(&self, session: &Session, assessment: CopyAssessment) -> Result<CopyActionReview> {
        let ready = session
            .ready
            .lock()
            .map_err(|_| failed("Ready copy state is unavailable."))?;
        let (id, digest, action) = if let Some(ready) = ready.as_ref() {
            (
                ready.plan().id.clone(),
                ready.digest(),
                CopyAction::CopyFiles,
            )
        } else {
            (
                session.namespace.id().into(),
                session.namespace.digest(),
                CopyAction::PrepareNames,
            )
        };
        let source = session.namespace.sources();
        let files = source
            .files
            .iter()
            .map(|file| {
                Ok(CopyReviewFile {
                    id: file.id.clone(),
                    source_path: native::decode_path(&file.location.relative_native)?
                        .to_string_lossy()
                        .into_owned(),
                    target_path: format!("{}/{}", session.namespace.namespace(), file.target),
                    bytes: file.bytes.to_string(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(CopyActionReview {
            id,
            plan_id: source.owner_id.clone(),
            manifest_id: source.manifest_id.clone(),
            digest,
            action,
            source_root_id: source.root_id.clone(),
            destination_display_path: session.selected_path.to_string_lossy().into_owned(),
            output_folder: session.namespace.namespace().into(),
            files,
            assessment,
            sources_retained: true,
            independent_backup: false,
        })
    }
    pub(crate) fn create_review(
        &self,
        library: &Library,
        owner: String,
        manifest: String,
        selection: String,
        path: PathBuf,
        selected_identity: (FileIdentity, String),
    ) -> Result<CopyActionReview> {
        let (identity, volume) = selected_identity;
        crate::operations::validate_id(&selection)?;
        let token = uuid::Uuid::new_v4().to_string();
        {
            let mut entries = self
                .sessions
                .lock()
                .map_err(|_| failed("Copy review state is unavailable."))?;
            if entries.len() >= 64 {
                return Err(AppError::new(
                    ErrorCode::Busy,
                    "This session has 64 retained copy reviews. Close unused reviews before creating more.",
                ));
            }
            entries.insert(token.clone(), None);
        }
        let result = (|| {
            let assessment =
                library.organization_copy_assessment(owner.clone(), manifest.clone())?;
            let sources = Snapshot::build(&self.catalog, owner, manifest)?;
            if identity.volume != sources.destination.volume
                || identity.file != sources.destination.file
                || volume != sources.destination_volume
            {
                return Err(failed(
                    "Select the saved draft's actual destination folder.",
                ));
            }
            let namespace = Arc::new(NamespaceJournal::bind_snapshot(
                self.catalog.clone(),
                sources,
                token.clone(),
                true,
            )?);
            let session = Arc::new(Session {
                namespace,
                selection: selection.clone(),
                selected_path: path.clone(),
                ready: Mutex::new(None),
            });
            Self::check_selection(&session, &selection, &path, &identity, &volume)?;
            let view = self.view(&session, assessment)?;
            self.sessions
                .lock()
                .map_err(|_| failed("Copy review state is unavailable."))?
                .insert(token.clone(), Some(session));
            Ok(view)
        })();
        if result.is_err() {
            self.sessions
                .lock()
                .map_err(|_| failed("Copy review state is unavailable."))?
                .remove(&token);
        }
        result
    }
    pub(crate) fn get_review(
        &self,
        library: &Library,
        id: String,
        selection: String,
        path: PathBuf,
        identity: FileIdentity,
        volume: String,
    ) -> Result<CopyActionReview> {
        let session = self.session(&id)?;
        Self::check_selection(&session, &selection, &path, &identity, &volume)?;
        let assessment = library.organization_copy_prepared_assessment(
            session.namespace.sources().owner_id.clone(),
            session.namespace.sources().manifest_id.clone(),
        )?;
        self.view(&session, assessment)
    }
    pub(crate) fn approve(
        &self,
        library: &Library,
        input: CopyApprovalInput,
        selection: String,
        path: PathBuf,
        identity: FileIdentity,
        volume: String,
    ) -> Result<CopyActionApproval> {
        let session = self.session(&input.review_id)?;
        Self::check_selection(&session, &selection, &path, &identity, &volume)?;
        if self
            .active
            .lock()
            .map_err(|_| failed("Copy job state is unavailable."))?
            .is_some()
        {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Finish the active organization job before approving another.",
            ));
        }
        let assessment = self.fresh(library, &session)?;
        let view = self.view(&session, assessment)?;
        if input.review_id != view.id || input.digest != view.digest || input.action != view.action
        {
            return Err(failed("This approval belongs to a changed copy review."));
        }
        let review = match input.action {
            CopyAction::PrepareNames => session.namespace.preparation_review(&selection)?,
            CopyAction::CopyFiles => {
                if !input.fidelity_limits_acknowledged {
                    return Err(failed(
                        "Acknowledge the displayed byte-only copy fidelity limits before approval.",
                    ));
                }
                let ready = session
                    .ready
                    .lock()
                    .map_err(|_| failed("Ready copy state is unavailable."))?;
                let ready = ready
                    .as_ref()
                    .ok_or_else(|| failed("Prepare the exact output names first."))?;
                if ready.history_count()? != 0 {
                    return Err(failed(
                        "This copy has execution history. Inspect recovery before further writes.",
                    ));
                }
                let scope = session
                    .namespace
                    .sources()
                    .open_scope(&self.catalog, Access::Read)?;
                session
                    .namespace
                    .validate_native(scope.directory(1).leaf())?;
                ready.copy_review(&selection)?
            }
        };
        Ok(CopyActionApproval {
            id: self.approvals.issue(review)?,
            review_id: view.id,
            action: view.action,
            expires_in_seconds: 600,
        })
    }
    pub(crate) fn start(
        self: &Arc<Self>,
        library: Arc<Library>,
        input: CopyStartInput,
        selection: String,
        path: PathBuf,
        identity: FileIdentity,
        volume: String,
    ) -> Result<CopyWorkflowJob> {
        let session = self.session(&input.review_id)?;
        Self::check_selection(&session, &selection, &path, &identity, &volume)?;
        let permit = self.gate.try_start()?;
        let assessment = self.fresh(&library, &session)?;
        let view = self.view(&session, assessment.clone())?;
        if view.id != input.review_id || view.digest != input.digest {
            return Err(failed("The action review changed before job admission."));
        }
        let ready = session
            .ready
            .lock()
            .map_err(|_| failed("Ready copy state is unavailable."))?
            .clone();
        let review = if let Some(ready) = ready.as_ref() {
            if ready.history_count()? != 0 {
                return Err(failed(
                    "This copy has execution history. Inspect retained files before further writes.",
                ));
            }
            ready.copy_review(&selection)?
        } else {
            session.namespace.preparation_review(&selection)?
        };
        let proof = self.approvals.consume(&input.approval_id, &review)?;
        let now = crate::library::store::now();
        let job = CopyWorkflowJob {
            id: uuid::Uuid::new_v4().to_string(),
            plan_id: view.plan_id,
            review_id: view.id,
            source_root_id: view.source_root_id,
            action: view.action,
            state: CopyJobState::Queued,
            total_files: view.files.len() as u32,
            verified_files: 0,
            ready_review_id: None,
            message: "Approved organization work queued; originals remain in place.".into(),
            error: None,
            started_at: now.clone(),
            updated_at: now,
        };
        operation_jobs::create(&self.catalog, job.clone(), view.digest)?;
        let cancel = permit.cancellation();
        *self
            .active
            .lock()
            .map_err(|_| failed("Copy job state is unavailable."))? = Some(Active {
            id: job.id.clone(),
            root: job.source_root_id.clone(),
            session: session.namespace.id().into(),
            cancel: cancel.clone(),
        });
        let this = self.clone();
        let job_id = job.id.clone();
        let returned = job.clone();
        let spawned=std::thread::Builder::new().name("organization-operation".into()).spawn(move|| {
            let _permit=permit;
            let _active=ActiveGuard{workflow:this.clone(),id:job_id.clone()};
            let result=(||->Result<(CopyJobState,u32,Option<String>,String)> {
                if cancel.load(Ordering::Acquire) {return Ok((CopyJobState::CanceledWithPartialCopies,0,None,"Canceled before native work; originals remain unchanged.".into()));}
                this.update(&job_id,CopyJobState::Running,0,None,"Preparing approved native operation",None)?;
                let scope=session.namespace.sources().open_scope(&this.catalog,Access::Write)?;
                this.check_space(&session,&assessment,true)?;
                if let Some(ready)=ready.as_ref() {
                    session.namespace.validate_native(scope.directory(1).leaf())?;
                    let root=super::native::read_directory(scope.directory(1).leaf(),session.namespace.namespace())?;
                    if !ready.plan().destination.matches(&root)? {return Err(failed("Prepared namespace identity changed before copying."));}
                    let io=PacedIo::shared();
                    let validate=|| {
                        session.namespace.sources().validate_roots(&this.catalog)?;
                        let entry=super::native::lookup(scope.directory(1).leaf(),session.namespace.namespace())?.ok_or_else(||failed("Prepared namespace disappeared."))?;
                        if entry.kind!=super::native::EntryKind::Directory || entry.object!=ready.plan().destination {return Err(failed("Prepared namespace changed membership."));}
                        this.check_space(&session,&assessment,false)
                    };
                    let progress=|verified| {
                        let current=operation_jobs::load(&this.catalog,job_id.clone())?;
                        this.update(&job_id,current.state,verified,current.ready_review_id,"Verified file completion recorded; originals remain in place.",None)
                    };
                    let outcome=ready.execute_approved(CopyInputs{source_root:scope.directory(0).leaf(),destination:&root,io:&io,cancel:&cancel,validate_scope:&validate},&selection,proof,None,Some(&progress))?;
                    let verified=ready.verified_files()?;
                    Ok(match outcome {
                        CopyOutcome::CompletedVerified=>(CopyJobState::CompletedVerified,verified,None,"All selected file bytes were copied and fully compared. Originals remain; filesystem metadata fidelity limits still apply.".into()),
                        CopyOutcome::CanceledWithPartialCopies=>(CopyJobState::CanceledWithPartialCopies,verified,None,"Canceled at a safe boundary. Retain published outputs and partial stages for inspection.".into()),
                        CopyOutcome::Interrupted=>(CopyJobState::NeedsRecovery,verified,None,"Interrupted operation; inspect retained files and journal before further writes.".into()),
                    })
                } else {
                    let prepared=session.namespace.prepare_approved(scope.directory(1).leaf(),&cancel,&selection,proof)?;
                    prepared.validate_parent(scope.directory(1).leaf())?;
                    let ready=Arc::new(session.namespace.ready_journal()?);
                    let id=ready.plan().id.clone();
                    *session.ready.lock().map_err(|_|failed("Ready copy state is unavailable."))?=Some(ready);
                    Ok((CopyJobState::Prepared,0,Some(id),"Output names are prepared. No source bytes were copied; review and approve the separate copy action.".into()))
                }
            })();
            match result {
                Ok((state,count,ready,message))=>{let _=this.update(&job_id,state,count,ready,&message,None);}
                Err(error)=> {
                    let canceled=cancel.load(Ordering::Acquire);
                    let count=session.ready.lock().ok().and_then(|ready|ready.as_ref().map(|ready|ready.verified_files().unwrap_or(0))).unwrap_or(0);
                    let _=this.update(&job_id,if canceled {CopyJobState::CanceledWithPartialCopies} else {CopyJobState::NeedsAttention},count,None,"Operation stopped. Originals and all existing output objects are retained; inspect before retrying.",Some(error));
                }
            }
        });
        if let Err(error) = spawned {
            *self
                .active
                .lock()
                .map_err(|_| failed("Copy job state is unavailable."))? = None;
            self.update(
                &job.id,
                CopyJobState::NeedsAttention,
                0,
                None,
                "The native worker could not start. No native operation was begun.",
                Some(
                    AppError::new(
                        ErrorCode::WorkerFailed,
                        "Organization worker could not start.",
                    )
                    .caused_by(error),
                ),
            )?;
            return Err(AppError::new(
                ErrorCode::WorkerFailed,
                "Organization worker could not start. Review again before retrying.",
            ));
        }
        Ok(returned)
    }
    fn update(
        &self,
        id: &str,
        state: CopyJobState,
        verified: u32,
        ready: Option<String>,
        message: &str,
        error: Option<AppError>,
    ) -> Result<()> {
        let mut job = operation_jobs::load(&self.catalog, id.into())?;
        let expected = job.state;
        job.state = state;
        job.verified_files = verified;
        job.ready_review_id = ready;
        job.message = message.into();
        job.error = error;
        operation_jobs::transition(&self.catalog, job, expected)
    }
    pub(crate) fn cancel(&self, id: &str) -> Result<CopyWorkflowJob> {
        let active = self
            .active
            .lock()
            .map_err(|_| failed("Copy job state is unavailable."))?;
        let job = operation_jobs::load(&self.catalog, id.into())?;
        if let Some(active) = active.as_ref().filter(|active| active.id == id) {
            active.cancel.store(true, Ordering::Release);
            if matches!(
                job.state,
                CopyJobState::Queued | CopyJobState::Running | CopyJobState::PauseRequested
            ) {
                self.update(
                    id,
                    CopyJobState::CancelRequested,
                    job.verified_files,
                    job.ready_review_id,
                    "Cancellation requested; waiting for a safe native boundary.",
                    None,
                )?;
            }
        }
        operation_jobs::load(&self.catalog, id.into())
    }
    pub(crate) fn cancel_root(&self, root: &str) -> Result<bool> {
        let active = self
            .active
            .lock()
            .map_err(|_| failed("Copy job state is unavailable."))?;
        if let Some(active) = active.as_ref().filter(|active| active.root == root) {
            active.cancel.store(true, Ordering::Release);
            return Ok(true);
        }
        Ok(false)
    }
    pub(crate) fn has_active_root(&self, root: &str) -> Result<bool> {
        Ok(self
            .active
            .lock()
            .map_err(|_| failed("Copy job state is unavailable."))?
            .as_ref()
            .is_some_and(|active| active.root == root))
    }
    pub(crate) fn io_status(&self) -> Result<CopyIoStatus> {
        let io = PacedIo::shared();
        Ok(CopyIoStatus {
            aggregate_bytes_per_second: io.bytes_per_second()?.to_string(),
            byte_io_paused: io.is_paused()?,
            burst_bytes: crate::jobs::paced_io::ISSUED_BURST_BYTES as u32,
        })
    }
    pub(crate) fn set_io(
        &self,
        rate: Option<String>,
        paused: Option<bool>,
    ) -> Result<CopyIoStatus> {
        let io = PacedIo::shared();
        if let Some(rate) = rate {
            io.set_rate(
                rate.parse()
                    .map_err(|_| failed("Enter an exact whole-byte operation I/O rate."))?,
            )?;
        }
        if let Some(paused) = paused {
            io.set_paused(paused)?;
        }
        self.io_status()
    }
    pub(crate) fn discard(&self, id: &str) -> Result<()> {
        let session = self.session(id)?;
        if self
            .active
            .lock()
            .map_err(|_| failed("Copy job state is unavailable."))?
            .as_ref()
            .is_some_and(|active| active.session == session.namespace.id())
        {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Finish or cancel the active job before closing its review.",
            ));
        }
        self.sessions
            .lock()
            .map_err(|_| failed("Copy review state is unavailable."))?
            .remove(session.namespace.id());
        Ok(())
    }
}

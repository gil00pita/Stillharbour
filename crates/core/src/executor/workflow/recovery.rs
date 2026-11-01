//! A recovery review is newly observed state, never an imported approval.
use super::*;
use crate::executor::{CopyPlan, resume};

struct RecoverySession {
    prior_job: CopyWorkflowJob,
    journal: Arc<CatalogJournal>,
    sources: Snapshot,
    namespace: String,
    selection: String,
    path: PathBuf,
    review: resume::Review,
    view: CopyContinuationReview,
}
#[derive(Default)]
pub(super) struct Sessions(Mutex<BTreeMap<String, Option<Arc<RecoverySession>>>>);

fn capacity(
    assessment: &mut CopyAssessment,
    review: &resume::Review,
    plan: &CopyPlan,
) -> Result<()> {
    let unit = assessment
        .capacity
        .allocation_unit_bytes
        .parse::<u128>()
        .map_err(|_| failed("Invalid continuation allocation estimate."))?;
    if unit == 0 {
        return Err(failed("Continuation allocation unit is unavailable."));
    }
    let observations = review.observations();
    if observations.len() != plan.files.len() {
        return Err(failed("Continuation observation count changed."));
    }
    let mut additional = 0u128;
    for (file, (stage, published)) in plan.files.iter().zip(observations) {
        let retained = stage.unwrap_or(0) as u128;
        let bytes = file.bytes as u128;
        if !published && retained < bytes {
            additional += bytes.div_ceil(unit) * unit - (retained / unit) * unit;
        }
    }
    let metadata = assessment
        .capacity
        .metadata_allowance_bytes
        .parse::<u128>()
        .map_err(|_| failed("Invalid continuation metadata allowance."))?;
    let journal = assessment
        .capacity
        .journal_allowance_bytes
        .parse::<u128>()
        .map_err(|_| failed("Invalid continuation journal allowance."))?;
    let available = assessment
        .capacity
        .observed_available_bytes
        .parse::<u128>()
        .map_err(|_| failed("Invalid continuation space observation."))?;
    let headroom = additional.div_ceil(10).max(64 * 1024 * 1024);
    let peak = additional + metadata + journal + headroom;
    assessment.capacity.policy = "copy-continuation-estimate-v1".into();
    assessment.capacity.rounded_data_bytes = additional.to_string();
    assessment.capacity.headroom_bytes = headroom.to_string();
    assessment.capacity.estimated_peak_additional_bytes = peak.to_string();
    assessment.capacity.status = if peak <= available {
        CopyCapacityStatus::SufficientForEstimate
    } else {
        CopyCapacityStatus::InsufficientForEstimate
    };
    Ok(())
}
fn phase(value: u8) -> Result<CopyRecordedPhase> {
    Ok(match value {
        0 => CopyRecordedPhase::NotStarted,
        1 => CopyRecordedPhase::IntentRecorded,
        2 => CopyRecordedPhase::StageRecorded,
        3 => CopyRecordedPhase::StageFlushed,
        4 => CopyRecordedPhase::StageVerified,
        5 => CopyRecordedPhase::PublicationIntent,
        6 => CopyRecordedPhase::CompletedRecorded,
        _ => return Err(failed("Unknown recorded continuation phase.")),
    })
}
impl CopyWorkflow {
    fn recovery_session(&self, id: &str) -> Result<Arc<RecoverySession>> {
        crate::operations::validate_id(id)?;
        self.recovery
            .0
            .lock()
            .map_err(|_| failed("Continuation reviews are unavailable."))?
            .get(id)
            .and_then(Clone::clone)
            .ok_or_else(|| {
                failed("This continuation review is no longer live. Review retained work again.")
            })
    }
    fn recovery_selection(
        session: &RecoverySession,
        id: &str,
        path: &Path,
        identity: &FileIdentity,
        volume: &str,
    ) -> Result<()> {
        if session.selection != id
            || identity.volume != session.sources.destination.volume
            || identity.file != session.sources.destination.file
            || volume != session.sources.destination_volume
        {
            return Err(failed(
                "The continuation destination selection changed. Review it again.",
            ));
        }
        let selected = native::open_root(path, Some(identity))?;
        if !session.sources.destination.matches(&selected)?
            || native::volume_token(&selected)? != volume
        {
            return Err(failed(
                "The selected continuation destination changed identity or volume.",
            ));
        }
        Ok(())
    }
    fn fresh_continuation(
        &self,
        library: &Library,
        session: &RecoverySession,
    ) -> Result<CopyAssessment> {
        let source = &session.sources;
        let mut assessment = library.organization_copy_prepared_assessment(
            source.owner_id.clone(),
            source.manifest_id.clone(),
        )?;
        let snapshot = Snapshot::build(
            &self.catalog,
            source.owner_id.clone(),
            source.manifest_id.clone(),
        )?;
        if serde_json::to_vec(&snapshot).ok() != serde_json::to_vec(source).ok() {
            return Err(failed(
                "Continuation source scope changed. Review it again.",
            ));
        }
        let scope = source.open_scope(&self.catalog, Access::Read)?;
        let inventory = session
            .journal
            .recovery_inventory(scope.directory(1).leaf())?;
        let output =
            super::super::native::read_directory(scope.directory(1).leaf(), &session.namespace)?;
        if !session.journal.plan().destination.matches(&output)? {
            return Err(failed("Continuation namespace changed identity."));
        }
        let cancel = AtomicBool::new(false);
        let io = PacedIo::shared();
        let validate = || {
            source.validate_roots(&self.catalog)?;
            inventory.validate()
        };
        let inputs = CopyInputs {
            source_root: scope.directory(0).leaf(),
            destination: &output,
            io: &io,
            cancel: &cancel,
            validate_scope: &validate,
        };
        let current = session.journal.recovery_review(&inputs)?;
        if current.digest() != session.review.digest() {
            return Err(failed(
                "Retained objects or journal changed after continuation review.",
            ));
        }
        capacity(&mut assessment, &current, session.journal.plan())?;
        self.check_scope_space(source, &session.path, &assessment, true)?;
        Ok(assessment)
    }
    pub(crate) fn create_continuation_review(
        &self,
        library: &Library,
        job_id: String,
        selection: String,
        path: PathBuf,
        selected: (FileIdentity, String),
    ) -> Result<CopyContinuationReview> {
        crate::operations::validate_id(&selection)?;
        let _permit = self.gate.try_start()?;
        let id = uuid::Uuid::new_v4().to_string();
        {
            let mut sessions = self
                .recovery
                .0
                .lock()
                .map_err(|_| failed("Continuation reviews are unavailable."))?;
            if sessions.len() >= 64 {
                return Err(AppError::new(
                    ErrorCode::Busy,
                    "Close unused continuation reviews before opening more.",
                ));
            }
            sessions.insert(id.clone(), None);
        }
        let result = (|| {
            let RetainedCopy {
                job,
                journal,
                sources,
                namespace,
            } = self.retained_copy(job_id, &path, &selected.0, &selected.1)?;
            let mut assessment = library.organization_copy_prepared_assessment(
                sources.owner_id.clone(),
                sources.manifest_id.clone(),
            )?;
            let scope = sources.open_scope(&self.catalog, Access::Read)?;
            let inventory = journal.recovery_inventory(scope.directory(1).leaf())?;
            let output =
                super::super::native::read_directory(scope.directory(1).leaf(), &namespace)?;
            if !journal.plan().destination.matches(&output)? {
                return Err(failed("Continuation output namespace changed."));
            }
            let cancel = AtomicBool::new(false);
            let io = PacedIo::shared();
            let validate = || {
                sources.validate_roots(&self.catalog)?;
                inventory.validate()
            };
            let inputs = CopyInputs {
                source_root: scope.directory(0).leaf(),
                destination: &output,
                io: &io,
                cancel: &cancel,
                validate_scope: &validate,
            };
            let review = journal.recovery_review(&inputs)?;
            let (cursors, _, terminal) = journal.recovery_history()?;
            let observations = review.observations();
            capacity(&mut assessment, &review, journal.plan())?;
            self.check_scope_space(&sources, &path, &assessment, true)?;
            let files = sources
                .files
                .iter()
                .zip(cursors)
                .zip(observations)
                .map(|((file, cursor), (bytes, published))| {
                    Ok(CopyContinuationFile {
                        file: CopyReviewFile {
                            id: file.id.clone(),
                            source_path: native::decode_path(&file.location.relative_native)?
                                .to_string_lossy()
                                .into_owned(),
                            target_path: format!("{namespace}/{}", file.target),
                            bytes: file.bytes.to_string(),
                        },
                        recorded_phase: phase(cursor.phase)?,
                        retained_stage_bytes: bytes.map(|value| value.to_string()),
                        output_present: published,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let view = CopyContinuationReview {
                id: id.clone(),
                prior_job_id: job.id.clone(),
                plan_id: sources.owner_id.clone(),
                manifest_id: sources.manifest_id.clone(),
                digest: review.digest().into(),
                source_root_id: sources.root_id.clone(),
                destination_display_path: path.to_string_lossy().into_owned(),
                output_folder: namespace.clone(),
                files,
                assessment,
                recorded_complete: terminal,
                sources_retained: true,
                independent_backup: false,
            };
            self.recovery
                .0
                .lock()
                .map_err(|_| failed("Continuation reviews are unavailable."))?
                .insert(
                    id.clone(),
                    Some(Arc::new(RecoverySession {
                        prior_job: job,
                        journal: Arc::new(journal),
                        sources,
                        namespace,
                        selection,
                        path,
                        review,
                        view: view.clone(),
                    })),
                );
            Ok(view)
        })();
        if result.is_err() {
            self.recovery
                .0
                .lock()
                .map_err(|_| failed("Continuation reviews are unavailable."))?
                .remove(&id);
        }
        result
    }
    pub(crate) fn approve_continuation(
        &self,
        library: &Library,
        input: CopyApprovalInput,
        selection: String,
        path: PathBuf,
        selected: (FileIdentity, String),
    ) -> Result<CopyActionApproval> {
        let _permit = self.gate.try_start()?;
        let session = self.recovery_session(&input.review_id)?;
        Self::recovery_selection(&session, &selection, &path, &selected.0, &selected.1)?;
        if input.action != CopyAction::CopyFiles
            || input.digest != session.review.digest()
            || !input.fidelity_limits_acknowledged
        {
            return Err(failed(
                "Approve the exact continuation review and acknowledge its byte-only fidelity limits.",
            ));
        }
        self.fresh_continuation(library, &session)?;
        let expected = session
            .review
            .approval(session.journal.plan(), &selection)?;
        Ok(CopyActionApproval {
            id: self.approvals.issue(expected)?,
            review_id: session.view.id.clone(),
            action: CopyAction::CopyFiles,
            expires_in_seconds: 600,
        })
    }
    pub(crate) fn start_continuation(
        self: &Arc<Self>,
        library: Arc<Library>,
        input: CopyStartInput,
        selection: String,
        path: PathBuf,
        selected: (FileIdentity, String),
    ) -> Result<CopyWorkflowJob> {
        library.ensure_work_admission()?;
        let permit = self.gate.try_start()?;
        let session = self.recovery_session(&input.review_id)?;
        Self::recovery_selection(&session, &selection, &path, &selected.0, &selected.1)?;
        if input.digest != session.review.digest() {
            return Err(failed("Continuation review digest changed before start."));
        }
        let assessment = self.fresh_continuation(&library, &session)?;
        let expected = session
            .review
            .approval(session.journal.plan(), &selection)?;
        let proof = self.approvals.consume(&input.approval_id, &expected)?;
        let now = crate::library::store::now();
        let job = CopyWorkflowJob {
            id: uuid::Uuid::new_v4().to_string(),
            plan_id: session.sources.owner_id.clone(),
            review_id: session.journal.plan().id.clone(),
            source_root_id: session.sources.root_id.clone(),
            action: CopyAction::CopyFiles,
            state: CopyJobState::Queued,
            total_files: session.sources.files.len() as u32,
            verified_files: 0,
            ready_review_id: None,
            message: "Reviewed continuation queued; all retained bytes require fresh verification."
                .into(),
            error: None,
            started_at: now.clone(),
            updated_at: now,
        };
        let related_roots = self
            .copy_keeper_check(&session.sources)?
            .map_or_else(Vec::new, |checker| checker.root_ids());
        operation_jobs::create_continuation(
            &self.catalog,
            job.clone(),
            session.review.digest().into(),
            operation_jobs::ContinuationRecord {
                policy: "application-copy-continuation-v1".into(),
                prior_job_id: session.prior_job.id.clone(),
                stream_id: session.journal.plan().id.clone(),
                binding_digest: session.journal.digest(),
                review: session.review.record(session.journal.plan()),
            },
        )?;
        let cancel = permit.cancellation();
        *self
            .active
            .lock()
            .map_err(|_| failed("Copy job state is unavailable."))? = Some(Active {
            id: job.id.clone(),
            root: job.source_root_id.clone(),
            related_roots,
            session: session.view.id.clone(),
            cancel: cancel.clone(),
        });
        if let Err(error) = library.ensure_work_admission() {
            cancel.store(true, Ordering::Release);
            *self
                .active
                .lock()
                .map_err(|_| failed("Copy job state is unavailable."))? = None;
            self.update(
                &job.id,
                CopyJobState::NeedsAttention,
                0,
                None,
                "Maintenance blocked continuation admission; retained objects are unchanged.",
                Some(error.clone()),
            )?;
            return Err(error);
        }
        let this = self.clone();
        let job_id = job.id.clone();
        let spawned=std::thread::Builder::new().name("organization-continuation".into()).spawn(move|| {
            let _permit=permit;let _active=ActiveGuard{workflow:this.clone(),id:job_id.clone()};
            let result=(||->Result<CopyOutcome> {
                if cancel.load(Ordering::Acquire) {return Ok(CopyOutcome::CanceledWithPartialCopies);}
                this.update(&job_id,CopyJobState::Running,0,None,"Checking retained namespace and freshly verifying continuation.",None)?;
                let (scope,keepers)=this.copy_scope(&session.sources)?;
                let mut keeper_stats=super::super::fingerprints::Stats::default();
                if let Some(checker)=&keepers && matches!(checker.verify(&this,&scope,&cancel,&mut keeper_stats,&|_,_|Ok(()))?,crate::jobs::paced_io::IoStep::Canceled) {return Ok(CopyOutcome::CanceledWithPartialCopies);}
                let inventory=session.journal.recovery_inventory(scope.directory(1).leaf())?;
                let output=super::super::native::read_directory(scope.directory(1).leaf(),&session.namespace)?;
                if !session.journal.plan().destination.matches(&output)? {return Err(failed("Continuation namespace changed before native admission."));}
                this.check_scope_space(&session.sources,&session.path,&assessment,true)?;
                let validate=|| {session.sources.validate_roots(&this.catalog)?;inventory.validate()?;this.check_scope_space(&session.sources,&session.path,&assessment,false)};
                let progress=|count| {let current=operation_jobs::load(&this.catalog,job_id.clone())?;this.update(&job_id,current.state,count,None,"Fresh byte comparison completed; originals remain in place.",None)};
                let io=PacedIo::shared();
                let outcome=session.journal.resume_approved(CopyInputs{source_root:scope.directory(0).leaf(),destination:&output,io:&io,cancel:&cancel,validate_scope:&validate},&selection,session.review.clone(),proof,None,Some(&progress))?;
                if outcome==CopyOutcome::CompletedVerified && let Some(checker)=&keepers && matches!(checker.verify(&this,&scope,&cancel,&mut keeper_stats,&|_,_|Ok(()))?,crate::jobs::paced_io::IoStep::Canceled) {return Ok(CopyOutcome::CanceledWithPartialCopies);}
                Ok(outcome)
            })();
            let current=operation_jobs::load(&this.catalog,job_id.clone());
            let verified=current.as_ref().map(|job|job.verified_files).unwrap_or(0);
            match result {
                Ok(CopyOutcome::CompletedVerified)=>{let _=this.update(&job_id,CopyJobState::CompletedVerified,session.sources.files.len() as u32,None,"All outputs freshly byte-verified. Retained completed files were not copied again; originals remain.",None);}
                Ok(CopyOutcome::CanceledWithPartialCopies)=>{let _=this.update(&job_id,CopyJobState::CanceledWithPartialCopies,verified,None,"Continuation canceled at a safe boundary. Retain outputs and stages for inspection.",None);}
                Ok(CopyOutcome::Interrupted)=>{let _=this.update(&job_id,CopyJobState::NeedsRecovery,verified,None,"Continuation interrupted. Fresh review and approval are required before further writes.",None);}
                Err(error)=>{let _=this.update(&job_id,if cancel.load(Ordering::Acquire){CopyJobState::CanceledWithPartialCopies}else{CopyJobState::NeedsAttention},verified,None,"Continuation stopped; originals and retained output objects remain. Inspect before retrying.",Some(error));}
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
                "Continuation worker could not start; no native work began.",
                Some(
                    AppError::new(
                        ErrorCode::WorkerFailed,
                        "Continuation worker could not start.",
                    )
                    .caused_by(error),
                ),
            )?;
            return Err(AppError::new(
                ErrorCode::WorkerFailed,
                "Continuation worker could not start. Review again.",
            ));
        }
        Ok(job)
    }
    pub(crate) fn discard_continuation(&self, id: &str) -> Result<()> {
        crate::operations::validate_id(id)?;
        if self
            .active
            .lock()
            .map_err(|_| failed("Copy job state is unavailable."))?
            .as_ref()
            .is_some_and(|active| active.session == id)
        {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Stop the active continuation before discarding its review.",
            ));
        }
        self.recovery
            .0
            .lock()
            .map_err(|_| failed("Continuation reviews are unavailable."))?
            .remove(id);
        Ok(())
    }
}

impl CopyWorkflow {
    pub(crate) fn reopen_preparation(
        &self,
        library: &Library,
        job_id: String,
        selection: String,
        path: PathBuf,
        identity: (FileIdentity, String),
    ) -> Result<CopyPreparationReopened> {
        crate::operations::validate_id(&selection)?;
        let _permit = self.gate.try_start()?;
        let job = operation_jobs::load(&self.catalog, job_id.clone())?;
        if job.action != CopyAction::PrepareNames
            || !matches!(
                job.state,
                CopyJobState::Prepared
                    | CopyJobState::NeedsAttention
                    | CopyJobState::NeedsRecovery
                    | CopyJobState::CanceledWithPartialCopies
            )
        {
            return Err(failed(
                "Reopen only a stopped name-preparation job. Existing copy execution requires retained inspection and continuation.",
            ));
        }
        let digest = operation_jobs::scope_digest(&self.catalog, job_id.clone())?;
        let namespace = Arc::new(NamespaceJournal::open_retained(
            self.catalog.clone(),
            job.plan_id.clone(),
            job.review_id.clone(),
            digest,
        )?);
        if namespace.sources().root_id != job.source_root_id
            || namespace.sources().files.len() != job.total_files as usize
        {
            return Err(failed(
                "Preparation job differs from its retained native scope.",
            ));
        }
        let session = Arc::new(Session {
            namespace,
            selection: selection.clone(),
            selected_path: path.clone(),
            ready: Mutex::new(None),
        });
        Self::check_selection(&session, &selection, &path, &identity.0, &identity.1)?;
        let assessment = self.fresh(library, &session)?;
        let scope = session
            .namespace
            .sources()
            .open_scope(&self.catalog, Access::Read)?;
        session
            .namespace
            .validate_native(scope.directory(1).leaf())?;
        {
            let entries = self
                .sessions
                .lock()
                .map_err(|_| failed("Copy reviews are unavailable."))?;
            if entries.len() >= 64 && !entries.contains_key(session.namespace.id()) {
                return Err(AppError::new(
                    ErrorCode::Busy,
                    "Close unused copy reviews before reopening preparation.",
                ));
            }
        }
        // Planning metadata may be reconstructed after complete retained/native
        // validation. This never recreates a directory/stage or copies a byte.
        let ready = Arc::new(session.namespace.ready_journal()?);
        if job
            .ready_review_id
            .as_ref()
            .is_some_and(|expected| expected != &ready.plan().id)
        {
            return Err(failed(
                "The retained ready-plan identity differs from this job.",
            ));
        }
        if ready.history_count()? != 0 {
            return Err(failed(
                "This namespace has copy execution history. Inspect it and approve continuation instead.",
            ));
        }
        self.approvals.revoke_review(session.namespace.id())?;
        self.approvals.revoke_review(&ready.plan().id)?;
        *session
            .ready
            .lock()
            .map_err(|_| failed("Ready copy review is unavailable."))? = Some(ready);
        let view = self.view(&session, assessment)?;
        let manifest = library
            .organization_copy_manifest_open(view.plan_id.clone(), view.manifest_id.clone())?;
        session
            .namespace
            .validate_native(scope.directory(1).leaf())?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| failed("Copy reviews are unavailable."))?;
        if sessions.len() >= 64 && !sessions.contains_key(session.namespace.id()) {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Close unused copy reviews before reopening preparation.",
            ));
        }
        sessions.insert(session.namespace.id().into(), Some(session));
        Ok(CopyPreparationReopened {
            job_id,
            manifest,
            review: view,
        })
    }
}

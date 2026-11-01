//! All-existing import completion: fresh direct comparisons and a durable
//! catalog receipt. No filesystem output, rename, unlink or source rewrite.
use super::import_planning::{self, Stored};
use super::*;
use crate::{
    executor::{
        approval::{Action, Review},
        fingerprints::{self, Entry, Stats},
    },
    filesystem::DirectoryChain,
    jobs::{
        operation_locks::{Access, Scope},
        paced_io::IoStep,
    },
    library::{native, operation_store},
};
use rusqlite::params;
use serde::{Deserialize, Serialize};

const START: &str = "import_skip_attempt_v1";
const FINISH: &str = "import_skip_outcome_v1";
const MAX: usize = 16384;
type IndexedKeeper = (
    Option<String>,
    Option<Vec<u8>>,
    Option<String>,
    Option<String>,
    i64,
    String,
    bool,
);
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    policy: String,
    job: ImportSession,
}
struct Run {
    view: Mutex<ImportSession>,
    cancel: AtomicBool,
    discard: AtomicBool,
    roots: Vec<String>,
}
struct PauseGuard;
impl Drop for PauseGuard {
    fn drop(&mut self) {
        let _ = PacedIo::shared().set_paused(false);
    }
}
#[derive(Default)]
pub(super) struct Runs(Mutex<BTreeMap<String, Arc<Run>>>);
fn active(state: ImportSessionState) -> bool {
    matches!(
        state,
        ImportSessionState::Queued
            | ImportSessionState::Running
            | ImportSessionState::CancelRequested
    )
}
impl Runs {
    fn get(&self, id: &str) -> Result<Option<Arc<Run>>> {
        validate_id(id)?;
        Ok(self
            .0
            .lock()
            .map_err(|_| failed("Import sessions are unavailable."))?
            .get(id)
            .cloned())
    }
    pub(super) fn root(&self, id: &str, cancel: bool) -> Result<bool> {
        let entries = self
            .0
            .lock()
            .map_err(|_| failed("Import sessions are unavailable."))?;
        let mut found = false;
        for run in entries.values() {
            let mut view = run
                .view
                .lock()
                .map_err(|_| failed("Import session state is unavailable."))?;
            if run.roots.iter().any(|root| root == id) && active(view.state) {
                found = true;
                if cancel {
                    run.cancel.store(true, Ordering::Release);
                    view.state = ImportSessionState::CancelRequested;
                }
            }
        }
        Ok(found)
    }
}
fn update(run: &Run, change: impl FnOnce(&mut ImportSession)) -> Result<()> {
    let mut view = run
        .view
        .lock()
        .map_err(|_| failed("Import session state is unavailable."))?;
    change(&mut view);
    Ok(())
}
fn review(plan: &Stored) -> Result<Review> {
    if !plan.view.copy_file_ids.is_empty()
        || plan.view.skip_files.is_empty()
        || plan.view.skip_files.len() != plan.source.files.len()
    {
        return Err(failed(
            "This execution step requires every selected bundle to have a complete chosen existing group. Mixed copy/skip imports need the copy integration.",
        ));
    }
    Review::new(
        Action::RecordImportSkip,
        plan.view.id.clone(),
        plan.view.digest.clone(),
        plan.source.root_id.clone(),
    )
}
fn encode(job: &ImportSession) -> Result<String> {
    let text = serde_json::to_string(&Record {
        policy: "verified-import-skip-v1".into(),
        job: job.clone(),
    })
    .map_err(|error| failed("Could not record import state.").caused_by(error))?;
    if text.len() > MAX {
        return Err(failed("Import session state exceeds its bound."));
    }
    Ok(text)
}
fn write(catalog: &Catalog, job: ImportSession, terminal: bool) -> Result<()> {
    catalog.call(move|db| {
        let tx=db.transaction()?;
        let plan=import_planning::load(&tx,&job.plan_id,&job.import_plan_id)?;
        if plan.view.digest!=job.digest || job.total_files as usize!=plan.view.skip_files.len() || job.verified_files>job.total_files || !job.sources_retained || job.output_created {return Err(failed("Import session does not match its immutable decisions."));}
        let prior:i64=tx.query_row("SELECT count(*) FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase IN (?3,?4)",params![job.plan_id,job.id,START,FINISH],|row|row.get(0))?;
        if prior!=i64::from(terminal) || (terminal && (active(job.state)||job.state==ImportSessionState::NeedsRecovery)) || (!terminal && job.state!=ImportSessionState::Queued) {return Err(failed("Import session history changed; no completion was recorded."));}
        if terminal {
            let initial=load(&tx,&job.plan_id,&job.id)?;
            if initial.import_plan_id!=job.import_plan_id || initial.digest!=job.digest {return Err(failed("Import outcome belongs to another attempt."));}
        }
        tx.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,?3,?4,?5)",params![job.plan_id,job.id,if terminal {FINISH}else{START},encode(&job)?,if terminal {job.finished_at.as_ref().ok_or_else(||failed("Import outcome has no completion time."))?} else {&job.created_at}])?;
        tx.commit()?;Ok(())
    })
}
fn load(db: &rusqlite::Connection, owner: &str, id: &str) -> Result<ImportSession> {
    validate_id(owner)?;
    validate_id(id)?;
    let rows=db.prepare("SELECT phase,CASE WHEN length(CAST(evidence AS BLOB))<=?5 THEN evidence END FROM operation_journal WHERE plan_id=?1 AND item_id=?2 AND phase IN (?3,?4) ORDER BY sequence LIMIT 3")?.query_map(params![owner,id,START,FINISH,MAX as i64],|row|Ok((row.get::<_,String>(0)?,row.get::<_,Option<String>>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
    if rows.is_empty()
        || rows.len() > 2
        || rows[0].0 != START
        || (rows.len() == 2 && rows[1].0 != FINISH)
    {
        return Err(failed("Import attempt history is missing or ambiguous."));
    }
    let mut records = Vec::new();
    for (phase, text) in rows {
        let text = text.ok_or_else(|| failed("Import attempt evidence exceeds its bound."))?;
        let record: Record = serde_json::from_str(&text)
            .map_err(|error| failed("Import attempt evidence is invalid.").caused_by(error))?;
        let job = record.job;
        if record.policy != "verified-import-skip-v1"
            || job.id != id
            || job.plan_id != owner
            || !job.sources_retained
            || job.output_created
            || job.total_files == 0
            || job.total_files > 512
            || job.verified_files > job.total_files
            || job.bytes_read.len() > 39
            || job.bytes_read.parse::<u128>().is_err()
            || (phase == START
                && (job.state != ImportSessionState::Queued
                    || job.verified_files != 0
                    || job.finished_at.is_some()))
            || (phase == FINISH
                && (active(job.state)
                    || job.state == ImportSessionState::NeedsRecovery
                    || job.finished_at.is_none()
                    || (job.state == ImportSessionState::Completed
                        && (job.verified_files != job.total_files || job.error.is_some()))))
        {
            return Err(failed("Retained import attempt has invalid state."));
        }
        records.push(job);
    }
    if records.len() == 2
        && (records[0].import_plan_id != records[1].import_plan_id
            || records[0].digest != records[1].digest
            || records[0].total_files != records[1].total_files
            || records[0].created_at != records[1].created_at)
    {
        return Err(failed("Import attempt and outcome bindings disagree."));
    }
    let mut job = records
        .pop()
        .ok_or_else(|| failed("Import attempt is missing."))?;
    let plan = import_planning::load(db, owner, &job.import_plan_id)?;
    if plan.view.digest != job.digest || plan.view.skip_files.len() != job.total_files as usize {
        return Err(failed(
            "Import attempt differs from its retained decisions.",
        ));
    }
    if active(job.state) {
        job.state = ImportSessionState::NeedsRecovery;
    }
    Ok(job)
}
fn check_roots(catalog: &Arc<Catalog>, plan: &Stored) -> Result<()> {
    plan.source.validate_read_roots(catalog)?;
    let roots = plan.roots.clone();
    let source = plan.source.root_id.clone();
    catalog.call(move |db| {
        let availability: String = db.query_row(
            "SELECT availability FROM roots WHERE id=?",
            [source],
            |row| row.get(0),
        )?;
        if availability != "online" {
            return Err(failed("The import source is offline or inaccessible."));
        }
        for root in roots {
            let (path, object, volume) = crate::executor::sources::root(db, &root.id, false)?;
            let (scan, availability): (Option<String>, String) = db.query_row(
                "SELECT CASE WHEN length(last_scan)<=128 THEN last_scan END,availability FROM roots WHERE id=?",
                [&root.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            if path != root.native_path
                || object != root.object
                || volume != root.volume
                || scan != root.scan
                || availability != "online"
            {
                return Err(failed(
                    "The selected keeper root changed or is unavailable.",
                ));
            }
        }
        Ok(())
    })
}
pub(in crate::executor) struct KeeperCheck {
    plan: Stored,
}
impl KeeperCheck {
    pub(in crate::executor) fn root_ids(&self) -> Vec<String> {
        self.plan.view.scope_root_ids.clone()
    }
    pub(super) fn scopes(&self, workflow: &CopyWorkflow) -> Result<Vec<Scope>> {
        if self
            .plan
            .source
            .files
            .iter()
            .any(|file| file.object.volume != self.plan.source.source.volume)
        {
            return Err(failed(
                "Import sources crossing native volumes need a separate reviewed root.",
            ));
        }
        for root in &self.plan.roots {
            if root.id == self.plan.source.root_id || root.object == self.plan.source.source {
                return Err(failed(
                    "The source root cannot be its own existing-library scope.",
                ));
            }
            if self
                .plan
                .keepers
                .iter()
                .any(|file| file.root_id == root.id && file.object.volume != root.object.volume)
            {
                return Err(failed(
                    "An import keeper crosses its registered native volume.",
                ));
            }
        }
        self.plan.validate_native(&workflow.catalog)?;
        let mut scopes = Vec::new();
        for root in &self.plan.roots {
            let chain =
                DirectoryChain::open(&native::decode_path(&root.native_path)?.canonicalize()?)?;
            if !root.object.matches(chain.leaf())?
                || native::volume_token(chain.leaf())? != root.volume
            {
                return Err(failed("The keeper root changed native identity."));
            }
            scopes.push(Scope::new(chain, Access::Read));
        }
        Ok(scopes)
    }
    pub(in crate::executor) fn verify(
        &self,
        workflow: &CopyWorkflow,
        scope: &crate::jobs::operation_locks::Lease,
        cancel: &AtomicBool,
        stats: &mut Stats,
        progress: &dyn Fn(u32, u128) -> Result<()>,
    ) -> Result<IoStep<()>> {
        if cancel.load(Ordering::Acquire) {
            return Ok(IoStep::Canceled);
        }
        let plan = &self.plan;
        if !plan.source.source.matches(scope.directory(0).leaf())?
            || !plan.source.destination.matches(scope.directory(1).leaf())?
        {
            return Err(failed(
                "Import verification received another native source/destination scope.",
            ));
        }
        for (index, root) in plan.roots.iter().enumerate() {
            if !root.object.matches(scope.directory(2 + index).leaf())? {
                return Err(failed(
                    "Import verification received another native keeper scope.",
                ));
            }
        }
        check_roots(&workflow.catalog, plan)?;
        let io = PacedIo::shared();
        let mut verified = Vec::new();
        for selected in &plan.view.skip_files {
            if cancel.load(Ordering::Acquire) {
                return Ok(IoStep::Canceled);
            }
            check_roots(&workflow.catalog, plan)?;
            let member = plan
                .source
                .files
                .iter()
                .find(|file| file.id == selected.file_id)
                .ok_or_else(|| failed("A selected import source is missing."))?;
            let candidate = plan
                .keepers
                .iter()
                .find(|entry| {
                    entry.id == selected.keeper_asset_id && entry.root_id == selected.keeper_root_id
                })
                .ok_or_else(|| failed("A selected keeper is missing."))?;
            let root_index = plan
                .roots
                .iter()
                .position(|root| root.id == candidate.root_id)
                .ok_or_else(|| failed("A selected keeper is outside its root scope."))?;
            let expected = candidate.clone();
            let owner = plan.view.plan_id.clone();
            let source_location = member.location.relative_native.clone();
            workflow.catalog.call(move|db| {
            let (root,path,identity,revision,bytes,presence,media):IndexedKeeper=db.query_row("SELECT CASE WHEN length(root_id)<=128 THEN root_id END,CASE WHEN length(relative_native)<=8192 THEN relative_native END,CASE WHEN length(native_file)<=128 THEN native_file END,CASE WHEN length(revision)=64 THEN revision END,bytes,presence,is_media FROM assets WHERE id=?",[&expected.id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?)))?;
            let primary=operation_store::load(db,&owner)?.sources.iter().any(|source|native::decode_path(&source.native_path).is_ok_and(|path|native::decode_path(&source_location).is_ok_and(|member|path==member)));
            if root.as_deref()!=Some(expected.root_id.as_str()) || path.as_ref().is_none_or(|path|native::decode_path(path).ok()!=native::decode_path(&expected.location.relative_native).ok()) || identity.as_deref()!=Some(format!("{}:{}",expected.object.volume,expected.object.file).as_str()) || revision.as_deref()!=Some(expected.revision.as_str()) || bytes<0 || bytes as u64!=expected.bytes || presence!="present" || (primary && !media) {return Err(failed("The indexed keeper changed, is unavailable or is no longer a media primary. Compare and review again."));}Ok(())
        })?;
            let original = Entry {
                id: member.id.clone(),
                root_id: plan.source.root_id.clone(),
                location: member.location.clone(),
                object: member.object.clone(),
                revision: member.revision.clone(),
                bytes: member.bytes,
                display: member.display_name.clone(),
            };
            let compared = fingerprints::equal(
                (&original, scope.directory(0).leaf()),
                (candidate, scope.directory(2 + root_index).leaf()),
                &io,
                cancel,
                stats,
            );
            progress(verified.len() as u32, stats.bytes_read)?;
            match compared? {
                IoStep::Canceled => return Ok(IoStep::Canceled),
                IoStep::Completed(Some(witness)) if witness.digest == selected.digest => {}
                _ => {
                    return Err(failed(
                        "The chosen existing file no longer matches complete source bytes. Nothing was skipped; compare and review again.",
                    ));
                }
            }
            verified.push((original, candidate, root_index));
            progress(verified.len() as u32, stats.bytes_read)?;
        }
        check_roots(&workflow.catalog, plan)?;
        for (source_file, keeper, index) in verified {
            source_file.open(scope.directory(0).leaf())?;
            keeper.open(scope.directory(2 + index).leaf())?;
        }
        if cancel.load(Ordering::Acquire) {
            Ok(IoStep::Canceled)
        } else {
            Ok(IoStep::Completed(()))
        }
    }
}
fn verify(workflow: &CopyWorkflow, plan: &Stored, run: &Run) -> Result<IoStep<()>> {
    if run.cancel.load(Ordering::Acquire) {
        return Ok(IoStep::Canceled);
    }
    let checker = KeeperCheck { plan: plan.clone() };
    let extras = checker.scopes(workflow)?;
    let scope = plan
        .source
        .open_scope_with(&workflow.catalog, Access::Read, extras)?;
    checker.verify(
        workflow,
        &scope,
        &run.cancel,
        &mut Stats::default(),
        &|verified, bytes| {
            update(run, |view| {
                view.verified_files = verified;
                view.bytes_read = bytes.to_string();
            })
        },
    )
}

impl CopyWorkflow {
    pub(super) fn copy_keeper_check(&self, sources: &Snapshot) -> Result<Option<KeeperCheck>> {
        let Some(selection) = sources.import_selection.as_ref() else {
            return Ok(None);
        };
        let owner = sources.owner_id.clone();
        let id = selection.id.clone();
        let expected = selection.digest.clone();
        let snapshot = sources.clone();
        self.catalog.call(move |db| {
            snapshot.validate(db)?;
            let plan = import_planning::load(db, &owner, &id)?;
            if plan.view.digest != expected {
                return Err(failed(
                    "Import decisions changed before native keeper admission.",
                ));
            }
            Ok(Some(KeeperCheck { plan }))
        })
    }
    pub(in crate::executor) fn copy_scope(
        &self,
        sources: &Snapshot,
    ) -> Result<(crate::jobs::operation_locks::Lease, Option<KeeperCheck>)> {
        let checker = self.copy_keeper_check(sources)?;
        let extras = match &checker {
            Some(checker) => checker.scopes(self)?,
            None => Vec::new(),
        };
        let scope = sources.open_scope_with(&self.catalog, Access::Write, extras)?;
        Ok((scope, checker))
    }
    pub(crate) fn import_history(&self, owner: String, manifest: String) -> Result<ImportHistory> {
        let plans = self.import_plan_summaries(owner.clone(), Some(manifest.clone()))?;
        let retained=self.catalog.call({let owner=owner.clone();move|db| {
            let ids=db.prepare("SELECT j.item_id FROM operation_journal j JOIN operation_journal p ON p.plan_id=j.plan_id AND p.item_id=json_extract(j.evidence,'$.job.importPlanId') AND p.phase='import_selection_plan_v1' WHERE j.plan_id=?1 AND j.phase=?2 AND json_extract(p.evidence,'$.view.manifestId')=?3 ORDER BY j.sequence DESC LIMIT 32")?.query_map(params![owner,START,manifest],|row|row.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(ids)
        }})?;
        let sessions = retained
            .into_iter()
            .map(|id| self.import_session(owner.clone(), id))
            .collect::<Result<Vec<_>>>()?;
        Ok(ImportHistory { plans, sessions })
    }
    pub(crate) fn approve_import_skip(
        &self,
        library: &Library,
        input: ImportSkipApprovalInput,
    ) -> Result<ImportSkipApproval> {
        library.ensure_work_admission()?;
        if !input.sources_retained_acknowledged {
            return Err(failed(
                "Acknowledge that sources remain and every selected keeper will be freshly compared before recording this import.",
            ));
        }
        let plan = self
            .catalog
            .call(move |db| import_planning::load(db, &input.plan_id, &input.import_plan_id))?;
        if plan.view.digest != input.digest {
            return Err(failed("Import decisions changed. Review again."));
        }
        plan.validate_native(&self.catalog)?;
        check_roots(&self.catalog, &plan)?;
        self.approvals.revoke_review(&plan.view.id)?;
        Ok(ImportSkipApproval {
            id: self.approvals.issue(review(&plan)?)?,
            import_plan_id: plan.view.id,
            digest: plan.view.digest,
            expires_in_seconds: 600,
        })
    }
    pub(crate) fn start_import_skip(
        self: &Arc<Self>,
        library: &Library,
        input: ImportSkipStartInput,
    ) -> Result<ImportSession> {
        library.ensure_work_admission()?;
        let permit = self.gate.try_start()?;
        let plan = self
            .catalog
            .call(move |db| import_planning::load(db, &input.plan_id, &input.import_plan_id))?;
        review(&plan)?;
        let expected = Review::new(
            Action::RecordImportSkip,
            plan.view.id.clone(),
            input.digest.clone(),
            plan.source.root_id.clone(),
        )?;
        self.approvals
            .consume(&input.approval_id, &expected)?
            .authorize(&expected)?;
        if plan.view.digest != input.digest {
            return Err(failed("Import decisions changed. Review again."));
        }
        let view = ImportSession {
            id: uuid::Uuid::new_v4().to_string(),
            plan_id: plan.view.plan_id.clone(),
            import_plan_id: plan.view.id.clone(),
            digest: plan.view.digest.clone(),
            state: ImportSessionState::Queued,
            total_files: plan.view.skip_files.len() as u32,
            verified_files: 0,
            bytes_read: "0".into(),
            sources_retained: true,
            output_created: false,
            created_at: crate::library::store::now(),
            finished_at: None,
            error: None,
        };
        let mut root_ids = plan.view.scope_root_ids.clone();
        root_ids.push(plan.source.root_id.clone());
        let run = Arc::new(Run {
            view: Mutex::new(view.clone()),
            cancel: AtomicBool::new(false),
            discard: AtomicBool::new(false),
            roots: root_ids,
        });
        {
            let mut entries = self
                .import_runs
                .0
                .lock()
                .map_err(|_| failed("Import sessions are unavailable."))?;
            if entries.len() >= 8 {
                return Err(AppError::new(
                    ErrorCode::Busy,
                    "Close a retained import session before starting another (limit eight).",
                ));
            }
            entries.insert(view.id.clone(), run.clone());
        }
        if let Err(error) = library
            .ensure_work_admission()
            .and_then(|()| write(&self.catalog, view.clone(), false))
        {
            self.import_runs
                .0
                .lock()
                .map_err(|_| failed("Import sessions are unavailable."))?
                .remove(&view.id);
            return Err(error);
        }
        let workflow = self.clone();
        let job = run.clone();
        let spawned = std::thread::Builder::new()
            .name("stillharbour-import-skip".into())
            .spawn(move || {
                let pause = PauseGuard;
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    update(&job, |view| {
                        view.state = if job.cancel.load(Ordering::Acquire) {
                            ImportSessionState::CancelRequested
                        } else {
                            ImportSessionState::Running
                        }
                    })?;
                    verify(&workflow, &plan, &job)
                }))
                .unwrap_or_else(|_| {
                    Err(AppError::new(
                        ErrorCode::WorkerFailed,
                        "The import verifier stopped unexpectedly. Source files were retained.",
                    ))
                });
                let mut terminal = job
                    .view
                    .lock()
                    .map(|view| view.clone())
                    .unwrap_or_else(|poison| poison.into_inner().clone());
                terminal.finished_at = Some(crate::library::store::now());
                match outcome {
                    Ok(IoStep::Completed(())) if !job.cancel.load(Ordering::Acquire) => {
                        terminal.state = ImportSessionState::Completed
                    }
                    Ok(_) => terminal.state = ImportSessionState::Canceled,
                    Err(error) => {
                        terminal.state = if job.cancel.load(Ordering::Acquire) {
                            ImportSessionState::Canceled
                        } else {
                            ImportSessionState::Failed
                        };
                        terminal.error = Some(error);
                    }
                }
                if let Err(error) = write(&workflow.catalog, terminal.clone(), true) {
                    terminal.state = ImportSessionState::Failed;
                    terminal.error = Some(error);
                }
                drop(pause);
                drop(permit);
                let _ = update(&job, |view| *view = terminal);
                if job.discard.load(Ordering::Acquire)
                    && let Ok(mut entries) = workflow.import_runs.0.lock()
                    && let Ok(view) = job.view.lock()
                {
                    entries.remove(&view.id);
                }
            });
        if let Err(error) = spawned {
            let mut terminal = view.clone();
            terminal.state = ImportSessionState::Failed;
            terminal.finished_at = Some(crate::library::store::now());
            terminal.error = Some(
                AppError::new(
                    ErrorCode::WorkerFailed,
                    "Could not start import verification.",
                )
                .caused_by(error),
            );
            if let Err(error) = write(&self.catalog, terminal.clone(), true) {
                terminal.error = Some(error);
            }
            update(&run, |view| *view = terminal)?;
        }
        Ok(view)
    }
    pub(crate) fn import_session(&self, owner: String, id: String) -> Result<ImportSession> {
        if let Some(run) = self.import_runs.get(&id)? {
            let view = run
                .view
                .lock()
                .map_err(|_| failed("Import session state is unavailable."))?
                .clone();
            if view.plan_id != owner {
                return Err(failed("Import session belongs to another plan."));
            }
            return Ok(view);
        }
        self.catalog.call(move |db| load(db, &owner, &id))
    }
    pub(crate) fn cancel_import_session(&self, owner: String, id: String) -> Result<ImportSession> {
        let view = self.import_session(owner.clone(), id.clone())?;
        if active(view.state)
            && let Some(run) = self.import_runs.get(&id)?
        {
            run.cancel.store(true, Ordering::Release);
            update(&run, |view| {
                if active(view.state) {
                    view.state = ImportSessionState::CancelRequested;
                }
            })?;
        }
        self.import_session(owner, id)
    }
    pub(crate) fn discard_import_session(&self, owner: String, id: String) -> Result<()> {
        self.cancel_import_session(owner, id.clone())?;
        if let Some(run) = self.import_runs.get(&id)? {
            run.discard.store(true, Ordering::Release);
            if active(
                run.view
                    .lock()
                    .map_err(|_| failed("Import session state is unavailable."))?
                    .state,
            ) {
                return Ok(());
            }
        }
        self.import_runs
            .0
            .lock()
            .map_err(|_| failed("Import sessions are unavailable."))?
            .remove(&id);
        Ok(())
    }
}

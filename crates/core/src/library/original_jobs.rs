//! Separate immutable organization plans and durable jobs. A retained record
//! never authorizes execution or adopts a source root. Writers are fixture-only.
use crate::{
    catalog::Catalog,
    domain::{AppError, ErrorCode, Result},
    operations::original_jobs::*,
};
use rusqlite::{Connection, params};

pub(crate) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE original_operation_plans(
      id TEXT PRIMARY KEY,catalog_id TEXT NOT NULL,root_id TEXT NOT NULL,
      kind TEXT NOT NULL CHECK(kind IN ('rename','undo')),
      reverse_of TEXT REFERENCES original_operation_plans(id),
      digest TEXT NOT NULL CHECK(length(digest)=64),
      total_moves INTEGER NOT NULL CHECK(total_moves BETWEEN 0 AND 1024),
      scope TEXT NOT NULL CHECK(scope='generated_fixture'),created_at TEXT NOT NULL,
      payload TEXT NOT NULL CHECK(json_valid(payload) AND length(CAST(payload AS BLOB))<=2097152),
      CHECK((kind='rename' AND reverse_of IS NULL) OR (kind='undo' AND reverse_of IS NOT NULL)));
    CREATE TRIGGER original_plan_no_update BEFORE UPDATE ON original_operation_plans BEGIN SELECT RAISE(ABORT,'Organization plans are immutable'); END;
    CREATE TRIGGER original_plan_no_delete BEFORE DELETE ON original_operation_plans BEGIN SELECT RAISE(ABORT,'Retain organization plans'); END;
    CREATE TABLE original_operation_jobs(
      id TEXT PRIMARY KEY,plan_id TEXT NOT NULL REFERENCES original_operation_plans(id),
      state TEXT NOT NULL CHECK(state IN ('queued','running','completed_verified','needs_recovery')),
      payload TEXT NOT NULL CHECK(json_valid(payload) AND length(CAST(payload AS BLOB))<=65536),
      started_at TEXT NOT NULL,updated_at TEXT NOT NULL);
    CREATE INDEX original_operation_jobs_recent ON original_operation_jobs(updated_at DESC,id DESC);
    CREATE UNIQUE INDEX original_operation_job_active ON original_operation_jobs(plan_id) WHERE state IN ('queued','running');
    CREATE TRIGGER original_job_identity_immutable BEFORE UPDATE OF id,plan_id,started_at ON original_operation_jobs BEGIN SELECT RAISE(ABORT,'Organization job identity is immutable'); END;
    CREATE TRIGGER original_job_no_delete BEFORE DELETE ON original_operation_jobs BEGIN SELECT RAISE(ABORT,'Retain organization jobs'); END;
    CREATE TRIGGER original_job_state_order BEFORE UPDATE OF state ON original_operation_jobs WHEN NOT
      ((OLD.state='queued' AND NEW.state IN ('running','needs_recovery')) OR
       (OLD.state='running' AND NEW.state IN ('completed_verified','needs_recovery')))
      BEGIN SELECT RAISE(ABORT,'Invalid organization job transition'); END;
    CREATE TABLE original_operation_events(sequence INTEGER PRIMARY KEY,job_id TEXT NOT NULL REFERENCES original_operation_jobs(id),
      state TEXT NOT NULL CHECK(state IN ('queued','running','completed_verified','needs_recovery')),
      payload TEXT NOT NULL CHECK(json_valid(payload) AND length(CAST(payload AS BLOB))<=65536),created_at TEXT NOT NULL);
    CREATE INDEX original_operation_events_job ON original_operation_events(job_id,sequence);
    CREATE TRIGGER original_event_no_update BEFORE UPDATE ON original_operation_events BEGIN SELECT RAISE(ABORT,'Organization job events are immutable'); END;
    CREATE TRIGGER original_event_no_delete BEFORE DELETE ON original_operation_events BEGIN SELECT RAISE(ABORT,'Retain organization job events'); END;")?;
    Ok(())
}
fn failed(message: &str) -> AppError {
    AppError::new(ErrorCode::Catalog, message)
}
fn encode<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|error| failed("Invalid organization job record.").caused_by(error))
}
fn name<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(encode(value)?.trim_matches('"').to_owned())
}
fn decode(row: &rusqlite::Row<'_>) -> Result<OriginalOperationJob> {
    let payload: Option<String> = row.get(0)?;
    let job: OriginalOperationJob = serde_json::from_str(
        &payload.ok_or_else(|| failed("Organization job payload exceeds its bound."))?,
    )
    .map_err(|error| failed("Unsupported organization job history.").caused_by(error))?;
    if job.id != row.get::<_, String>(1)?
        || job.plan_id != row.get::<_, String>(2)?
        || name(&job.state)? != row.get::<_, String>(3)?
        || job.started_at != row.get::<_, String>(4)?
        || job.updated_at != row.get::<_, String>(5)?
        || job.source_root_id != row.get::<_, String>(6)?
        || name(&job.kind)? != row.get::<_, String>(7)?
        || job.total_moves != row.get::<_, u32>(8)?
        || !job.generated_fixture_only
        || job.verified_moves > job.total_moves
        || job.message.len() > 1024
    {
        return Err(failed(
            "Organization job history differs from retained identity/state.",
        ));
    }
    for id in [&job.id, &job.plan_id, &job.source_root_id] {
        crate::operations::validate_id(id)?;
    }
    if (job.state == OriginalJobState::CompletedVerified) != (job.verified_moves == job.total_moves)
        && job.total_moves != 0
    {
        return Err(failed("Organization job completion counters disagree."));
    }
    Ok(job)
}
const COLUMNS: &str = "CASE WHEN length(CAST(j.payload AS BLOB))<=65536 THEN j.payload END,j.id,j.plan_id,j.state,j.started_at,j.updated_at,p.root_id,p.kind,p.total_moves";
pub(super) fn load_db(db: &Connection, id: &str) -> Result<OriginalOperationJob> {
    let mut statement = db.prepare(&format!("SELECT {COLUMNS} FROM original_operation_jobs j JOIN original_operation_plans p ON p.id=j.plan_id WHERE j.id=?"))?;
    let mut rows = statement.query([id])?;
    decode(
        rows.next()?
            .ok_or_else(|| failed("Organization job was not found."))?,
    )
}
#[cfg(any(test, feature = "fixture-probes"))]
pub(crate) fn load(catalog: &Catalog, id: String) -> Result<OriginalOperationJob> {
    crate::operations::validate_id(&id)?;
    catalog.call(move |db| load_db(db, &id))
}
pub(crate) fn list(catalog: &Catalog) -> Result<Vec<OriginalOperationJob>> {
    catalog.call(|db| {
        let mut statement = db.prepare(&format!("SELECT {COLUMNS} FROM original_operation_jobs j JOIN original_operation_plans p ON p.id=j.plan_id ORDER BY j.updated_at DESC,j.id DESC LIMIT 32"))?;
        let mut rows = statement.query([])?; let mut jobs = Vec::new();
        while let Some(row) = rows.next()? { jobs.push(decode(row)?); } Ok(jobs)
    })
}
/// A stopped original operation can leave retained stages while catalog paths
/// still describe the pre-operation namespace. Scans must not reconcile that
/// transient namespace. A verified later attempt resolves that same plan only.
pub(crate) fn require_reconciled_root(catalog: &Catalog, root_id: String) -> Result<()> {
    catalog.call(move |db| require_reconciled_root_db(db, &root_id))
}
pub(crate) fn require_reconciled_root_db(db: &Connection, root_id: &str) -> Result<()> {
    let pending = db.query_row("SELECT EXISTS(SELECT 1 FROM original_operation_plans p JOIN original_operation_jobs j ON j.plan_id=p.id WHERE p.root_id=? AND (j.state IN ('queued','running') OR (j.state='needs_recovery' AND NOT EXISTS(SELECT 1 FROM original_operation_jobs done WHERE done.plan_id=p.id AND done.state='completed_verified'))))", [root_id], |row| row.get::<_, bool>(0))?;
    if pending {
        return Err(AppError::new(
            ErrorCode::Busy,
            "Organization recovery is pending for this library. Retained files and catalog paths must be reconciled before scanning or removing it.",
        ));
    }
    Ok(())
}
#[cfg(any(test, feature = "fixture-probes"))]
pub(crate) fn retain_plan(
    catalog: &Catalog,
    root: String,
    source: (u64, u64),
    graph: crate::operations::rename_graph::Plan,
    reverse_of: Option<String>,
) -> Result<()> {
    crate::operations::validate_id(&root)?;
    let canonical = crate::operations::rename_graph::build(
        graph.id.clone(),
        graph.members.clone(),
        graph.name_facts.clone(),
    )?;
    if !graph.issues.is_empty() || encode(&canonical)? != encode(&graph)? {
        return Err(failed(
            "An unresolved organization plan cannot be retained as executable scope.",
        ));
    }
    catalog.call(move |db| {
        let catalog_id: String = db.query_row("SELECT uuid FROM catalog_identity WHERE singleton=1", [], |row| row.get(0))?;
        let valid: bool = db.query_row("SELECT count(*)=1 FROM roots WHERE id=? AND mode='managed' AND availability='online' AND device=? AND inode=?", params![root,source.0.to_string(),source.1.to_string()], |row| row.get(0))?;
        if !valid { return Err(failed("Organization source root authority changed.")); }
        if let Some(parent) = &reverse_of {
            crate::operations::validate_id(parent)?;
            let valid: bool = db.query_row("SELECT count(*)=1 FROM original_operation_plans WHERE id=? AND root_id=? AND catalog_id=? AND kind='rename'", params![parent,root,catalog_id], |row| row.get(0))?;
            if !valid { return Err(failed("The original reverse-plan relationship changed.")); }
        }
        let payload = encode(&serde_json::json!({"policy":"generated-original-plan-v1","catalogId":catalog_id,"rootId":root,"sourceDevice":source.0.to_string(),"sourceFile":source.1.to_string(),"graph":graph,"reverseOf":reverse_of}))?;
        if payload.len()>2097152 { return Err(failed("Organization plan exceeds its bound.")); }
        let digest = blake3::hash(payload.as_bytes()).to_hex().to_string();
        let existing: Option<String> = db.query_row("SELECT payload FROM original_operation_plans WHERE id=?", [&graph.id], |row| row.get(0)).optional()?;
        if let Some(existing) = existing { if existing != payload { return Err(failed("Retained organization plan changed.")); } return Ok(()); }
        db.execute("INSERT INTO original_operation_plans(id,catalog_id,root_id,kind,reverse_of,digest,total_moves,scope,created_at,payload) VALUES(?,?,?,?,?,?,?,'generated_fixture',?,?)", params![graph.id,catalog_id,root,if reverse_of.is_some(){"undo"}else{"rename"},reverse_of,digest,graph.steps.len() as u32,super::store::now(),payload])?; Ok(())
    })
}
#[cfg(any(test, feature = "fixture-probes"))]
pub(crate) fn create(catalog: &Catalog, plan: String) -> Result<OriginalOperationJob> {
    crate::operations::validate_id(&plan)?;
    catalog.call(move |db| {
        let (root,kind,total):(String,String,u32)=db.query_row("SELECT root_id,kind,total_moves FROM original_operation_plans WHERE id=? AND scope='generated_fixture'", [&plan], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)))?;
        let now=super::store::now();
        let job=OriginalOperationJob { id:uuid::Uuid::new_v4().to_string(),plan_id:plan,source_root_id:root,kind:match kind.as_str(){"rename"=>OriginalJobKind::Rename,"undo"=>OriginalJobKind::Undo,_=>return Err(failed("Unsupported organization job kind."))},state:OriginalJobState::Queued,total_moves:total,verified_moves:0,started_at:now.clone(),updated_at:now,message:"Queued generated organization; no filesystem effects yet.".into(),generated_fixture_only:true };
        let payload=encode(&job)?;let tx=db.transaction()?;
        tx.execute("INSERT INTO original_operation_jobs VALUES(?,?,'queued',?,?,?)",params![job.id,job.plan_id,payload,job.started_at,job.updated_at])?;
        event(&tx,&job)?;tx.commit()?;Ok(job)
    })
}
fn event(db: &Connection, job: &OriginalOperationJob) -> Result<()> {
    db.execute(
        "INSERT INTO original_operation_events(job_id,state,payload,created_at) VALUES(?,?,?,?)",
        params![job.id, name(&job.state)?, encode(job)?, job.updated_at],
    )?;
    Ok(())
}
fn transition_db(
    db: &Connection,
    id: &str,
    state: OriginalJobState,
    message: &str,
) -> Result<OriginalOperationJob> {
    let mut job = load_db(db, id)?;
    if !matches!(
        (job.state, state),
        (
            OriginalJobState::Queued,
            OriginalJobState::Running | OriginalJobState::NeedsRecovery
        ) | (
            OriginalJobState::Running,
            OriginalJobState::CompletedVerified | OriginalJobState::NeedsRecovery
        )
    ) {
        return Err(failed("Invalid organization job transition."));
    }
    let old = name(&job.state)?;
    job.state = state;
    job.updated_at = super::store::now();
    job.message = message.into();
    if job.message.len() > 1024 {
        return Err(failed("Organization job message exceeds its bound."));
    }
    if state == OriginalJobState::CompletedVerified {
        job.verified_moves = job.total_moves;
    }
    if db.execute(
        "UPDATE original_operation_jobs SET state=?,payload=?,updated_at=? WHERE id=? AND state=?",
        params![name(&state)?, encode(&job)?, job.updated_at, job.id, old],
    )? != 1
    {
        return Err(failed("Organization job changed concurrently."));
    }
    event(db, &job)?;
    Ok(job)
}
#[cfg(any(test, feature = "fixture-probes"))]
pub(crate) fn transition(
    catalog: &Catalog,
    id: String,
    state: OriginalJobState,
    message: String,
) -> Result<OriginalOperationJob> {
    if state == OriginalJobState::CompletedVerified {
        return Err(failed(
            "Verified completion belongs to the native/catalog commit transaction.",
        ));
    }
    catalog.call(move |db| {
        let tx = db.transaction()?;
        let job = transition_db(&tx, &id, state, &message)?;
        tx.commit()?;
        Ok(job)
    })
}
#[cfg(any(test, feature = "fixture-probes"))]
pub(crate) fn finish_verified_db(db: &Connection, id: &str, plan: &str, moves: u32) -> Result<()> {
    let job = load_db(db, id)?;
    if job.plan_id != plan || job.total_moves != moves {
        return Err(failed(
            "The verified native plan does not match this organization job.",
        ));
    }
    transition_db(
        db,
        id,
        OriginalJobState::CompletedVerified,
        "Native objects, bytes, catalog paths and retained receipt verified.",
    )?;
    Ok(())
}
pub(crate) fn recover(catalog: &Catalog) -> Result<()> {
    catalog.call(|db| {
        let ids=db.prepare("SELECT id FROM original_operation_jobs WHERE state IN ('queued','running') ORDER BY id LIMIT 65")?.query_map([], |row| row.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        if ids.len()>64 {return Err(failed("Too many interrupted organization jobs for this bounded recovery."));}
        let tx=db.transaction()?;for id in ids {transition_db(&tx,&id,OriginalJobState::NeedsRecovery,"The previous worker stopped. Inspect current files; no approval or I/O was restored.")?;}tx.commit()?;Ok(())
    })
}
#[cfg(any(test, feature = "fixture-probes"))]
use rusqlite::OptionalExtension;

//! Durable operation jobs are separate from discovery/thumbnail scans. Job
//! history is audit data and never reconstructs session approval or resumes I/O.
use crate::{
    catalog::Catalog,
    domain::{AppError, ErrorCode, Result},
    operations::{CopyAction, CopyJobState, CopyWorkflowJob},
};
use rusqlite::{Connection, params};

pub(crate) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch(r#"
    CREATE TABLE operation_jobs(
      id TEXT PRIMARY KEY,plan_id TEXT NOT NULL REFERENCES organization_plans(id),
      root_id TEXT NOT NULL,review_id TEXT NOT NULL,scope_digest TEXT NOT NULL,
      kind TEXT NOT NULL CHECK(kind IN ('prepare_names','copy_files')),
      state TEXT NOT NULL CHECK(state IN ('queued','running','pause_requested','cancel_requested','prepared','completed_verified','canceled_with_partial_copies','needs_attention','needs_recovery')),
      total_files INTEGER NOT NULL CHECK(total_files BETWEEN 1 AND 15600),
      verified_files INTEGER NOT NULL CHECK(verified_files BETWEEN 0 AND total_files),
      payload TEXT NOT NULL CHECK(json_valid(payload) AND length(CAST(payload AS BLOB))<=65536),
      started_at TEXT NOT NULL,updated_at TEXT NOT NULL);
    CREATE INDEX operation_jobs_recent ON operation_jobs(updated_at DESC,id DESC);
    CREATE TRIGGER operation_job_identity_immutable BEFORE UPDATE OF id,plan_id,root_id,review_id,scope_digest,kind,total_files,started_at ON operation_jobs BEGIN SELECT RAISE(ABORT,'Operation job identity is immutable'); END;
    CREATE TRIGGER operation_job_history_retained BEFORE DELETE ON operation_jobs BEGIN SELECT RAISE(ABORT,'Operation job history is retained'); END;
    "#)?;
    Ok(())
}
fn failed(message: &str) -> AppError {
    AppError::new(ErrorCode::Catalog, message)
}
fn encode<T: serde::Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|error| failed("Could not encode operation job state.").caused_by(error))
}
fn state_name(state: CopyJobState) -> Result<String> {
    Ok(encode(&state)?.trim_matches('"').into())
}
fn decode_row(row: &rusqlite::Row<'_>) -> Result<CopyWorkflowJob> {
    let text: Option<String> = row.get("bounded_payload")?;
    let job: CopyWorkflowJob = serde_json::from_str(
        &text.ok_or_else(|| failed("Operation job exceeds its retained payload bound."))?,
    )
    .map_err(|error| failed("Invalid retained operation job.").caused_by(error))?;
    if job.id != row.get::<_, String>("id")?
        || job.plan_id != row.get::<_, String>("plan_id")?
        || job.source_root_id != row.get::<_, String>("root_id")?
        || job.review_id != row.get::<_, String>("review_id")?
        || state_name(job.state)? != row.get::<_, String>("state")?
        || encode(&job.action)?.trim_matches('"') != row.get::<_, String>("kind")?
        || job.total_files != row.get::<_, u32>("total_files")?
        || job.verified_files != row.get::<_, u32>("verified_files")?
        || job.started_at != row.get::<_, String>("started_at")?
        || job.updated_at != row.get::<_, String>("updated_at")?
    {
        return Err(failed(
            "Operation job payload differs from its durable identity/state.",
        ));
    }
    crate::operations::validate_id(&job.id)?;
    crate::operations::validate_id(&job.review_id)?;
    crate::operations::validate_id(&job.plan_id)?;
    Ok(job)
}
const COLUMNS: &str = "id,plan_id,root_id,review_id,kind,state,total_files,verified_files,started_at,updated_at,CASE WHEN length(CAST(payload AS BLOB))<=65536 THEN payload END AS bounded_payload";
pub(crate) fn load(catalog: &Catalog, id: String) -> Result<CopyWorkflowJob> {
    crate::operations::validate_id(&id)?;
    catalog.call(move |db| {
        let mut query = db.prepare(&format!("SELECT {COLUMNS} FROM operation_jobs WHERE id=?"))?;
        let mut rows = query.query([id])?;
        decode_row(
            rows.next()?
                .ok_or_else(|| failed("Operation job was not found."))?,
        )
    })
}
pub(crate) fn list(catalog: &Catalog) -> Result<Vec<CopyWorkflowJob>> {
    catalog.call(|db| {
        let mut query = db.prepare(&format!(
            "SELECT {COLUMNS} FROM operation_jobs ORDER BY updated_at DESC,id DESC LIMIT 32"
        ))?;
        let mut rows = query.query([])?;
        let mut jobs = Vec::new();
        while let Some(row) = rows.next()? {
            jobs.push(decode_row(row)?);
        }
        Ok(jobs)
    })
}
#[cfg(any(test, feature = "copy-workflow"))]
pub(crate) fn create(catalog: &Catalog, job: CopyWorkflowJob, digest: String) -> Result<()> {
    let payload = encode(&job)?;
    if payload.len() > 65536
        || job.message.len() > 2048
        || digest.len() != 64
        || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        || job.state != CopyJobState::Queued
        || job.verified_files != 0
        || job.ready_review_id.is_some()
    {
        return Err(failed("Invalid initial operation job scope/state."));
    }
    crate::operations::validate_id(&job.id)?;
    crate::operations::validate_id(&job.review_id)?;
    crate::operations::validate_id(&job.plan_id)?;
    catalog.call(move|db| {
        let tx=db.transaction()?;
        tx.execute("INSERT INTO operation_jobs(id,plan_id,root_id,review_id,scope_digest,kind,state,total_files,verified_files,payload,started_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,0,?9,?10,?11)",params![job.id,job.plan_id,job.source_root_id,job.review_id,digest,encode(&job.action)?.trim_matches('"'),state_name(job.state)?,job.total_files,payload,job.started_at,job.updated_at])?;
        tx.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,'operation_job_state_v1',?3,?4)",params![job.plan_id,job.id,encode(&serde_json::json!({"scopeDigest":digest,"job":job}))?,job.updated_at])?;
        tx.commit()?;
        Ok(())
    })
}
fn active(state: CopyJobState) -> bool {
    matches!(
        state,
        CopyJobState::Queued
            | CopyJobState::Running
            | CopyJobState::PauseRequested
            | CopyJobState::CancelRequested
    )
}
pub(crate) fn transition(
    catalog: &Catalog,
    mut next: CopyWorkflowJob,
    expected: CopyJobState,
) -> Result<()> {
    if !active(expected)
        || next.state == CopyJobState::Queued
        || (expected == CopyJobState::CancelRequested && next.state == CopyJobState::Running)
        || (expected == CopyJobState::Queued
            && matches!(
                next.state,
                CopyJobState::Prepared | CopyJobState::CompletedVerified
            ))
        || next.verified_files > next.total_files
        || next.message.len() > 2048
        || (next.state == CopyJobState::CompletedVerified
            && (next.verified_files != next.total_files || next.action != CopyAction::CopyFiles))
        || (next.state == CopyJobState::Prepared
            && (next.action != CopyAction::PrepareNames
                || next.ready_review_id.is_none()
                || next.verified_files != 0))
    {
        return Err(failed("Invalid operation job transition/progress."));
    }
    next.updated_at = super::store::now();
    let payload = encode(&next)?;
    if payload.len() > 65536 {
        return Err(failed("Operation job state exceeds 64 KiB."));
    }
    catalog.call(move|db| {
        let tx=db.transaction()?;
        let changed=tx.execute("UPDATE operation_jobs SET state=?1,verified_files=?2,payload=?3,updated_at=?4 WHERE id=?5 AND plan_id=?6 AND root_id=?7 AND review_id=?8 AND total_files=?9 AND kind=?10 AND state=?11 AND verified_files<=?2 AND started_at=?12",params![state_name(next.state)?,next.verified_files,payload,next.updated_at,next.id,next.plan_id,next.source_root_id,next.review_id,next.total_files,encode(&next.action)?.trim_matches('"'),state_name(expected)?,next.started_at])?;
        if changed!=1 {return Err(failed("Operation job changed before its state could be committed."));}
        tx.execute("INSERT INTO operation_journal(plan_id,item_id,phase,evidence,created_at) VALUES(?1,?2,'operation_job_state_v1',?3,?4)",params![next.plan_id,next.id,payload,next.updated_at])?;
        tx.commit()?;Ok(())
    })
}
pub(crate) fn recover(catalog: &Catalog) -> Result<()> {
    let ids=catalog.call(|db| {
        Ok(db.prepare("SELECT id FROM operation_jobs WHERE state IN ('queued','running','pause_requested','cancel_requested') LIMIT 65")?.query_map([],|row|row.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)
    })?;
    if ids.len() > 64 {
        return Err(failed(
            "Too many interrupted operation jobs require catalog review.",
        ));
    }
    for id in ids {
        let mut job = load(catalog, id)?;
        let expected = job.state;
        job.state = CopyJobState::NeedsRecovery;
        job.message =
            "Interrupted operation: inspect retained files and journal before any further writes."
                .into();
        transition(catalog, job, expected)?;
    }
    Ok(())
}

//! Schema-owned immutable backup bindings and append-only executor events.
use crate::domain::Result;
use rusqlite::Connection;

pub(crate) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch(r#"
CREATE TABLE backup_sync_jobs(
 id TEXT PRIMARY KEY,catalog_id TEXT NOT NULL,root_id TEXT NOT NULL,comparison_run TEXT NOT NULL,
 scope_json TEXT NOT NULL CHECK(json_valid(scope_json) AND length(CAST(scope_json AS BLOB))<=33554432),
 created_at TEXT NOT NULL,updated_at TEXT NOT NULL,state TEXT NOT NULL,phase TEXT NOT NULL,
 action TEXT NOT NULL,verified_files INTEGER NOT NULL DEFAULT 0 CHECK(verified_files>=0),
 ready_review_id TEXT,message TEXT NOT NULL
);
CREATE TRIGGER backup_sync_job_binding_immutable BEFORE UPDATE OF id,catalog_id,root_id,comparison_run,scope_json,created_at ON backup_sync_jobs BEGIN SELECT RAISE(ABORT,'Immutable sync scope'); END;
CREATE TRIGGER backup_sync_job_no_delete BEFORE DELETE ON backup_sync_jobs BEGIN SELECT RAISE(ABORT,'Sync history is retained'); END;
CREATE TABLE backup_sync_plans(
 job_id TEXT PRIMARY KEY REFERENCES backup_sync_jobs(id),digest TEXT NOT NULL,
 plan_json TEXT NOT NULL CHECK(json_valid(plan_json) AND length(CAST(plan_json AS BLOB))<=33554432)
);
CREATE TRIGGER backup_sync_plan_no_update BEFORE UPDATE ON backup_sync_plans BEGIN SELECT RAISE(ABORT,'Immutable sync plan'); END;
CREATE TRIGGER backup_sync_plan_no_delete BEFORE DELETE ON backup_sync_plans BEGIN SELECT RAISE(ABORT,'Sync plans are retained'); END;
CREATE TABLE backup_sync_events(
 job_id TEXT NOT NULL REFERENCES backup_sync_jobs(id),ordinal INTEGER NOT NULL CHECK(ordinal>0),
 item INTEGER,phase TEXT NOT NULL,
 evidence TEXT NOT NULL CHECK(json_valid(evidence) AND length(CAST(evidence AS BLOB))<=65536),
 PRIMARY KEY(job_id,ordinal)
);
CREATE TRIGGER backup_sync_event_no_update BEFORE UPDATE ON backup_sync_events BEGIN SELECT RAISE(ABORT,'Append-only sync events'); END;
CREATE TRIGGER backup_sync_event_no_delete BEFORE DELETE ON backup_sync_events BEGIN SELECT RAISE(ABORT,'Sync events are retained'); END;
"#)?;
    Ok(())
}

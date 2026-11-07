//! Read-only protection prerequisites. These observations never grant permission
//! to mutate originals: the whole-library executor remains unavailable. Its future
//! admission must recheck these prerequisites atomically with backup/sync starts
//! and schedule activation, then hold native write scopes through completion.
use super::{
    Library,
    model::{OrganizationProtectionBlocker as Blocker, OrganizationProtectionStatus},
};
use crate::domain::{AppError, ErrorCode, Result};
use rusqlite::Connection;

impl Library {
    pub fn organization_protection_status(&self) -> Result<OrganizationProtectionStatus> {
        let backup_active = !self.snapshots.idle()?;
        let sync_active = !self.comparisons.idle()?;
        #[cfg(any(test, feature = "backup-workflow", feature = "fixture-probes"))]
        let sync_active = sync_active || !self.sync_workflow.idle()?;
        let mut blockers = self.catalog.call(stored_blockers)?;
        if backup_active {
            blockers.insert(0, Blocker::BackupActive);
        }
        if sync_active {
            blockers.insert(0, Blocker::SyncActive);
        }
        Ok(OrganizationProtectionStatus { blockers })
    }
}

fn stored_blockers(db: &mut Connection) -> Result<Vec<Blocker>> {
    let mut blockers = Vec::new();
    if unfinished_jobs(db, true)? {
        blockers.push(Blocker::BackupRecoveryRequired);
    }
    if unfinished_jobs(db, false)? {
        blockers.push(Blocker::SyncRecoveryRequired);
    }
    if super::snapshots::has_restructure_blocking_schedule(db)? {
        blockers.push(Blocker::BackupScheduled);
    }
    Ok(blockers)
}

fn unfinished_jobs(db: &Connection, snapshot: bool) -> Result<bool> {
    // Query all retained history, not the latest UI page. Bound decoded state
    // sizes and distinct values; unfamiliar/corrupt history fails closed.
    let sql = if snapshot {
        "SELECT DISTINCT CASE WHEN length(CAST(record AS BLOB))<=33554432 AND json_type(record,'$.view.state')='text' AND length(json_extract(record,'$.view.state'))<=64 THEN json_extract(record,'$.view.state') END FROM backup_snapshot_jobs LIMIT 32"
    } else {
        "SELECT DISTINCT CASE WHEN length(state)<=64 THEN state END FROM backup_sync_jobs LIMIT 32"
    };
    let states = db
        .prepare(sql)?
        .query_map([], |row| row.get::<_, Option<String>>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut unfinished = false;
    for state in states {
        let terminal = match (snapshot, state.as_deref()) {
            (true, Some("completed_verified" | "completed_with_unverified_items"))
            | (false, Some("completed_verified" | "completed_partial" | "canceled" | "failed")) => {
                true
            }
            (_, Some("queued" | "running" | "cancel_requested" | "needs_recovery"))
            | (false, Some("paused" | "needs_review"))
            | (true, Some("canceled_with_partial_copies")) => false,
            _ => {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "Backup or sync history could not be verified. Resolve its recovery state before restructuring.",
                ));
            }
        };
        unfinished |= !terminal;
    }
    Ok(unfinished)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    fn fixture() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE backup_snapshot_jobs(id TEXT,record TEXT); CREATE TABLE backup_sync_jobs(id TEXT,state TEXT); CREATE TABLE backup_snapshot_schedules(profile_id TEXT,record TEXT);").unwrap();
        db
    }
    fn snapshot(db: &Connection, state: &str) {
        db.execute(
            "INSERT INTO backup_snapshot_jobs VALUES('generated',?)",
            [serde_json::json!({"view":{"state":state}}).to_string()],
        )
        .unwrap();
    }
    fn schedule(db: &Connection, enabled: bool, next_due: &str) {
        let id = uuid::Uuid::new_v4().to_string();
        let record = serde_json::json!({"view":{"profileId":id,"intervalMinutes":60,"timezone":"UTC","dstPolicy":"elapsed_interval","enabled":enabled,"catchUp":false,"nextDue":next_due,"lastRun":null,"fullyQuitSupported":false,"approvedDigest":"fixture"},"profile_digest":"fixture","manifest_digest":"fixture","instance":"different-imported-instance"});
        db.execute(
            "INSERT INTO backup_snapshot_schedules VALUES(?,?)",
            params![id, record.to_string()],
        )
        .unwrap();
    }
    #[test]
    fn every_enabled_schedule_blocks_even_far_future_overdue_or_deauthorized() {
        for due in ["2099-01-01T00:00:00Z", "2000-01-01T00:00:00Z", ""] {
            let mut db = fixture();
            schedule(&db, true, due);
            assert_eq!(
                stored_blockers(&mut db).unwrap(),
                vec![Blocker::BackupScheduled]
            );
            assert!(
                db.query_row(
                    "SELECT json_extract(record,'$.view.enabled') FROM backup_snapshot_schedules",
                    [],
                    |r| r.get::<_, bool>(0)
                )
                .unwrap(),
                "The check must never disable schedules"
            );
        }
    }
    #[test]
    fn disabled_schedules_and_terminal_history_do_not_block() {
        let mut db = fixture();
        schedule(&db, false, "2099-01-01T00:00:00Z");
        for state in ["completed_verified", "completed_with_unverified_items"] {
            snapshot(&db, state);
        }
        for state in [
            "completed_verified",
            "completed_partial",
            "canceled",
            "failed",
        ] {
            db.execute(
                "INSERT INTO backup_sync_jobs VALUES('generated',?)",
                [state],
            )
            .unwrap();
        }
        assert!(stored_blockers(&mut db).unwrap().is_empty());
    }
    #[test]
    fn unfinished_or_canceled_partial_backups_require_recovery() {
        for state in [
            "queued",
            "running",
            "cancel_requested",
            "needs_recovery",
            "canceled_with_partial_copies",
        ] {
            let mut db = fixture();
            snapshot(&db, state);
            assert_eq!(
                stored_blockers(&mut db).unwrap(),
                vec![Blocker::BackupRecoveryRequired]
            );
        }
    }
    #[test]
    fn paused_pending_and_interrupted_syncs_cannot_hide_behind_recent_history() {
        for state in [
            "queued",
            "running",
            "paused",
            "cancel_requested",
            "needs_review",
            "needs_recovery",
        ] {
            let mut db = fixture();
            db.execute("INSERT INTO backup_sync_jobs VALUES('old',?)", [state])
                .unwrap();
            for n in 0..256 {
                db.execute(
                    "INSERT INTO backup_sync_jobs VALUES(?,'completed_verified')",
                    [n.to_string()],
                )
                .unwrap();
            }
            assert_eq!(
                stored_blockers(&mut db).unwrap(),
                vec![Blocker::SyncRecoveryRequired]
            );
        }
    }
    #[test]
    fn malformed_history_and_recurring_records_fail_closed() {
        let mut db = fixture();
        snapshot(&db, "future_unknown_state");
        assert!(stored_blockers(&mut db).is_err());
        db.execute("DELETE FROM backup_snapshot_jobs", []).unwrap();
        db.execute("INSERT INTO backup_sync_jobs VALUES('broken',NULL)", [])
            .unwrap();
        assert!(stored_blockers(&mut db).is_err());
        db.execute("DELETE FROM backup_sync_jobs", []).unwrap();
        db.execute(
            "INSERT INTO backup_snapshot_schedules VALUES('not-an-id','{}')",
            [],
        )
        .unwrap();
        assert!(stored_blockers(&mut db).is_err());
    }
    #[test]
    fn all_blockers_remain_visible_and_no_records_are_changed() {
        let mut db = fixture();
        snapshot(&db, "needs_recovery");
        db.execute(
            "INSERT INTO backup_sync_jobs VALUES('interrupted','needs_recovery')",
            [],
        )
        .unwrap();
        schedule(&db, true, "2099-01-01T00:00:00Z");
        let before = db.total_changes();
        assert_eq!(
            stored_blockers(&mut db).unwrap(),
            vec![
                Blocker::BackupRecoveryRequired,
                Blocker::SyncRecoveryRequired,
                Blocker::BackupScheduled
            ]
        );
        assert_eq!(db.total_changes(), before);
    }

    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn library_boundary_reads_native_history_without_enabling_execution() {
        let owned = tempfile::tempdir().unwrap();
        let data = owned.path().canonicalize().unwrap();
        let catalog = std::sync::Arc::new(
            crate::catalog::Catalog::open(data.join("Generated.stillharbour")).unwrap(),
        );
        let library = Library::new(
            catalog.clone(),
            super::super::LibraryOptions {
                data_directory: data.clone(),
                workers: super::super::WorkerPaths {
                    executable: data.join("missing-worker"),
                    exiftool: None,
                    working_directory: data.join("worker"),
                },
            },
        )
        .unwrap();
        assert!(
            library
                .organization_protection_status()
                .unwrap()
                .blockers
                .is_empty()
        );
        // No profile, credentials, helpers or source library are authorized.
        // A retained interrupted observation alone must still block the boundary.
        catalog.call(|db| {
            db.execute("INSERT INTO backup_snapshot_jobs(id,profile_id,record) VALUES('generated','generated',?)", [serde_json::json!({"view":{"state":"needs_recovery"}}).to_string()])?;
            Ok(())
        }).unwrap();
        assert_eq!(
            library.organization_protection_status().unwrap().blockers,
            vec![Blocker::BackupRecoveryRequired]
        );
    }
}

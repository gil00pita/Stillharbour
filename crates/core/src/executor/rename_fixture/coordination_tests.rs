use super::*;
use crate::{
    domain::ErrorCode,
    library::{Library, LibraryOptions, model::*, original_jobs, store, workers::WorkerPaths},
    operations::original_jobs::OriginalJobState,
};

fn approve(session: &mut GeneratedRenameSession) -> GeneratedRenameReview {
    let view = session.inspect().unwrap();
    session
        .request(GeneratedRenameRequest::Approve {
            plan_id: view.plan_id,
            acknowledged: true,
        })
        .unwrap()
}
fn queued(session: &mut GeneratedRenameSession) -> String {
    let approved = approve(session);
    session
        .prepare_start(approved.plan_id, approved.ticket.unwrap())
        .unwrap()
        .0
        .id
}
fn reader(path: &std::path::Path) -> Result<crate::jobs::operation_locks::Lease> {
    Registry::shared().try_acquire(vec![Scope::new(root(path)?, Access::Read)])
}
fn scan_and_check(
    library: &std::sync::Arc<Library>,
    root_id: &str,
    before: &GeneratedRenameReview,
) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let job = loop {
        match library.scan(root_id.into(), ScanFilter::default()) {
            Ok(job) => break job,
            Err(error) if error.code == ErrorCode::Busy && std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(10))
            }
            Err(error) => panic!("Settled scan refused: {error}"),
        }
    };
    loop {
        let job = store::job(&library.catalog, job.id.clone()).unwrap();
        if matches!(job.state.as_str(), "completed" | "partial") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Owned settled scan did not finish: {}",
            job.state
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    // These are deliberately non-media text payloads. This tests discovery and
    // identity reconciliation, not decoder/format support or thumbnail success.
    let rows = library
        .catalog
        .call(|db| {
            Ok(db
                .prepare(
                    "SELECT id,relative_path,presence,favorite,rating,tags FROM assets ORDER BY id",
                )?
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, bool>(3)?,
                        r.get::<_, u32>(4)?,
                        r.get::<_, String>(5)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        })
        .unwrap();
    assert_eq!(rows.len(), 5, "No staged/probe phantom assets");
    for member in &before.members {
        let row = rows
            .iter()
            .find(|r| r.0 == member.catalog_asset_id)
            .unwrap();
        assert_eq!(row.1, member.catalog_path);
        assert_eq!(row.2, "present");
        assert!(row.3 && row.4 == 4 && row.5.contains("generated-annotation"));
    }
}

#[test]
fn generated_original_coordination_retains_lease_through_commit_and_defers_stopped_scans() {
    // Exercise the same independent paced-I/O budget as the desktop process,
    // rather than sharing a liveness deadline with unrelated fault matrices.
    if crate::test_process::isolated(
        "executor::rename_fixture::session::coordination_tests::generated_original_coordination_retains_lease_through_commit_and_defers_stopped_scans",
        60,
    ) {
        return;
    }
    let mut session = GeneratedRenameSession::create().unwrap();
    let db = session.reader();
    let path = session.fixture.run.path().canonicalize().unwrap();
    let data = path.join("application-data");
    std::fs::create_dir(&data).unwrap();
    let library = Library::new(
        db.clone(),
        LibraryOptions {
            data_directory: data.clone(),
            workers: WorkerPaths {
                executable: data.join("missing-generated-worker"),
                exiftool: None,
                working_directory: data.join("worker"),
            },
        },
    )
    .unwrap();
    let root_id = store::roots(&db).unwrap()[0].view.id.clone();
    let source = path.join("originals");
    original_jobs::require_reconciled_root(&db, root_id.clone()).unwrap();
    let job = queued(&mut session);
    original_jobs::transition(
        &db,
        job.clone(),
        OriginalJobState::Running,
        "Generated coordination test".into(),
    )
    .unwrap();
    {
        let scope = session.fixture.execution_scope().unwrap();
        assert!(reader(&source).is_err(), "Backup-style readers must wait");
        assert_eq!(
            library
                .scan(root_id.clone(), ScanFilter::default())
                .unwrap_err()
                .code,
            ErrorCode::Busy
        );
        assert!(
            !session
                .fixture
                .execute_in_scope(&session.active, Some(Fault::AfterMove(0)), None, &scope)
                .unwrap()
        );
    }
    session.record_stopped(&job, failed("Generated intentional interruption"));
    assert!(reader(&source).is_ok(), "No leaked active lease after stop");
    assert_eq!(
        library
            .scan(root_id.clone(), ScanFilter::default())
            .unwrap_err()
            .code,
        ErrorCode::Busy
    );
    assert!(
        store::jobs(&db).unwrap().is_empty(),
        "No phantom scan job before reconciliation"
    );
    assert_eq!(
        library
            .prepare_maintenance(MaintenanceTarget::Library {
                root_id: root_id.clone()
            })
            .unwrap_err()
            .code,
        ErrorCode::Busy
    );
    let pending = session.inspect().unwrap();
    assert!(!pending.catalog_committed);
    assert!(pending.members.iter().all(|m| m.catalog_path == m.before));
    let other_path = path.join("other-generated-root");
    std::fs::create_dir(&other_path).unwrap();
    let other = root(&other_path).unwrap();
    let other_id = store::add_root(
        &db,
        other_path,
        filesystem::identity(other.leaf()).unwrap(),
        source_native::volume_token(other.leaf()).unwrap(),
        "Unrelated generated root".into(),
        RootMode::Referenced,
        ScanFilter::default(),
    )
    .unwrap();
    library.start_monitoring().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let status = library.status().unwrap();
        let deferred = status.monitoring.as_ref().is_some_and(|views| {
            views.iter().any(|view| {
                view.root_id == root_id
                    && view.mode == "deferred"
                    && view.message.contains("Organization recovery")
            })
        });
        let other_scanned = status
            .jobs
            .iter()
            .any(|job| job.root_id == other_id && job.state == "completed");
        if deferred && other_scanned {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Monitor must explain pending recovery and reconcile the unrelated root"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    library.cancel_all();
    while std::sync::Arc::strong_count(&library) != 1 {
        assert!(
            std::time::Instant::now() < deadline,
            "Owned monitor/scan must release its library before continuation"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    scan_and_check(&library, &other_id, &pending);
    let job = queued(&mut session);
    original_jobs::transition(
        &db,
        job.clone(),
        OriginalJobState::Running,
        "Fresh approved continuation".into(),
    )
    .unwrap();
    {
        let scope = session.fixture.execution_scope().unwrap();
        assert!(
            session
                .fixture
                .execute_in_scope(&session.active, None, None, &scope)
                .unwrap()
        );
        assert!(
            reader(&source).is_err(),
            "No read gap after final native movement"
        );
        let displaced = path.join("displaced-originals");
        #[cfg(windows)]
        assert!(
            std::fs::rename(&source, &displaced).is_err(),
            "Held native directory refuses delete sharing"
        );
        #[cfg(unix)]
        {
            std::fs::rename(&source, &displaced).unwrap();
            std::fs::create_dir(&source).unwrap();
            std::fs::write(
                source.join("foreign-keeper.txt"),
                b"retain this new occupant",
            )
            .unwrap();
            assert!(session.commit_catalog(job.clone(), &scope).is_err());
            assert_ne!(
                original_jobs::load(&db, job.clone()).unwrap().state,
                OriginalJobState::CompletedVerified
            );
            std::fs::rename(&source, path.join("retained-foreign-root")).unwrap();
            std::fs::rename(&displaced, &source).unwrap();
        }
        session.commit_catalog(job, &scope).unwrap();
        assert!(
            reader(&source).is_err(),
            "Scope remains held through catalog commit"
        );
    }
    assert!(reader(&source).is_ok());
    original_jobs::require_reconciled_root(&db, root_id.clone()).unwrap();
    let complete = session.inspect().unwrap();
    scan_and_check(&library, &root_id, &complete);
    session
        .request(GeneratedRenameRequest::ReviewReverse {})
        .unwrap();
    original_jobs::require_reconciled_root(&db, root_id.clone()).unwrap();
    let reverse_job = queued(&mut session);
    original_jobs::transition(
        &db,
        reverse_job,
        OriginalJobState::NeedsRecovery,
        "Stopped before undo".into(),
    )
    .unwrap();
    assert!(
        original_jobs::require_reconciled_root(&db, root_id.clone()).is_err(),
        "Forward completion must not resolve a different reverse plan"
    );
    let approved = approve(&mut session);
    let restored = session
        .request(GeneratedRenameRequest::Start {
            plan_id: approved.plan_id,
            ticket: approved.ticket.unwrap(),
        })
        .unwrap();
    original_jobs::require_reconciled_root(&db, root_id.clone()).unwrap();
    scan_and_check(&library, &root_id, &restored);
    #[cfg(unix)]
    assert_eq!(
        std::fs::read(path.join("retained-foreign-root/foreign-keeper.txt")).unwrap(),
        b"retain this new occupant"
    );
    drop(library);
}

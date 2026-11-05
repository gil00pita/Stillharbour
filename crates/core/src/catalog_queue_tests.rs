//! Exercise the actual bounded actor, including stopped and interrupted work.
use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[test]
fn full_catalog_queue_names_each_rejected_operation_and_never_runs_it() {
    let owned = tempfile::tempdir().unwrap();
    let catalog =
        Catalog::open(owned.path().join("private-generated-catalog.stillharbour")).unwrap();
    let (entered, ready) = mpsc::sync_channel(1);
    let (release, held) = mpsc::sync_channel::<()>(1);
    catalog
        .sender
        .send(Request::Run(Box::new(move |_, _| {
            entered.send(()).unwrap();
            let _ = held.recv_timeout(Duration::from_secs(10));
        })))
        .unwrap();
    ready.recv_timeout(Duration::from_secs(10)).unwrap();
    for _ in 0..16 {
        let (reply, _response) = mpsc::sync_channel(1);
        catalog.sender.try_send(Request::Summary(reply)).unwrap();
    }
    let ran = Arc::new(AtomicBool::new(false));
    let observed = ran.clone();
    let output = owned.path().join("private-generated-snapshot.stillharbour");
    let errors = [
        ("catalog summary", catalog.summary().unwrap_err()),
        (
            "catalog query or update",
            catalog
                .call(move |_| {
                    observed.store(true, Ordering::Release);
                    Ok(())
                })
                .unwrap_err(),
        ),
        (
            "catalog snapshot",
            catalog.snapshot(output.clone()).unwrap_err(),
        ),
        (
            "catalog recovery",
            catalog.freeze_writes_for_restore().unwrap_err(),
        ),
    ];
    for (operation, error) in errors {
        assert_eq!(error.code, ErrorCode::Busy);
        assert!(error.message.contains("queue is full"), "{}", error.message);
        assert!(
            error.message.contains("16 waiting requests"),
            "{}",
            error.message
        );
        assert!(error.message.contains(operation), "{}", error.message);
        assert!(error.message.contains("not queued"), "{}", error.message);
        assert!(error.message.contains("try again"), "{}", error.message);
        if operation == "catalog recovery" {
            assert!(error.message.contains("Close and reopen Stillharbour"));
        } else {
            assert!(error.message.contains("Wait a moment"));
        }
        assert!(
            !serde_json::to_string(&error)
                .unwrap()
                .contains("private-generated")
        );
    }
    drop(release);
    drop(catalog);
    assert!(!ran.load(Ordering::Acquire));
    assert!(!output.exists());
}

#[test]
fn disconnected_catalog_worker_is_not_reported_as_a_full_queue() {
    let owned = tempfile::tempdir().unwrap();
    let mut catalog = Catalog::open(owned.path().join("generated.stillharbour")).unwrap();
    catalog.sender.send(Request::Stop).unwrap();
    catalog.worker.take().unwrap().join().unwrap();
    let errors = [
        catalog.summary().unwrap_err(),
        catalog.call(|_| Ok(())).unwrap_err(),
        catalog
            .snapshot(owned.path().join("unused.stillharbour"))
            .unwrap_err(),
        catalog.freeze_writes_for_restore().unwrap_err(),
    ];
    for error in errors {
        assert_eq!(error.code, ErrorCode::Catalog);
        assert!(error.message.contains("worker"));
        assert!(error.message.contains("not queued"));
        assert!(error.message.contains("reopen Stillharbour"));
        assert!(!error.message.contains("queue is full"));
    }
}

#[test]
fn lost_catalog_reply_reports_unconfirmed_completion_without_exposing_worker_panic() {
    let owned = tempfile::tempdir().unwrap();
    let path = owned.path().join("generated.stillharbour");
    let catalog = Catalog::open(path.clone()).unwrap();
    let error = catalog
        .call::<()>(|db| {
            // A lost reply is not proof that accepted work was rolled back.
            db.execute_batch("CREATE TABLE generated_completed_change(value INTEGER); INSERT INTO generated_completed_change VALUES(7);")?;
            panic!("generated-private-panic-detail")
        })
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Catalog);
    assert!(error.message.contains("catalog query or update"));
    assert!(error.message.contains("completion could not be confirmed"));
    assert!(!error.message.contains("not queued"));
    assert!(
        !serde_json::to_string(&error)
            .unwrap()
            .contains("generated-private")
    );
    drop(catalog);
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(
        db.query_row("SELECT value FROM generated_completed_change", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap(),
        7
    );
}

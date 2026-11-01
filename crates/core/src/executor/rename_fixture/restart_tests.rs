//! Real child exits using the same typed recovery decoder as native reopening.
//! Only the parent owns cleanup; no renderer path or approval is imported.
use super::*;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
const PHASES: [&str; 6] = [
    "before_staging",
    "after_staging",
    "during_targets",
    "before_receipt",
    "after_receipt",
    "after_commit",
];
fn save(session: &GeneratedRenameSession) {
    super::recovery_scope::save(session, "catalog-child-scope.json").unwrap();
}
fn reopen(path: &Path, token: &str) -> GeneratedRenameSession {
    let path = path.canonicalize().unwrap();
    assert!(path.starts_with(std::env::temp_dir().canonicalize().unwrap()));
    assert!(
        path.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("stillharbour-rename-fixture-")
    );
    super::recovery_scope::reopen(
        &path,
        token,
        "catalog-child-scope.json",
        Run::Child(path.clone()),
    )
    .unwrap()
}
fn approve(session: &mut GeneratedRenameSession) -> GeneratedRenameReview {
    let view = session.inspect().unwrap();
    session
        .request(GeneratedRenameRequest::Approve {
            plan_id: view.plan_id,
            acknowledged: true,
        })
        .unwrap()
}
fn start(
    session: &mut GeneratedRenameSession,
    view: GeneratedRenameReview,
) -> Result<GeneratedRenameReview> {
    session.request(GeneratedRenameRequest::Start {
        plan_id: view.plan_id,
        ticket: view.ticket.unwrap(),
    })
}
#[test]
fn generated_catalog_exit_child() {
    let Some(path) = std::env::var_os("STILLHARBOUR_GENERATED_CATALOG_ROOT") else {
        return;
    };
    let token = std::env::var("STILLHARBOUR_GENERATED_CATALOG_TOKEN").unwrap();
    let phase = std::env::var("STILLHARBOUR_GENERATED_CATALOG_EXIT").unwrap();
    assert!(PHASES.contains(&phase.as_str()));
    let mut session = reopen(&PathBuf::from(path), &token);
    let view = approve(&mut session);
    start(&mut session, view).unwrap();
    panic!("The requested owned catalog child exit did not occur");
}
#[test]
fn generated_catalog_cold_scope_rejects_unknown_fields_and_replaced_version_or_catalog_objects() {
    let mut session = GeneratedRenameSession::create().unwrap();
    save(&session);
    let token = session.fixture.plan.id.clone();
    let path = session.fixture.run.path().canonicalize().unwrap();
    let owner = match std::mem::replace(&mut session.fixture.run, Run::Child(path.clone())) {
        Run::Owned(owner) => owner,
        Run::Child(_) => panic!("Expected owned generated scope"),
    };
    drop(session);
    let scope_path = path.join("catalog-child-scope.json");
    let original = std::fs::read(&scope_path).unwrap();
    for variant in 0..3 {
        let mut changed: serde_json::Value = serde_json::from_slice(&original).unwrap();
        match variant {
            0 => {
                changed["unknownScope"] = serde_json::json!("untrusted-field");
            }
            1 => {
                let member = changed["originals"]
                    .as_object_mut()
                    .unwrap()
                    .values_mut()
                    .next()
                    .unwrap();
                member["version"]["file"] =
                    serde_json::json!(member["version"]["file"].as_u64().unwrap() ^ 1);
            }
            _ => {
                changed["catalog"]["file"]["file"] =
                    serde_json::json!(changed["catalog"]["file"]["file"].as_u64().unwrap() ^ 1);
            }
        }
        // Only this exact constructor-owned control file is changed. Originals,
        // retained versions and the actual catalog remain untouched.
        std::fs::write(&scope_path, serde_json::to_vec(&changed).unwrap()).unwrap();
        assert!(std::panic::catch_unwind(|| reopen(&path, &token)).is_err());
        std::fs::write(&scope_path, &original).unwrap();
        let recovered = reopen(&path, &token);
        let inspected = recovered.inspect().unwrap();
        assert_eq!(inspected.state, GeneratedRenameState::Review);
        assert_eq!(inspected.completed_steps, 0);
        assert!(!inspected.pending_intent && !inspected.catalog_committed);
        assert!(
            inspected
                .members
                .iter()
                .all(|member| member.observed == member.before
                    && member.catalog_path == member.before)
        );
        drop(recovered);
    }
    drop(owner);
}
#[test]
fn generated_catalog_process_exit_reopens_cold_without_approval_or_blind_native_replay() {
    for reverse in [false, true] {
        for phase in PHASES {
            let mut session = GeneratedRenameSession::create().unwrap();
            if reverse {
                let grant = approve(&mut session);
                start(&mut session, grant).unwrap();
                session
                    .request(GeneratedRenameRequest::ReviewReverse {})
                    .unwrap();
            }
            let initial = session.inspect().unwrap();
            let old = approve(&mut session);
            save(&session);
            let token = session.fixture.plan.id.clone();
            let path = session.fixture.run.path().canonicalize().unwrap();
            // The parent alone retains cleanup ownership; both restarted scopes
            // borrow it. Close SQLite/its lifetime lock before starting a child.
            let owner = match std::mem::replace(&mut session.fixture.run, Run::Child(path.clone()))
            {
                Run::Owned(owner) => owner,
                Run::Child(_) => panic!("Expected constructor-owned cleanup"),
            };
            drop(session);
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "executor::rename_fixture::session::restart_tests::generated_catalog_exit_child", "--nocapture"])
                .env("STILLHARBOUR_GENERATED_CATALOG_ROOT", &path).env("STILLHARBOUR_GENERATED_CATALOG_TOKEN", &token)
                .env("STILLHARBOUR_GENERATED_CATALOG_EXIT", phase)
                .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().unwrap();
            let deadline = Instant::now() + Duration::from_secs(45);
            while child.try_wait().unwrap().is_none() {
                if Instant::now() >= deadline {
                    child.kill().unwrap();
                    child.wait().unwrap();
                    panic!("Owned generated catalog child exceeded 45 seconds");
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let result = child.wait_with_output().unwrap();
            assert_eq!(
                result.status.code(),
                Some(81),
                "reverse={reverse}, phase={phase}: {} {}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
            let mut recovered = reopen(&path, &token);
            let journal = recovered.fixture.journal(&recovered.active).unwrap();
            assert_eq!(
                Fixture::cursor(&journal, &recovered.active).unwrap(),
                (10, false)
            );
            let facts: i64 = journal
                .query_row("SELECT count(*) FROM events", [], |row| row.get(0))
                .unwrap();
            let inspected = recovered.inspect().unwrap();
            assert_eq!(
                inspected.state,
                if phase == "after_commit" {
                    GeneratedRenameState::Completed
                } else {
                    GeneratedRenameState::NeedsCatalogCommit
                }
            );
            assert!(inspected.ticket.is_none());
            let job = inspected
                .jobs
                .iter()
                .find(|job| job.plan_id == inspected.plan_id)
                .unwrap();
            assert_eq!(
                job.state,
                if phase == "after_commit" {
                    crate::operations::original_jobs::OriginalJobState::CompletedVerified
                } else {
                    crate::operations::original_jobs::OriginalJobState::NeedsRecovery
                }
            );
            assert!(
                inspected
                    .members
                    .iter()
                    .all(|member| member.observed == member.after
                        && &member.catalog_path
                            == if phase == "after_commit" {
                                &member.after
                            } else {
                                &member.before
                            })
            );
            assert!(start(&mut recovered, old).is_err());
            if inspected.state != GeneratedRenameState::Completed {
                let grant = approve(&mut recovered);
                start(&mut recovered, grant).unwrap();
            }
            let done = recovered.inspect().unwrap();
            assert_eq!(done.state, GeneratedRenameState::Completed);
            assert!(done.catalog_committed);
            assert!(done.jobs.iter().any(|job| job.plan_id == done.plan_id
                && job.state
                    == crate::operations::original_jobs::OriginalJobState::CompletedVerified
                && job.verified_moves == 10));
            assert!(
                done.members
                    .iter()
                    .all(|member| member.catalog_path == member.observed
                        && initial.members.iter().any(|before| before.id == member.id
                            && before.catalog_asset_id == member.catalog_asset_id))
            );
            assert_eq!(
                journal
                    .query_row("SELECT count(*) FROM events", [], |row| row
                        .get::<_, i64>(0))
                    .unwrap(),
                facts
            );
            drop(journal);
            drop(recovered);
            drop(owner);
        }
    }
}

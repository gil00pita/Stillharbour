//! Real child exits over a durable, bounded generated scope. These helpers are
//! test-only; no desktop IPC accepts or adopts this file or an arbitrary path.
use super::*;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginalScope {
    object: super::super::super::Object,
    version: super::super::super::Object,
    modified: SystemTime,
    bytes: u64,
    readonly: bool,
    created: Option<SystemTime>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scope {
    policy: String,
    forward: Plan,
    active: Plan,
    reverse: bool,
    source: super::super::super::Object,
    versions: super::super::super::Object,
    originals: BTreeMap<String, OriginalScope>,
    probes: Vec<super::super::super::Object>,
    catalog: super::super::catalog::ReopenScope,
}
const PHASES: [&str; 6] = [
    "before_staging",
    "after_staging",
    "during_targets",
    "before_receipt",
    "after_receipt",
    "after_commit",
];
fn write_owned(path: &Path, name: &str, bytes: &[u8]) {
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path.join(name))
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
fn bounded(parent: &File, name: &str, limit: u64) -> Vec<u8> {
    let file = native::read(parent, name).unwrap();
    assert_eq!(filesystem::identity(&file).unwrap().links, 1);
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).unwrap();
    assert!(bytes.len() as u64 <= limit);
    bytes
}
fn save(session: &GeneratedRenameSession) {
    let scope = Scope {
        policy: "generated-rename-cold-v1".into(),
        forward: session.fixture.plan.clone(),
        active: session.active.clone(),
        reverse: session.reverse,
        source: session.fixture.source_root.clone(),
        versions: session.fixture.version_root.clone(),
        originals: session
            .fixture
            .originals
            .iter()
            .map(|(id, original)| {
                (
                    id.clone(),
                    OriginalScope {
                        object: super::super::super::Object {
                            volume: original.identity.volume,
                            file: original.identity.file,
                        },
                        version: super::super::super::Object {
                            volume: original.version.volume,
                            file: original.version.file,
                        },
                        modified: original.modified,
                        bytes: original.bytes,
                        readonly: original.readonly,
                        created: original.created,
                    },
                )
            })
            .collect(),
        probes: session.fixture.probes.clone(),
        catalog: session.catalog.reopen_scope(&session.fixture).unwrap(),
    };
    let bytes = serde_json::to_vec(&scope).unwrap();
    assert!(bytes.len() <= 128 * 1024);
    write_owned(
        session.fixture.run.path(),
        "catalog-child-owner",
        session.fixture.plan.id.as_bytes(),
    );
    write_owned(
        session.fixture.run.path(),
        "catalog-child-scope.json",
        &bytes,
    );
}
fn reopen(path: &Path, token: &str) -> GeneratedRenameSession {
    uuid::Uuid::parse_str(token).unwrap();
    let path = path.canonicalize().unwrap();
    assert!(path.starts_with(std::env::temp_dir().canonicalize().unwrap()));
    assert!(
        path.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("stillharbour-rename-fixture-")
    );
    let container = root(&path).unwrap();
    assert_eq!(
        bounded(container.leaf(), "catalog-child-owner", 36),
        token.as_bytes()
    );
    let scope: Scope = serde_json::from_slice(&bounded(
        container.leaf(),
        "catalog-child-scope.json",
        128 * 1024,
    ))
    .unwrap();
    assert_eq!(scope.policy, "generated-rename-cold-v1");
    assert_eq!(scope.forward.id, token);
    assert_eq!(scope.forward.members.len(), MEMBERS.len());
    assert_eq!(scope.active.members.len(), MEMBERS.len());
    assert_eq!(scope.originals.len(), MEMBERS.len());
    assert_eq!(scope.active.root_id, scope.forward.root_id);
    assert!(scope.probes.len() <= 64);
    for plan in [&scope.forward, &scope.active] {
        let canonical = graph::build(
            plan.id.clone(),
            plan.members.clone(),
            plan.name_facts.clone(),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_vec(&canonical).unwrap(),
            serde_json::to_vec(plan).unwrap()
        );
        assert!(plan.issues.is_empty());
    }
    let directory = root(&path.join("originals")).unwrap();
    let versions = root(&path.join("retained")).unwrap();
    assert!(scope.source.matches(directory.leaf()).unwrap());
    assert!(scope.versions.matches(versions.leaf()).unwrap());
    let mut seen = std::collections::BTreeSet::new();
    let mut originals = BTreeMap::new();
    for member in &scope.forward.members {
        let index = MEMBERS
            .iter()
            .position(|(before, after)| member.before == *before && member.after == *after)
            .unwrap();
        assert!(seen.insert(index));
        let active = scope
            .active
            .members
            .iter()
            .find(|active| active.id == member.id)
            .unwrap();
        assert_eq!(
            (&active.before, &active.after),
            if scope.reverse {
                (&member.after, &member.before)
            } else {
                (&member.before, &member.after)
            }
        );
        let retained = &scope.originals[&member.id];
        let expected = payload(index);
        assert_eq!(retained.bytes, expected.len() as u64);
        let version = native::read(versions.leaf(), &member.id).unwrap();
        assert!(retained.version.matches(&version).unwrap());
        assert_eq!(
            bounded(versions.leaf(), &member.id, retained.bytes),
            expected.as_bytes()
        );
        originals.insert(
            member.id.clone(),
            Original {
                identity: FileIdentity {
                    volume: retained.object.volume,
                    file: retained.object.file,
                    links: 1,
                },
                version: FileIdentity {
                    volume: retained.version.volume,
                    file: retained.version.file,
                    links: 1,
                },
                modified: retained.modified,
                bytes: retained.bytes,
                readonly: retained.readonly,
                created: retained.created,
            },
        );
    }
    for (index, object) in scope.probes.iter().enumerate() {
        let file = native::read(directory.leaf(), &format!(".stillharbour-probe-{index}")).unwrap();
        assert!(object.matches(&file).unwrap());
        assert_eq!(file.metadata().unwrap().len(), 0);
        assert_eq!(filesystem::identity(&file).unwrap().links, 1);
    }
    let fixture = Fixture {
        run: Run::Child(path),
        root_id: scope.forward.root_id.clone(),
        source_root: scope.source,
        version_root: scope.versions,
        originals,
        probes: scope.probes,
        plan: scope.forward,
    };
    let journal = fixture.journal(&scope.active).unwrap();
    let positions = fixture
        .locations(&scope.active, directory.leaf(), &journal)
        .unwrap();
    fixture
        .verify(directory.leaf(), versions.leaf(), &positions)
        .unwrap();
    drop(journal);
    let catalog = super::super::catalog::GeneratedCatalog::reopen(&fixture, scope.catalog).unwrap();
    let session = GeneratedRenameSession {
        catalog,
        fixture,
        active: scope.active,
        reverse: scope.reverse,
        approvals: approval::Approvals::new(),
    };
    session.inspect().unwrap();
    session
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

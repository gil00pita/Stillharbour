//! Actual generated-only child process exits. These are NOT SIGKILL tests.
use super::application_tests::{Fixture, wait};
use super::*;
use crate::library::{LibraryOptions, WorkerPaths};
use std::{
    io::{Read, Write},
    process::{Child, Command, Stdio},
};
const CHILD_TEST: &str = "library::snapshots::process_failures::snapshot_actual_process_exit_child";
const ROOT_ENV: &str = "STILLHARBOUR_SNAPSHOT_EXIT_ROOT";
const OWNER_ENV: &str = "STILLHARBOUR_SNAPSHOT_EXIT_OWNER";
const PHASE_ENV: &str = "STILLHARBOUR_SNAPSHOT_EXIT_PHASE";
const PHASES: [&str; 8] = [
    "staging",
    "catalog_snapshot",
    "catalog_copy_step",
    "snapshotting",
    "snapshot_included",
    "comparing",
    "snapshot_recorded",
    "restore_published",
];
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnedScope {
    owner: String,
    catalog: String,
    profile: String,
    root: String,
    credential: String,
    identity: FileIdentity,
    volume: String,
    baseline_snapshot: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExitProof {
    phase: String,
    job: String,
    review: String,
    snapshot: Option<String>,
    action: String,
    #[serde(default)]
    catalog_pages_remaining: Option<i32>,
}
fn read_exit_proof(path: &Path) -> ExitProof {
    let mut bytes = Vec::new();
    File::open(path)
        .unwrap()
        .take(8193)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 8192);
    serde_json::from_slice(&bytes).unwrap()
}
fn helpers() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/backup-helpers")
        .join(if cfg!(windows) {
            "windows-x86_64"
        } else {
            "macos-aarch64"
        })
        .canonicalize()
        .unwrap()
}
fn write_owned(path: &Path, bytes: &[u8]) {
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
fn load_scope(root: &Path, owner: &str) -> OwnedScope {
    assert!(uuid::Uuid::parse_str(owner).is_ok());
    let root = root.canonicalize().unwrap();
    let temp = std::env::temp_dir().canonicalize().unwrap();
    assert_eq!(root.parent(), Some(temp.as_path()));
    let file = File::open(root.join(format!("snapshot-process-{owner}.json"))).unwrap();
    let mut bytes = Vec::new();
    file.take(8193).read_to_end(&mut bytes).unwrap();
    assert!(bytes.len() <= 8192);
    let scope: OwnedScope = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(scope.owner, owner);
    for id in [
        &scope.catalog,
        &scope.profile,
        &scope.root,
        &scope.credential,
    ] {
        assert!(uuid::Uuid::parse_str(id).is_ok());
    }
    snapshot_id(&scope.baseline_snapshot).unwrap();
    let held = native::open_root(&root, Some(&scope.identity)).unwrap();
    assert_eq!(native::volume_token(&held).unwrap(), scope.volume);
    scope
}
pub(super) fn phase(workflow: &SnapshotWorkflow, job: &JobRecord, phase: &str) {
    let Some(root) = std::env::var_os(ROOT_ENV) else {
        return;
    };
    let requested = std::env::var(PHASE_ENV).unwrap();
    let prepare_catalog = requested == "catalog_copy_step" && phase == "catalog_snapshot";
    if requested != phase && !prepare_catalog {
        return;
    }
    assert!(PHASES.contains(&phase));
    let owner = std::env::var(OWNER_ENV).unwrap();
    let root = PathBuf::from(root).canonicalize().unwrap();
    let scope = load_scope(&root, &owner);
    assert_eq!(workflow.catalog.summary().unwrap().id, scope.catalog);
    assert_eq!(workflow.data.path().unwrap(), root.join("app"));
    assert_eq!(job.profile.view.id, scope.profile);
    assert_eq!(job.profile.view.root_ids, vec![scope.root]);
    assert_eq!(
        job.view.action,
        if phase == "restore_published" {
            "restore"
        } else {
            "backup"
        }
    );
    let proof = ExitProof {
        phase: if prepare_catalog {
            requested
        } else {
            phase.into()
        },
        job: job.view.id.clone(),
        review: job.review_id.clone(),
        snapshot: job.view.snapshot_id.clone(),
        action: job.view.action.clone(),
        catalog_pages_remaining: None,
    };
    write_owned(
        &root.join(if prepare_catalog {
            "catalog-copy-prepared.json".into()
        } else {
            format!("exit-{phase}.json")
        }),
        &serde_json::to_vec(&proof).unwrap(),
    );
    if prepare_catalog {
        return;
    }
    // Bypass all Rust drops and terminal persistence at the real durable phase.
    std::process::exit(89);
}

/// Test-only authorization before changing SQLite backup step size. Never calls
/// the actor recursively: the supplied connection is its current source.
pub(crate) fn catalog_copy_requested(source: &Connection, destination: &Path) -> bool {
    if std::env::var(PHASE_ENV).ok().as_deref() != Some("catalog_copy_step") {
        return false;
    }
    let Some(root) = std::env::var_os(ROOT_ENV) else {
        return false;
    };
    let owner = std::env::var(OWNER_ENV).unwrap();
    let root = PathBuf::from(root).canonicalize().unwrap();
    let scope = load_scope(&root, &owner);
    let catalog: String = source
        .query_row(
            "SELECT uuid FROM catalog_identity WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(catalog, scope.catalog);
    let app = native::open_root(&root.join("app"), None).unwrap();
    let identity = filesystem::identity(&app).unwrap();
    let chain = DirectoryChain::open(destination.parent().unwrap()).unwrap();
    assert!(
        chain
            .identities()
            .iter()
            .any(|id| id.volume == identity.volume && id.file == identity.file)
    );
    assert!(destination.starts_with(root.join("app")));
    let prepared = read_exit_proof(&root.join("catalog-copy-prepared.json"));
    assert_eq!(prepared.phase, "catalog_copy_step");
    assert_eq!(prepared.action, "backup");
    true
}

pub(crate) fn catalog_copy_step(remaining: i32) {
    assert!(
        remaining > 0,
        "This exit must occur before the SQLite copy is complete"
    );
    let root = PathBuf::from(std::env::var_os(ROOT_ENV).unwrap())
        .canonicalize()
        .unwrap();
    let owner = std::env::var(OWNER_ENV).unwrap();
    let _scope = load_scope(&root, &owner);
    let mut proof = read_exit_proof(&root.join("catalog-copy-prepared.json"));
    proof.catalog_pages_remaining = Some(remaining);
    write_owned(
        &root.join("exit-catalog_copy_step.json"),
        &serde_json::to_vec(&proof).unwrap(),
    );
    std::process::exit(89);
}
fn reopen(root: &Path) -> Arc<Library> {
    let data = root.join("app");
    let library = Library::new(
        Arc::new(Catalog::open(data.join("Generated.stillharbour")).unwrap()),
        LibraryOptions {
            data_directory: data.clone(),
            workers: WorkerPaths {
                executable: data.join("missing-worker"),
                exiftool: None,
                working_directory: data.join("worker"),
            },
        },
    )
    .unwrap();
    library.configure_snapshot_helpers(helpers()).unwrap();
    library
}
fn authorize(library: &Arc<Library>, scope: &OwnedScope, root: &Path) {
    let selection = Fixture::selection(&root.join("backup"));
    assert!(matches!(
        library
            .backup_snapshots(
                SnapshotRequest::ProfileOpen {
                    profile_id: scope.profile.clone(),
                    selection_id: Some(selection.id.clone())
                },
                Some(selection)
            )
            .unwrap(),
        SnapshotReply::Profile(_)
    ));
}
fn start_review(library: &Arc<Library>, review: SnapshotReview, action: &str) -> SnapshotJob {
    let approval = library
        .snapshots
        .approve(&review.id, &review.digest, action)
        .unwrap();
    let frozen = library
        .snapshots
        .take_approval(&review.id, &approval.id, action)
        .unwrap();
    library
        .snapshots
        .start(library.clone(), frozen, action)
        .unwrap()
}
fn fresh_restore(
    library: &Arc<Library>,
    scope: &OwnedScope,
    root: &Path,
    snapshot: &str,
) -> PathBuf {
    let review = library
        .snapshots
        .restore_review(
            library,
            library.snapshots.profile(&scope.profile).unwrap(),
            snapshot,
            Fixture::selection(&root.join("restore")),
        )
        .unwrap();
    let output = root
        .join("restore")
        .join(format!("stillharbour-restored-{}", review.id));
    let job = start_review(library, review, "restore");
    let done = wait(library, &job.id);
    assert_eq!(done.state, "completed_verified", "{}", done.message);
    assert_eq!(
        std::fs::read(output.join("roots").join(&scope.root).join("original.txt")).unwrap(),
        b"generated preserved original"
    );
    assert_eq!(
        std::fs::read(
            output
                .join("roots")
                .join(&scope.root)
                .join("hidden-source/companion.xmp")
        )
        .unwrap(),
        b"generated sidecar outside gallery filter"
    );
    verify_catalog(
        &output.join("Catalog.snapshot.stillharbour"),
        &scope.catalog,
        crate::catalog::SCHEMA_VERSION,
    )
    .unwrap();
    output
}
#[test]
fn snapshot_actual_process_exit_child() {
    let Some(root) = std::env::var_os(ROOT_ENV) else {
        return;
    };
    let owner = std::env::var(OWNER_ENV).unwrap();
    let root = PathBuf::from(root).canonicalize().unwrap();
    let scope = load_scope(&root, &owner);
    let phase = std::env::var(PHASE_ENV).unwrap();
    assert!(PHASES.contains(&phase.as_str()));
    let library = reopen(&root);
    authorize(&library, &scope, &root);
    let review = if phase == "restore_published" {
        library
            .snapshots
            .restore_review(
                &library,
                library.snapshots.profile(&scope.profile).unwrap(),
                &scope.baseline_snapshot,
                Fixture::selection(&root.join("restore")),
            )
            .unwrap()
    } else {
        library
            .snapshots
            .review(library.snapshots.profile(&scope.profile).unwrap())
            .unwrap()
    };
    let action = if phase == "restore_published" {
        "restore"
    } else {
        "backup"
    };
    let job = start_review(&library, review, action);
    let done = wait(&library, &job.id);
    panic!(
        "Actual phase exit was not reached: {} / {}",
        done.phase, done.message
    );
}
struct OwnedChild(Child);
struct OwnedRun(Option<tempfile::TempDir>, FileIdentity, String);
impl Drop for OwnedRun {
    fn drop(&mut self) {
        let boundary = self.0.as_ref().is_some_and(|run| {
            native::open_root(run.path(), Some(&self.1))
                .ok()
                .is_some_and(|root| {
                    native::volume_token(&root).is_ok_and(|volume| volume == self.2)
                })
        });
        if (std::thread::panicking() || !boundary)
            && let Some(run) = self.0.take()
        {
            let _ = run.keep();
        }
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            #[cfg(unix)]
            {
                let mut info = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
                loop {
                    // SAFETY: exact direct generated child. WNOWAIT reserves
                    // its PID before signaling; ECHILD grants no authority.
                    if unsafe {
                        libc::waitid(
                            libc::P_PID,
                            self.0.id() as libc::id_t,
                            info.as_mut_ptr(),
                            libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                        )
                    } == 0
                    {
                        break;
                    }
                    if std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                        return;
                    }
                }
            }
            let _ = self.0.kill();
            loop {
                match self.0.wait() {
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    _ => break,
                }
            }
        }
    }
}
fn child_exit(root: &Path, owner: &str, phase: &str) {
    let diagnostics = root.join(format!("child-{phase}.stderr"));
    let errors = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&diagnostics)
        .unwrap();
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CHILD_TEST, "--nocapture"])
            .env(ROOT_ENV, root)
            .env(OWNER_ENV, owner)
            .env(PHASE_ENV, phase)
            .env_remove("STILLHARBOUR_M4_GENERATED_REPOSITORY_DRIVE")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(errors))
            .spawn()
            .unwrap(),
    );
    let until = Instant::now() + Duration::from_secs(180);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert_eq!(
                status.code(),
                Some(89),
                "actual child failed before durable {phase} exit: {}",
                {
                    let mut bytes = String::new();
                    File::open(&diagnostics)
                        .unwrap()
                        .take(65536)
                        .read_to_string(&mut bytes)
                        .unwrap();
                    bytes
                }
            );
            break;
        }
        assert!(Instant::now() < until, "actual child timed out at {phase}");
        std::thread::sleep(Duration::from_millis(25));
    }
}
fn tree_bytes(root: &Path) -> BTreeMap<PathBuf, (FileIdentity, String)> {
    fn visit(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, (FileIdentity, String)>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let metadata = entry.file_type().unwrap();
            assert!(!metadata.is_symlink());
            if metadata.is_dir() {
                visit(root, &entry.path(), out);
            } else {
                assert!(metadata.is_file());
                let file = filesystem::open_readonly(&entry.path()).unwrap();
                let identity = filesystem::identity(&file).unwrap();
                let bytes = std::fs::read(entry.path()).unwrap();
                out.insert(
                    entry.path().strip_prefix(root).unwrap().to_path_buf(),
                    (identity, blake3::hash(&bytes).to_hex().to_string()),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}
#[test]
#[ignore = "actual process exit/restart requires native OS credentials and verified pinned helpers; generated fixtures only"]
fn native_snapshot_actual_process_exit_matrix_retains_and_reconciles_without_reupload() {
    assert!(
        std::env::var_os("STILLHARBOUR_M4_GENERATED_REPOSITORY_DRIVE").is_none(),
        "This matrix owns only one generated system-temp fixture."
    );
    let f = Fixture::new();
    f.library.configure_snapshot_helpers(helpers()).unwrap();
    let catalog = f.library.catalog.summary().unwrap().id;
    let reference = CredentialStore::store(
        &catalog,
        CredentialPurpose::Repository,
        &uuid::Uuid::new_v4().to_string(),
    )
    .unwrap();
    struct CredentialCleanup {
        catalog: String,
        reference: String,
    }
    impl Drop for CredentialCleanup {
        fn drop(&mut self) {
            let _ = CredentialStore::delete_owned(
                &self.catalog,
                CredentialPurpose::Repository,
                &self.reference,
            );
        }
    }
    let _credential = CredentialCleanup {
        catalog: catalog.clone(),
        reference: reference.clone(),
    };
    let selected = Fixture::selection(&f.backup);
    let profile = f
        .library
        .snapshots
        .profile_create(
            &f.library,
            "Generated process-exit matrix".into(),
            vec![f.root.clone()],
            reference.clone(),
            None,
            SnapshotDestinationInput::Local {
                selection_id: selected.id.clone(),
            },
            Some(selected),
        )
        .unwrap();
    let review = f
        .library
        .snapshots
        .review(f.library.snapshots.profile(&profile.id).unwrap())
        .unwrap();
    let baseline = start_review(&f.library, review, "backup");
    let baseline = wait(&f.library, &baseline.id);
    assert_eq!(baseline.state, "completed_verified", "{}", baseline.message);
    let root = f._run.path().canonicalize().unwrap();
    let held = native::open_root(&root, None).unwrap();
    let scope = OwnedScope {
        owner: uuid::Uuid::new_v4().to_string(),
        catalog,
        profile: profile.id,
        root: f.root.clone(),
        credential: reference,
        identity: filesystem::identity(&held).unwrap(),
        volume: native::volume_token(&held).unwrap(),
        baseline_snapshot: baseline.snapshot_id.unwrap(),
    };
    // Release the Windows no-delete root handle before the owned TempDir's
    // final cleanup guard. Its native identity remains in the immutable scope.
    drop(held);
    write_owned(
        &root.join(format!("snapshot-process-{}.json", scope.owner)),
        &serde_json::to_vec(&scope).unwrap(),
    );
    write_owned(
        &root.join("backup/neighbor.txt"),
        b"generated destination-only neighbor",
    );
    write_owned(
        &root.join("restore/neighbor.txt"),
        b"generated restore neighbor",
    );
    let source_before = tree_bytes(&f.source);
    drop(f.library);
    let _run = OwnedRun(Some(f._run), scope.identity.clone(), scope.volume.clone());
    for phase in PHASES {
        child_exit(&root, &scope.owner, phase);
        let proof = read_exit_proof(&root.join(format!("exit-{phase}.json")));
        assert_eq!(proof.phase, phase);
        if phase == "catalog_copy_step" {
            assert!(
                proof
                    .catalog_pages_remaining
                    .is_some_and(|remaining| remaining > 0)
            );
        }
        assert_eq!(tree_bytes(&f.source), source_before);
        assert_eq!(
            std::fs::read(root.join("backup/neighbor.txt")).unwrap(),
            b"generated destination-only neighbor"
        );
        assert_eq!(
            std::fs::read(root.join("restore/neighbor.txt")).unwrap(),
            b"generated restore neighbor"
        );
        let library = reopen(&root);
        let interrupted = library
            .snapshots
            .jobs(None)
            .unwrap()
            .into_iter()
            .find(|job| job.id == proof.job)
            .unwrap();
        assert_eq!(interrupted.state, "needs_recovery");
        assert!(library.snapshots.state.lock().unwrap().approvals.is_empty());
        assert!(library.snapshots.state.lock().unwrap().reviews.is_empty());
        assert!(
            library
                .snapshots
                .take_approval(
                    &proof.review,
                    &uuid::Uuid::new_v4().to_string(),
                    &proof.action
                )
                .is_err()
        );
        let profile = library.snapshots.profile(&scope.profile).unwrap();
        assert!(library.snapshots.authorized(&profile).is_err());
        let repository = library
            .snapshots
            .repository(&profile)
            .unwrap()
            .unwrap()
            .local
            .unwrap()
            .path()
            .unwrap();
        let repository_before = tree_bytes(&repository);
        authorize(&library, &scope, &root);
        let text = library
            .catalog
            .call({
                let key = proof.job.clone();
                move |db| {
                    Ok(db.query_row(
                        "SELECT record FROM backup_snapshot_jobs WHERE id=?",
                        [key],
                        |row| row.get::<_, String>(0),
                    )?)
                }
            })
            .unwrap();
        let original: JobRecord = decode(text).unwrap();
        let work = original.work.as_ref().unwrap().path().unwrap();
        let partial_before = tree_bytes(&work);
        assert!(work.starts_with(root.join("app")));
        let records_before = library.snapshots.records(&profile).unwrap();
        assert!(
            records_before
                .iter()
                .any(|record| record.id == scope.baseline_snapshot)
        );
        let restored_before = tree_bytes(&root.join("restore"));
        if proof.action == "backup" && proof.snapshot.is_some() {
            let inspected = library
                .snapshots
                .reconcile(
                    library.clone(),
                    &scope.profile,
                    &proof.job,
                    proof.snapshot.clone(),
                )
                .unwrap();
            let inspected = wait(&library, &inspected.id);
            assert_eq!(
                inspected.state, "completed_verified",
                "{}",
                inspected.message
            );
            assert_eq!(inspected.action, "reconcile");
            let output = fresh_restore(&library, &scope, &root, proof.snapshot.as_deref().unwrap());
            assert!(output.exists());
        } else if proof.action == "restore" {
            assert!(
                library
                    .snapshots
                    .reconcile(
                        library.clone(),
                        &scope.profile,
                        &proof.job,
                        proof.snapshot.clone()
                    )
                    .is_err()
            );
            let published = root
                .join("restore")
                .join(format!("stillharbour-restored-{}", proof.review));
            assert!(published.is_dir());
            assert_eq!(
                std::fs::read(
                    published
                        .join("roots")
                        .join(&scope.root)
                        .join("original.txt")
                )
                .unwrap(),
                b"generated preserved original"
            );
            let next = fresh_restore(&library, &scope, &root, &scope.baseline_snapshot);
            assert_ne!(next, published);
        } else {
            let result =
                library
                    .snapshots
                    .reconcile(library.clone(), &scope.profile, &proof.job, None);
            if let Ok(job) = result {
                let done = wait(&library, &job.id);
                assert_eq!(done.state, "needs_recovery");
                assert!(done.snapshot_id.is_none());
            }
            if phase == "catalog_copy_step" {
                let restored = fresh_restore(&library, &scope, &root, &scope.baseline_snapshot);
                assert!(restored.is_dir());
            }
        }
        assert_eq!(
            tree_bytes(&work),
            partial_before,
            "cold recovery must retain all prior owned staging/verification files at {phase}"
        );
        let restored_after = tree_bytes(&root.join("restore"));
        for (path, version) in restored_before {
            assert_eq!(restored_after.get(&path), Some(&version));
        }
        let records = library.snapshots.records(&profile).unwrap();
        for prior in records_before {
            assert!(records.iter().any(|record| record.id == prior.id));
        }
        assert_eq!(tree_bytes(&f.source), source_before);
        assert_eq!(
            tree_bytes(&repository),
            repository_before,
            "cold inspection and new restore must not reupload, unlock, delete or rewrite repository objects at {phase}"
        );
        assert!(library.snapshots.idle().unwrap());
        drop(library);
    }
}

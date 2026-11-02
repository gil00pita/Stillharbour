use super::*;
use crate::library::{
    LibraryOptions, WorkerPaths,
    model::{RootMode, ScanFilter},
};

#[cfg(any(windows, target_os = "macos"))]
struct Fixture {
    owned: tempfile::TempDir,
    library: Arc<Library>,
    workflow: Arc<SyncWorkflow>,
    source: PathBuf,
    destination: PathBuf,
    root: String,
}
#[cfg(any(windows, target_os = "macos"))]
impl Fixture {
    fn new() -> Self {
        let owned = tempfile::Builder::new()
            .prefix("stillharbour-sync-application-")
            .tempdir()
            .unwrap();
        let base = owned.path().canonicalize().unwrap();
        let source = base.join("source");
        let destination = base.join("destination");
        let data = base.join("data");
        std::fs::write(base.join("owner"), uuid::Uuid::new_v4().to_string()).unwrap();
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&destination).unwrap();
        std::fs::create_dir(&data).unwrap();
        for (name, bytes) in [
            ("movie.mp4", b"opaque video".as_slice()),
            ("movie.xmp", b"sidecar"),
            (".hidden", b"unknown hidden data"),
            ("empty.bin", b""),
        ] {
            std::fs::write(source.join(name), bytes).unwrap();
        }
        std::fs::write(destination.join("outside.bin"), b"unique neighbor").unwrap();
        let catalog = Arc::new(Catalog::open(data.join("Fixture.stillharbour")).unwrap());
        let library = Library::new(
            catalog.clone(),
            LibraryOptions {
                data_directory: data.clone(),
                workers: WorkerPaths {
                    executable: std::env::current_exe().unwrap(),
                    exiftool: None,
                    working_directory: data.join("workers"),
                },
            },
        )
        .unwrap();
        let handle = source::open_root(&source, None).unwrap();
        let root = store::add_root(
            &catalog,
            source.clone(),
            filesystem::identity(&handle).unwrap(),
            source::volume_token(&handle).unwrap(),
            "Generated source".into(),
            RootMode::Referenced,
            ScanFilter {
                extensions: vec!["notindexed".into()],
                ..Default::default()
            },
        )
        .unwrap();
        let workflow = Arc::new(SyncWorkflow::new(catalog, data).unwrap());
        Self {
            owned,
            library,
            workflow,
            source,
            destination,
            root,
        }
    }
    fn compare(&self) -> String {
        let destination = source::open_root(&self.destination, None).unwrap();
        let run = self
            .library
            .backup_compare_start(
                self.destination.clone(),
                (
                    filesystem::identity(&destination).unwrap(),
                    source::volume_token(&destination).unwrap(),
                ),
                crate::backup::comparison::CompareMode::Verified,
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let current = self.library.backup_comparison(run.id.clone()).unwrap();
            if current.state != crate::backup::comparison::ComparisonState::Comparing {
                assert_eq!(
                    current.state,
                    crate::backup::comparison::ComparisonState::NeedsReview,
                    "{}",
                    current.message
                );
                return run.id;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn review(&self) -> SyncReview {
        self.workflow
            .review_append(&self.library, self.compare(), vec![])
            .unwrap()
    }
    fn approved(&self, review: &SyncReview) -> SyncApproval {
        self.workflow
            .approve(&self.library, review.id.clone(), true, true)
            .unwrap()
    }
    fn wait(&self, id: &str) -> SyncJob {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let job = self.workflow.job(id).unwrap();
            let stopped = self
                .workflow
                .active
                .lock()
                .unwrap()
                .as_ref()
                .is_none_or(|a| a.id != id);
            if stopped
                && !matches!(
                    job.state,
                    SyncState::Queued
                        | SyncState::Running
                        | SyncState::Paused
                        | SyncState::CancelRequested
                )
            {
                return job;
            }
            assert!(
                Instant::now() < deadline,
                "sync job timeout: {} {}",
                job.phase,
                job.message
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn prepared(&self) -> (SyncJob, SyncReview) {
        let review = self.review();
        let approval = self.approved(&review);
        let job = self
            .workflow
            .start(self.library.clone(), approval.id)
            .unwrap();
        let job = self.wait(&job.id);
        assert_eq!(job.state, SyncState::NeedsReview, "{}", job.message);
        let copy = self
            .workflow
            .review(job.ready_review_id.as_deref().unwrap())
            .unwrap()
            .view
            .clone();
        (job, copy)
    }
    fn dataset(&self) -> PathBuf {
        self.destination
            .join(".stillharbour-copy")
            .join(self.library.catalog.summary().unwrap().id)
            .join(&self.root)
    }
    fn audit(&self) {
        assert!(
            self.owned
                .path()
                .canonicalize()
                .unwrap()
                .starts_with(std::env::temp_dir().canonicalize().unwrap())
        );
        for (name, bytes) in [
            ("movie.mp4", b"opaque video".as_slice()),
            ("movie.xmp", b"sidecar"),
            (".hidden", b"unknown hidden data"),
            ("empty.bin", b""),
        ] {
            assert_eq!(std::fs::read(self.source.join(name)).unwrap(), bytes);
        }
        assert_eq!(
            std::fs::read(self.destination.join("outside.bin")).unwrap(),
            b"unique neighbor"
        );
    }
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_separate_approvals_prepare_copy_verify_and_retain_all_sources() {
    let f = Fixture::new();
    let review = f.review();
    assert_eq!(review.copy_files, 4);
    assert_eq!(review.action, SyncAction::PrepareNames);
    assert!(!f.dataset().exists());
    assert!(
        f.workflow
            .approve(&f.library, review.id.clone(), true, false)
            .is_err()
    );
    let approval = f.approved(&review);
    let token = approval.id.clone();
    let job = f.workflow.start(f.library.clone(), approval.id).unwrap();
    assert!(f.workflow.start(f.library.clone(), token).is_err());
    let prepared = f.wait(&job.id);
    assert_eq!(
        prepared.state,
        SyncState::NeedsReview,
        "{}",
        prepared.message
    );
    assert_eq!(
        std::fs::read_dir(f.dataset()).unwrap().count(),
        0,
        "prospective probes must be separate from published files"
    );
    let copy = f
        .workflow
        .review(prepared.ready_review_id.as_deref().unwrap())
        .unwrap()
        .view
        .clone();
    assert_eq!(copy.action, SyncAction::CopyFiles);
    assert_ne!(copy.digest, review.digest);
    let approval = f.approved(&copy);
    let job = f.workflow.start(f.library.clone(), approval.id).unwrap();
    let result = f.wait(&job.id);
    assert_eq!(
        result.state,
        SyncState::CompletedVerified,
        "{}",
        result.message
    );
    assert_eq!(result.verified_files, 4);
    for name in ["movie.mp4", "movie.xmp", ".hidden", "empty.bin"] {
        assert_eq!(
            std::fs::read(f.source.join(name)).unwrap(),
            std::fs::read(f.dataset().join(name)).unwrap()
        );
    }
    f.audit();
    let history = f.workflow.list().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].id, job.id);
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_changed_source_revokes_approved_scope_before_creating_any_output() {
    let f = Fixture::new();
    let review = f.review();
    let approval = f.approved(&review);
    std::fs::write(
        f.source.join("movie.mp4"),
        b"different-length source revision",
    )
    .unwrap();
    assert!(f.workflow.start(f.library.clone(), approval.id).is_err());
    assert!(!f.dataset().exists());
    assert_eq!(
        std::fs::read(f.destination.join("outside.bin")).unwrap(),
        b"unique neighbor"
    );
    assert!(f.workflow.list().unwrap().is_empty());
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_late_target_is_retained_and_copy_approval_is_rejected() {
    let f = Fixture::new();
    let (_, copy) = f.prepared();
    std::fs::write(f.dataset().join("movie.mp4"), b"late unique destination").unwrap();
    assert!(f.workflow.approve(&f.library, copy.id, true, true).is_err());
    assert_eq!(
        std::fs::read(f.dataset().join("movie.mp4")).unwrap(),
        b"late unique destination"
    );
    f.audit();
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_restart_does_not_reconstruct_an_approval_and_requires_fresh_native_review() {
    let f = Fixture::new();
    let (job, copy) = f.prepared();
    let approval = f.approved(&copy);
    let reopened = Arc::new(
        SyncWorkflow::new(
            f.library.catalog.clone(),
            f.source.parent().unwrap().join("data"),
        )
        .unwrap(),
    );
    assert!(reopened.start(f.library.clone(), approval.id).is_err());
    assert_eq!(
        reopened.job(&job.id).unwrap().state,
        SyncState::NeedsRecovery
    );
    let recovery = reopened.recovery_review(&f.library, &job.id).unwrap();
    assert_eq!(recovery.action, SyncAction::ResumeFiles);
    assert_ne!(recovery.id, copy.id);
    let proof = reopened
        .approve(&f.library, recovery.id, true, true)
        .unwrap();
    let resumed = reopened.start(f.library.clone(), proof.id).unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let current = reopened.job(&resumed.id).unwrap();
        if reopened.active.lock().unwrap().is_none() {
            assert_eq!(
                current.state,
                SyncState::CompletedVerified,
                "{}",
                current.message
            );
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    f.audit();
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_kept_different_revisions_remain_explicitly_partial() {
    let f = Fixture::new();
    std::fs::create_dir_all(f.dataset()).unwrap();
    std::fs::write(
        f.dataset().join("movie.mp4"),
        b"manually edited destination",
    )
    .unwrap();
    std::fs::write(f.dataset().join("unique.bin"), b"destination only").unwrap();
    let run = f.compare();
    let rows = f.library.backup_comparison_page(run.clone(), 0).unwrap();
    let keep = rows
        .entries
        .iter()
        .find(|r| r.display_path == "movie.mp4")
        .unwrap()
        .id
        .clone();
    assert!(
        f.workflow
            .review_append(&f.library, run.clone(), vec![])
            .is_err()
    );
    let review = f
        .workflow
        .review_append(&f.library, run, vec![keep])
        .unwrap();
    assert_eq!(review.uncopied_source_revisions, 1);
    assert!(
        f.workflow
            .approve(&f.library, review.id.clone(), false, true)
            .is_err()
    );
    let approval = f.approved(&review);
    let prep = f.workflow.start(f.library.clone(), approval.id).unwrap();
    let prep = f.wait(&prep.id);
    let copy = f
        .workflow
        .review(prep.ready_review_id.as_deref().unwrap())
        .unwrap()
        .view
        .clone();
    let approval = f.approved(&copy);
    let copy = f.workflow.start(f.library.clone(), approval.id).unwrap();
    let result = f.wait(&copy.id);
    assert_eq!(
        result.state,
        SyncState::CompletedPartial,
        "{}",
        result.message
    );
    assert_eq!(result.uncopied_source_revisions, 1);
    assert_eq!(
        std::fs::read(f.dataset().join("movie.mp4")).unwrap(),
        b"manually edited destination"
    );
    assert_eq!(
        std::fs::read(f.dataset().join("unique.bin")).unwrap(),
        b"destination only"
    );
    f.audit();
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_unidentified_preparation_occupant_blocks_recovery_without_adoption() {
    let f = Fixture::new();
    let view = f.review();
    let review = f.workflow.review(&view.id).unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    f.workflow.create_job(&id, &review.binding).unwrap();
    let stage = preparation::stage_name(&id);
    f.workflow
        .event(
            &id,
            None,
            "prepare/stage_intent",
            serde_json::json!({"path":stage}),
        )
        .unwrap();
    std::fs::create_dir(f.destination.join(stage)).unwrap();
    let recovery = f.workflow.recovery_review(&f.library, &id).unwrap();
    assert!(!recovery.execution_available);
    assert!(
        f.workflow
            .approve(&f.library, recovery.id, true, true)
            .is_err()
    );
    assert!(!f.dataset().exists());
    f.audit();
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_root_revocation_invalidates_unused_approvals() {
    let f = Fixture::new();
    let review = f.review();
    let approval = f.approved(&review);
    f.workflow.cancel_root(&f.root).unwrap();
    assert!(f.workflow.start(f.library.clone(), approval.id).is_err());
    assert!(!f.dataset().exists());
    f.audit();
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_shutdown_retires_grants_and_refuses_new_workers() {
    let f = Fixture::new();
    let review = f.review();
    let approval = f.approved(&review);
    f.workflow.cancel_all();
    assert!(f.workflow.idle().unwrap());
    assert!(f.workflow.start(f.library.clone(), approval.id).is_err());
    assert!(
        f.workflow
            .approve(&f.library, review.id, true, true)
            .is_err()
    );
    assert!(!f.dataset().exists());
    f.audit();
}

const COPY_FAULTS: [CopyFault; 9] = [
    CopyFault::AfterIntent,
    CopyFault::AfterStageCreation,
    CopyFault::AfterCopyChunk,
    CopyFault::AfterFlush,
    CopyFault::AfterVerification,
    CopyFault::BeforePublish,
    CopyFault::AfterPublish,
    CopyFault::AfterItemCommit,
    CopyFault::AfterPlanCommit,
];

#[cfg(any(windows, target_os = "macos"))]
fn retained_fixture() -> (Fixture, String, String) {
    let f = Fixture::new();
    let (job, review) = f.prepared();
    let approval = f.approved(&review);
    f.workflow.start(f.library.clone(), approval.id).unwrap();
    assert_eq!(f.wait(&job.id).state, SyncState::CompletedVerified);
    // Unique manually edited OLD destination bytes are absent from source.
    std::fs::write(
        f.dataset().join("movie.mp4"),
        b"unique old destination edit",
    )
    .unwrap();
    std::fs::write(
        f.dataset().join("movie.xmp"),
        b"unique old destination companion",
    )
    .unwrap();
    let run = f.compare();
    let page = f.library.backup_comparison_page(run.clone(), 0).unwrap();
    let entry = page
        .entries
        .iter()
        .find(|r| r.display_path == "movie.mp4")
        .unwrap()
        .id
        .clone();
    (f, run, entry)
}

#[cfg(any(windows, target_os = "macos"))]
fn synthetic_retained_proof(f: &Fixture, evidence: &str) {
    // Test-only gate injection is deliberately absent from release builds.
    // It exercises executor failure recovery; it is NOT disaster/fidelity proof.
    *f.workflow.recovery_proof.lock().unwrap() = Some(RecoveryProof {
        evidence_id: evidence.into(),
        scope_digest: "a".repeat(64),
        independent_failure_domain: true,
        essential_metadata_verified: true,
    });
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_retained_replacement_rejects_source_only_or_asserted_recovery_evidence() {
    let (f, run, entry) = retained_fixture();
    let review = f
        .workflow
        .review_replacement(&f.library, run, entry, "e".repeat(64))
        .unwrap();
    assert!(!review.execution_available);
    assert!(
        f.workflow
            .approve(&f.library, review.id, true, true)
            .is_err()
    );
    assert_eq!(
        std::fs::read(f.dataset().join("movie.mp4")).unwrap(),
        b"unique old destination edit"
    );
    assert_eq!(
        std::fs::read(f.dataset().join("movie.xmp")).unwrap(),
        b"unique old destination companion"
    );
    f.audit();
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_private_old_destination_backup_scope_contains_exact_unique_companion_versions()
{
    let (f, run, entry) = retained_fixture();
    let files = f
        .workflow
        .recovery_snapshot_scope(&f.library, run.clone(), entry)
        .unwrap();
    assert_eq!(files.len(), 2);
    for file in files {
        assert_eq!(
            source::decode_path(&file.native_root).unwrap(),
            f.destination
        );
        let path = source::decode_path(&file.relative_path).unwrap();
        assert!(path.starts_with(f.dataset().strip_prefix(&f.destination).unwrap()));
        let bytes = std::fs::read(f.destination.join(&path)).unwrap();
        assert_eq!(file.full_digest, blake3::hash(&bytes).to_hex().to_string());
        assert_eq!(file.bytes, bytes.len() as u64);
        assert!(file.essential_metadata_digest.is_some());
        let source_bytes = std::fs::read(f.source.join(path.file_name().unwrap())).unwrap();
        assert_ne!(
            source_bytes, bytes,
            "Old destination evidence must protect unique edits absent from sources."
        );
    }
    assert!(
        f.workflow
            .recovery_snapshot_scope(&f.library, run, uuid::Uuid::new_v4().to_string())
            .is_err()
    );
    f.audit();
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_private_old_scope_keeps_multi_extension_companion_metadata() {
    let (f, _, _) = retained_fixture();
    let leaf = "movie.mp4.supplemental-metadata.json";
    std::fs::write(f.source.join(leaf), b"incoming supplemental metadata").unwrap();
    std::fs::write(f.dataset().join(leaf), b"unique old supplemental metadata").unwrap();
    let run = f.compare();
    let page = f.library.backup_comparison_page(run.clone(), 0).unwrap();
    let entry = page
        .entries
        .iter()
        .find(|r| r.display_path == "movie.mp4")
        .unwrap()
        .id
        .clone();
    let files = f
        .workflow
        .recovery_snapshot_scope(&f.library, run.clone(), entry)
        .unwrap();
    assert_eq!(files.len(), 3);
    let companion = page
        .entries
        .iter()
        .find(|r| r.display_path == leaf)
        .unwrap()
        .id
        .clone();
    assert_eq!(
        f.workflow
            .recovery_snapshot_scope(&f.library, run, companion)
            .unwrap()
            .len(),
        3
    );
    assert!(files.iter().any(|file| {
        source::decode_path(&file.relative_path)
            .unwrap()
            .file_name()
            == Some(std::ffi::OsStr::new(leaf))
            && file.full_digest
                == blake3::hash(b"unique old supplemental metadata")
                    .to_hex()
                    .to_string()
    }));
    assert_eq!(
        std::fs::read(f.source.join(leaf)).unwrap(),
        b"incoming supplemental metadata"
    );
    f.audit();
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_corrupt_incoming_recovery_stage_blocks_before_any_old_destination_move() {
    let (f, run, entry) = retained_fixture();
    let evidence = "e".repeat(64);
    synthetic_retained_proof(&f, &evidence);
    let review = f
        .workflow
        .review_replacement(&f.library, run, entry, evidence)
        .unwrap();
    let approval = f.approved(&review);
    *f.workflow.fault.lock().unwrap() = Some(Fault::Copy(CopyFault::AfterPlanCommit));
    let job = f.workflow.start(f.library.clone(), approval.id).unwrap();
    assert_eq!(f.wait(&job.id).state, SyncState::NeedsRecovery);
    *f.workflow.fault.lock().unwrap() = None;
    std::fs::write(
        f.destination
            .join(format!(".stillharbour-retained-{}/new-0.verified", job.id)),
        b"corrupt staged incoming",
    )
    .unwrap();
    let recovery = f.workflow.recovery_review(&f.library, &job.id).unwrap();
    assert!(!recovery.execution_available);
    assert!(
        f.workflow
            .approve(&f.library, recovery.id, true, true)
            .is_err()
    );
    assert_eq!(
        std::fs::read(f.dataset().join("movie.mp4")).unwrap(),
        b"unique old destination edit"
    );
    assert_eq!(
        std::fs::read(f.dataset().join("movie.xmp")).unwrap(),
        b"unique old destination companion"
    );
    assert!(
        !f.workflow
            .events(&job.id)
            .unwrap()
            .iter()
            .any(|(_, phase, _)| phase == "retained/old_intent")
    );
    f.audit();
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_retained_group_is_durable_and_restore_creates_new_protected_outputs() {
    let (f, run, entry) = retained_fixture();
    let evidence = "e".repeat(64);
    synthetic_retained_proof(&f, &evidence);
    let review = f
        .workflow
        .review_replacement(&f.library, run, entry, evidence)
        .unwrap();
    assert_eq!(review.copy_files, 2);
    assert!(review.execution_available, "{}", review.reason);
    let approval = f.approved(&review);
    let job = f.workflow.start(f.library.clone(), approval.id).unwrap();
    let result = f.wait(&job.id);
    assert_eq!(
        result.state,
        SyncState::CompletedVerified,
        "{}",
        result.message
    );
    for name in ["movie.mp4", "movie.xmp"] {
        assert_eq!(
            std::fs::read(f.dataset().join(name)).unwrap(),
            std::fs::read(f.source.join(name)).unwrap()
        );
    }
    let versions = f.workflow.retained_versions(&f.library, &job.id).unwrap();
    assert_eq!(versions.len(), 2);
    assert!(versions.iter().all(|v| v.retained && v.restore_available));
    let restore = f
        .workflow
        .review_retained_restore(&f.library, &job.id)
        .unwrap();
    let output = PathBuf::from(&restore.dataset_display_path);
    assert!(!output.exists());
    let approval = f.approved(&restore);
    let restored = f.workflow.start(f.library.clone(), approval.id).unwrap();
    let result = f.wait(&restored.id);
    if result.state != SyncState::CompletedVerified {
        eprintln!("restore events {:?}", f.workflow.events(&restored.id));
    }
    assert_eq!(
        result.state,
        SyncState::CompletedVerified,
        "{}",
        result.message
    );
    let mut restored_bytes = std::fs::read_dir(&output)
        .unwrap()
        .map(|e| std::fs::read(e.unwrap().path()).unwrap())
        .collect::<Vec<_>>();
    for entry in std::fs::read_dir(&output).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        let index = name
            .strip_prefix("restored-")
            .unwrap()
            .split('-')
            .next()
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let retained = File::open(f.destination.join(format!(
            ".stillharbour-retained-{}/old-{index}.retained",
            job.id
        )))
        .unwrap();
        crate::backup::native_metadata::MetadataManifest::inspect(&retained)
            .unwrap()
            .compare(&File::open(entry.path()).unwrap())
            .unwrap();
    }
    restored_bytes.sort();
    let mut expected = vec![
        b"unique old destination edit".to_vec(),
        b"unique old destination companion".to_vec(),
    ];
    expected.sort();
    assert_eq!(restored_bytes, expected);
    assert!(versions.iter().all(|v| {
        f.workflow
            .retained_versions(&f.library, &job.id)
            .unwrap()
            .iter()
            .any(|current| {
                current.id == v.id && current.full_digest == v.full_digest && current.retained
            })
    }));
    f.audit();
}

const RETAINED_FAULTS: [Fault; 8] = [
    Fault::RetainedDirectoryCreated,
    Fault::RetainedOldIntent,
    Fault::RetainedOldMoved,
    Fault::RetainedOldRecorded,
    Fault::RetainedNewIntent,
    Fault::RetainedNewMoved,
    Fault::RetainedNewRecorded,
    Fault::RetainedCompleted,
];

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_retained_metadata_restore_reconciles_only_identified_new_outputs() {
    for fault in [
        Fault::RetainedMetadataApplied,
        Fault::RetainedMetadataRecorded,
    ] {
        let (f, run, entry) = retained_fixture();
        let evidence = "e".repeat(64);
        synthetic_retained_proof(&f, &evidence);
        let review = f
            .workflow
            .review_replacement(&f.library, run, entry, evidence)
            .unwrap();
        let approval = f.approved(&review);
        let original = f.workflow.start(f.library.clone(), approval.id).unwrap();
        assert_eq!(f.wait(&original.id).state, SyncState::CompletedVerified);
        let review = f
            .workflow
            .review_retained_restore(&f.library, &original.id)
            .unwrap();
        let approval = f.approved(&review);
        *f.workflow.fault.lock().unwrap() = Some(fault);
        let job = f.workflow.start(f.library.clone(), approval.id).unwrap();
        assert_eq!(f.wait(&job.id).state, SyncState::NeedsRecovery);
        *f.workflow.fault.lock().unwrap() = None;
        let review = f.workflow.recovery_review(&f.library, &job.id).unwrap();
        assert!(review.execution_available, "{}", review.reason);
        let approval = f.approved(&review);
        f.workflow.start(f.library.clone(), approval.id).unwrap();
        let result = f.wait(&job.id);
        assert_eq!(
            result.state,
            SyncState::CompletedVerified,
            "{fault:?}: {}",
            result.message
        );
        assert_eq!(
            f.workflow
                .events(&job.id)
                .unwrap()
                .iter()
                .filter(|(_, phase, _)| phase == "retained/metadata_verified")
                .count(),
            2
        );
        assert!(
            f.workflow
                .retained_versions(&f.library, &original.id)
                .unwrap()
                .iter()
                .all(|v| v.retained)
        );
        f.audit();
    }
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_retained_move_publication_and_commit_boundaries_require_fresh_recovery() {
    for fault in RETAINED_FAULTS {
        let (f, run, entry) = retained_fixture();
        let evidence = "e".repeat(64);
        synthetic_retained_proof(&f, &evidence);
        let review = f
            .workflow
            .review_replacement(&f.library, run, entry, evidence)
            .unwrap();
        let approval = f.approved(&review);
        *f.workflow.fault.lock().unwrap() = Some(fault);
        let job = f.workflow.start(f.library.clone(), approval.id).unwrap();
        let stopped = f.wait(&job.id);
        assert_eq!(
            stopped.state,
            SyncState::NeedsRecovery,
            "{fault:?}: {}",
            stopped.message
        );
        f.audit();
        *f.workflow.fault.lock().unwrap() = None;
        let review = f.workflow.recovery_review(&f.library, &job.id).unwrap();
        if fault == Fault::RetainedDirectoryCreated {
            assert!(!review.execution_available);
            assert!(
                f.workflow
                    .approve(&f.library, review.id, true, true)
                    .is_err()
            );
        } else {
            assert!(review.execution_available, "{fault:?}: {}", review.reason);
            let approval = f.approved(&review);
            f.workflow.start(f.library.clone(), approval.id).unwrap();
            let completed = f.wait(&job.id);
            assert_eq!(
                completed.state,
                SyncState::CompletedVerified,
                "{fault:?}: {}",
                completed.message
            );
            let versions = f.workflow.retained_versions(&f.library, &job.id).unwrap();
            assert!(versions.iter().all(|v| v.retained));
        }
        f.audit();
    }
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn retained_recovery_scope_accepts_only_complete_journal_owned_old_objects() {
    // Synthetic admission reaches executor boundaries only. This test issues
    // no snapshot certificate and proves neither audit nor disk independence.
    for fault in [Fault::RetainedOldMoved, Fault::RetainedCompleted] {
        let (f, run, entry) = retained_fixture();
        let evidence = "e".repeat(64);
        synthetic_retained_proof(&f, &evidence);
        let review = f
            .workflow
            .review_replacement(&f.library, run, entry, evidence)
            .unwrap();
        let approval = f.approved(&review);
        *f.workflow.fault.lock().unwrap() = Some(fault);
        let job = f.workflow.start(f.library.clone(), approval.id).unwrap();
        assert_eq!(f.wait(&job.id).state, SyncState::NeedsRecovery);
        let binding = f.workflow.binding(&job.id).unwrap();
        let scope = RetainedRecoveryScope::new(&f.workflow, &f.library, &binding).unwrap();
        let originals = scope.originals();
        let current = scope.current().unwrap();
        assert_eq!(originals.len(), 2);
        assert_eq!(current.len(), 2);
        for (index, object) in current.iter().enumerate() {
            let mut digest_input = object.file().try_clone().unwrap();
            digest_input.seek(SeekFrom::Start(0)).unwrap();
            assert_eq!(object.index(), index);
            assert_eq!(
                filesystem::identity(object.file()).unwrap(),
                originals[index].identity
            );
            assert_eq!(
                crate::executor::snapshot_bridge::digest(
                    &mut digest_input,
                    &AtomicBool::new(false)
                )
                .unwrap(),
                originals[index].full_digest
            );
        }
        let expected_old_digest = originals[0].full_digest.clone();
        drop(current);
        drop(scope);
        let history = f
            .destination
            .join(format!(".stillharbour-retained-{}", job.id));
        let missing = history.join("old-0.retained");
        let asserted = history.join("arbitrarily-asserted-old.retained");
        std::fs::rename(&missing, &asserted).unwrap();
        // Same native object and bytes elsewhere do not grant path authority;
        // absence of one companion cannot produce a partial-group proof.
        assert!(RetainedRecoveryScope::new(&f.workflow, &f.library, &binding).is_err());
        assert!(asserted.is_file());
        assert_eq!(
            blake3::hash(&std::fs::read(&asserted).unwrap())
                .to_hex()
                .to_string(),
            expected_old_digest
        );
        f.audit();
    }
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_copy_faults_reconcile_existing_stream_with_fresh_approval() {
    for fault in COPY_FAULTS {
        let f = Fixture::new();
        let (job, copy) = f.prepared();
        *f.workflow.fault.lock().unwrap() = Some(Fault::Copy(fault));
        let approval = f.approved(&copy);
        f.workflow.start(f.library.clone(), approval.id).unwrap();
        let stopped = f.wait(&job.id);
        assert_eq!(
            stopped.state,
            SyncState::NeedsRecovery,
            "{fault:?}: {}",
            stopped.message
        );
        f.audit();
        *f.workflow.fault.lock().unwrap() = None;
        let recovery = f.workflow.recovery_review(&f.library, &job.id).unwrap();
        let approval = f.approved(&recovery);
        f.workflow.start(f.library.clone(), approval.id).unwrap();
        let result = f.wait(&job.id);
        assert_eq!(
            result.state,
            SyncState::CompletedVerified,
            "{fault:?}: {}",
            result.message
        );
        f.audit();
    }
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_preparation_gaps_preserve_all_objects_and_only_resume_identified_ownership() {
    for fault in [
        Fault::StageCreated,
        Fault::ProbeCreated,
        Fault::NamesProven,
        Fault::ProbeMoved,
        Fault::PlanBound,
    ] {
        let f = Fixture::new();
        let review = f.review();
        *f.workflow.fault.lock().unwrap() = Some(fault);
        let approval = f.approved(&review);
        let job = f.workflow.start(f.library.clone(), approval.id).unwrap();
        let result = f.wait(&job.id);
        assert_eq!(result.state, SyncState::NeedsRecovery, "{fault:?}");
        f.audit();
        *f.workflow.fault.lock().unwrap() = None;
        let recovery = f.workflow.recovery_review(&f.library, &job.id).unwrap();
        if matches!(fault, Fault::StageCreated | Fault::ProbeCreated) {
            assert!(!recovery.execution_available);
            assert!(
                f.workflow
                    .approve(&f.library, recovery.id, true, true)
                    .is_err()
            );
        } else {
            assert!(recovery.execution_available);
            let approval = f.approved(&recovery);
            f.workflow.start(f.library.clone(), approval.id).unwrap();
            let result = f.wait(&job.id);
            if result.state == SyncState::NeedsReview {
                let review = f
                    .workflow
                    .review(result.ready_review_id.as_deref().unwrap())
                    .unwrap()
                    .view
                    .clone();
                let approval = f.approved(&review);
                f.workflow.start(f.library.clone(), approval.id).unwrap();
            }
            let result = f.wait(&job.id);
            assert_eq!(
                result.state,
                SyncState::CompletedVerified,
                "{fault:?}: {}",
                result.message
            );
        }
        f.audit();
    }
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_cold_child() {
    let Some(root) = std::env::var_os("STILLHARBOUR_SYNC_COLD_ROOT") else {
        return;
    };
    let root = PathBuf::from(root).canonicalize().unwrap();
    assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap()));
    assert!(
        root.file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("stillharbour-sync-application-")
    );
    let owner = std::env::var("STILLHARBOUR_SYNC_COLD_OWNER").unwrap();
    assert!(uuid::Uuid::parse_str(&owner).is_ok());
    assert_eq!(std::fs::read(root.join("owner")).unwrap(), owner.as_bytes());
    assert_eq!(std::fs::read_dir(root.join("source")).unwrap().count(), 4);
    for (name, bytes) in [
        ("movie.mp4", b"opaque video".as_slice()),
        ("movie.xmp", b"sidecar"),
        (".hidden", b"unknown hidden data"),
        ("empty.bin", b""),
    ] {
        assert_eq!(
            std::fs::read(root.join("source").join(name)).unwrap(),
            bytes
        );
    }
    assert_eq!(
        std::fs::read(root.join("destination/outside.bin")).unwrap(),
        b"unique neighbor"
    );
    let data = root.join("data");
    let catalog = Arc::new(Catalog::open(data.join("Fixture.stillharbour")).unwrap());
    let library = Library::new(
        catalog.clone(),
        LibraryOptions {
            data_directory: data.clone(),
            workers: WorkerPaths {
                executable: std::env::current_exe().unwrap(),
                exiftool: None,
                working_directory: data.join("workers"),
            },
        },
    )
    .unwrap();
    let workflow = Arc::new(SyncWorkflow::new(catalog, data).unwrap());
    let job = std::env::var("STILLHARBOUR_SYNC_COLD_JOB").unwrap();
    let binding = workflow.binding(&job).unwrap();
    assert_eq!(
        source::decode_path(&binding.scope.source).unwrap(),
        root.join("source")
    );
    assert_eq!(
        source::decode_path(&binding.scope.destination).unwrap(),
        root.join("destination")
    );
    let name = std::env::var("STILLHARBOUR_SYNC_COLD_FAULT").unwrap();
    let fault = if let Some(retained) = binding.retained.as_ref() {
        *workflow.recovery_proof.lock().unwrap() = Some(RecoveryProof {
            evidence_id: retained.evidence_id.clone(),
            scope_digest: "a".repeat(64),
            independent_failure_domain: true,
            essential_metadata_verified: true,
        });
        RETAINED_FAULTS
            .into_iter()
            .find(|f| format!("{f:?}") == name)
            .unwrap()
    } else {
        Fault::Copy(
            COPY_FAULTS
                .into_iter()
                .find(|f| format!("{f:?}") == name)
                .unwrap(),
        )
    };
    *workflow.fault.lock().unwrap() = Some(fault);
    workflow.crash.store(true, Ordering::Release);
    CRASH_ON_FAULT.store(true, Ordering::Release);
    let review = workflow.recovery_review(&library, &job).unwrap();
    let approval = workflow.approve(&library, review.id, true, true).unwrap();
    workflow.start(library, approval.id).unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        assert!(
            Instant::now() < deadline,
            "Cold child missed its requested copy boundary."
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_actual_process_exit_preserves_retained_group_at_all_eight_native_boundaries() {
    for fault in RETAINED_FAULTS {
        let (f, run, entry) = retained_fixture();
        let evidence = "e".repeat(64);
        synthetic_retained_proof(&f, &evidence);
        let review = f
            .workflow
            .review_replacement(&f.library, run, entry, evidence.clone())
            .unwrap();
        let approval = f.approved(&review);
        *f.workflow.fault.lock().unwrap() = Some(Fault::BeforeRun);
        let job = f.workflow.start(f.library.clone(), approval.id).unwrap();
        assert_eq!(f.wait(&job.id).state, SyncState::NeedsRecovery);
        assert!(f.workflow.events(&job.id).unwrap().is_empty());
        let Fixture {
            owned,
            library,
            workflow,
            source,
            destination,
            root,
        } = f;
        let path = owned.path().canonicalize().unwrap();
        let owner = std::fs::read_to_string(path.join("owner")).unwrap();
        drop(workflow);
        drop(library);
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "executor::sync_workflow::tests::sync_application_cold_child",
                "--nocapture",
            ])
            .env("STILLHARBOUR_SYNC_COLD_ROOT", &path)
            .env("STILLHARBOUR_SYNC_COLD_OWNER", owner)
            .env("STILLHARBOUR_SYNC_COLD_JOB", &job.id)
            .env("STILLHARBOUR_SYNC_COLD_FAULT", format!("{fault:?}"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(40);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() > deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("Retained child timeout at {fault:?}");
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(status.code(), Some(88), "{fault:?}");
        let data = path.join("data");
        let catalog = Arc::new(Catalog::open(data.join("Fixture.stillharbour")).unwrap());
        let library = Library::new(
            catalog.clone(),
            LibraryOptions {
                data_directory: data.clone(),
                workers: WorkerPaths {
                    executable: std::env::current_exe().unwrap(),
                    exiftool: None,
                    working_directory: data.join("workers"),
                },
            },
        )
        .unwrap();
        let workflow = Arc::new(SyncWorkflow::new(catalog, data).unwrap());
        let f = Fixture {
            owned,
            library,
            workflow,
            source,
            destination,
            root,
        };
        synthetic_retained_proof(&f, &evidence);
        f.audit();
        assert_eq!(
            f.workflow.job(&job.id).unwrap().state,
            SyncState::NeedsRecovery
        );
        let review = f.workflow.recovery_review(&f.library, &job.id).unwrap();
        if fault == Fault::RetainedDirectoryCreated {
            assert!(!review.execution_available);
            assert!(
                f.workflow
                    .approve(&f.library, review.id, true, true)
                    .is_err()
            );
        } else {
            let approval = f.approved(&review);
            f.workflow.start(f.library.clone(), approval.id).unwrap();
            let result = f.wait(&job.id);
            assert_eq!(
                result.state,
                SyncState::CompletedVerified,
                "{fault:?}: {}",
                result.message
            );
            let versions = f.workflow.retained_versions(&f.library, &job.id).unwrap();
            assert_eq!(versions.len(), 2);
            assert!(versions.iter().all(|v| v.retained));
        }
        f.audit();
    }
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn sync_application_actual_process_exit_reopens_catalog_and_recovers_all_nine_copy_boundaries() {
    for fault in COPY_FAULTS {
        let f = Fixture::new();
        let (job, _) = f.prepared();
        let Fixture {
            owned,
            library,
            workflow,
            source,
            destination,
            root,
        } = f;
        let path = owned.path().canonicalize().unwrap();
        let owner = std::fs::read_to_string(path.join("owner")).unwrap();
        drop(workflow);
        drop(library);
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "executor::sync_workflow::tests::sync_application_cold_child",
                "--nocapture",
            ])
            .env("STILLHARBOUR_SYNC_COLD_ROOT", &path)
            .env("STILLHARBOUR_SYNC_COLD_OWNER", owner)
            .env("STILLHARBOUR_SYNC_COLD_JOB", &job.id)
            .env("STILLHARBOUR_SYNC_COLD_FAULT", format!("{fault:?}"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(40);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() > deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("Cold sync child timed out at {fault:?}");
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(status.code(), Some(77), "{fault:?}");
        let data = path.join("data");
        let catalog = Arc::new(Catalog::open(data.join("Fixture.stillharbour")).unwrap());
        let library = Library::new(
            catalog.clone(),
            LibraryOptions {
                data_directory: data.clone(),
                workers: WorkerPaths {
                    executable: std::env::current_exe().unwrap(),
                    exiftool: None,
                    working_directory: data.join("workers"),
                },
            },
        )
        .unwrap();
        let workflow = Arc::new(SyncWorkflow::new(catalog, data).unwrap());
        let f = Fixture {
            owned,
            library,
            workflow,
            source,
            destination,
            root,
        };
        f.audit();
        assert_eq!(
            f.workflow.job(&job.id).unwrap().state,
            SyncState::NeedsRecovery
        );
        let review = f.workflow.recovery_review(&f.library, &job.id).unwrap();
        let approval = f.approved(&review);
        f.workflow.start(f.library.clone(), approval.id).unwrap();
        let result = f.wait(&job.id);
        assert_eq!(
            result.state,
            SyncState::CompletedVerified,
            "{fault:?}: {}",
            result.message
        );
        f.audit();
    }
}

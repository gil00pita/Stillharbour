//! Generated-only application-adapter transport probe. Secrets enter bounded stdin only.
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::BTreeMap,
    io::Read,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use stillharbour_core::{
    backup::{
        credentials::{CredentialPurpose, CredentialStore},
        helpers::credential_context,
        snapshots::{
            SnapshotDestinationInput, SnapshotJob, SnapshotReply, SnapshotRequest, SnapshotReview,
        },
    },
    catalog::Catalog,
    domain::ErrorCode,
    filesystem,
    library::{
        Library, LibraryOptions, WorkerPaths,
        model::{RootMode, ScanFilter},
        native,
        snapshots::{NativeSnapshotSelection, SnapshotFixtureFailure, snapshot_fixture_error},
    },
};
use zeroize::{Zeroize, Zeroizing};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Input {
    destination: SnapshotDestinationInput,
    repository_password: String,
    transport_password: Option<String>,
    failure: Option<FailureInput>,
}
#[derive(Clone, Copy, Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum FailureScenario {
    DisconnectUpload,
    QuotaExhausted,
    CredentialExpired,
    PublicationAckLost,
    RepositoryCorrupted,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FailureInput {
    scenario: FailureScenario,
    owner_nonce: String,
}
impl Drop for Input {
    fn drop(&mut self) {
        self.repository_password.zeroize();
        if let Some(secret) = &mut self.transport_password {
            secret.zeroize();
        }
    }
}
struct FaultControl {
    directory: std::path::PathBuf,
    nonce: String,
    scenario: FailureScenario,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FaultEvidence {
    owner_nonce: String,
    scenario: FailureScenario,
    payload_bytes: u64,
    publication_observed: bool,
    injection_applied: bool,
}
impl FaultControl {
    fn new(input: &FailureInput) -> Result<Self> {
        require(
            "fault-nonce",
            input.owner_nonce.len() == 32
                && input
                    .owner_nonce
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
        )?;
        let directory = io("fault-cwd", std::env::current_dir())?;
        let directory = io("fault-canonical-cwd", directory.canonicalize())?;
        let prefix = format!("stillharbour-m4-failure-{}-", input.owner_nonce);
        require(
            "fault-owned-cwd",
            directory
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with(&prefix)),
        )?;
        let mut control = Self {
            directory,
            nonce: input.owner_nonce.clone(),
            scenario: input.scenario,
        };
        require(
            "fault-owner",
            control.read("owner")? == control.nonce.as_bytes(),
        )?;
        // Controls are a fixed sibling of the generated application source, so native
        // source ancestor handles do not interfere with atomic server evidence publication.
        let child = control.directory.join("failure-control");
        require(
            "fault-control-directory",
            io("fault-control-metadata", std::fs::symlink_metadata(&child))?
                .file_type()
                .is_dir(),
        )?;
        let child = io("fault-control-canonical", child.canonicalize())?;
        require(
            "fault-control-containment",
            child.parent() == Some(control.directory.as_path()),
        )?;
        control.directory = child;
        require(
            "fault-control-owner",
            control.read("owner")? == control.nonce.as_bytes(),
        )?;
        for name in [
            "failure.arm",
            "failure.disarm",
            "failure.triggered",
            "failure.ack",
        ] {
            require(
                "fault-fresh-control",
                !control.directory.join(name).exists(),
            )?;
        }
        Ok(control)
    }
    fn read(&self, name: &str) -> Result<Vec<u8>> {
        let path = self.directory.join(name);
        let meta = io("fault-control-metadata", std::fs::symlink_metadata(&path))?;
        require(
            "fault-control-file",
            meta.file_type().is_file() && meta.len() <= 4096,
        )?;
        let mut file = io("fault-control-open", std::fs::File::open(path))?;
        let mut bytes = Vec::new();
        io(
            "fault-control-read",
            file.by_ref().take(4097).read_to_end(&mut bytes),
        )?;
        require("fault-control-bound", bytes.len() <= 4096)?;
        Ok(bytes)
    }
    fn write(&self, name: &str) -> Result<()> {
        use std::io::Write;
        require("fault-owner", self.read("owner")? == self.nonce.as_bytes())?;
        let mut file = io(
            "fault-control-create",
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(self.directory.join(name)),
        )?;
        io("fault-control-write", file.write_all(self.nonce.as_bytes()))?;
        io("fault-control-flush", file.sync_all())
    }
    fn evidence(&self, name: &str) -> Result<FaultEvidence> {
        let until = Instant::now() + Duration::from_secs(15);
        while !self.directory.join(name).exists() {
            require("fault-evidence-deadline", Instant::now() < until)?;
            std::thread::sleep(Duration::from_millis(25));
        }
        let evidence: FaultEvidence =
            serde_json::from_slice(&self.read(name)?).map_err(|_| fail("fault-evidence-schema"))?;
        require(
            "fault-evidence-owner",
            evidence.owner_nonce == self.nonce
                && evidence.scenario == self.scenario
                && evidence.injection_applied,
        )?;
        require(
            "fault-reached-transfer",
            self.scenario == FailureScenario::CredentialExpired || evidence.payload_bytes > 0,
        )?;
        require(
            "fault-publication-observed",
            self.scenario != FailureScenario::PublicationAckLost || evidence.publication_observed,
        )?;
        Ok(evidence)
    }
}
impl Drop for FaultControl {
    fn drop(&mut self) {
        if self.directory.join("failure.arm").is_file()
            && !self.directory.join("failure.disarm").exists()
        {
            let _ = self.write("failure.disarm");
        }
    }
}
struct Credentials {
    catalog: String,
    references: Vec<(CredentialPurpose, String)>,
}
impl Credentials {
    fn cleanup(&mut self) -> Result<()> {
        self.references.retain(|(purpose, reference)| {
            CredentialStore::delete_owned(&self.catalog, *purpose, reference).is_err()
        });
        if self.references.is_empty() {
            Ok(())
        } else {
            Err(Failure {
                stage: "credential-cleanup",
                code: ErrorCode::Io,
                job: None,
                repository_locked: false,
            })
        }
    }
}
impl Drop for Credentials {
    fn drop(&mut self) {
        for (purpose, reference) in &self.references {
            let _ = CredentialStore::delete_owned(&self.catalog, *purpose, reference);
        }
    }
}
struct Failure {
    stage: &'static str,
    code: ErrorCode,
    job: Option<serde_json::Value>,
    repository_locked: bool,
}
struct RunScope {
    library: Option<Arc<Library>>,
    // Drop the last Library/catalog handles before removing the owned tree.
    tree: Option<tempfile::TempDir>,
}
impl Drop for RunScope {
    fn drop(&mut self) {
        let Some(library) = &self.library else {
            return;
        };
        library.cancel_all();
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            let terminal = matches!(library.backup_snapshots(SnapshotRequest::Jobs { profile_id: None }, None),
                Ok(SnapshotReply::Jobs(jobs)) if jobs.iter().all(|job| !["queued", "running", "cancel_requested"].contains(&job.state.as_str())));
            if terminal {
                return;
            }
            if Instant::now() >= until {
                // Keep an owned partial tree if a blocked native worker has not stopped.
                // Never recursively remove its files concurrently with that worker.
                if let Some(tree) = self.tree.take() {
                    let _ = tree.keep();
                }
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
type Result<T> = std::result::Result<T, Failure>;
fn fail(stage: &'static str) -> Failure {
    Failure {
        stage,
        code: ErrorCode::Conflict,
        job: None,
        repository_locked: false,
    }
}
fn app<T>(stage: &'static str, value: stillharbour_core::domain::Result<T>) -> Result<T> {
    value.map_err(|error| Failure {
        repository_locked: snapshot_fixture_error(&error)
            == Some(SnapshotFixtureFailure::RepositoryLocked),
        stage,
        code: error.code,
        job: None,
    })
}
fn io<T>(stage: &'static str, value: std::io::Result<T>) -> Result<T> {
    value.map_err(|_| Failure {
        stage,
        code: ErrorCode::Io,
        job: None,
        repository_locked: false,
    })
}
fn require(stage: &'static str, value: bool) -> Result<()> {
    if value { Ok(()) } else { Err(fail(stage)) }
}
fn selection(path: &Path) -> Result<NativeSnapshotSelection> {
    let handle = app("selection", native::open_root(path, None))?;
    Ok(NativeSnapshotSelection {
        id: uuid::Uuid::new_v4().to_string(),
        path: path.into(),
        identity: app("selection", filesystem::identity(&handle))?,
        volume: app("selection", native::volume_token(&handle))?,
    })
}
fn endpoint(destination: &SnapshotDestinationInput) -> Result<&'static str> {
    let (protocol, host, port) = match destination {
        SnapshotDestinationInput::Local { .. } => return Ok("local"),
        SnapshotDestinationInput::Sftp { host, port, .. } => ("sftp", host, *port),
        SnapshotDestinationInput::Ftps { host, port, .. } => ("ftps", host, *port),
        SnapshotDestinationInput::Ftp {
            host,
            port,
            insecure_acknowledged,
            ..
        } => {
            require("ftp-consent", *insecure_acknowledged)?;
            ("ftp", host, *port)
        }
        SnapshotDestinationInput::Smb { host, port, .. } => ("smb", host, *port),
    };
    require(
        "loopback-disposable-endpoint",
        host == "127.0.0.1" && port >= 1024,
    )?;
    Ok(protocol)
}
fn call(library: &Arc<Library>, request: SnapshotRequest) -> Result<SnapshotReply> {
    app(
        "application-command",
        library.backup_snapshots(request, None),
    )
}
fn review(reply: SnapshotReply) -> Result<SnapshotReview> {
    if let SnapshotReply::Review(value) = reply {
        Ok(value)
    } else {
        Err(fail("review-reply"))
    }
}
fn wait_terminal(library: &Arc<Library>, initial: SnapshotJob) -> Result<SnapshotJob> {
    let until = Instant::now() + Duration::from_secs(180);
    loop {
        let SnapshotReply::Job(job) = call(
            library,
            SnapshotRequest::Job {
                job_id: initial.id.clone(),
            },
        )?
        else {
            return Err(fail("job-reply"));
        };
        require(
            "job-identity",
            job.id == initial.id && job.profile_id == initial.profile_id,
        )?;
        if !["queued", "running", "cancel_requested"].contains(&job.state.as_str()) {
            return Ok(job);
        }
        if Instant::now() >= until {
            let _ = call(
                library,
                SnapshotRequest::Cancel {
                    job_id: initial.id.clone(),
                },
            );
            return Err(fail("job-deadline"));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
fn wait(library: &Arc<Library>, initial: SnapshotJob) -> Result<SnapshotJob> {
    let job = wait_terminal(library, initial)?;
    if job.state != "completed_verified" {
        let mut failure = fail("completed-native-job");
        failure.job = Some(terminal_diagnostics(&job));
        failure.repository_locked = repository_locked(library, &job)?;
        return Err(failure);
    }
    require(
        "earned-evidence",
        job.included
            && job.structural_checked
            && job.data_read_verified
            && job.restored_compared
            && job.source_retained
            && !job.independent_recovery_verified,
    )?;
    Ok(job)
}
fn no_earned_flags(job: &SnapshotJob) -> bool {
    !job.included && verification_not_earned(job)
}
fn verification_not_earned(job: &SnapshotJob) -> bool {
    !job.structural_checked
        && !job.data_read_verified
        && !job.restored_compared
        && !job.independent_recovery_verified
}
fn repository_locked(library: &Arc<Library>, job: &SnapshotJob) -> Result<bool> {
    Ok(app(
        "fixture-failure-classification",
        library.snapshot_fixture_failure(&job.id),
    )? == Some(SnapshotFixtureFailure::RepositoryLocked))
}
fn terminal_diagnostics(job: &SnapshotJob) -> serde_json::Value {
    // Never forward extensible backend strings: only known, public state/phase tokens.
    let state = match job.state.as_str() {
        "completed_verified" => "completed_verified",
        "canceled_with_partial_copies" => "canceled_with_partial_copies",
        "failed" => "failed",
        "needs_recovery" => "needs_recovery",
        _ => "unknown",
    };
    let phase = match job.phase.as_str() {
        "review_validated" => "review_validated",
        "staging" => "staging",
        "catalog_snapshot" => "catalog_snapshot",
        "snapshotting" => "snapshotting",
        "snapshot_included" => "snapshot_included",
        "structural_check" => "structural_check",
        "data_read" => "data_read",
        "restoring" => "restoring",
        "comparing" => "comparing",
        "completed" => "completed",
        "publication_name_preflight" => "publication_name_preflight",
        "terminal" => "terminal",
        _ => "unknown",
    };
    json!({"jobState":state,"jobPhase":phase,"included":job.included,
        "structuralChecked":job.structural_checked,"dataReadVerified":job.data_read_verified,
        "restoredCompared":job.restored_compared})
}
fn execute(library: &Arc<Library>, review: SnapshotReview) -> Result<SnapshotJob> {
    let job = start(library, review, None)?;
    wait(library, job)
}
fn start(
    library: &Arc<Library>,
    review: SnapshotReview,
    fault: Option<&FaultControl>,
) -> Result<SnapshotJob> {
    require(
        "execution-capability",
        review.execution_available && review.source_retained,
    )?;
    let backup = review.action == "backup";
    require("review-action", backup || review.action == "restore")?;
    let request = if backup {
        SnapshotRequest::BackupApprove {
            review_id: review.id.clone(),
            digest: review.digest.clone(),
        }
    } else {
        SnapshotRequest::RestoreApprove {
            review_id: review.id.clone(),
            digest: review.digest.clone(),
        }
    };
    let SnapshotReply::Approval(approval) = call(library, request)? else {
        return Err(fail("approval-reply"));
    };
    require(
        "approval-identity",
        approval.review_id == review.id && approval.digest == review.digest,
    )?;
    let request = if backup {
        SnapshotRequest::BackupStart {
            review_id: review.id,
            approval_id: approval.id,
        }
    } else {
        SnapshotRequest::RestoreStart {
            review_id: review.id,
            approval_id: approval.id,
        }
    };
    if let Some(fault) = fault {
        fault.write("failure.arm")?;
    }
    let SnapshotReply::Job(job) = call(library, request)? else {
        return Err(fail("start-reply"));
    };
    Ok(job)
}
fn restore_audit(
    library: &Arc<Library>,
    profile: &str,
    snapshot: &str,
    restore: &Path,
    root_id: &str,
    baseline: &BTreeMap<&str, Vec<u8>>,
) -> Result<SnapshotJob> {
    let output = selection(restore)?;
    let plan = review(app(
        "baseline-restore-review",
        library.backup_snapshots(
            SnapshotRequest::RestoreReview {
                profile_id: profile.into(),
                snapshot_id: snapshot.into(),
                selection_id: output.id.clone(),
            },
            Some(output),
        ),
    )?)?;
    let restored_root = restore.join(format!("stillharbour-restored-{}", plan.id));
    let restored = execute(library, plan)?;
    for (relative, bytes) in baseline {
        require(
            "baseline-restore-full-bytes",
            io(
                "baseline-restore-read",
                std::fs::read(restored_root.join("roots").join(root_id).join(relative)),
            )? == *bytes,
        )?;
    }
    require(
        "baseline-restore-catalog",
        restored_root
            .join("Catalog.snapshot.stillharbour")
            .is_file(),
    )?;
    Ok(restored)
}
struct FailureScope<'a> {
    library: &'a Arc<Library>,
    profile: &'a str,
    snapshot: &'a str,
    source: &'a Path,
    restore: &'a Path,
    local: &'a Path,
    root_id: &'a str,
    baseline: &'a BTreeMap<&'a str, Vec<u8>>,
}
fn failure_run(scope: FailureScope<'_>, input: &FailureInput) -> Result<serde_json::Value> {
    let FailureScope {
        library,
        profile,
        snapshot,
        source,
        restore,
        local,
        root_id,
        baseline,
    } = scope;
    let fault = FaultControl::new(input)?;
    // Earn restoration evidence before changing the generated source or arming a server.
    let baseline_restored = restore_audit(library, profile, snapshot, restore, root_id, baseline)?;
    let mut current = baseline.clone();
    let mut state = 0x93a6_741d_8752_0bcfu64;
    let payload: Vec<u8> = (0..2 * 1024 * 1024)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect();
    io(
        "fault-generated-change",
        std::fs::write(source.join("photo.bin"), &payload),
    )?;
    current.insert("photo.bin", payload);
    let plan = review(call(
        library,
        SnapshotRequest::BackupReview {
            profile_id: profile.into(),
        },
    )?)?;
    let initial = start(library, plan, Some(&fault))?;
    let second = wait_terminal(library, initial)?;
    fault.write("failure.disarm")?;
    let triggered = fault.evidence("failure.triggered")?;
    let ack = fault.evidence("failure.ack")?;
    let recovered_ack = input.scenario == FailureScenario::PublicationAckLost
        && second.state == "completed_verified"
        && second.included
        && second.structural_checked
        && second.data_read_verified
        && second.restored_compared;
    require(
        "fault-native-terminal-failure",
        recovered_ack
            || ["failed", "needs_recovery", "canceled_with_partial_copies"]
                .contains(&second.state.as_str()),
    )?;
    require(
        "fault-corruption-reached-verification",
        input.scenario != FailureScenario::RepositoryCorrupted
            || (second.included
                && (!second.structural_checked
                    || !second.data_read_verified
                    || !second.restored_compared)),
    )?;
    for (relative, expected) in &current {
        require(
            "fault-source-preserved",
            io("fault-source-audit", std::fs::read(source.join(relative)))? == *expected,
        )?;
    }
    let mut reconcile_locked = false;
    let mut reconcile_no_candidate = false;
    let reconciliation = if second.state == "needs_recovery" {
        match call(
            library,
            SnapshotRequest::Reconcile {
                profile_id: profile.into(),
                job_id: second.id.clone(),
                snapshot_id: second.snapshot_id.clone(),
            },
        ) {
            Ok(SnapshotReply::Job(job)) => {
                require(
                    "fault-reconcile-identity",
                    job.id != second.id && job.profile_id == profile && job.action == "reconcile",
                )?;
                let terminal = wait_terminal(library, job)?;
                let classification = app(
                    "fixture-reconcile-classification",
                    library.snapshot_fixture_failure(&terminal.id),
                )?;
                reconcile_locked = classification == Some(SnapshotFixtureFailure::RepositoryLocked);
                reconcile_no_candidate =
                    classification == Some(SnapshotFixtureFailure::NoCompletedSnapshotVisible);
                if reconcile_no_candidate {
                    require(
                        "fault-reconcile-no-candidate-evidence",
                        terminal.state == "needs_recovery"
                            && terminal.snapshot_id.is_none()
                            && no_earned_flags(&terminal)
                            && second.snapshot_id.is_none()
                            && no_earned_flags(&second),
                    )?;
                }
                json!({"attempted":true,"repositoryLocked":reconcile_locked,"noCompletedSnapshotVisible":reconcile_no_candidate,
                    "snapshotPresent":terminal.snapshot_id.is_some(),"job":terminal_diagnostics(&terminal)})
            }
            Ok(_) => return Err(fail("fault-reconcile-reply")),
            Err(error) => {
                reconcile_locked = error.repository_locked;
                json!({"attempted":true,"repositoryLocked":reconcile_locked,"rejected":true,"errorCode":error.code})
            }
        }
    } else {
        json!({"attempted":false,"notApplicable":true})
    };
    let mut verify_locked = false;
    let verification = match call(
        library,
        SnapshotRequest::Verify {
            profile_id: profile.into(),
            snapshot_id: snapshot.into(),
        },
    ) {
        Ok(SnapshotReply::Job(job)) => {
            let terminal = wait_terminal(library, job)?;
            verify_locked = repository_locked(library, &terminal)?;
            if verify_locked {
                require(
                    "fault-locked-verification-no-earned-flags",
                    verification_not_earned(&terminal),
                )?;
                json!({"state":"blocked_repository_locked","repositoryLocked":true,"verified":false,"job":terminal_diagnostics(&terminal)})
            } else {
                require(
                    "fault-baseline-verification",
                    input.scenario == FailureScenario::RepositoryCorrupted
                        || (terminal.state == "completed_verified"
                            && terminal.included
                            && terminal.structural_checked
                            && terminal.data_read_verified
                            && terminal.restored_compared),
                )?;
                terminal_diagnostics(&terminal)
            }
        }
        Ok(_) => return Err(fail("fault-baseline-verify-reply")),
        Err(error) if error.repository_locked => {
            verify_locked = true;
            json!({"state":"blocked_repository_locked","repositoryLocked":true,"verified":false})
        }
        Err(error) if input.scenario == FailureScenario::RepositoryCorrupted => {
            json!({"rejected":true,"errorCode":error.code})
        }
        Err(error) => return Err(error),
    };
    let mut restore_locked = false;
    let (post_restored, post_restore_evidence) = match restore_audit(
        library, profile, snapshot, restore, root_id, baseline,
    ) {
        Ok(job) => (true, terminal_diagnostics(&job)),
        Err(error) if error.repository_locked => {
            restore_locked = true;
            (
                false,
                json!({"state":"blocked_repository_locked","repositoryLocked":true,"restoredCompared":false,"job":error.job}),
            )
        }
        Err(error) if input.scenario == FailureScenario::RepositoryCorrupted => (
            false,
            json!({"rejected":true,"errorCode":error.code,"job":error.job}),
        ),
        Err(error) => return Err(error),
    };
    let unresolved_lock = verify_locked || restore_locked;
    if unresolved_lock {
        require(
            "fault-unresolved-lock-exact-evidence",
            verify_locked
                && restore_locked
                && (reconcile_locked || reconcile_no_candidate)
                && second.state == "needs_recovery"
                && no_earned_flags(&second)
                && baseline_restored.included
                && baseline_restored.structural_checked
                && baseline_restored.data_read_verified
                && baseline_restored.restored_compared,
        )?;
    }
    let SnapshotReply::Snapshots(history) = call(
        library,
        SnapshotRequest::Snapshots {
            profile_id: profile.into(),
        },
    )?
    else {
        return Err(fail("fault-baseline-history"));
    };
    require(
        "fault-baseline-retained",
        history
            .iter()
            .any(|record| record.id == snapshot && record.catalog_included),
    )?;
    if recovered_ack {
        require(
            "fault-publication-history",
            second.snapshot_id.as_ref().is_some_and(|id| {
                history.iter().any(|record| {
                    record.id == *id
                        && record.included
                        && record.structural_checked
                        && record.data_read_verified
                        && record.restored_compared
                })
            }),
        )?;
    }
    require(
        "fault-local-neighbor",
        io(
            "fault-neighbor-read",
            std::fs::read(local.join("neighbor.txt")),
        )? == b"generated repository neighbor",
    )?;
    require(
        "fault-restore-neighbor",
        io(
            "fault-neighbor-read",
            std::fs::read(restore.join("neighbor.txt")),
        )? == b"generated restore neighbor",
    )?;
    for (relative, expected) in &current {
        require(
            "fault-final-source-preserved",
            io(
                "fault-final-source-audit",
                std::fs::read(source.join(relative)),
            )? == *expected,
        )?;
    }
    let stopped_until = Instant::now() + Duration::from_secs(5);
    while !app("fault-helper-shutdown", library.backup_shutdown_ready())? {
        require("fault-helpers-stopped", Instant::now() < stopped_until)?;
        std::thread::sleep(Duration::from_millis(25));
    }
    Ok(
        json!({"scenario":input.scenario,"outcome":if unresolved_lock{"preserved_unresolved_lock"}else if post_restored{"preserved_verified"}else{"preserved_verification_rejected"},"repositoryLocked":unresolved_lock,"helpersStopped":true,"recoveryCompleted":post_restored && !unresolved_lock,"injectionApplied":true,"payloadBytes":triggered.payload_bytes,
        "publicationObserved":triggered.publication_observed,"disarmed":ack.injection_applied,
        "nativeFailure":terminal_diagnostics(&second),"sourceUnchangedAfterFault":true,
        "sourceFilesCompared":current.len(),"baselineHistoryRetained":true,
        "baselineRestoredBeforeFault":baseline_restored.restored_compared,"baselineRestoredAfterFault":post_restored,
        "baselineVerification":verification,"baselineRestore":post_restore_evidence,
        "reconciliation":reconciliation,"restoreNeighborPreserved":true,"localNeighborPreserved":true}),
    )
}
fn run() -> Result<serde_json::Value> {
    let mut bytes = Zeroizing::new(Vec::new());
    io(
        "stdin",
        std::io::stdin().take(65537).read_to_end(&mut bytes),
    )?;
    require("bounded-input", bytes.len() <= 65536)?;
    let input: Input = serde_json::from_slice(&bytes).map_err(|_| fail("typed-input"))?;
    require(
        "bounded-secret",
        !input.repository_password.is_empty()
            && input.repository_password.len() <= 4096
            && input
                .transport_password
                .as_ref()
                .is_none_or(|secret| !secret.is_empty() && secret.len() <= 4096),
    )?;
    let protocol = endpoint(&input.destination)?;
    let run = io(
        "temporary-fixture",
        tempfile::Builder::new()
            .prefix("stillharbour-m4-app-protocol-")
            .tempdir(),
    )?;
    let root = run
        .path()
        .canonicalize()
        .map_err(|_| fail("fixture-boundary"))?;
    let mut scope = RunScope {
        tree: Some(run),
        library: None,
    };
    let source = root.join("source");
    let data = root.join("data");
    let local = root.join("repository");
    let restore = root.join("restore");
    for path in [&source, &data, &local, &restore] {
        io("fixture-directories", std::fs::create_dir(path))?;
    }
    io("fixture-nested", std::fs::create_dir(source.join("nested")))?;
    let baseline = BTreeMap::from([
        ("photo.bin", b"generated application media bytes".to_vec()),
        (
            "photo.xmp",
            b"<x:xmpmeta>generated sidecar</x:xmpmeta>".to_vec(),
        ),
        (".hidden", b"generated hidden scope".to_vec()),
        ("nested/movie.bin", vec![0x73; 131073]),
        ("nested/empty.bin", Vec::new()),
    ]);
    for (relative, content) in &baseline {
        io(
            "fixture-payload",
            std::fs::write(source.join(relative), content),
        )?;
    }
    io(
        "fixture-neighbor",
        std::fs::write(local.join("neighbor.txt"), b"generated repository neighbor"),
    )?;
    io(
        "fixture-neighbor",
        std::fs::write(restore.join("neighbor.txt"), b"generated restore neighbor"),
    )?;
    let catalog = Arc::new(app(
        "catalog",
        Catalog::open(data.join("Generated.stillharbour")),
    )?);
    let catalog_id = app("catalog-summary", catalog.summary())?.id;
    let library = app(
        "library",
        Library::new(
            catalog,
            LibraryOptions {
                data_directory: data.clone(),
                workers: WorkerPaths {
                    executable: data.join("missing-worker"),
                    exiftool: None,
                    working_directory: data.join("worker"),
                },
            },
        ),
    )?;
    scope.library = Some(library.clone());
    let platform = if cfg!(windows) {
        "windows-x86_64"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "macos-aarch64"
    } else {
        return Err(fail("supported-native-platform"));
    };
    let helpers = io(
        "canonical-helper-directory",
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/backup-helpers")
            .join(platform)
            .canonicalize(),
    )?;
    app(
        "pinned-helpers",
        library.configure_snapshot_helpers(helpers),
    )?;
    let native = selection(&source)?;
    let scan = app(
        "register-generated-root",
        library.register_verified(
            source.clone(),
            (native.identity, native.volume),
            "Generated protocol scope".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        ),
    )?;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let status = app("scan-status", library.status())?;
        if let Some(job) = status.jobs.iter().find(|job| job.id == scan.id) {
            if job.state == "completed" {
                break;
            }
            require(
                "scan-result",
                job.state != "failed" && job.state != "canceled",
            )?;
        }
        require("scan-deadline", Instant::now() < deadline)?;
        std::thread::sleep(Duration::from_millis(20));
    }
    let mut credentials = Credentials {
        catalog: catalog_id.clone(),
        references: Vec::new(),
    };
    let reference = app(
        "repository-vault",
        CredentialStore::store(
            &catalog_id,
            CredentialPurpose::Repository,
            &input.repository_password,
        ),
    )?;
    credentials
        .references
        .push((CredentialPurpose::Repository, reference.clone()));
    let transport = if protocol == "local" {
        None
    } else {
        let password = input
            .transport_password
            .as_deref()
            .ok_or_else(|| fail("transport-secret"))?;
        let (context, _) = app("endpoint-binding", credential_context(&input.destination))?;
        let reference = app(
            "transport-vault",
            CredentialStore::store_bound(
                &catalog_id,
                CredentialPurpose::Transport,
                Some(&context),
                password,
            ),
        )?;
        credentials
            .references
            .push((CredentialPurpose::Transport, reference.clone()));
        Some(reference)
    };
    let local_selection = selection(&local)?;
    let destination = if protocol == "local" {
        SnapshotDestinationInput::Local {
            selection_id: local_selection.id.clone(),
        }
    } else {
        input.destination.clone()
    };
    let reply = app(
        "profile-create",
        library.backup_snapshots(
            SnapshotRequest::ProfileCreate {
                label: "Generated application protocol".into(),
                root_ids: vec![scan.root_id.clone()],
                repository_credential_ref: reference,
                transport_credential_ref: transport,
                destination,
            },
            if protocol == "local" {
                Some(local_selection)
            } else {
                None
            },
        ),
    )?;
    let SnapshotReply::Profile(profile) = reply else {
        return Err(fail("profile-reply"));
    };
    let backup_review = review(call(
        &library,
        SnapshotRequest::BackupReview {
            profile_id: profile.id.clone(),
        },
    )?)?;
    require(
        "whole-generated-scope",
        backup_review.catalog_included && backup_review.files as usize == baseline.len(),
    )?;
    let backup = execute(&library, backup_review)?;
    let snapshot = backup
        .snapshot_id
        .clone()
        .ok_or_else(|| fail("snapshot-id"))?;
    if let Some(failure) = &input.failure {
        require("fault-remote-only", protocol != "local")?;
        let evidence = failure_run(
            FailureScope {
                library: &library,
                profile: &profile.id,
                snapshot: &snapshot,
                source: &source,
                restore: &restore,
                local: &local,
                root_id: &scan.root_id,
                baseline: &baseline,
            },
            failure,
        )?;
        library.cancel_all();
        credentials.cleanup()?;
        return Ok(
            json!({"passed":true,"applicationAdapter":true,"scope":"constructor-owned-generated-only",
            "protocol":protocol,"nativeOs":std::env::consts::OS,"failure":evidence,"credentialCleanupVerified":true}),
        );
    }
    for (relative, expected) in &baseline {
        require(
            "source-preserved",
            io("source-audit", std::fs::read(source.join(relative)))? == *expected,
        )?;
        let target = source.join(relative);
        let canonical = io("remove-boundary", target.canonicalize())?;
        require(
            "generated-removal-boundary",
            canonical.starts_with(&source) && canonical.is_file(),
        )?;
        io("generated-source-removal", std::fs::remove_file(&target))?;
    }
    let output = selection(&restore)?;
    let restore_review = review(app(
        "restore-review",
        library.backup_snapshots(
            SnapshotRequest::RestoreReview {
                profile_id: profile.id.clone(),
                snapshot_id: snapshot.clone(),
                selection_id: output.id.clone(),
            },
            Some(output),
        ),
    )?)?;
    let restored_root = restore.join(format!("stillharbour-restored-{}", restore_review.id));
    let restored = execute(&library, restore_review)?;
    for (relative, expected) in &baseline {
        require(
            "restored-full-bytes",
            io(
                "restored-read",
                std::fs::read(
                    restored_root
                        .join("roots")
                        .join(&scan.root_id)
                        .join(relative),
                ),
            )? == *expected,
        )?;
    }
    require(
        "restored-catalog",
        restored_root
            .join("Catalog.snapshot.stillharbour")
            .is_file(),
    )?;
    require(
        "restore-neighbor",
        io(
            "neighbor-audit",
            std::fs::read(restore.join("neighbor.txt")),
        )? == b"generated restore neighbor",
    )?;
    require(
        "local-neighbor",
        io("neighbor-audit", std::fs::read(local.join("neighbor.txt")))?
            == b"generated repository neighbor",
    )?;
    let SnapshotReply::Snapshots(history) = call(
        &library,
        SnapshotRequest::Snapshots {
            profile_id: profile.id,
        },
    )?
    else {
        return Err(fail("history-reply"));
    };
    require(
        "snapshot-retained",
        history
            .iter()
            .any(|record| record.id == snapshot && record.catalog_included),
    )?;
    library.cancel_all();
    credentials.cleanup()?;
    Ok(
        json!({"passed":true,"scope":"constructor-owned-generated-only","nativeOs":std::env::consts::OS,
        "protocol":protocol,"filesCompared":baseline.len(),"sourceUnchangedBeforeRemoval":true,"sourceRemoved":true,
        "restoredCompared":restored.restored_compared,"catalogIncluded":true,"catalogServiceVerified":true,
        "destinationOnlyPreserved":if protocol=="local"{Some(true)}else{None},
        "restoreNeighborPreserved":true,"historyRetained":true,"independentRecoveryVerified":false,
        "applicationAdapter":true,"helperPlatform":platform,"credentialCleanupVerified":true}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordinary_busy_and_worker_errors_never_authorize_locked_preservation() {
        for code in [
            ErrorCode::Busy,
            ErrorCode::Conflict,
            ErrorCode::WorkerFailed,
        ] {
            let error = stillharbour_core::domain::AppError::new(
                code,
                "untrusted message mentions repository locked exit 11",
            );
            let result: Result<()> = app("test-classification", Err(error));
            assert!(matches!(
                result,
                Err(Failure {
                    repository_locked: false,
                    ..
                })
            ));
        }
    }
    #[test]
    fn failure_controls_accept_only_fixed_scenarios_and_bounded_owned_authority() {
        for invalid in [
            r#"{"scenario":"arbitrary_command","ownerNonce":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#,
            r#"{"scenario":"disconnect_upload","ownerNonce":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","path":"/outside"}"#,
        ] {
            assert!(serde_json::from_str::<FailureInput>(invalid).is_err());
        }
        for nonce in ["../outside", "AAAA", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/"] {
            assert!(
                FaultControl::new(&FailureInput {
                    scenario: FailureScenario::DisconnectUpload,
                    owner_nonce: nonce.into()
                })
                .is_err()
            );
        }
        assert!(serde_json::from_str::<FaultEvidence>(r#"{"ownerNonce":"a","scenario":"disconnect_upload","payloadBytes":1,"publicationObserved":false,"injectionApplied":true,"message":"private"}"#).is_err());
    }
    #[test]
    fn terminal_diagnostics_never_forward_unknown_strings_or_job_messages() {
        let private = "secret-or-private-path";
        let mut job = SnapshotJob {
            id: private.into(),
            profile_id: private.into(),
            action: private.into(),
            state: private.into(),
            phase: private.into(),
            files: 0,
            bytes: "0".into(),
            snapshot_id: None,
            message: private.into(),
            started_at: private.into(),
            finished_at: None,
            included: true,
            structural_checked: false,
            data_read_verified: false,
            restored_compared: false,
            source_retained: true,
            independent_recovery_verified: false,
        };
        let diagnostic = terminal_diagnostics(&job);
        // Listing an existing baseline earns inclusion before a lock blocks deeper checks.
        assert!(verification_not_earned(&job));
        assert!(!no_earned_flags(&job));
        job.structural_checked = true;
        assert!(!verification_not_earned(&job));
        job.structural_checked = false;
        assert_eq!(diagnostic["jobState"], "unknown");
        assert_eq!(diagnostic["jobPhase"], "unknown");
        assert!(!diagnostic.to_string().contains(private));
        job.state = "needs_recovery".into();
        job.phase = "snapshotting".into();
        let diagnostic = terminal_diagnostics(&job);
        assert_eq!(diagnostic["jobState"], "needs_recovery");
        assert_eq!(diagnostic["jobPhase"], "snapshotting");
        assert_eq!(diagnostic["included"], true);
    }
    #[test]
    fn remote_authority_is_only_loopback_high_port_and_explicit_ftp_consent() {
        let destination = |host: &str, port, insecure_acknowledged| SnapshotDestinationInput::Ftp {
            host: host.into(),
            port,
            user: "generated".into(),
            remote_path: "/owned".into(),
            insecure_acknowledged,
        };
        assert!(endpoint(&destination("127.0.0.1", 49152, true)).is_ok());
        for candidate in [
            destination("example.org", 49152, true),
            destination("localhost", 49152, true),
            destination("127.0.0.1", 21, true),
            destination("127.0.0.1", 49152, false),
        ] {
            assert!(endpoint(&candidate).is_err());
        }
    }
    #[test]
    fn input_cannot_select_source_paths_or_unknown_settings() {
        let input = br#"{"destination":{"kind":"local","selection_id":"ignored"},"repositoryPassword":"generated","transportPassword":null,"sourcePath":"/outside"}"#;
        assert!(serde_json::from_slice::<Input>(input).is_err());
    }
}
fn main() {
    #[cfg(target_os = "macos")]
    if let Some(code) = stillharbour_core::backup::rclone_worker::dispatch() {
        std::process::exit(code);
    }
    #[cfg(all(target_os = "macos", feature = "fixture-probes"))]
    if let Some(code) = stillharbour_core::backup::rclone_worker::guard_probe_dispatch() {
        std::process::exit(code);
    }
    match run() {
        Ok(receipt) => println!("{receipt}"),
        Err(error) => {
            // Neither helper output, endpoint settings, passwords nor private paths are printed.
            println!(
                "{}",
                json!({"passed":false,"stage":error.stage,"errorCode":error.code,"job":error.job})
            );
            std::process::exit(1);
        }
    }
}

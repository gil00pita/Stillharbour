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
        snapshots::NativeSnapshotSelection,
    },
};
use zeroize::{Zeroize, Zeroizing};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Input {
    destination: SnapshotDestinationInput,
    repository_password: String,
    transport_password: Option<String>,
}
impl Drop for Input {
    fn drop(&mut self) {
        self.repository_password.zeroize();
        if let Some(secret) = &mut self.transport_password {
            secret.zeroize();
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
    }
}
fn app<T>(stage: &'static str, value: stillharbour_core::domain::Result<T>) -> Result<T> {
    value.map_err(|error| Failure {
        stage,
        code: error.code,
    })
}
fn io<T>(stage: &'static str, value: std::io::Result<T>) -> Result<T> {
    value.map_err(|_| Failure {
        stage,
        code: ErrorCode::Io,
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
fn wait(library: &Arc<Library>, initial: SnapshotJob) -> Result<SnapshotJob> {
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
            require("completed-native-job", job.state == "completed_verified")?;
            require(
                "earned-evidence",
                job.included
                    && job.structural_checked
                    && job.data_read_verified
                    && job.restored_compared
                    && job.source_retained
                    && !job.independent_recovery_verified,
            )?;
            return Ok(job);
        }
        require("job-deadline", Instant::now() < until)?;
        std::thread::sleep(Duration::from_millis(50));
    }
}
fn execute(library: &Arc<Library>, review: SnapshotReview) -> Result<SnapshotJob> {
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
    let SnapshotReply::Job(job) = call(library, request)? else {
        return Err(fail("start-reply"));
    };
    wait(library, job)
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
                json!({"passed":false,"stage":error.stage,"errorCode":error.code})
            );
            std::process::exit(1);
        }
    }
}

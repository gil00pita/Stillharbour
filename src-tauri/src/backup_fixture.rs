//! Instrumented-build-only owned application fixture. Never accepts paths/secrets.
use super::*;
use std::collections::BTreeMap;
use stillharbour_core::backup::credentials::{CredentialPurpose, CredentialStore};

pub(super) struct ApplicationFixture {
    source: tempfile::TempDir,
    repository: tempfile::TempDir,
    restore: tempfile::TempDir,
    baseline: BTreeMap<String, Vec<u8>>,
    catalog_id: String,
    credential_ref: String,
    root_id: String,
    removed: bool,
}
impl Drop for ApplicationFixture {
    fn drop(&mut self) {
        // Only the exact generated catalog/purpose/ref is deleted. No listing,
        // wildcard lookup or real application credential can enter this scope.
        let _ = CredentialStore::delete_owned(
            &self.catalog_id,
            CredentialPurpose::Repository,
            &self.credential_ref,
        );
    }
}
fn fail(message: &str) -> AppError {
    AppError::new(ErrorCode::Conflict, message)
}
impl ApplicationFixture {
    fn new(state: &Foundation) -> Result<Self> {
        let source = tempfile::Builder::new()
            .prefix("stillharbour-m4-app-source-")
            .tempdir()?;
        let repository = tempfile::Builder::new()
            .prefix("stillharbour-m4-app-repository-")
            .tempdir()?;
        let restore = tempfile::Builder::new()
            .prefix("stillharbour-m4-app-restore-")
            .tempdir()?;
        let baseline = BTreeMap::from([
            (
                "photo.bin".into(),
                b"generated M4 original photo bytes".to_vec(),
            ),
            (
                "photo.xmp".into(),
                b"<x:xmpmeta>generated companion</x:xmpmeta>".to_vec(),
            ),
            ("nested/movie.bin".into(), vec![0x73; 131_073]),
            (
                ".hidden-note".into(),
                b"hidden generated file must survive".to_vec(),
            ),
        ]);
        std::fs::create_dir(source.path().join("nested"))?;
        for (relative, bytes) in &baseline {
            std::fs::write(source.path().join(relative), bytes)?;
        }
        std::fs::write(
            repository.path().join("destination-only.txt"),
            b"retain repository neighbor",
        )?;
        std::fs::write(
            restore.path().join("restore-neighbor.txt"),
            b"retain restore neighbor",
        )?;
        let catalog_id = state._catalog.summary()?.id;
        let handle = filesystem::open_directory_readonly(source.path())?;
        let job = state.library.register_verified(
            source.path().to_owned(),
            (
                filesystem::identity(&handle)?,
                stillharbour_core::library::native::volume_token(&handle)?,
            ),
            "Generated M4 application scope".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        )?;
        let secret = zeroize::Zeroizing::new(format!("generated-fixture-{}", uuid::Uuid::new_v4()));
        let credential_ref =
            CredentialStore::store(&catalog_id, CredentialPurpose::Repository, &secret)?;
        Ok(Self {
            source,
            repository,
            restore,
            baseline,
            catalog_id,
            credential_ref,
            root_id: job.root_id,
            removed: false,
        })
    }
    fn check_source(&self) -> Result<()> {
        if self.removed {
            return Err(fail("The generated source has already been removed."));
        }
        for (relative, expected) in &self.baseline {
            if std::fs::read(self.source.path().join(relative))? != *expected {
                return Err(fail("The generated original changed unexpectedly."));
            }
        }
        Ok(())
    }
    fn audit(&self) -> Result<serde_json::Value> {
        if !self.removed {
            self.check_source()?;
        }
        if std::fs::read(self.repository.path().join("destination-only.txt"))?
            != b"retain repository neighbor"
            || std::fs::read(self.restore.path().join("restore-neighbor.txt"))?
                != b"retain restore neighbor"
        {
            return Err(fail("A generated destination-only neighbor changed."));
        }
        fn inventory(path: &std::path::Path, depth: usize, out: &mut Vec<Vec<u8>>) -> Result<()> {
            if depth > 8 {
                return Err(fail("Generated restored output exceeded its depth bound."));
            }
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let kind = entry.file_type()?;
                if kind.is_symlink() {
                    return Err(fail("A restored generated output is a link."));
                }
                if kind.is_dir() {
                    inventory(&entry.path(), depth + 1, out)?;
                } else if kind.is_file() {
                    if out.len() >= 100 || entry.metadata()?.len() > 32 * 1024 * 1024 {
                        return Err(fail("Restored fixture exceeded its bounded scope."));
                    }
                    out.push(std::fs::read(entry.path())?);
                } else {
                    return Err(fail("Unexpected restored special object."));
                }
            }
            Ok(())
        }
        let mut restored = Vec::new();
        inventory(self.restore.path(), 0, &mut restored)?;
        let compared = self
            .baseline
            .values()
            .all(|expected| restored.iter().any(|actual| actual == expected));
        let catalog_header = restored
            .iter()
            .any(|bytes| bytes.starts_with(b"SQLite format 3\0"));
        if !compared || !catalog_header {
            return Err(fail(
                "The actual application restore did not recover every generated file and catalog.",
            ));
        }
        Ok(
            serde_json::json!({"scope":"generated-temporary-only","nativeOs":std::env::consts::OS,"sourceUnchangedBeforeRemoval":true,"sourceRemoved":self.removed,"destinationOnlyPreserved":true,"restoredCompared":compared,"restoredFilesCompared":self.baseline.len(),"catalogHeaderFound":catalog_header,"onlyFixtureWrites":true}),
        )
    }
}
pub(super) fn handle(state: &Foundation, action: &str) -> Result<serde_json::Value> {
    let mut guard = state
        .backup_application_fixture
        .lock()
        .map_err(|_| fail("Generated application fixture is unavailable."))?;
    if guard.is_none() {
        *guard = Some(ApplicationFixture::new(state)?);
    }
    let fixture = guard
        .as_mut()
        .ok_or_else(|| fail("Generate the owned application fixture first."))?;
    match action {
        "m4_application_catalog_audit" => {
            let prepared = state
                .catalog_restore_receipt
                .lock()
                .map_err(|_| fail("Generated catalog receipt is unavailable."))?
                .clone()
                .ok_or_else(|| fail("No actual native catalog preparation completed."))?;
            if prepared.catalog_id != fixture.catalog_id {
                return Err(fail(
                    "Catalog preparation belongs to another generated catalog.",
                ));
            }
            uuid::Uuid::parse_str(&prepared.transaction_id)
                .map_err(|_| fail("Invalid generated transaction."))?;
            let transaction = state
                .appdata
                .join("catalog-restores")
                .join(&prepared.transaction_id);
            if prepared.pre_restore.native_path()? != transaction.join("pre-restore.stillharbour")
                || prepared.candidate.native_path()? != transaction.join("candidate.stillharbour")
            {
                return Err(fail(
                    "Generated catalog artifacts escaped their exact transaction.",
                ));
            }
            state._catalog_selection.revalidate()?;
            let current = stillharbour_core::catalog_restore::capture_current(
                &state._catalog_selection.path,
            )?;
            if current.identity != prepared.current.identity
                || current.volume != prepared.current.volume
                || current.path != prepared.current.path
            {
                return Err(fail("The original generated catalog identity changed."));
            }
            let schema = stillharbour_core::catalog::current_schema_fingerprint()?;
            stillharbour_core::catalog_restore::validate_frozen(&prepared.pre_restore)?;
            stillharbour_core::catalog_restore::validate_frozen(&prepared.candidate)?;
            let retained = stillharbour_core::catalog_restore::validate_catalog(
                &prepared.pre_restore.native_path()?,
                &fixture.catalog_id,
                &schema,
            )?;
            let candidate = stillharbour_core::catalog_restore::validate_catalog(
                &prepared.candidate.native_path()?,
                &fixture.catalog_id,
                &schema,
            )?;
            if retained.revision != prepared.current_catalog_revision
                || candidate.revision != prepared.candidate_catalog_revision
                || !retained
                    .roots
                    .iter()
                    .any(|root| root.root_id == fixture.root_id)
                || !candidate
                    .roots
                    .iter()
                    .any(|root| root.root_id == fixture.root_id)
            {
                return Err(fail(
                    "Generated retained/candidate catalog validation failed.",
                ));
            }
            Ok(
                serde_json::json!({"transactionId":prepared.transaction_id,"catalogId":fixture.catalog_id,"pendingActivation":true,"activationClaimed":false,"currentCatalogRetained":true,"preRestoreVerified":true,"candidateVerified":true,"fixturePickerOnly":true,"sourceAudit":fixture.audit()?}),
            )
        }
        "m4_application_audit" => fixture.audit(),
        "m4_application_remove_source" => {
            fixture.check_source()?;
            for relative in fixture.baseline.keys() {
                let target = fixture.source.path().join(relative);
                let resolved = target.canonicalize()?;
                if !resolved.starts_with(fixture.source.path().canonicalize()?)
                    || !resolved.is_file()
                {
                    return Err(fail("Generated-source cleanup escaped its exact scope."));
                }
                std::fs::remove_file(&target)?;
            }
            fixture.removed = true;
            Ok(serde_json::json!({"sourceRemoved":true,"sourceUnchangedBeforeRemoval":true}))
        }
        "m4_application_source"
        | "m4_application_repository"
        | "m4_application_restore"
        | "m4_application_catalog_root_picker" => {
            let path = match action {
                "m4_application_source" | "m4_application_catalog_root_picker" => {
                    fixture.source.path()
                }
                "m4_application_repository" => fixture.repository.path(),
                _ => fixture.restore.path(),
            }
            .canonicalize()?;
            let file = filesystem::open_directory_readonly(&path)?;
            let selection_id = uuid::Uuid::new_v4().to_string();
            let native = NativeSelection {
                id: selection_id.clone(),
                path,
                identity: Some(filesystem::identity(&file)?),
                volume: Some(stillharbour_core::library::native::volume_token(&file)?),
            };
            if action == "m4_application_catalog_root_picker" {
                *state
                    .catalog_restore_picker
                    .lock()
                    .map_err(|_| fail("Generated picker state is unavailable."))? =
                    Some(native.clone());
            }
            *state
                .selection
                .lock()
                .map_err(|_| fail("Fixture picker selection is unavailable."))? = Some(native);
            Ok(
                serde_json::json!({"selectionId":selection_id,"rootId":fixture.root_id,"catalogId":fixture.catalog_id,"credentialRef":fixture.credential_ref}),
            )
        }
        _ => Err(fail("Unknown owned application fixture action.")),
    }
}

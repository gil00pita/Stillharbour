//! Instrumented-build-only owned application fixture. Never accepts paths/secrets.
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use stillharbour_core::backup::credentials::{CredentialPurpose, CredentialStore};
use stillharbour_core::catalog_restore::{
    CurrentCatalogBinding, PreparedCatalogRestore,
    activation::{CatalogFixtureRole, CatalogFixtureSession},
};
enum FixtureDirectory {
    Temporary(tempfile::TempDir),
    Persistent(PathBuf),
}
impl FixtureDirectory {
    fn path(&self) -> &std::path::Path {
        match self {
            Self::Temporary(v) => v.path(),
            Self::Persistent(v) => v.as_path(),
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoleIdentity {
    identity: FileIdentity,
    volume: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PersistentState {
    version: u32,
    owner_nonce: String,
    catalog_id: String,
    root_id: String,
    credential_ref: String,
    original_catalog: CurrentCatalogBinding,
    appdata: RoleIdentity,
    source: RoleIdentity,
    repository: RoleIdentity,
    restore: RoleIdentity,
    source_files: BTreeMap<String, FileIdentity>,
    repository_neighbor: FileIdentity,
    restore_neighbor: FileIdentity,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OwnedMarker {
    version: u32,
    owner_nonce: String,
}
struct CreatedCredential {
    catalog: String,
    reference: String,
    armed: bool,
}
impl Drop for CreatedCredential {
    fn drop(&mut self) {
        if self.armed {
            let _ = CredentialStore::delete_owned(
                &self.catalog,
                CredentialPurpose::Repository,
                &self.reference,
            );
        }
    }
}

pub(super) struct ApplicationFixture {
    source: FixtureDirectory,
    repository: FixtureDirectory,
    restore: FixtureDirectory,
    baseline: BTreeMap<String, Vec<u8>>,
    catalog_id: String,
    credential_ref: String,
    root_id: String,
    removed: bool,
    persistent: Option<(Arc<CatalogFixtureSession>, PersistentState)>,
}
impl Drop for ApplicationFixture {
    fn drop(&mut self) {
        if self.persistent.is_some() {
            return;
        }
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
fn baseline() -> BTreeMap<String, Vec<u8>> {
    BTreeMap::from([
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
    ])
}
fn json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value)
        .map_err(|e| fail("Generated session metadata could not be encoded.").caused_by(e))
}
fn read_session(session: &CatalogFixtureSession, name: &str) -> Result<Option<Vec<u8>>> {
    use std::io::Read;
    session.revalidate()?;
    let path = session.path().join(name);
    match std::fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
        Ok(m) if !m.is_file() => {
            return Err(fail("Generated session metadata is not an ordinary file."));
        }
        Ok(_) => (),
    }
    let file = filesystem::open_readonly(&path)?;
    let id = filesystem::identity(&file)?;
    let revision = stillharbour_core::library::native::revision(&file)?;
    if id.links != 1 || file.metadata()?.len() > 65536 {
        return Err(fail("Generated session metadata exceeds its native bound."));
    }
    let mut bytes = Vec::new();
    file.take(65537).read_to_end(&mut bytes)?;
    let current = filesystem::open_readonly(&path)?;
    if bytes.len() > 65536
        || filesystem::identity(&current)? != id
        || stillharbour_core::library::native::revision(&current)? != revision
    {
        return Err(fail("Generated session metadata changed."));
    }
    session.revalidate()?;
    Ok(Some(bytes))
}
fn write_session(session: &CatalogFixtureSession, name: &str, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    session.revalidate()?;
    if bytes.len() > 65536 {
        return Err(fail("Generated session metadata exceeds its bound."));
    }
    if let Some(existing) = read_session(session, name)? {
        if existing == bytes {
            return Ok(());
        }
        return Err(fail(
            "Existing generated session metadata is never overwritten.",
        ));
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(session.path().join(name))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    if filesystem::identity(&file)?.links != 1 {
        return Err(fail("Generated session metadata has another native link."));
    }
    session.revalidate()
}
fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    serde_json::from_slice(bytes)
        .map_err(|e| fail("Generated session metadata is invalid.").caused_by(e))
}
fn same_directory(a: &FileIdentity, b: &FileIdentity) -> bool {
    a.volume == b.volume && a.file == b.file
}
fn role(session: &CatalogFixtureSession, kind: CatalogFixtureRole) -> Result<RoleIdentity> {
    let (identity, volume) = session.role_binding(kind)?;
    Ok(RoleIdentity { identity, volume })
}
fn removed(session: &CatalogFixtureSession) -> Result<bool> {
    match read_session(session, "source-removed.json")? {
        None => Ok(false),
        Some(bytes) => {
            let marker: OwnedMarker = decode(&bytes)?;
            if marker.version != 1 || marker.owner_nonce != session.owner() {
                return Err(fail("Generated removal ownership changed."));
            }
            Ok(true)
        }
    }
}
fn identity_json(value: &FileIdentity) -> serde_json::Value {
    serde_json::json!({"volume":value.volume.to_string(),"file":value.file.to_string(),"links":value.links.to_string()})
}
impl ApplicationFixture {
    fn persistent(state: &Foundation, session: Arc<CatalogFixtureSession>) -> Result<Self> {
        session.revalidate()?;
        let baseline = baseline();
        let record = if let Some(bytes) = read_session(&session, "catalog-fixture.json")? {
            decode::<PersistentState>(&bytes)?
        } else {
            if !state._catalog_selection.creation_allowed
                || state._catalog_selection.prepared.is_some()
            {
                return Err(fail(
                    "Persistent fixture initialization requires the actual first new catalog seed.",
                ));
            }
            let source = session.create_role(CatalogFixtureRole::Source)?;
            let repository = session.create_role(CatalogFixtureRole::Repository)?;
            let restore = session.create_role(CatalogFixtureRole::Restore)?;
            std::fs::create_dir(source.join("nested"))?;
            for (relative, bytes) in &baseline {
                std::fs::write(source.join(relative), bytes)?;
            }
            std::fs::write(
                repository.join("destination-only.txt"),
                b"retain repository neighbor",
            )?;
            std::fs::write(
                restore.join("restore-neighbor.txt"),
                b"retain restore neighbor",
            )?;
            let catalog_id = state._catalog.summary()?.id;
            let handle = filesystem::open_directory_readonly(&source)?;
            let job = state.library.register_verified(
                source.clone(),
                (
                    filesystem::identity(&handle)?,
                    stillharbour_core::library::native::volume_token(&handle)?,
                ),
                "Generated M4 application scope".into(),
                RootMode::Referenced,
                ScanFilter::default(),
            )?;
            let secret =
                zeroize::Zeroizing::new(format!("generated-fixture-{}", uuid::Uuid::new_v4()));
            let credential_ref =
                CredentialStore::store(&catalog_id, CredentialPurpose::Repository, &secret)?;
            let mut created = CreatedCredential {
                catalog: catalog_id.clone(),
                reference: credential_ref.clone(),
                armed: true,
            };
            let mut source_files = BTreeMap::new();
            for relative in baseline.keys() {
                source_files.insert(
                    relative.clone(),
                    filesystem::identity(&filesystem::open_readonly(&source.join(relative))?)?,
                );
            }
            let record = PersistentState {
                version: 1,
                owner_nonce: session.owner().into(),
                catalog_id: catalog_id.clone(),
                root_id: job.root_id,
                credential_ref: credential_ref.clone(),
                original_catalog: stillharbour_core::catalog_restore::capture_current(
                    &state._catalog_selection.path,
                )?,
                appdata: role(&session, CatalogFixtureRole::Appdata)?,
                source: role(&session, CatalogFixtureRole::Source)?,
                repository: role(&session, CatalogFixtureRole::Repository)?,
                restore: role(&session, CatalogFixtureRole::Restore)?,
                source_files,
                repository_neighbor: filesystem::identity(&filesystem::open_readonly(
                    &repository.join("destination-only.txt"),
                )?)?,
                restore_neighbor: filesystem::identity(&filesystem::open_readonly(
                    &restore.join("restore-neighbor.txt"),
                )?)?,
            };
            write_session(&session, "catalog-fixture.json", &json(&record)?)?;
            created.armed = false;
            record
        };
        if record.version != 1
            || record.owner_nonce != session.owner()
            || record.catalog_id != state._catalog.summary()?.id
            || uuid::Uuid::parse_str(&record.credential_ref).is_err()
            || uuid::Uuid::parse_str(&record.root_id).is_err()
            || record.source_files.len() != baseline.len()
        {
            return Err(fail("Persistent generated catalog/ref ownership changed."));
        }
        let fixture = Self {
            source: FixtureDirectory::Persistent(session.role_path(CatalogFixtureRole::Source)),
            repository: FixtureDirectory::Persistent(
                session.role_path(CatalogFixtureRole::Repository),
            ),
            restore: FixtureDirectory::Persistent(session.role_path(CatalogFixtureRole::Restore)),
            baseline,
            catalog_id: record.catalog_id.clone(),
            credential_ref: record.credential_ref.clone(),
            root_id: record.root_id.clone(),
            removed: removed(&session)?,
            persistent: Some((session, record)),
        };
        fixture.verify_persistent(state)?;
        Ok(fixture)
    }
    fn verify_persistent(&self, state: &Foundation) -> Result<()> {
        let (session, saved) = self
            .persistent
            .as_ref()
            .ok_or_else(|| fail("This action requires the owned persistent native session."))?;
        session.revalidate()?;
        for (kind, expected) in [
            (CatalogFixtureRole::Appdata, &saved.appdata),
            (CatalogFixtureRole::Source, &saved.source),
            (CatalogFixtureRole::Repository, &saved.repository),
            (CatalogFixtureRole::Restore, &saved.restore),
        ] {
            let actual = role(session, kind)?;
            if !same_directory(&actual.identity, &expected.identity)
                || actual.volume != expected.volume
            {
                return Err(fail("A persistent fixed-role native directory changed."));
            }
        }
        if state._catalog.summary()?.id != saved.catalog_id {
            return Err(fail("The persistent catalog UUID changed."));
        }
        let root = state
            ._catalog
            .fixture_catalog_recovery_root_binding(&self.root_id)?;
        if root.path != stillharbour_core::library::native::encode_path(self.source.path())
            || !same_directory(&root.identity, &saved.source.identity)
            || root.volume != saved.source.volume
        {
            return Err(fail(
                "The active generated catalog root no longer maps to the fixed source native object.",
            ));
        }
        if stillharbour_core::library::native::decode_path(&saved.original_catalog.path)?
            != session
                .role_path(CatalogFixtureRole::Appdata)
                .join("Foundation.stillharbour")
        {
            return Err(fail(
                "The original generated catalog escaped its fixed namespace.",
            ));
        }
        let original = stillharbour_core::catalog_restore::capture_current(
            &stillharbour_core::library::native::decode_path(&saved.original_catalog.path)?,
        )?;
        if original.identity != saved.original_catalog.identity
            || original.volume != saved.original_catalog.volume
        {
            return Err(fail(
                "The original generated catalog native object changed.",
            ));
        }
        for (path, expected) in [
            (
                self.repository.path().join("destination-only.txt"),
                &saved.repository_neighbor,
            ),
            (
                self.restore.path().join("restore-neighbor.txt"),
                &saved.restore_neighbor,
            ),
        ] {
            if filesystem::identity(&filesystem::open_readonly(&path)?)? != *expected {
                return Err(fail("A persistent neighbor native object changed."));
            }
        }
        if self.removed {
            for relative in self.baseline.keys() {
                match std::fs::symlink_metadata(self.source.path().join(relative)) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    _ => {
                        return Err(fail(
                            "An intentionally removed generated source was rewritten.",
                        ));
                    }
                }
            }
        } else {
            self.check_source()?;
        }
        if removed(session)? != self.removed {
            return Err(fail("Persistent removal state changed."));
        }
        session.revalidate()
    }
    fn new(state: &Foundation) -> Result<Self> {
        if let Some(session) = state._test_data.session() {
            return Self::persistent(state, session.clone());
        }
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
            source: FixtureDirectory::Temporary(source),
            repository: FixtureDirectory::Temporary(repository),
            restore: FixtureDirectory::Temporary(restore),
            baseline,
            catalog_id,
            credential_ref,
            root_id: job.root_id,
            removed: false,
            persistent: None,
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
            if let Some((_, saved)) = &self.persistent {
                let recorded = saved
                    .source_files
                    .get(relative)
                    .ok_or_else(|| fail("Generated source native identity is missing."))?;
                if recorded.links != 1
                    || filesystem::identity(&filesystem::open_readonly(
                        &self.source.path().join(relative),
                    )?)? != *recorded
                {
                    return Err(fail("A generated original/sidecar native object changed."));
                }
            }
        }
        Ok(())
    }
    fn receipt(&self, name: &str) -> Result<Option<PreparedCatalogRestore>> {
        let (session, _) = self
            .persistent
            .as_ref()
            .ok_or_else(|| fail("Persistent session required."))?;
        let Some(bytes) = read_session(session, name)? else {
            return Ok(None);
        };
        let prepared: PreparedCatalogRestore = decode(&bytes)?;
        prepared.validate_provenance()?;
        let directory = session
            .role_path(CatalogFixtureRole::Appdata)
            .join("catalog-restores")
            .join(&prepared.transaction_id);
        if prepared.catalog_id != self.catalog_id
            || prepared.candidate.native_path()? != directory.join("candidate.stillharbour")
            || prepared.pre_restore.native_path()? != directory.join("pre-restore.stillharbour")
        {
            return Err(fail(
                "Saved native preparation escaped the fixed session layout.",
            ));
        }
        Ok(Some(prepared))
    }
    fn save_receipt(&self, prepared: &PreparedCatalogRestore) -> Result<()> {
        let Some((session, saved)) = &self.persistent else {
            return Ok(());
        };
        match self.receipt("catalog-prepared-first.json")? {
            None => {
                if prepared.current.path != saved.original_catalog.path
                    || prepared.current.identity != saved.original_catalog.identity
                {
                    return Err(fail(
                        "The first preparation did not preserve the initial current catalog.",
                    ));
                }
                write_session(session, "catalog-prepared-first.json", &json(prepared)?)
            }
            Some(first) if first.transaction_id == prepared.transaction_id => {
                write_session(session, "catalog-prepared-first.json", &json(prepared)?)
            }
            Some(first) => {
                if prepared.source_kind!=stillharbour_core::backup::snapshots::CatalogRestoreSourceKind::RetainedPreRestore||prepared.retained_transaction_id.as_deref()!=Some(first.transaction_id.as_str())||prepared.current.path!=first.candidate.path||prepared.current.identity!=first.candidate.identity{return Err(fail("Return preparation was not a new forward transaction from the active native catalog."));}
                write_session(session, "catalog-prepared-second.json", &json(prepared)?)
            }
        }
    }
    fn active_audit(&self, state: &Foundation) -> Result<serde_json::Value> {
        self.verify_persistent(state)?;
        let first = self
            .receipt("catalog-prepared-first.json")?
            .ok_or_else(|| fail("First native preparation receipt is missing."))?;
        let second = self.receipt("catalog-prepared-second.json")?;
        let active = state._catalog_selection.prepared.as_ref().ok_or_else(|| {
            fail("This launch did not select a prepared catalog through native startup.")
        })?;
        let expected = if active.transaction_id == first.transaction_id {
            &first
        } else {
            second
                .as_ref()
                .filter(|p| p.transaction_id == active.transaction_id)
                .ok_or_else(|| {
                    fail("Active native transaction is not one of this session's prepared outputs.")
                })?
        };
        if json(active)? != json(expected)?
            || state._catalog_selection.path != active.candidate.native_path()?
            || state._catalog_selection.identity != active.candidate.identity
        {
            return Err(fail(
                "The running catalog did not come from the actual sealed startup candidate.",
            ));
        }
        state._catalog_selection.revalidate()?;
        let label = state
            ._catalog
            .fixture_catalog_recovery_label(&self.root_id)?;
        let proof = state
            .activation
            .latest_retained_source(&active.transaction_id)?;
        let retained_label = stillharbour_core::catalog_restore::fixture_catalog_label(
            proof.source(),
            &self.catalog_id,
            &self.root_id,
        )?;
        let source = proof.source();
        if source.identity != active.pre_restore.identity
            || source.digest != active.pre_restore.digest
        {
            return Err(fail(
                "Latest retained source differs from the authenticated pre-restore copy.",
            ));
        }
        stillharbour_core::catalog_restore::validate_frozen(&first.pre_restore)?;
        let first_candidate =
            stillharbour_core::catalog_restore::capture_current(&first.candidate.native_path()?)?;
        if first_candidate.identity != first.candidate.identity
            || first_candidate.volume != first.candidate.volume
        {
            return Err(fail("A prior generated catalog native object disappeared."));
        }
        if let Some(second) = &second {
            stillharbour_core::catalog_restore::validate_frozen(&second.pre_restore)?;
        }
        let phase = if second
            .as_ref()
            .is_some_and(|p| p.transaction_id == active.transaction_id)
        {
            3
        } else {
            2
        };
        let source_audit = self.audit()?;
        Ok(
            serde_json::json!({"nativePid":std::process::id(),"phase":phase,"catalogId":self.catalog_id,"rootId":self.root_id,"activeCatalogIdentity":identity_json(&state._catalog_selection.identity),"activeTransactionId":active.transaction_id,"latestRetainedTransactionId":state.activation.latest_retained_transaction_id()?,"firstEditActive":label=="Generated first newer catalog edit","secondEditActive":label=="Generated second newer catalog edit","firstEditRetained":retained_label=="Generated first newer catalog edit","secondEditRetained":retained_label=="Generated second newer catalog edit","preRestoreVerified":true,"candidateNativeVerified":true,"priorCatalogsRetained":true,"retainedPriorCatalogCount":if second.is_some(){4}else{2},"repositoryCredentialUnavailable":CredentialStore::fixture_owned_absent(&self.catalog_id,CredentialPurpose::Repository,&self.credential_ref)?,"sourceAudit":source_audit}),
        )
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
    if fixture.persistent.is_some() {
        fixture.verify_persistent(state)?;
    }
    match action {
        "m4_application_catalog_newer_edit" | "m4_application_catalog_second_edit" => {
            fixture.verify_persistent(state)?;
            let second = action == "m4_application_catalog_second_edit";
            if second {
                let first = fixture
                    .receipt("catalog-prepared-first.json")?
                    .ok_or_else(|| {
                        fail(
                            "The real first preparation is required before the second launch edit.",
                        )
                    })?;
                if state
                    ._catalog_selection
                    .prepared
                    .as_ref()
                    .map(|p| p.transaction_id.as_str())
                    != Some(first.transaction_id.as_str())
                    || fixture.receipt("catalog-prepared-second.json")?.is_some()
                {
                    return Err(fail(
                        "The second edit requires the actual first candidate startup, before return preparation.",
                    ));
                }
            } else if state._catalog_selection.prepared.is_some()
                || fixture.receipt("catalog-prepared-first.json")?.is_some()
            {
                return Err(fail(
                    "The first edit requires the original first native launch.",
                ));
            }
            state
                ._catalog
                .fixture_catalog_recovery_edit(&fixture.root_id, second)?;
            if second {
                CredentialStore::delete_owned(
                    &fixture.catalog_id,
                    CredentialPurpose::Repository,
                    &fixture.credential_ref,
                )?;
                if !CredentialStore::fixture_owned_absent(
                    &fixture.catalog_id,
                    CredentialPurpose::Repository,
                    &fixture.credential_ref,
                )? {
                    return Err(fail("The generated repository credential was not removed."));
                }
            }
            let label = state
                ._catalog
                .fixture_catalog_recovery_label(&fixture.root_id)?;
            Ok(
                serde_json::json!({"nativePid":std::process::id(),"catalogId":fixture.catalog_id,"rootId":fixture.root_id,"firstEditApplied":!second&&label=="Generated first newer catalog edit","secondEditApplied":second&&label=="Generated second newer catalog edit","repositoryCredentialUnavailable":second}),
            )
        }
        "m4_application_catalog_active_audit" => fixture.active_audit(state),
        "m4_application_catalog_session_cleanup" => {
            let audit = fixture.active_audit(state)?;
            if audit.get("phase").and_then(serde_json::Value::as_u64) != Some(3)
                || audit.get("firstEditActive") != Some(&serde_json::Value::Bool(true))
                || audit.get("secondEditActive") != Some(&serde_json::Value::Bool(false))
                || audit.get("secondEditRetained") != Some(&serde_json::Value::Bool(true))
            {
                return Err(fail(
                    "Final native catalog session proof has not completed.",
                ));
            }
            let (session, _) = fixture
                .persistent
                .as_ref()
                .ok_or_else(|| fail("Persistent native session required."))?;
            CredentialStore::delete_owned(
                &fixture.catalog_id,
                CredentialPurpose::Repository,
                &fixture.credential_ref,
            )?;
            if !CredentialStore::fixture_owned_absent(
                &fixture.catalog_id,
                CredentialPurpose::Repository,
                &fixture.credential_ref,
            )? {
                return Err(fail("Generated repository credential cleanup failed."));
            }
            session.cleanup_authority(&state.activation)?;
            write_session(
                session,
                "session-cleaned.json",
                &json(&OwnedMarker {
                    version: 1,
                    owner_nonce: session.owner().into(),
                })?,
            )?;
            Ok(
                serde_json::json!({"nativePid":std::process::id(),"repositoryCredentialRemoved":true,"activationAuthorityRemoved":true,"sessionScopeVerified":true,"audit":audit}),
            )
        }
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
            fixture.save_receipt(&prepared)?;
            Ok(
                serde_json::json!({"nativePid":std::process::id(),"transactionId":prepared.transaction_id,"catalogId":fixture.catalog_id,"activeCatalogIdentity":identity_json(&current.identity),"pendingActivation":true,"activationClaimed":false,"currentCatalogRetained":true,"preRestoreVerified":true,"candidateVerified":true,"fixturePickerOnly":true,"sourceAudit":fixture.audit()?}),
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
            if let Some((session, _)) = &fixture.persistent {
                write_session(
                    session,
                    "source-removed.json",
                    &json(&OwnedMarker {
                        version: 1,
                        owner_nonce: session.owner().into(),
                    })?,
                )?;
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

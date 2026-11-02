//! Generated activation-layer tests, distinct from actual backup/prepare proof.
#![cfg(any(windows, target_os = "macos"))]
use super::*;
use crate::catalog::Catalog;
use std::{
    process::{Child, Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

struct Fixture {
    catalog: Option<Arc<Catalog>>,
    selection: Option<StartupSelection>,
    store: Option<ActivationStore>,
    _authority: OwnedAuthority,
    owned: tempfile::TempDir,
    nonce: String,
}
struct OwnedAuthority(String);
impl Drop for OwnedAuthority {
    fn drop(&mut self) {
        let result = authority::delete_owned_fixture(&self.0);
        if !std::thread::panicking() {
            result.unwrap();
        }
    }
}
impl Fixture {
    fn new() -> Self {
        let nonce = uuid::Uuid::new_v4().to_string();
        let owned = tempfile::Builder::new()
            .prefix(&format!("stillharbour-activation-{nonce}-"))
            .tempdir()
            .unwrap();
        let path = owned.path().canonicalize().unwrap();
        fs::write(path.join("fixture-owner"), &nonce).unwrap();
        let store = ActivationStore::open(&path).unwrap();
        let scope = store.scope.clone();
        let authority = OwnedAuthority(scope);
        let selection = store.startup_selection().unwrap();
        assert!(selection.creation_allowed);
        let catalog = Catalog::open_bound_seed(
            selection.path.clone(),
            selection.identity.clone(),
            selection.volume.clone(),
        )
        .unwrap();
        store.confirm_initialized(&selection).unwrap();
        let source = path.join("generated-source");
        fs::create_dir(&source).unwrap();
        fs::write(
            source.join("original.bin"),
            b"original bytes are never activated or rewritten",
        )
        .unwrap();
        let held = DirectoryChain::open(&source).unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let native_path = native::encode_path(&source);
        let identity = held.identity().clone();
        let volume = native::volume_token(held.leaf()).unwrap();
        catalog.call(move|db|{db.execute("INSERT INTO roots(id,label,native_path,display_path,mode,is_base,platform,volume,device,inode,availability,filter) VALUES(?1,'Generated',?2,'Generated','referenced',0,?6,?3,?4,?5,'offline','{}')",rusqlite::params![id,native_path,volume,identity.volume.to_string(),identity.file.to_string(),std::env::consts::OS])?;Ok(())}).unwrap();
        Self {
            catalog: Some(Arc::new(catalog)),
            selection: Some(selection),
            store: Some(store),
            _authority: authority,
            owned,
            nonce,
        }
    }
    fn path(&self) -> PathBuf {
        self.owned.path().canonicalize().unwrap()
    }
    fn prepare(&self) -> PreparedCatalogRestore {
        let catalog = self.catalog.as_ref().unwrap();
        let store = self.store.as_ref().unwrap();
        let transaction_id = uuid::Uuid::new_v4().to_string();
        let directory = bridge::make_directory(&store.restores, &transaction_id).unwrap();
        let current = super::binding(&self.path().join(DEFAULT_CATALOG)).unwrap();
        catalog
            .snapshot(directory.path.join("pre-restore.stillharbour"))
            .unwrap();
        catalog
            .snapshot(directory.path.join("candidate.stillharbour"))
            .unwrap();
        let catalog_id = catalog.summary().unwrap().id;
        let validated = super::super::validate_catalog(
            &directory.path.join("candidate.stillharbour"),
            &catalog_id,
            &crate::catalog::current_schema_fingerprint().unwrap(),
        )
        .unwrap();
        let capture = |path: PathBuf| {
            let mut file = filesystem::open_readonly(&path).unwrap();
            CatalogFileBinding {
                path: native::encode_path(&path),
                identity: filesystem::identity(&file).unwrap(),
                volume: native::volume_token(&file).unwrap(),
                bytes: file.metadata().unwrap().len(),
                digest: bridge::digest(&mut file, &AtomicBool::new(false)).unwrap(),
            }
        };
        let prepared = PreparedCatalogRestore {
            version: 1,
            transaction_id,
            catalog_id,
            source_restore_job_id: uuid::Uuid::new_v4().to_string(),
            source_snapshot_id: "a".repeat(64),
            source_catalog_revision: validated.revision.clone(),
            candidate_catalog_revision: validated.revision.clone(),
            current_catalog_revision: validated.revision,
            schema: crate::catalog::SCHEMA_VERSION,
            current,
            pre_restore: capture(directory.path.join("pre-restore.stillharbour")),
            candidate: capture(directory.path.join("candidate.stillharbour")),
            roots: validated.roots,
        };
        bridge::write_new(&directory, "prepared.json", &encode(&prepared).unwrap()).unwrap();
        prepared
    }
    fn close(&mut self) {
        drop(self.catalog.take());
        drop(self.selection.take());
        drop(self.store.take());
    }
    fn audit_source(&self) {
        assert_eq!(
            fs::read(self.path().join("generated-source/original.bin")).unwrap(),
            b"original bytes are never activated or rewritten"
        );
        assert_eq!(
            fs::read_to_string(self.path().join("fixture-owner")).unwrap(),
            self.nonce
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.close();
        self.audit_source();
    }
}

#[test]
fn activation_lock_initial_seed_and_truncation_never_fall_back_empty() {
    let mut fixture = Fixture::new();
    let path = fixture.path();
    assert!(matches!(ActivationStore::open(&path),Err(e) if e.code==ErrorCode::CatalogInUse));
    fixture.close();
    let store = ActivationStore::open(&path).unwrap();
    let selected = store.startup_selection().unwrap();
    assert!(!selected.creation_allowed);
    drop(selected);
    drop(store);
    OpenOptions::new()
        .write(true)
        .open(path.join(DEFAULT_CATALOG))
        .unwrap()
        .set_len(0)
        .unwrap();
    let store = ActivationStore::open(&path).unwrap();
    assert!(store.startup_selection().is_err());
    drop(store);
}

#[test]
fn activation_native_generation_rejects_descriptor_tamper_and_old_pending_replay() {
    let mut fixture = Fixture::new();
    let prepared = fixture.prepare();
    fixture
        .store
        .as_ref()
        .unwrap()
        .stage_pending(&prepared)
        .unwrap();
    fixture.close();
    let path = fixture.path();
    let original = fs::read(path.join(DEFAULT_CATALOG)).unwrap();
    let store = ActivationStore::open(&path).unwrap();
    let selection = store.startup_selection().unwrap();
    assert!(selection.activated_now);
    assert_eq!(selection.path, prepared.candidate.native_path().unwrap());
    assert!(store.stage_pending(&prepared).is_err());
    assert_eq!(fs::read(path.join(DEFAULT_CATALOG)).unwrap(), original);
    let descriptor = store
        .restores
        .path
        .join(&prepared.transaction_id)
        .join("prepared.json");
    let mut bytes = fs::read(&descriptor).unwrap();
    bytes.push(b' ');
    fs::write(&descriptor, bytes).unwrap();
    drop(selection);
    drop(store);
    let store = ActivationStore::open(&path).unwrap();
    assert!(store.startup_selection().is_err());
    drop(store);
    fixture.audit_source();
}

struct OwnedChild(Child);
impl OwnedChild {
    fn finish(&mut self) -> std::process::ExitStatus {
        let until = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                return status;
            }
            assert!(
                Instant::now() < until,
                "Owned activation child exceeded its bound"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}
#[test]
fn activation_process_child() {
    let Some(path) = std::env::var_os("STILLHARBOUR_ACTIVATION_FIXTURE") else {
        return;
    };
    let path = PathBuf::from(path).canonicalize().unwrap();
    let nonce = std::env::var("STILLHARBOUR_ACTIVATION_NONCE").unwrap();
    assert!(
        uuid::Uuid::parse_str(&nonce).is_ok()
            && path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(&format!("stillharbour-activation-{nonce}-"))
    );
    assert_eq!(
        fs::read_to_string(path.join("fixture-owner")).unwrap(),
        nonce
    );
    let store = ActivationStore::open(&path).unwrap();
    let phase = std::env::var("STILLHARBOUR_ACTIVATION_PHASE").unwrap();
    if phase == "pending" {
        let id = std::env::var("STILLHARBOUR_ACTIVATION_TXN").unwrap();
        let (prepared, _) = store.descriptor(&id).unwrap();
        store.stage_pending(&prepared).unwrap();
        std::process::exit(91);
    }
    if phase == "promoted" {
        let selected = store.startup_selection().unwrap();
        assert!(selected.activated_now);
        std::process::exit(92);
    }
    panic!("Unknown generated child phase");
}
#[test]
fn activation_actual_process_exits_preserve_original_and_monotonic_selected_candidate() {
    let mut fixture = Fixture::new();
    let prepared = fixture.prepare();
    fixture.close();
    let path = fixture.path();
    let original = fs::read(path.join(DEFAULT_CATALOG)).unwrap();
    let prior = fs::read(prepared.pre_restore.native_path().unwrap()).unwrap();
    for (phase, code) in [("pending", 91), ("promoted", 92)] {
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "catalog_restore::activation::tests::activation_process_child",
                "--nocapture",
            ])
            .env("STILLHARBOUR_ACTIVATION_FIXTURE", &path)
            .env("STILLHARBOUR_ACTIVATION_NONCE", &fixture.nonce)
            .env("STILLHARBOUR_ACTIVATION_PHASE", phase)
            .env("STILLHARBOUR_ACTIVATION_TXN", &prepared.transaction_id)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        assert_eq!(OwnedChild(child).finish().code(), Some(code));
        assert_eq!(fs::read(path.join(DEFAULT_CATALOG)).unwrap(), original);
        assert_eq!(
            fs::read(prepared.pre_restore.native_path().unwrap()).unwrap(),
            prior
        );
        fixture.audit_source();
    }
    let store = ActivationStore::open(&path).unwrap();
    let selected = store.startup_selection().unwrap();
    assert!(!selected.activated_now);
    assert_eq!(selected.path, prepared.candidate.native_path().unwrap());
    let catalog = Catalog::open_bound_existing(
        selected.path.clone(),
        selected.identity.clone(),
        selected.volume.clone(),
    )
    .unwrap();
    assert_eq!(catalog.summary().unwrap().id, prepared.catalog_id);
    drop(catalog);
    drop(selected);
    drop(store);
}

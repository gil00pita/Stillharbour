use super::*;
use std::sync::Arc;

struct Run {
    directory: tempfile::TempDir,
    owner: String,
}
impl Run {
    fn new() -> Self {
        let owner = uuid::Uuid::new_v4().simple().to_string();
        let directory = tempfile::Builder::new()
            .prefix(&format!("stillharbour-m4-catalog-{owner}-"))
            .tempdir()
            .unwrap();
        fs::write(
            directory.path().join("session-owner.json"),
            format!(r#"{{"version":1,"ownerNonce":"{owner}"}}"#),
        )
        .unwrap();
        Self { directory, owner }
    }
    fn path(&self) -> PathBuf {
        self.directory.path().canonicalize().unwrap()
    }
    fn proof(&self) -> Arc<CatalogFixtureSession> {
        Arc::new(CatalogFixtureSession::open(&self.path(), &self.owner).unwrap())
    }
}
impl Drop for Run {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.directory.disable_cleanup(true);
        }
    }
}

#[test]
fn native_session_refuses_wrong_owner_nested_scope_unknown_fields_and_linked_marker() {
    let run = Run::new();
    assert!(
        CatalogFixtureSession::open(&run.path(), &uuid::Uuid::new_v4().simple().to_string())
            .is_err()
    );
    let nested = run
        .path()
        .join(format!("stillharbour-m4-catalog-{}-nested", run.owner));
    fs::create_dir(&nested).unwrap();
    fs::copy(
        run.path().join("session-owner.json"),
        nested.join("session-owner.json"),
    )
    .unwrap();
    assert!(CatalogFixtureSession::open(&nested, &run.owner).is_err());
    fs::write(
        run.path().join("session-owner.json"),
        format!(
            r#"{{"version":1,"ownerNonce":"{}","path":"untrusted"}}"#,
            run.owner
        ),
    )
    .unwrap();
    assert!(CatalogFixtureSession::open(&run.path(), &run.owner).is_err());
    fs::write(run.path().join("session-owner.json"), b"x".repeat(257)).unwrap();
    assert!(CatalogFixtureSession::open(&run.path(), &run.owner).is_err());
    fs::write(
        run.path().join("session-owner.json"),
        format!(r#"{{"version":1,"ownerNonce":"{}"}}"#, run.owner),
    )
    .unwrap();
    fs::hard_link(
        run.path().join("session-owner.json"),
        run.path().join("marker-alias"),
    )
    .unwrap();
    assert!(CatalogFixtureSession::open(&run.path(), &run.owner).is_err());
}

#[test]
fn native_session_role_refuses_existing_file_and_never_overwrites_it() {
    let run = Run::new();
    let proof = run.proof();
    let target = proof.role_path(CatalogFixtureRole::Appdata);
    fs::write(&target, b"existing model file is not a role directory").unwrap();
    assert!(proof.role_binding(CatalogFixtureRole::Appdata).is_err());
    assert!(proof.create_role(CatalogFixtureRole::Appdata).is_err());
    assert_eq!(
        fs::read(target).unwrap(),
        b"existing model file is not a role directory"
    );
}

#[cfg(unix)]
#[test]
fn native_session_refuses_symlinked_role_without_following_foreign_target() {
    let run = Run::new();
    let foreign = Run::new();
    let proof = run.proof();
    fs::write(
        foreign.path().join("neighbor"),
        b"retain this foreign generated model",
    )
    .unwrap();
    std::os::unix::fs::symlink(foreign.path(), proof.role_path(CatalogFixtureRole::Appdata))
        .unwrap();
    assert!(proof.role_binding(CatalogFixtureRole::Appdata).is_err());
    assert!(proof.create_role(CatalogFixtureRole::Appdata).is_err());
    assert_eq!(
        fs::read(foreign.path().join("neighbor")).unwrap(),
        b"retain this foreign generated model"
    );
}

#[cfg(any(windows, target_os = "macos"))]
#[test]
fn native_session_cleanup_denies_foreign_store_and_preserves_pending_authority() {
    struct Owned(String);
    impl Drop for Owned {
        fn drop(&mut self) {
            let result = authority::delete_owned_fixture(&self.0);
            if !std::thread::panicking() {
                result.unwrap();
            }
        }
    }
    let a = Run::new();
    let b = Run::new();
    let pa = a.proof();
    let pb = b.proof();
    pa.create_role(CatalogFixtureRole::Appdata).unwrap();
    pb.create_role(CatalogFixtureRole::Appdata).unwrap();
    let sa = ActivationStore::open(&pa.role_path(CatalogFixtureRole::Appdata)).unwrap();
    let _ca = Owned(sa.scope.clone());
    let sb = ActivationStore::open(&pb.role_path(CatalogFixtureRole::Appdata)).unwrap();
    let _cb = Owned(sb.scope.clone());
    for store in [&sa, &sb] {
        let selection = store.startup_selection().unwrap();
        let catalog = crate::catalog::Catalog::open_bound_seed(
            selection.path.clone(),
            selection.identity.clone(),
            selection.volume.clone(),
        )
        .unwrap();
        store.confirm_initialized(&selection).unwrap();
        drop(catalog);
        drop(selection);
    }
    let before = authority::read(&sb.scope).unwrap().unwrap();
    assert!(pa.cleanup_authority(&sb).is_err());
    assert_eq!(authority::read(&sb.scope).unwrap().unwrap(), before);
    let active = authority::read(&sa.scope).unwrap().unwrap();
    let pending = authority::stage(
        &sa.scope,
        &active,
        authority::Selector {
            transaction_id: Some(uuid::Uuid::new_v4().to_string()),
            digest: "a".repeat(64),
        },
    )
    .unwrap();
    assert!(pa.cleanup_authority(&sa).is_err());
    assert_eq!(authority::read(&sa.scope).unwrap().unwrap(), pending);
    pb.cleanup_authority(&sb).unwrap();
    assert!(authority::read(&sb.scope).unwrap().is_none());
    assert_eq!(authority::read(&sa.scope).unwrap().unwrap(), pending);
}

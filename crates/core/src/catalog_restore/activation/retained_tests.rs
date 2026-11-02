//! Activation-source fixtures do not substitute for native backup/prepare proof.
use super::*;

fn raw_v1(fixture: &Fixture, prepared: &PreparedCatalogRestore) -> Vec<u8> {
    let mut value = serde_json::to_value(prepared).unwrap();
    let object = value.as_object_mut().unwrap();
    for field in [
        "source_kind",
        "retained_transaction_id",
        "retained_generation",
    ] {
        object.remove(field);
    }
    let bytes = serde_json::to_vec(&value).unwrap();
    fs::write(
        fixture
            .path()
            .join("catalog-restores")
            .join(&prepared.transaction_id)
            .join("prepared.json"),
        &bytes,
    )
    .unwrap();
    bytes
}
fn activate(
    fixture: &mut Fixture,
    prepared: &PreparedCatalogRestore,
) -> (Arc<ActivationStore>, StartupSelection, Arc<Catalog>) {
    fixture
        .store
        .as_ref()
        .unwrap()
        .stage_pending(prepared)
        .unwrap();
    fixture.close();
    let store = Arc::new(ActivationStore::open(&fixture.path()).unwrap());
    let selected = store.startup_selection().unwrap();
    let catalog = Arc::new(
        Catalog::open_bound_existing(
            selected.path.clone(),
            selected.identity.clone(),
            selected.volume.clone(),
        )
        .unwrap(),
    );
    store.confirm_initialized(&selected).unwrap();
    (store, selected, catalog)
}
fn retained_forward(
    store: &ActivationStore,
    current: &Catalog,
    proof: &RetainedCatalogSource,
) -> PreparedCatalogRestore {
    let transaction_id = uuid::Uuid::new_v4().to_string();
    let directory = bridge::make_directory(&store.restores, &transaction_id).unwrap();
    let current_binding = crate::catalog_restore::capture_current(current.native_path()).unwrap();
    current
        .snapshot(directory.path.join("pre-restore.stillharbour"))
        .unwrap();
    let source = proof.source().native_path().unwrap();
    let mut input = filesystem::open_readonly(&source).unwrap();
    let mut output = create_new(&directory.path.join("candidate.stillharbour"), b"").unwrap();
    std::io::copy(&mut input, &mut output).unwrap();
    output.sync_all().unwrap();
    drop(output);
    let schema = crate::catalog::current_schema_fingerprint().unwrap();
    let candidate = crate::catalog_restore::validate_catalog(
        &directory.path.join("candidate.stillharbour"),
        proof.catalog_id(),
        &schema,
    )
    .unwrap();
    let pre = crate::catalog_restore::validate_catalog(
        &directory.path.join("pre-restore.stillharbour"),
        proof.catalog_id(),
        &schema,
    )
    .unwrap();
    let prepared = PreparedCatalogRestore {
        version: 2,
        transaction_id,
        catalog_id: proof.catalog_id().into(),
        source_kind: crate::backup::snapshots::CatalogRestoreSourceKind::RetainedPreRestore,
        source_restore_job_id: None,
        source_snapshot_id: None,
        retained_transaction_id: Some(proof.transaction_id().into()),
        retained_generation: Some(proof.generation()),
        source_catalog_revision: proof.revision().into(),
        candidate_catalog_revision: candidate.revision,
        current_catalog_revision: pre.revision,
        schema: crate::catalog::SCHEMA_VERSION,
        current: current_binding,
        pre_restore: crate::catalog_restore::capture_file(
            &directory.path.join("pre-restore.stillharbour"),
        )
        .unwrap(),
        candidate: crate::catalog_restore::capture_file(
            &directory.path.join("candidate.stillharbour"),
        )
        .unwrap(),
        roots: candidate.roots,
    };
    bridge::write_new(&directory, "prepared.json", &encode(&prepared).unwrap()).unwrap();
    prepared
}

#[test]
fn latest_retained_proof_derives_original_roots_and_preserves_v1_raw_seal() {
    let mut fixture = Fixture::new();
    assert!(
        fixture
            .store
            .as_ref()
            .unwrap()
            .latest_retained_transaction_id()
            .unwrap()
            .is_none()
    );
    let mut prepared = fixture.prepare();
    let original_root = prepared.roots[0].root_id.clone();
    // Represent an older snapshot with a different root set from CURRENT's
    // retained pre-backup. This must never become retained source root authority.
    let candidate_id = uuid::Uuid::new_v4().to_string();
    let db = rusqlite::Connection::open(prepared.candidate.native_path().unwrap()).unwrap();
    db.execute("UPDATE roots SET id=?", [&candidate_id])
        .unwrap();
    db.close().unwrap();
    prepared.candidate =
        crate::catalog_restore::capture_file(&prepared.candidate.native_path().unwrap()).unwrap();
    prepared.roots = crate::catalog_restore::validate_catalog(
        &prepared.candidate.native_path().unwrap(),
        &prepared.catalog_id,
        &crate::catalog::current_schema_fingerprint().unwrap(),
    )
    .unwrap()
    .roots;
    let raw = raw_v1(&fixture, &prepared);
    let (store, selected, catalog) = activate(&mut fixture, &prepared);
    assert!(
        store
            .latest_retained_source(&uuid::Uuid::new_v4().to_string())
            .is_err()
    );
    let proof = store
        .latest_retained_source(&prepared.transaction_id)
        .unwrap();
    proof.revalidate().unwrap();
    assert_eq!(proof.source().digest, prepared.pre_restore.digest);
    assert_eq!(proof.current().identity, selected.identity);
    assert_eq!(proof.current().path, native::encode_path(&selected.path));
    assert_eq!(proof.roots()[0].root_id, original_root);
    assert_ne!(proof.roots()[0].root_id, candidate_id);
    assert_eq!(
        fs::read(
            store
                .restores
                .path
                .join(&prepared.transaction_id)
                .join("prepared.json")
        )
        .unwrap(),
        raw
    );
    assert!(proof.generation().parse::<u64>().is_ok());
    drop(proof);
    drop(catalog);
    drop(selected);
    drop(store);
    fixture.audit_source();
}

#[test]
fn retained_return_is_forward_new_candidate_and_old_source_generation_is_retired() {
    let mut fixture = Fixture::new();
    let original = fixture.prepare();
    let (store, selected, catalog) = activate(&mut fixture, &original);
    catalog
        .call(|db| {
            db.execute("UPDATE roots SET label='newer post-activation edit'", [])?;
            db.execute(
                "UPDATE library_state SET revision=revision+1 WHERE singleton=1",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    let proof = store
        .latest_retained_source(&original.transaction_id)
        .unwrap();
    let next = retained_forward(&store, &catalog, &proof);
    assert_ne!(next.transaction_id, original.transaction_id);
    assert_ne!(next.candidate.identity, selected.identity);
    assert_ne!(next.candidate.identity, proof.source().identity);
    assert!(next.source_restore_job_id.is_none() && next.source_snapshot_id.is_none());
    let descriptor = store
        .restores
        .path
        .join(&next.transaction_id)
        .join("prepared.json");
    let mut stale = next.clone();
    stale.retained_generation = Some((proof.generation().parse::<u64>().unwrap() + 1).to_string());
    fs::write(&descriptor, encode(&stale).unwrap()).unwrap();
    assert!(store.stage_pending(&stale).is_err());
    stale = next.clone();
    stale.retained_transaction_id = Some(uuid::Uuid::new_v4().to_string());
    fs::write(&descriptor, encode(&stale).unwrap()).unwrap();
    assert!(store.stage_pending(&stale).is_err());
    fs::write(&descriptor, encode(&next).unwrap()).unwrap();
    store.stage_pending(&next).unwrap();
    assert!(
        proof.revalidate().is_err(),
        "A pending forward activation retires old review proof"
    );
    drop(catalog);
    drop(selected);
    let next_selected = store.startup_selection().unwrap();
    assert!(next_selected.activated_now);
    assert_eq!(next_selected.path, next.candidate.native_path().unwrap());
    assert!(
        store
            .latest_retained_source(&original.transaction_id)
            .is_err()
    );
    let newer = store.latest_retained_source(&next.transaction_id).unwrap();
    assert_eq!(newer.source().identity, next.pre_restore.identity);
    assert_eq!(newer.revision(), next.current_catalog_revision);
    let retained_db = rusqlite::Connection::open_with_flags(
        newer.source().native_path().unwrap(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        retained_db
            .query_row("SELECT label FROM roots", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "newer post-activation edit"
    );
    drop(retained_db);
    assert_ne!(newer.generation(), proof.generation());
    assert_eq!(
        store.latest_retained_transaction_id().unwrap().as_deref(),
        Some(next.transaction_id.as_str())
    );
    newer.revalidate().unwrap();
    drop(newer);
    drop(proof);
    drop(next_selected);
    drop(store);
    fixture.audit_source();
}

#[test]
fn retained_proof_rejects_changed_source_without_adopting_or_repairing_it() {
    let mut fixture = Fixture::new();
    let prepared = fixture.prepare();
    let (store, selected, catalog) = activate(&mut fixture, &prepared);
    let proof = store
        .latest_retained_source(&prepared.transaction_id)
        .unwrap();
    let path = proof.source().native_path().unwrap();
    let before = fs::read(&path).unwrap();
    let changed = fs::write(&path, b"changed generated frozen catalog");
    #[cfg(windows)]
    {
        assert!(
            changed.is_err(),
            "Held frozen source refuses a new Windows writer"
        );
        proof.revalidate().unwrap();
    }
    #[cfg(target_os = "macos")]
    {
        changed.unwrap();
        assert!(proof.revalidate().is_err());
    }
    drop(proof);
    #[cfg(windows)]
    fs::write(&path, b"changed generated frozen catalog").unwrap();
    assert!(
        store
            .latest_retained_source(&prepared.transaction_id)
            .is_err()
    );
    assert_ne!(fs::read(&path).unwrap(), before);
    drop(catalog);
    drop(selected);
    drop(store);
    fixture.audit_source();
}

use super::*;
use crate::{
    library::{model::RootMode, native},
    operations::*,
};
#[test]
fn preferred_import_context_requires_this_catalog_and_managed_root_but_retains_offline_hints() {
    let fixture = Fixture::with_media_destination(
        true,
        generated_photo_bytes().unwrap(),
        None,
        Some(RootMode::Managed),
    )
    .unwrap();
    let library = &fixture.library;
    let catalog_id = library.catalog.summary().unwrap().id;
    let owner = fixture.owner.clone();
    let id = library
        .catalog
        .call(move |db| {
            Ok(operation_store::load(db, &owner)?
                .view
                .destination_root_id
                .unwrap())
        })
        .unwrap();
    library
        .validate_import_destination_preference(&catalog_id, id.clone())
        .unwrap();
    let saved = library
        .set_import_destination(catalog_id.clone(), Some(id.clone()))
        .unwrap();
    assert_eq!(saved.root_id, Some(id.clone()));
    let snapshot_path = fixture
        ._run
        .path()
        .join("data/import-preference-snapshot.stillharbour");
    library.catalog.snapshot(snapshot_path.clone()).unwrap();
    let snapshot = Catalog::open(snapshot_path).unwrap();
    assert_eq!(
        crate::library::import_destination::load(&snapshot)
            .unwrap()
            .root_id,
        Some(id.clone())
    );
    assert_eq!(snapshot.summary().unwrap().id, catalog_id);
    drop(snapshot);
    library.catalog.call(|db|{db.execute_batch("CREATE TRIGGER deny_import_preference BEFORE UPDATE ON import_destination_preference BEGIN SELECT RAISE(ABORT,'fixture preference denial'); END;")?;Ok(())}).unwrap();
    assert!(
        library
            .set_import_destination(catalog_id.clone(), None)
            .is_err()
    );
    assert_eq!(
        crate::library::import_destination::load(&library.catalog)
            .unwrap()
            .root_id,
        Some(id.clone())
    );
    library
        .catalog
        .call(|db| {
            db.execute_batch("DROP TRIGGER deny_import_preference")?;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        library
            .status()
            .unwrap()
            .import_destination
            .unwrap()
            .catalog_id,
        catalog_id
    );
    assert!(
        library
            .set_import_destination(uuid::Uuid::new_v4().to_string(), None)
            .is_err()
    );
    assert!(
        library
            .validate_import_destination_preference(&uuid::Uuid::new_v4().to_string(), id.clone())
            .is_err()
    );
    assert!(
        library
            .validate_import_destination_preference(&catalog_id, uuid::Uuid::new_v4().to_string())
            .is_err()
    );
    library
        .catalog
        .call({
            let id = id.clone();
            move |db| {
                db.execute("UPDATE roots SET availability='offline' WHERE id=?", [id])?;
                Ok(())
            }
        })
        .unwrap();
    library
        .validate_import_destination_preference(&catalog_id, id.clone())
        .unwrap();
    library
        .set_import_destination(catalog_id.clone(), Some(id.clone()))
        .unwrap();
    library
        .catalog
        .call({
            let id = id.clone();
            move |db| {
                db.execute("UPDATE roots SET mode='referenced' WHERE id=?", [id])?;
                Ok(())
            }
        })
        .unwrap();
    assert!(
        library
            .validate_import_destination_preference(&catalog_id, id.clone())
            .is_err()
    );
    assert!(
        library
            .set_import_destination(catalog_id.clone(), Some(id.clone()))
            .is_err()
    );
    assert_eq!(
        library
            .set_import_destination(catalog_id.clone(), None)
            .unwrap()
            .root_id,
        None
    );
    assert_eq!(library.catalog.summary().unwrap().id, catalog_id);
    library
        .catalog
        .call({
            let id = id.clone();
            move |db| {
                db.execute("UPDATE roots SET mode='managed' WHERE id=?", [id])?;
                Ok(())
            }
        })
        .unwrap();
    library
        .set_import_destination(catalog_id, Some(id.clone()))
        .unwrap();
    library
        .catalog
        .call(move |db| {
            db.execute("DELETE FROM roots WHERE id=?", [id])?;
            Ok(())
        })
        .unwrap();
    assert_eq!(
        crate::library::import_destination::load(&library.catalog)
            .unwrap()
            .root_id,
        None
    );
    assert!(fixture.preserved().unwrap());
}

fn wait(fixture: &Fixture, id: &str) -> CopyWorkflowJob {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let job = fixture.library.organization_copy_job(id.into()).unwrap();
        if !matches!(
            job.state,
            CopyJobState::Queued
                | CopyJobState::Running
                | CopyJobState::PauseRequested
                | CopyJobState::CancelRequested
        ) {
            return job;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Managed destination job did not stop: {:?}",
            job.state
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
#[test]
fn managed_destination_requires_explicit_root_policy_and_fresh_grant_for_both_actions() {
    if crate::test_process::isolated(
        "executor::catalog_journal::managed_destination_tests::managed_destination_requires_explicit_root_policy_and_fresh_grant_for_both_actions",
        60,
    ) {
        return;
    }
    let fixture = Fixture::with_media_destination(
        true,
        generated_photo_bytes().unwrap(),
        None,
        Some(RootMode::Managed),
    )
    .unwrap();
    let catalog = &fixture.library.catalog;
    let owner = fixture.owner.clone();
    let draft = catalog
        .call(move |db| operation_store::load(db, &owner))
        .unwrap();
    let target_id = draft.view.destination_root_id.clone().unwrap();
    let source = &draft.view.preview.rows[0];
    let assets = vec![source.asset_id.clone()];
    let selected = native::open_root(&fixture.destination, None).unwrap();
    let identity = filesystem::identity(&selected).unwrap();
    let volume = native::volume_token(&selected).unwrap();
    drop(selected);
    let check = |path, expected, target| {
        fixture.library.organization_destination_for_root(
            assets.clone(),
            draft.view.preview.template.clone(),
            draft.view.preview.fingerprint.clone(),
            path,
            expected,
            target,
        )
    };
    assert!(
        check(
            fixture.destination.clone(),
            (identity.clone(), volume.clone()),
            None
        )
        .is_err(),
        "Default external policy remains strict"
    );
    let child_path = fixture.destination.join("organized");
    let child = native::open_root(&child_path, None).unwrap();
    let child_identity = filesystem::identity(&child).unwrap();
    drop(child);
    assert!(
        check(
            child_path,
            (child_identity, volume.clone()),
            Some(target_id.clone())
        )
        .is_err(),
        "Managed grant is for the root itself"
    );
    let original = native::open_root(&fixture.source, None).unwrap();
    let original_identity = filesystem::identity(&original).unwrap();
    let original_volume = native::volume_token(&original).unwrap();
    drop(original);
    assert!(
        check(
            fixture.source.clone(),
            (original_identity, original_volume),
            Some(source.root_id.clone())
        )
        .is_err(),
        "Source cannot be destination"
    );
    assert!(
        check(
            fixture.destination.clone(),
            (identity.clone(), volume.clone()),
            Some(uuid::Uuid::new_v4().to_string())
        )
        .is_err()
    );
    let selection = uuid::Uuid::new_v4().to_string();
    let mut review = fixture
        .library
        .organization_copy_review_create(
            fixture.owner.clone(),
            fixture.manifest.clone(),
            selection.clone(),
            fixture.destination.clone(),
            (identity.clone(), volume.clone()),
        )
        .unwrap();
    for action in [CopyAction::PrepareNames, CopyAction::CopyFiles] {
        assert_eq!(review.action, action);
        catalog
            .call({
                let target_id = target_id.clone();
                move |db| {
                    db.execute("UPDATE roots SET mode='referenced' WHERE id=?", [target_id])?;
                    Ok(())
                }
            })
            .unwrap();
        assert!(
            fixture
                .library
                .organization_plan(fixture.owner.clone())
                .unwrap()
                .stale
        );
        assert!(
            fixture
                .library
                .organization_copy_approve(
                    CopyApprovalInput {
                        review_id: review.id.clone(),
                        digest: review.digest.clone(),
                        action,
                        fidelity_limits_acknowledged: true
                    },
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone())
                )
                .is_err(),
            "Revoked target grant cannot authorize native work"
        );
        assert!(fixture.preserved().unwrap());
        catalog
            .call({
                let target_id = target_id.clone();
                move |db| {
                    db.execute("UPDATE roots SET mode='managed' WHERE id=?", [target_id])?;
                    Ok(())
                }
            })
            .unwrap();
        review = fixture
            .library
            .organization_copy_review(
                review.id,
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        let approval = fixture
            .library
            .organization_copy_approve(
                CopyApprovalInput {
                    review_id: review.id.clone(),
                    digest: review.digest.clone(),
                    action,
                    fidelity_limits_acknowledged: true,
                },
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        let denied_input = CopyStartInput {
            review_id: review.id.clone(),
            digest: review.digest.clone(),
            approval_id: approval.id,
        };
        let jobs_before = crate::library::operation_jobs::list(catalog).unwrap().len();
        catalog
            .call({
                let target_id = target_id.clone();
                move |db| {
                    db.execute("UPDATE roots SET mode='referenced' WHERE id=?", [target_id])?;
                    Ok(())
                }
            })
            .unwrap();
        assert!(
            fixture
                .library
                .organization_copy_start(
                    denied_input,
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone())
                )
                .is_err(),
            "A ticket issued before target grant revocation must not start work"
        );
        assert_eq!(
            crate::library::operation_jobs::list(catalog).unwrap().len(),
            jobs_before
        );
        if action == CopyAction::PrepareNames {
            assert!(!fixture.destination.join(&review.output_folder).exists());
        } else {
            assert!(
                !fixture
                    .destination
                    .join(&review.output_folder)
                    .join("organized/Photo.JPG")
                    .exists()
            );
        }
        assert!(fixture.preserved().unwrap());
        catalog
            .call({
                let target_id = target_id.clone();
                move |db| {
                    db.execute("UPDATE roots SET mode='managed' WHERE id=?", [target_id])?;
                    Ok(())
                }
            })
            .unwrap();
        review = fixture
            .library
            .organization_copy_review(
                review.id,
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        let approval = fixture
            .library
            .organization_copy_approve(
                CopyApprovalInput {
                    review_id: review.id.clone(),
                    digest: review.digest.clone(),
                    action,
                    fidelity_limits_acknowledged: true,
                },
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        let queued = fixture
            .library
            .organization_copy_start(
                CopyStartInput {
                    review_id: review.id.clone(),
                    digest: review.digest.clone(),
                    approval_id: approval.id,
                },
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        let finished = wait(&fixture, &queued.id);
        assert_eq!(
            finished.state,
            if action == CopyAction::PrepareNames {
                CopyJobState::Prepared
            } else {
                CopyJobState::CompletedVerified
            },
            "{:?}",
            finished.error
        );
        assert!(fixture.preserved().unwrap());
        review = fixture
            .library
            .organization_copy_review(
                review.id,
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
    }
    let inspection = fixture
        .library
        .organization_copy_inspect(
            review.id,
            selection,
            fixture.destination.clone(),
            (identity, volume),
        )
        .unwrap();
    assert!(inspection.all_published_bytes_verified);
    assert_eq!(
        inspection.states,
        vec![CopyInspectionState::PublishedVerified; 2]
    );
    assert_eq!(
        std::fs::read(fixture.destination.join("destination-only.bin")).unwrap(),
        NEIGHBOR_BYTES
    );
    assert_eq!(
        catalog
            .call(|db| Ok(db.query_row("SELECT count(*) FROM roots", [], |r| r.get::<_, u32>(0))?))
            .unwrap(),
        2
    );
}

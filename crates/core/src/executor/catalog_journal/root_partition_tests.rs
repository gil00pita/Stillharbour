use super::tests::{comparison_root, import_draft_for, wait_operation};
use super::*;
use crate::{
    executor::{CopyWorkflow, sources::Snapshot},
    library::native,
    operations::*,
};

#[test]
fn source_partition_preserves_complete_manifest_and_executes_only_each_reviewed_root() {
    if crate::test_process::isolated(
        "executor::catalog_journal::root_partition_tests::source_partition_preserves_complete_manifest_and_executes_only_each_reviewed_root",
        60,
    ) {
        return;
    }
    let fixture = Fixture::new().unwrap();
    let owner = fixture.owner.clone();
    let source = fixture
        .library
        .catalog
        .call(move |db| Ok(operation_store::load(db, &owner)?.sources[0].clone()))
        .unwrap();
    let original = Snapshot::build(
        &fixture.library.catalog,
        fixture.owner.clone(),
        fixture.manifest.clone(),
    )
    .unwrap();
    assert!(
        serde_json::to_value(&original)
            .unwrap()
            .get("root_partition")
            .is_none()
    );
    assert!(
        Snapshot::build_partition(
            &fixture.library.catalog,
            fixture.owner.clone(),
            fixture.manifest.clone(),
            source.root_id.clone()
        )
        .is_err()
    );
    let other_bytes = b"another generated root source".to_vec();
    let (other, other_path, ids) = comparison_root(&fixture, &[("Other.JPG", other_bytes.clone())]);
    let grant = other.clone();
    fixture
        .library
        .catalog
        .call(move |db| {
            db.execute("UPDATE roots SET mode='managed' WHERE id=?", [grant])?;
            db.execute("UPDATE library_state SET revision=revision+1", [])?;
            Ok(())
        })
        .unwrap();
    let manifest = import_draft_for(&fixture, vec![source.id, ids[0].clone()]);
    assert_eq!(manifest.files.len(), 3);
    assert!(
        Snapshot::build(
            &fixture.library.catalog,
            manifest.plan_id.clone(),
            manifest.id.clone()
        )
        .is_err()
    );
    let build = |root: String| {
        Snapshot::build_partition(
            &fixture.library.catalog,
            manifest.plan_id.clone(),
            manifest.id.clone(),
            root,
        )
    };
    assert!(build(uuid::Uuid::new_v4().to_string()).is_err());
    assert!(build("../root".into()).is_err());
    let first = build(source.root_id.clone()).unwrap();
    let second = build(other.clone()).unwrap();
    assert_eq!(first.files.len(), 2);
    assert_eq!(second.files.len(), 1);
    for snapshot in [&first, &second] {
        assert_eq!(snapshot.manifest_digest, manifest.digest);
        let retained = snapshot.clone();
        fixture
            .library
            .catalog
            .call(move |db| retained.validate(db))
            .unwrap();
    }
    let mut changed = first.clone();
    changed.root_partition = Some(other.clone());
    assert!(
        fixture
            .library
            .catalog
            .call(move |db| changed.validate(db))
            .is_err()
    );
    let mut changed = first.clone();
    changed.files.push(second.files[0].clone());
    assert!(
        fixture
            .library
            .catalog
            .call(move |db| changed.validate(db))
            .is_err()
    );
    let revoked = other.clone();
    fixture
        .library
        .catalog
        .call(move |db| {
            db.execute("UPDATE roots SET mode='referenced' WHERE id=?", [revoked])?;
            Ok(())
        })
        .unwrap();
    assert!(build(other.clone()).is_err());
    let restored = other.clone();
    fixture
        .library
        .catalog
        .call(move |db| {
            db.execute("UPDATE roots SET mode='managed' WHERE id=?", [restored])?;
            Ok(())
        })
        .unwrap();

    let selected = native::open_root(&fixture.destination, None).unwrap();
    let identity = filesystem::identity(&selected).unwrap();
    let volume = native::volume_token(&selected).unwrap();
    drop(selected);
    let workflow = CopyWorkflow::new(
        fixture.library.catalog.clone(),
        fixture._run.path().join("data"),
    )
    .unwrap();
    let mut folders = Vec::new();
    for snapshot in [first, second] {
        let selection = uuid::Uuid::new_v4().to_string();
        let root = snapshot.root_id.clone();
        let scope = (
            manifest.plan_id.clone(),
            manifest.id.clone(),
            Some(root.clone()),
        );
        let matched = workflow
            .effect_match_for_root(
                &fixture.library,
                scope.clone(),
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        assert!(matched.can_reuse_review);
        let mut review = workflow
            .create_review_for_root(
                &fixture.library,
                scope,
                selection.clone(),
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        assert_eq!(review.source_root_id, root);
        assert_eq!(review.files.len(), snapshot.files.len());
        assert_eq!(review.assessment.manifest_digest, manifest.digest);
        assert_eq!(
            review.assessment.capacity.logical_bytes,
            snapshot
                .files
                .iter()
                .map(|file| file.bytes as u128)
                .sum::<u128>()
                .to_string()
        );
        assert!(!folders.contains(&review.output_folder));
        folders.push(review.output_folder.clone());
        for action in [CopyAction::PrepareNames, CopyAction::CopyFiles] {
            assert_eq!(review.action, action);
            let approval = workflow
                .approve(
                    &fixture.library,
                    CopyApprovalInput {
                        review_id: review.id.clone(),
                        digest: review.digest.clone(),
                        action,
                        fidelity_limits_acknowledged: true,
                    },
                    selection.clone(),
                    fixture.destination.clone(),
                    identity.clone(),
                    volume.clone(),
                )
                .unwrap();
            let queued = workflow
                .start(
                    fixture.library.clone(),
                    CopyStartInput {
                        review_id: review.id.clone(),
                        digest: review.digest.clone(),
                        approval_id: approval.id,
                    },
                    selection.clone(),
                    fixture.destination.clone(),
                    identity.clone(),
                    volume.clone(),
                )
                .unwrap();
            let finished = wait_operation(|| workflow.job(queued.id.clone()));
            assert_eq!(finished.source_root_id, root);
            assert_eq!(finished.total_files, snapshot.files.len() as u32);
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
            if action == CopyAction::PrepareNames {
                workflow.discard(&review.id).unwrap();
                let reopened = workflow
                    .reopen_preparation(
                        &fixture.library,
                        finished.id,
                        selection.clone(),
                        fixture.destination.clone(),
                        (identity.clone(), volume.clone()),
                    )
                    .unwrap();
                assert_eq!(reopened.manifest.files.len(), 3);
                assert!(reopened.import_plan.is_none());
                review = reopened.review;
            } else {
                assert_eq!(finished.verified_files, snapshot.files.len() as u32);
            }
        }
        for file in &snapshot.files {
            let bytes = std::fs::read(
                fixture
                    .destination
                    .join(&review.output_folder)
                    .join(&file.target),
            )
            .unwrap();
            assert_eq!(bytes.len() as u64, file.bytes);
        }
        let matched = workflow
            .effect_match_for_root(
                &fixture.library,
                (manifest.plan_id.clone(), manifest.id.clone(), Some(root)),
                selection,
                fixture.destination.clone(),
                (identity.clone(), volume.clone()),
            )
            .unwrap();
        assert!(!matched.can_reuse_review);
        assert_eq!(matched.existing.len(), 1);
    }
    assert!(fixture.preserved().unwrap());
    assert_eq!(
        std::fs::read(other_path.join("Other.JPG")).unwrap(),
        other_bytes
    );
    assert_eq!(std::fs::read_dir(&fixture.destination).unwrap().count(), 3);
}

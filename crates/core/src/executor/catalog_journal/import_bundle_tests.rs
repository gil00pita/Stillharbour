//! Opaque generated companions exercise preservation, not codec support or
//! automatic recognition of a provider's Live Photo identifiers.
use super::tests::{comparison_root, import_draft_for, wait_comparison, wait_operation};
use super::*;
use crate::{executor::CopyWorkflow, library::native, operations::*};

#[test]
fn import_raw_motion_and_filename_sidecar_requires_complete_bundle_equality() {
    if crate::test_process::isolated(
        "executor::catalog_journal::import_bundle_tests::import_raw_motion_and_filename_sidecar_requires_complete_bundle_equality",
        60,
    ) {
        return;
    }
    for complete in [true, false] {
        let mut fixture = Fixture::new().unwrap();
        let extras = [
            ("Photo.CR3", b"generated opaque raw bytes".as_slice()),
            ("Photo.MOV", b"generated opaque motion bytes".as_slice()),
            ("Photo.JPG.AAE", b"generated adjustment sidecar".as_slice()),
        ];
        for (name, bytes) in extras {
            std::fs::write(fixture.source.join(name), bytes).unwrap();
            fixture.baseline.push((name.into(), bytes.to_vec()));
        }
        // An unrelated descendant must neither disappear nor become an
        // implicitly confirmed member of the selected filename bundle.
        std::fs::write(fixture.source.join("unknown.bin"), b"retain unknown").unwrap();
        fixture
            .baseline
            .push(("unknown.bin".into(), b"retain unknown".to_vec()));
        let owner = fixture.owner.clone();
        let asset = fixture
            .library
            .catalog
            .call(move |db| Ok(operation_store::load(db, &owner)?.sources[0].id.clone()))
            .unwrap();
        let mut keepers = vec![
            ("Renamed.JPG", fixture.baseline[0].1.clone()),
            ("Renamed.xmp", COMPANION_BYTES.to_vec()),
        ];
        for (name, bytes) in extras {
            keepers.push((
                match name {
                    "Photo.CR3" => "Renamed.CR3",
                    "Photo.MOV" => "Renamed.MOV",
                    _ => "Renamed.JPG.AAE",
                },
                if !complete && name == "Photo.CR3" {
                    b"different opaque raw bytes".to_vec()
                } else {
                    bytes.to_vec()
                },
            ));
        }
        let (keeper, keeper_path, _) = comparison_root(&fixture, &keepers);
        // Root registration changes catalog revision. Freeze the plan only
        // after all comparison roots exist, just as the real review requires.
        let manifest = import_draft_for(&fixture, vec![asset]);
        assert_eq!(manifest.files.len(), 5);
        assert!(
            manifest
                .files
                .iter()
                .all(|file| file.target_relative_path.starts_with("Imports/Photo."))
        );
        fixture.owner = manifest.plan_id;
        fixture.manifest = manifest.id;
        let workflow = CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        let queued = workflow
            .start_import_comparison(
                &fixture.library,
                ImportComparisonInput {
                    plan_id: fixture.owner.clone(),
                    manifest_id: fixture.manifest.clone(),
                    source_root_id: None,
                    scope_root_ids: vec![keeper],
                },
            )
            .unwrap();
        let compared = wait_comparison(&workflow, queued.id.clone());
        assert_eq!(
            compared.state,
            ImportComparisonState::Completed,
            "{:?}",
            compared.error
        );
        let report = compared.report.unwrap();
        assert_eq!(report.bundles.len(), 1);
        let bundle = &report.bundles[0];
        assert_eq!(bundle.file_ids.len(), 5);
        assert_eq!(bundle.existing_groups.len(), usize::from(complete));
        let plan = workflow
            .create_import_plan(
                &fixture.library,
                ImportPlanInput {
                    comparison_job_id: queued.id,
                    choices: vec![ImportBundleChoice {
                        file_id: bundle.file_ids[0].clone(),
                        existing_group_id: bundle
                            .existing_groups
                            .first()
                            .map(|group| group.id.clone()),
                    }],
                },
            )
            .unwrap();
        assert_eq!(plan.skip_files.len(), if complete { 5 } else { 0 });
        assert_eq!(plan.copy_file_ids.len(), if complete { 0 } else { 5 });
        if !complete {
            let selected = native::open_root(&fixture.destination, None).unwrap();
            let identity = filesystem::identity(&selected).unwrap();
            let volume = native::volume_token(&selected).unwrap();
            drop(selected);
            let selection = uuid::Uuid::new_v4().to_string();
            let mut review = workflow
                .create_import_review(
                    &fixture.library,
                    fixture.owner.clone(),
                    plan.id.clone(),
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone()),
                )
                .unwrap();
            for action in [CopyAction::PrepareNames, CopyAction::CopyFiles] {
                let approved = workflow
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
                            approval_id: approved.id,
                        },
                        selection.clone(),
                        fixture.destination.clone(),
                        identity.clone(),
                        volume.clone(),
                    )
                    .unwrap();
                let finished = wait_operation(|| workflow.job(queued.id.clone()));
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
                    review = workflow
                        .get_review(
                            &fixture.library,
                            review.id.clone(),
                            selection.clone(),
                            fixture.destination.clone(),
                            identity.clone(),
                            volume.clone(),
                        )
                        .unwrap();
                } else {
                    let receipt = workflow.import_copy_receipt(finished.id).unwrap().unwrap();
                    assert_eq!(receipt.copied_file_ids.len(), 5);
                    assert!(receipt.skipped_files.is_empty());
                }
            }
            for (name, bytes) in fixture
                .baseline
                .iter()
                .filter(|(name, _)| name != "unknown.bin")
            {
                assert_eq!(
                    std::fs::read(
                        fixture
                            .destination
                            .join(&review.output_folder)
                            .join("Imports")
                            .join(name)
                    )
                    .unwrap(),
                    *bytes
                );
            }
            assert!(
                !fixture
                    .destination
                    .join(&review.output_folder)
                    .join("Imports/unknown.bin")
                    .exists()
            );
        }
        assert!(fixture.preserved().unwrap());
        for (name, bytes) in keepers {
            assert_eq!(std::fs::read(keeper_path.join(name)).unwrap(), bytes);
        }
    }
}

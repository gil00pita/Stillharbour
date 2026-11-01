use super::tests::{
    comparison_root, import_draft_for, wait_comparison, wait_import, wait_operation,
};
use super::*;
use crate::{
    executor::{CopyWorkflow, sources::Snapshot},
    library::native,
    operations::*,
};

struct Case {
    fixture: Fixture,
    workflow: Arc<CopyWorkflow>,
    roots: [String; 2],
    keeper: String,
    other_path: std::path::PathBuf,
}
const OTHER: &[u8] = b"new generated source in a referenced second root";
impl Case {
    fn new() -> Self {
        let mut fixture = Fixture::new().unwrap();
        let owner = fixture.owner.clone();
        let first = fixture
            .library
            .catalog
            .call(move |db| Ok(operation_store::load(db, &owner)?.sources[0].clone()))
            .unwrap();
        let (other, other_path, ids) = comparison_root(&fixture, &[("Other.JPG", OTHER.to_vec())]);
        let (keeper, _, _) = comparison_root(
            &fixture,
            &[
                ("Renamed.JPG", generated_photo_bytes().unwrap()),
                ("Renamed.xmp", COMPANION_BYTES.to_vec()),
            ],
        );
        let manifest = import_draft_for(&fixture, vec![first.id, ids[0].clone()]);
        fixture.owner = manifest.plan_id;
        fixture.manifest = manifest.id;
        let workflow = CopyWorkflow::new(
            fixture.library.catalog.clone(),
            fixture._run.path().join("data"),
        )
        .unwrap();
        Self {
            fixture,
            workflow,
            roots: [first.root_id, other],
            keeper,
            other_path,
        }
    }
    fn input(&self, index: usize) -> ImportComparisonInput {
        ImportComparisonInput {
            plan_id: self.fixture.owner.clone(),
            manifest_id: self.fixture.manifest.clone(),
            scope_root_ids: vec![self.keeper.clone()],
            source_root_id: Some(self.roots[index].clone()),
        }
    }
    fn plan(&self, index: usize) -> ImportPlan {
        let queued = self
            .workflow
            .start_import_comparison(&self.fixture.library, self.input(index))
            .unwrap();
        assert_eq!(queued.source_root_id, self.roots[index]);
        assert_eq!(queued.total_source_files, if index == 0 { 2 } else { 1 });
        let job = wait_comparison(&self.workflow, queued.id);
        assert_eq!(
            job.state,
            ImportComparisonState::Completed,
            "{:?}",
            job.error
        );
        let plan = self
            .workflow
            .create_import_plan(
                &self.fixture.library,
                ImportPlanInput {
                    comparison_job_id: job.id.clone(),
                    choices: job
                        .report
                        .as_ref()
                        .unwrap()
                        .bundles
                        .iter()
                        .map(|bundle| ImportBundleChoice {
                            file_id: bundle.file_ids[0].clone(),
                            existing_group_id: bundle
                                .existing_groups
                                .first()
                                .map(|group| group.id.clone()),
                        })
                        .collect(),
                },
            )
            .unwrap();
        assert_eq!(plan.source_root_id, self.roots[index]);
        self.workflow.discard_import_comparison(job.id).unwrap();
        plan
    }
}

#[test]
fn import_partition_separately_records_existing_and_copies_new_referenced_source_without_cross_root_receipts()
 {
    if crate::test_process::isolated(
        "executor::catalog_journal::import_partition_tests::import_partition_separately_records_existing_and_copies_new_referenced_source_without_cross_root_receipts",
        60,
    ) {
        return;
    }
    let case = Case::new();
    let fixture = &case.fixture;
    let mut invalid = case.input(0);
    invalid.source_root_id = None;
    assert!(
        serde_json::to_value(&invalid)
            .unwrap()
            .get("sourceRootId")
            .is_none()
    );
    assert!(
        case.workflow
            .start_import_comparison(&fixture.library, invalid.clone())
            .is_err()
    );
    invalid.source_root_id = Some(uuid::Uuid::new_v4().to_string());
    assert!(
        case.workflow
            .start_import_comparison(&fixture.library, invalid.clone())
            .is_err()
    );
    invalid.source_root_id = Some("../root".into());
    assert!(
        case.workflow
            .start_import_comparison(&fixture.library, invalid)
            .is_err()
    );
    let mut same = case.input(0);
    same.scope_root_ids = vec![case.roots[0].clone()];
    assert!(
        case.workflow
            .start_import_comparison(&fixture.library, same)
            .is_err()
    );

    let existing = case.plan(0);
    assert_eq!(existing.skip_files.len(), 2);
    assert!(existing.copy_file_ids.is_empty());
    let new = case.plan(1);
    assert!(new.skip_files.is_empty());
    assert_eq!(new.copy_file_ids.len(), 1);
    assert_ne!(existing.id, new.id);
    assert_ne!(existing.digest, new.digest);
    let approval = case
        .workflow
        .approve_import_skip(
            &fixture.library,
            ImportSkipApprovalInput {
                plan_id: fixture.owner.clone(),
                import_plan_id: existing.id.clone(),
                digest: existing.digest.clone(),
                sources_retained_acknowledged: true,
            },
        )
        .unwrap();
    assert!(
        case.workflow
            .start_import_skip(
                &fixture.library,
                ImportSkipStartInput {
                    plan_id: fixture.owner.clone(),
                    import_plan_id: new.id.clone(),
                    digest: new.digest.clone(),
                    approval_id: approval.id.clone()
                }
            )
            .is_err()
    );
    let queued = case
        .workflow
        .start_import_skip(
            &fixture.library,
            ImportSkipStartInput {
                plan_id: fixture.owner.clone(),
                import_plan_id: existing.id.clone(),
                digest: existing.digest.clone(),
                approval_id: approval.id,
            },
        )
        .unwrap();
    let recorded = wait_import(fixture, &case.workflow, queued.id);
    assert_eq!(recorded.state, ImportSessionState::Completed);
    assert_eq!(recorded.verified_files, 2);
    assert_eq!(std::fs::read_dir(&fixture.destination).unwrap().count(), 1);

    let selected = native::open_root(&fixture.destination, None).unwrap();
    let identity = filesystem::identity(&selected).unwrap();
    let volume = native::volume_token(&selected).unwrap();
    drop(selected);
    let selection = uuid::Uuid::new_v4().to_string();
    let mut review = case
        .workflow
        .create_import_review(
            &fixture.library,
            fixture.owner.clone(),
            new.id.clone(),
            selection.clone(),
            fixture.destination.clone(),
            (identity.clone(), volume.clone()),
        )
        .unwrap();
    assert_eq!(review.source_root_id, case.roots[1]);
    assert_eq!(review.files.len(), 1);
    assert_eq!(
        review.assessment.capacity.logical_bytes,
        OTHER.len().to_string()
    );
    let mut copy_job = None;
    for action in [CopyAction::PrepareNames, CopyAction::CopyFiles] {
        assert_eq!(review.action, action);
        let approved = case
            .workflow
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
        let queued = case
            .workflow
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
        let finished = wait_operation(|| case.workflow.job(queued.id.clone()));
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
        assert_eq!(finished.source_root_id, case.roots[1]);
        if action == CopyAction::PrepareNames {
            case.workflow.discard(&review.id).unwrap();
            let reopened = case
                .workflow
                .reopen_preparation(
                    &fixture.library,
                    finished.id,
                    selection.clone(),
                    fixture.destination.clone(),
                    (identity.clone(), volume.clone()),
                )
                .unwrap();
            assert_eq!(reopened.manifest.files.len(), 3);
            assert_eq!(reopened.import_plan.as_ref().unwrap().id, new.id);
            review = reopened.review;
        } else {
            copy_job = Some(finished);
        }
    }
    let receipt = case
        .workflow
        .import_copy_receipt(copy_job.unwrap().id)
        .unwrap()
        .unwrap();
    assert_eq!(receipt.import_plan_id, new.id);
    assert_eq!(receipt.copied_file_ids, new.copy_file_ids);
    assert!(receipt.skipped_files.is_empty());
    assert_eq!(
        std::fs::read(
            fixture
                .destination
                .join(&review.output_folder)
                .join("Imports/Other.JPG")
        )
        .unwrap(),
        OTHER
    );
    let reopened = CopyWorkflow::new(
        fixture.library.catalog.clone(),
        fixture._run.path().join("data"),
    )
    .unwrap();
    assert_eq!(
        reopened
            .open_import_plan(fixture.owner.clone(), existing.id)
            .unwrap()
            .digest,
        existing.digest
    );
    assert_eq!(
        reopened
            .open_import_plan(fixture.owner.clone(), new.id)
            .unwrap()
            .digest,
        new.digest
    );
    assert!(fixture.preserved().unwrap());
    assert_eq!(
        std::fs::read(case.other_path.join("Other.JPG")).unwrap(),
        OTHER
    );
}

#[test]
fn import_partition_projection_keeps_root_and_full_manifest_identity_and_rejects_changed_tags() {
    let case = Case::new();
    let plan = case.plan(1);
    let snapshot = case
        .workflow
        .import_copy_sources(case.fixture.owner.clone(), plan.id)
        .unwrap();
    assert_eq!(snapshot.root_partition.as_ref(), Some(&case.roots[1]));
    assert_eq!(
        snapshot.required_mode(),
        crate::library::model::RootMode::Referenced
    );
    assert_eq!(snapshot.files.len(), 1);
    let retained = snapshot.clone();
    case.fixture
        .library
        .catalog
        .call(move |db| retained.validate(db))
        .unwrap();
    for root in [None, Some(case.roots[0].clone())] {
        let mut changed = snapshot.clone();
        changed.root_partition = root;
        assert!(
            case.fixture
                .library
                .catalog
                .call(move |db| changed.validate(db))
                .is_err()
        );
    }
    let mut changed = snapshot.clone();
    changed.import_selection.as_mut().unwrap().source_mode =
        crate::library::model::RootMode::Managed;
    assert!(
        case.fixture
            .library
            .catalog
            .call(move |db| changed.validate(db))
            .is_err()
    );
    let raw = Snapshot::build_read_partition(
        &case.fixture.library.catalog,
        case.fixture.owner.clone(),
        case.fixture.manifest.clone(),
        case.roots[1].clone(),
    )
    .unwrap();
    assert!(raw.import_selection.is_none());
    assert!(case.fixture.preserved().unwrap());
}

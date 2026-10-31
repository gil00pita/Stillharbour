#![cfg(windows)]
use std::{
    fs,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use stillharbour_core::{
    catalog::Catalog,
    domain::ErrorCode,
    filesystem,
    library::{Library, LibraryOptions, WorkerPaths, model::*},
};

fn wait(library: &Arc<Library>, job: &str) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let status = library.status().unwrap();
        if let Some(item) = status.jobs.iter().find(|j| j.id == job)
            && !matches!(
                item.state.as_str(),
                "queued" | "running" | "cancel_requested"
            )
        {
            assert!(
                matches!(item.state.as_str(), "completed" | "partial"),
                "{}: {}",
                item.state,
                item.message
            );
            return;
        }
        assert!(
            Instant::now() < deadline,
            "Windows fixture scan did not settle"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn open(data: &Path) -> (Arc<Catalog>, Arc<Library>) {
    let catalog = Arc::new(Catalog::open(data.join("Fixture.stillharbour")).unwrap());
    let library = Library::new(
        catalog.clone(),
        LibraryOptions {
            data_directory: data.to_owned(),
            workers: WorkerPaths {
                executable: env!("CARGO_BIN_EXE_stillharbour-media-worker").into(),
                exiftool: None,
                working_directory: data.join("worker"),
            },
        },
    )
    .unwrap();
    (catalog, library)
}

fn settle(library: &Arc<Library>) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let status = library.status().unwrap();
        if status
            .jobs
            .iter()
            .all(|j| !matches!(j.state.as_str(), "queued" | "running" | "cancel_requested"))
            && Arc::strong_count(library) == 1
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "Owned Windows scan/preview workers did not settle"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn local_ntfs_scan_indexes_filters_reconciles_and_preserves_generated_sources() {
    let source = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    fs::create_dir(source.path().join("nested")).unwrap();
    for path in ["one.png", "nested/two.JPG", "nested/three.webp"] {
        image::RgbImage::from_pixel(64, 32, image::Rgb([30, 80, 120]))
            .save(source.path().join(path))
            .unwrap();
    }
    fs::write(source.path().join("one.xmp"), b"unique sidecar").unwrap();
    let originals: Vec<_> = ["one.png", "nested/two.JPG", "nested/three.webp", "one.xmp"]
        .into_iter()
        .map(|p| (p, fs::read(source.path().join(p)).unwrap()))
        .collect();
    let (catalog, library) = open(data.path());
    #[cfg(feature = "fixture-probes")]
    library.hold_initial_previews_for_probe(true);
    let handle = filesystem::open_directory_readonly(source.path()).unwrap();
    let identity = filesystem::identity(&handle).unwrap();
    drop(handle);
    assert_eq!(
        library
            .register_verified(
                source.path().to_owned(),
                (identity.clone(), "wrong-volume".into()),
                "replaced volume".into(),
                RootMode::Referenced,
                ScanFilter::default()
            )
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert!(library.status().unwrap().roots.is_empty());
    let job = library
        .register(
            source.path().to_owned(),
            Some(identity.clone()),
            "Windows fixture".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        )
        .unwrap();
    wait(&library, &job.id);
    let page = library.query(AssetQuery::default(), None, 240).unwrap();
    assert_eq!(
        page.total,
        3,
        "{:?}",
        library.job_details(job.id.clone()).unwrap()
    );
    assert!(
        page.assets
            .iter()
            .all(|a| a.metadata_state == "ready" && a.width == Some(64) && a.height == Some(32))
    );
    assert!(page.assets.iter().all(|a| matches!(
        a.date_source.as_str(),
        "filesystem_modified" | "filesystem_created"
    )));
    let root_id = page.assets[0].root_id.clone();
    let removal = library
        .prepare_maintenance(MaintenanceTarget::Library {
            root_id: root_id.clone(),
        })
        .unwrap();
    assert_eq!(removal.asset_count, 4);
    assert!(library.root_exists(root_id.clone()).unwrap());
    assert!(library.status().unwrap().jobs.iter().all(|job| !matches!(
        job.state.as_str(),
        "queued" | "running" | "cancel_requested"
    )));
    #[cfg(feature = "fixture-probes")]
    library.hold_initial_previews_for_probe(false);
    library.generate_thumbnails(root_id.clone()).unwrap();
    // Destination preflight is read-only and bound to this exact preview.
    settle(&library);
    let template = stillharbour_core::organization::OrganizationTemplate {
        pattern: "{original_stem}.{ext}".into(),
        event: String::new(),
        missing_date: stillharbour_core::organization::MissingDatePolicy::Undated,
    };
    let ids = page
        .assets
        .iter()
        .map(|asset| asset.id.clone())
        .collect::<Vec<_>>();
    let preview = library
        .organization_preview(ids.clone(), template.clone())
        .unwrap();
    let destination = tempfile::tempdir().unwrap();
    fs::write(
        destination.path().join("destination-only.txt"),
        b"retain destination",
    )
    .unwrap();
    let check = |path: &Path, fingerprint: String, volume_override: Option<&str>| {
        let handle = filesystem::open_directory_readonly(path).unwrap();
        let expected = (
            filesystem::identity(&handle).unwrap(),
            volume_override.map(str::to_owned).unwrap_or_else(|| {
                stillharbour_core::library::native::volume_token(&handle).unwrap()
            }),
        );
        drop(handle);
        library.organization_destination(
            ids.clone(),
            template.clone(),
            fingerprint,
            path.to_owned(),
            expected,
        )
    };
    let review = check(destination.path(), preview.fingerprint.clone(), None).unwrap();
    assert!(!review.execution_available && !review.persisted);
    assert_eq!(review.preview_fingerprint, preview.fingerprint);
    assert_eq!(
        review.fingerprint,
        check(destination.path(), preview.fingerprint.clone(), None)
            .unwrap()
            .fingerprint
    );
    for path in [
        source.path(),
        &source.path().join("nested"),
        data.path(),
        destination.path().parent().unwrap(),
    ] {
        assert!(check(path, preview.fingerprint.clone(), None).is_err());
    }
    assert_eq!(
        check(destination.path(), "stale".into(), None)
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(
        check(
            destination.path(),
            preview.fingerprint.clone(),
            Some("wrong-volume")
        )
        .unwrap_err()
        .code,
        ErrorCode::Conflict
    );
    assert_eq!(
        fs::read(destination.path().join("destination-only.txt")).unwrap(),
        b"retain destination"
    );
    assert_eq!(fs::read_dir(destination.path()).unwrap().count(), 1);
    // Saved versions and drafts change only the catalog, survive reopening and
    // retain the old template even when a new version is saved.
    use stillharbour_core::operations::{
        PlanDraftInput, PlanExportFormat, PlanState, TemplateSave,
    };
    let saved = library
        .organization_template_save(TemplateSave {
            samples: vec![stillharbour_core::organization::TemplateSample {
                relative_path: "Family/Photo.JPG".into(),
                capture: None,
                sequence: 1,
                expected_target: "Photo.JPG".into(),
            }],
            previous: None,
            name: "Fixture template".into(),
            description: "Generated sources".into(),
            template: template.clone(),
        })
        .unwrap();
    let handle = filesystem::open_directory_readonly(destination.path()).unwrap();
    let expected = (
        filesystem::identity(&handle).unwrap(),
        stillharbour_core::library::native::volume_token(&handle).unwrap(),
    );
    drop(handle);
    let draft = library
        .organization_plan_create(
            PlanDraftInput {
                asset_ids: ids.clone(),
                template: template.clone(),
                preview_fingerprint: preview.fingerprint.clone(),
                template_reference: Some(saved.reference.clone()),
            },
            destination.path().to_owned(),
            expected,
        )
        .unwrap();
    assert_eq!(draft.plan.state, PlanState::Draft);
    assert!(!draft.plan.execution_available && !draft.stale);
    assert_eq!(draft.plan.blockers.len(), 4);
    let companion_source = draft
        .plan
        .preview
        .rows
        .iter()
        .find(|row| row.source_path.ends_with("one.png"))
        .unwrap();
    let companion_inventory = library
        .organization_bundle_inventory(
            companion_source.asset_id.clone(),
            companion_source.source_revision.clone(),
        )
        .unwrap();
    assert!(companion_inventory.inventory_complete);
    assert!(
        companion_inventory
            .entries
            .iter()
            .any(|entry| entry.name == "one.xmp")
    );
    let companion_decision = library
        .organization_bundle_confirm(stillharbour_core::operations::BundleConfirmationInput {
            plan_id: draft.plan.id.clone(),
            asset_id: companion_source.asset_id.clone(),
            source_revision: companion_source.source_revision.clone(),
            inventory_fingerprint: companion_inventory.fingerprint,
            companion_ids: companion_inventory
                .entries
                .iter()
                .filter(|entry| {
                    entry.selectable
                        && entry.relation
                            != stillharbour_core::operations::BundleRelation::Additional
                })
                .map(|entry| entry.id.clone())
                .collect(),
            reviewed_all_entries: true,
        })
        .unwrap();
    assert_eq!(companion_decision.files.len(), 2);
    let next = library
        .organization_template_save(TemplateSave {
            samples: vec![stillharbour_core::organization::TemplateSample {
                relative_path: "Family/Photo.JPG".into(),
                capture: None,
                sequence: 1,
                expected_target: "new/Photo.JPG".into(),
            }],
            previous: Some(saved.reference.clone()),
            name: "Fixture template".into(),
            description: "New version".into(),
            template: stillharbour_core::organization::OrganizationTemplate {
                pattern: "new/{original_stem}.{ext}".into(),
                ..template.clone()
            },
        })
        .unwrap();
    assert_eq!(next.reference.version, 2);
    assert_eq!(library.organization_templates().unwrap().len(), 1);
    assert_eq!(
        library
            .organization_plan(draft.plan.id.clone())
            .unwrap()
            .plan
            .template_reference
            .unwrap()
            .version,
        1
    );
    let json = library
        .organization_plan_export(draft.plan.id.clone(), PlanExportFormat::Json)
        .unwrap();
    assert!(!json.text.contains("destination_native"));
    let export_folder = tempfile::tempdir().unwrap();
    std::fs::write(
        export_folder.path().join("destination-only"),
        b"retain export neighbor",
    )
    .unwrap();
    let export_handle = filesystem::open_directory_readonly(export_folder.path()).unwrap();
    let export_expected = (
        filesystem::identity(&export_handle).unwrap(),
        stillharbour_core::library::native::volume_token(&export_handle).unwrap(),
    );
    drop(export_handle);
    let exported = library
        .organization_plan_save_export(
            draft.plan.id.clone(),
            PlanExportFormat::Json,
            export_folder.path().to_owned(),
            export_expected.clone(),
        )
        .unwrap();
    assert_eq!(std::fs::read(&exported.path).unwrap(), json.text.as_bytes());
    assert_eq!(
        exported.digest,
        blake3::hash(json.text.as_bytes()).to_hex().to_string()
    );
    assert_eq!(
        library
            .organization_plan_save_export(
                draft.plan.id.clone(),
                PlanExportFormat::Json,
                export_folder.path().to_owned(),
                export_expected
            )
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(std::fs::read(&exported.path).unwrap(), json.text.as_bytes());
    assert_eq!(
        std::fs::read(export_folder.path().join("destination-only")).unwrap(),
        b"retain export neighbor"
    );
    let forbidden_handle = filesystem::open_directory_readonly(source.path()).unwrap();
    let forbidden_expected = (
        filesystem::identity(&forbidden_handle).unwrap(),
        stillharbour_core::library::native::volume_token(&forbidden_handle).unwrap(),
    );
    drop(forbidden_handle);
    assert!(
        library
            .organization_plan_save_export(
                draft.plan.id.clone(),
                PlanExportFormat::Json,
                source.path().to_owned(),
                forbidden_expected
            )
            .is_err()
    );
    let csv = library
        .organization_plan_export(draft.plan.id.clone(), PlanExportFormat::Csv)
        .unwrap();
    assert_eq!(csv.text.lines().count(), 4);
    library
        .annotate(
            AssetSelection::Ids {
                ids: vec![ids[0].clone()],
            },
            AnnotationPatch {
                favorite: Some(true),
                rating: None,
                tags: None,
            },
        )
        .unwrap();
    assert!(
        library
            .organization_plan(draft.plan.id.clone())
            .unwrap()
            .stale
    );
    let frozen_digest = draft.plan.digest.clone();
    settle(&library);
    drop(library);
    drop(catalog);
    let (catalog, library) = open(data.path());
    let reopened = library.organization_plan(draft.plan.id.clone()).unwrap();
    let reopened_companions = library
        .organization_bundle_reviews(draft.plan.id.clone(), companion_decision.asset_id.clone())
        .unwrap();
    assert_eq!(
        reopened_companions[0].fingerprint,
        companion_decision.fingerprint
    );
    let retained_templates = library.organization_templates().unwrap();
    assert_eq!(retained_templates[0].reference.version, 2);
    assert_eq!(
        retained_templates[0].policies.sample_outputs,
        ["new/Photo.JPG"]
    );
    assert_eq!(saved.policies.sample_outputs, ["Photo.JPG"]);
    assert!(reopened.stale);
    assert_eq!(reopened.plan.digest, frozen_digest);
    assert_eq!(
        library.organization_plans(None).unwrap().plans[0].id,
        draft.plan.id
    );
    assert!(
        library
            .register(
                source.path().join("nested"),
                None,
                "overlap".into(),
                RootMode::Referenced,
                ScanFilter::default()
            )
            .is_err()
    );
    let wrong = tempfile::tempdir().unwrap();
    assert_eq!(
        library
            .register(
                wrong.path().to_owned(),
                Some(identity),
                "changed".into(),
                RootMode::Referenced,
                ScanFilter::default()
            )
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    let filter = ScanFilter {
        excluded_folders: vec!["nested".into()],
        ..Default::default()
    };
    let scan = library.scan(root_id.clone(), filter).unwrap();
    wait(&library, &scan.id);
    settle(&library);
    assert_eq!(
        library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .total,
        1
    );
    let scan = library.scan(root_id, ScanFilter::default()).unwrap();
    wait(&library, &scan.id);
    assert_eq!(
        library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .total,
        3
    );
    for (path, bytes) in originals {
        assert_eq!(fs::read(source.path().join(path)).unwrap(), bytes);
    }
    settle(&library);
    library.cancel_all();
    drop(library);
    drop(catalog);
}

#[test]
fn missing_and_replaced_ntfs_roots_retain_inventory_and_require_identity_match() {
    let fixture = tempfile::tempdir().unwrap();
    let source = fixture.path().join("source");
    let offline = fixture.path().join("offline-owned-source");
    fs::create_dir(&source).unwrap();
    image::RgbImage::from_pixel(32, 24, image::Rgb([70, 40, 100]))
        .save(source.join("photo.png"))
        .unwrap();
    let baseline = fs::read(source.join("photo.png")).unwrap();
    let data = tempfile::tempdir().unwrap();
    let (catalog, library) = open(data.path());
    let job = library
        .register(
            source.clone(),
            None,
            "Generated offline control".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        )
        .unwrap();
    wait(&library, &job.id);
    settle(&library);
    let root_id = library.status().unwrap().roots[0].id.clone();
    let destination = tempfile::tempdir().unwrap();
    let destination_review = || {
        let ids = library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .assets
            .into_iter()
            .map(|asset| asset.id)
            .collect::<Vec<_>>();
        let template = stillharbour_core::organization::OrganizationTemplate {
            pattern: "{original_stem}.{ext}".into(),
            event: String::new(),
            missing_date: stillharbour_core::organization::MissingDatePolicy::Undated,
        };
        let preview = library
            .organization_preview(ids.clone(), template.clone())
            .unwrap();
        let handle = filesystem::open_directory_readonly(destination.path()).unwrap();
        let expected = (
            filesystem::identity(&handle).unwrap(),
            stillharbour_core::library::native::volume_token(&handle).unwrap(),
        );
        drop(handle);
        library.organization_destination(
            ids,
            template,
            preview.fingerprint,
            destination.path().to_owned(),
            expected,
        )
    };
    // External fixture changes only, under this run's owned parent.
    fs::rename(&source, &offline).unwrap();
    assert_eq!(library.status().unwrap().roots[0].availability, "offline");
    assert!(destination_review().is_err());
    assert_eq!(
        library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .total,
        1
    );
    assert!(
        library
            .scan(root_id.clone(), ScanFilter::default())
            .is_err()
    );
    fs::create_dir(&source).unwrap();
    assert_eq!(
        library.status().unwrap().roots[0].availability,
        "needs_relink"
    );
    assert_eq!(destination_review().unwrap_err().code, ErrorCode::Conflict);
    assert_eq!(
        library
            .scan(root_id.clone(), ScanFilter::default())
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(
        library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .total,
        1
    );
    fs::remove_dir(&source).unwrap(); // only the positively owned, empty replacement
    fs::rename(&offline, &source).unwrap();
    let job = library
        .scan(root_id.clone(), ScanFilter::default())
        .unwrap();
    wait(&library, &job.id);
    settle(&library);
    assert_eq!(library.status().unwrap().roots[0].id, root_id);
    assert_eq!(fs::read(source.join("photo.png")).unwrap(), baseline);
    library.cancel_all();
    drop(library);
    drop(catalog);
}

#[test]
#[cfg(feature = "fixture-probes")]
fn shared_desktop_fixture_uses_valid_windows_names_and_preserves_its_baseline() {
    let fixture = stillharbour_core::library_fixture::LibraryFixture::generate().unwrap();
    assert!(fixture.verify_unchanged().unwrap() > 20);
}

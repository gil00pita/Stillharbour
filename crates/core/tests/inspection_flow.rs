#![cfg(any(windows, target_os = "macos"))]
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use stillharbour_core::{
    catalog::Catalog,
    inspection::{InspectionScope, model::*},
    library::{Library, LibraryOptions, WorkerPaths, model::*},
};
fn wait(library: &Library, id: String) -> InspectionJob {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let job = library.inspection_job(id.clone()).unwrap();
        if !matches!(
            job.state,
            InspectionState::Queued | InspectionState::Running | InspectionState::CancelRequested
        ) {
            assert!(
                matches!(
                    job.state,
                    InspectionState::Completed | InspectionState::Partial
                ),
                "{}",
                job.message
            );
            return job;
        }
        assert!(Instant::now() < deadline, "inspection deadline");
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn generated_native_jobs_use_the_supervised_decoder_and_preserve_all_source_bytes() {
    let owned = tempfile::tempdir().unwrap();
    let source = owned.path().join("source");
    let data = owned.path().join("data");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(&data).unwrap();
    let valid = source.join("valid.png");
    image::RgbImage::from_pixel(4, 3, image::Rgb([12, 34, 56]))
        .save(&valid)
        .unwrap();
    let bytes = std::fs::read(&valid).unwrap();
    std::fs::write(source.join("copy.dat"), &bytes).unwrap();
    std::fs::write(source.join("embedded-preview.dng"), &bytes).unwrap();
    let idat = bytes.windows(4).position(|part| part == b"IDAT").unwrap();
    std::fs::write(source.join("truncated.png"), &bytes[..idat + 4]).unwrap();
    image::RgbImage::from_pixel(12001, 1, image::Rgb([1, 2, 3]))
        .save(source.join("oversized.png"))
        .unwrap();
    let originals = std::fs::read_dir(&source)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            assert!(path.starts_with(owned.path()));
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect::<Vec<_>>();
    let catalog = Arc::new(Catalog::open(data.join("Fixture.stillharbour")).unwrap());
    let library = Library::new(
        catalog,
        LibraryOptions {
            data_directory: data.clone(),
            workers: WorkerPaths {
                executable: Path::new(env!("CARGO_BIN_EXE_stillharbour-media-worker")).to_owned(),
                exiftool: None,
                working_directory: data.join("worker"),
            },
        },
    )
    .unwrap();
    let scan = library
        .register(
            source,
            None,
            "Generated inspection source".into(),
            RootMode::Referenced,
            ScanFilter {
                extensions: vec!["notindexed".into()],
                ..ScanFilter::default()
            },
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let status = library.status().unwrap();
        if status.jobs.iter().any(|job| {
            job.id == scan.id
                && !matches!(
                    job.state.as_str(),
                    "queued" | "running" | "cancel_requested"
                )
        }) {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    let root = library.status().unwrap().roots[0].id.clone();
    let run = |task| {
        let job = library
            .inspection_start(InspectionInput {
                root_ids: vec![root.clone()],
                scope: InspectionScope::AllEligibleFiles,
                task,
            })
            .unwrap();
        wait(&library, job.id)
    };
    let deep = run(InspectionTask::Health {
        mode: HealthMode::Deep,
    });
    let rows = library.inspection_findings(deep.id, None).unwrap().rows;
    assert_eq!(
        rows.iter()
            .find(|row| row.name == "valid.png")
            .unwrap()
            .content,
        ContentState::DeepPass
    );
    assert_eq!(
        rows.iter()
            .find(|row| row.name == "embedded-preview.dng")
            .unwrap()
            .content,
        ContentState::Unsupported
    );
    assert_eq!(
        rows.iter()
            .find(|row| row.name == "truncated.png")
            .unwrap()
            .content,
        ContentState::ConfirmedDecodeFailure
    );
    assert_eq!(
        rows.iter()
            .find(|row| row.name == "oversized.png")
            .unwrap()
            .content,
        ContentState::Unsupported
    );
    let quick = run(InspectionTask::Health {
        mode: HealthMode::Quick,
    });
    let rows = library.inspection_findings(quick.id, None).unwrap().rows;
    assert_eq!(
        rows.iter()
            .find(|row| row.name == "truncated.png")
            .unwrap()
            .content,
        ContentState::QuickPass
    );
    let pixels = run(InspectionTask::Duplicates {
        options: DuplicateOptions {
            mode: MatchMode::NormalizedPixels,
            algorithm: ContentAlgorithm::Blake3,
            rules: PropertyRules {
                logic: RuleLogic::All,
                filename: false,
                extension: false,
                size: false,
                modified: false,
                created: false,
                readonly: false,
                content: true,
                tolerance_millis: 0,
                name: NamePolicy::default(),
            },
            exclude_hardlinked_objects: false,
        },
    });
    let groups = library
        .inspection_groups(pixels.id.clone(), None)
        .unwrap()
        .groups;
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].evidence, DuplicateEvidence::NormalizedPixels);
    assert!(!groups[0].mutation_available);
    assert_eq!(groups[0].member_count, 2);
    let group = &groups[0];
    let members = library
        .inspection_group_members(pixels.id.clone(), group.id.clone(), None)
        .unwrap();
    let protected = members.rows[0].folder_id.clone();
    let review = library
        .inspection_keeper_review(stillharbour_core::inspection::KeeperReviewInput {
            job_id: pixels.id.clone(),
            group_id: group.id.clone(),
            order: stillharbour_core::inspection::KeeperOrder::OldestModified,
            manual_keepers: Vec::new(),
            protected_folders: vec![protected],
            preferred_folders: Vec::new(),
        })
        .unwrap();
    assert_eq!(review.keepers.len(), 2);
    assert!(!review.mutation_available);
    assert_eq!(
        library
            .inspection_keeper_latest(pixels.id.clone(), group.id.clone())
            .unwrap()
            .unwrap()
            .id,
        review.id
    );
    let mut forged = review.input;
    forged.protected_folders = vec!["f".repeat(64)];
    assert!(library.inspection_keeper_review(forged).is_err());
    for (path, before) in originals {
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
}

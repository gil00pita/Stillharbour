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
    // External fixture changes only, under this run's owned parent.
    fs::rename(&source, &offline).unwrap();
    assert_eq!(library.status().unwrap().roots[0].availability, "offline");
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

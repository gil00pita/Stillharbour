#![cfg(any(windows, target_os = "macos"))]
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use stillharbour_core::{
    backup::comparison::*,
    catalog::Catalog,
    filesystem,
    library::{
        Library, LibraryOptions, WorkerPaths,
        model::{RootMode, ScanFilter},
    },
};
fn wait(library: &Library, id: &str) -> ComparisonRun {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let run = library.backup_comparison(id.into()).unwrap();
        if run.state != ComparisonState::Comparing {
            return run;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn comparison_previews_are_handle_authorized_bounded_and_preserve_both_sides() {
    let owned = tempfile::tempdir().unwrap();
    let parent = owned.path().canonicalize().unwrap();
    let source = parent.join("source");
    let destination = parent.join("destination");
    let data = parent.join("data");
    for path in [&source, &destination, &data] {
        std::fs::create_dir(path).unwrap();
    }
    image::RgbImage::from_pixel(24, 16, image::Rgb([10, 20, 30]))
        .save(source.join("photo.png"))
        .unwrap();
    std::fs::write(source.join("photo.xmp"), b"unique sidecar").unwrap();
    std::fs::write(source.join("damaged.png"), b"unsupported damaged payload").unwrap();
    let original = std::fs::read(source.join("photo.png")).unwrap();
    let catalog = Arc::new(Catalog::open(data.join("Fixture.stillharbour")).unwrap());
    let options = LibraryOptions {
        data_directory: data.clone(),
        workers: WorkerPaths {
            executable: Path::new(env!("CARGO_BIN_EXE_stillharbour-media-worker")).to_owned(),
            exiftool: None,
            working_directory: data.join("worker"),
        },
    };
    let library = Library::new(catalog.clone(), options).unwrap();
    let scan = library
        .register(
            source.clone(),
            None,
            "Generated comparison source".into(),
            RootMode::Referenced,
            ScanFilter {
                extensions: vec!["notindexed".into()],
                ..Default::default()
            },
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while library.status().unwrap().jobs.iter().any(|j| {
        j.id == scan.id && matches!(j.state.as_str(), "queued" | "running" | "cancel_requested")
    }) {
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let root = library.status().unwrap().roots[0].id.clone();
    let dataset = destination
        .join(".stillharbour-copy")
        .join(catalog.summary().unwrap().id)
        .join(root);
    assert!(dataset.starts_with(&parent));
    std::fs::create_dir_all(&dataset).unwrap();
    image::RgbImage::from_pixel(24, 16, image::Rgb([70, 80, 90]))
        .save(dataset.join("photo.png"))
        .unwrap();
    let retained = std::fs::read(dataset.join("photo.png")).unwrap();
    let handle = filesystem::open_directory_readonly(&destination).unwrap();
    let expected = (
        filesystem::identity(&handle).unwrap(),
        stillharbour_core::library::native::volume_token(&handle).unwrap(),
    );
    let run = library
        .backup_compare_start(destination, expected, CompareMode::Verified)
        .unwrap();
    let done = wait(&library, &run.id);
    assert_eq!(done.state, ComparisonState::NeedsReview, "{}", done.message);
    let page = library.backup_comparison_page(run.id.clone(), 0).unwrap();
    let entry = page
        .entries
        .iter()
        .find(|e| e.display_path == "photo.png")
        .unwrap()
        .id
        .clone();
    for side in [ComparisonSide::Source, ComparisonSide::Destination] {
        let library = library.clone();
        let id = run.id.clone();
        let entry = entry.clone();
        let preview =
            tokio::task::spawn_blocking(move || library.backup_comparison_preview(id, entry, side))
                .await
                .unwrap()
                .unwrap();
        assert!(preview.png.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(preview.png.len() <= 2 * 1024 * 1024);
        assert!(preview.message.contains("not integrity"));
    }
    assert_eq!(std::fs::read(source.join("photo.png")).unwrap(), original);
    assert!(
        library
            .backup_comparison_preview(run.id.clone(), "foreign".into(), ComparisonSide::Source)
            .is_err()
    );
    std::fs::write(source.join("photo.png"), b"late source occupant").unwrap();
    assert!(
        library
            .backup_comparison_preview(run.id.clone(), entry, ComparisonSide::Source)
            .is_err()
    );
    let damaged = page
        .entries
        .iter()
        .find(|e| e.display_path == "damaged.png")
        .unwrap()
        .id
        .clone();
    let copy = library.clone();
    let id = run.id.clone();
    assert!(
        tokio::task::spawn_blocking(move || copy.backup_comparison_preview(
            id,
            damaged,
            ComparisonSide::Source
        ))
        .await
        .unwrap()
        .is_err()
    );
    assert_eq!(
        std::fs::read(source.join("photo.png")).unwrap(),
        b"late source occupant"
    );
    assert!(!original.is_empty());
    assert_eq!(std::fs::read(dataset.join("photo.png")).unwrap(), retained);
    assert_eq!(
        std::fs::read(source.join("photo.xmp")).unwrap(),
        b"unique sidecar"
    );
    assert_eq!(
        std::fs::read(source.join("damaged.png")).unwrap(),
        b"unsupported damaged payload"
    );
}

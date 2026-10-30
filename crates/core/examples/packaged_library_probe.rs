#[cfg(target_os = "macos")]
fn main() {
    use std::{
        path::PathBuf,
        sync::Arc,
        time::{Duration, Instant},
    };
    use stillharbour_core::{
        catalog::Catalog,
        library::{Library, LibraryOptions, THUMBNAIL_POLICY, WorkerPaths, model::*},
        library_fixture::LibraryFixture,
    };
    let app = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .expect("Supply the local built .app bundle; no source paths are accepted"),
    );
    let app = app
        .canonicalize()
        .expect("Resolve the selected app bundle before changing worker directories");
    let executable = app.join("Contents/MacOS/stillharbour");
    let exiftool = app.join("Contents/Resources/helpers/exiftool/exiftool");
    assert!(executable.is_file() && exiftool.is_file());
    let fixture = LibraryFixture::generate_native_formats().unwrap();
    let data = tempfile::tempdir().unwrap();
    let catalog = Arc::new(Catalog::open(data.path().join("generated.stillharbour")).unwrap());
    let library = Library::new(
        catalog,
        LibraryOptions {
            data_directory: data.path().to_owned(),
            workers: WorkerPaths {
                executable,
                exiftool: Some(exiftool),
                working_directory: data.path().join("worker"),
            },
        },
    )
    .unwrap();
    let started = Instant::now();
    let job = library
        .register(
            fixture.path().to_owned(),
            None,
            "Packaged generated fixtures".into(),
            RootMode::Referenced,
            ScanFilter {
                excluded_folders: vec!["Private".into()],
                ..Default::default()
            },
        )
        .unwrap();
    loop {
        let job = library.job_details(job.id.clone()).unwrap().job;
        if !matches!(job.state.as_str(), "running" | "cancel_requested") {
            assert!(matches!(job.state.as_str(), "completed" | "partial"));
            break;
        }
        assert!(started.elapsed() < Duration::from_secs(45));
        std::thread::sleep(Duration::from_millis(20));
    }
    let scan_ms = started.elapsed().as_secs_f64() * 1000.;
    let page = library.query(AssetQuery::default(), None, 240).unwrap();
    assert_eq!(page.total, 24);
    let dated = page
        .assets
        .iter()
        .find(|a| a.relative_path == "Coast/Generated-00.jpg")
        .unwrap();
    assert_eq!(dated.date, "1964-02-29T13:15:20.1234");
    assert_eq!(dated.offset.as_deref(), Some("+05:30"));
    let keys: Vec<_> = ["jpg", "png", "webp"]
        .into_iter()
        .map(|extension| {
            let asset = page
                .assets
                .iter()
                .find(|a| a.extension == extension && a.width.is_some())
                .unwrap();
            ThumbnailKey {
                asset_id: asset.id.clone(),
                revision: asset.revision.clone(),
                edge: 256,
            }
        })
        .collect();
    let start = Instant::now();
    loop {
        let results = library.thumbnails(keys.clone(), "1".into()).unwrap();
        assert!(
            results
                .iter()
                .all(|r| r.state == "pending" || r.state == "ready"),
            "{results:?}"
        );
        if results.iter().all(|r| r.state == "ready") {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(25));
        std::thread::sleep(Duration::from_millis(30));
    }
    let preview_ms = start.elapsed().as_secs_f64() * 1000.;
    let bytes = keys
        .into_iter()
        .map(|key| {
            let bytes = library.cached_thumbnail(key, THUMBNAIL_POLICY).unwrap();
            let image = image::load_from_memory(&bytes).unwrap();
            assert!(image.width() <= 256 && image.height() <= 256);
            bytes.len()
        })
        .sum::<usize>();
    let mut original_formats = Vec::new();
    for extension in ["jpg", "png", "webp", "heic"] {
        let asset = page
            .assets
            .iter()
            .find(|a| a.extension == extension && a.width.is_some())
            .unwrap();
        let original = library
            .original_image(
                asset.id.clone(),
                asset.revision.clone(),
                stillharbour_core::library::ORIGINAL_IMAGE_POLICY,
            )
            .unwrap();
        assert!(!original.bytes.is_empty());
        if extension == "heic" {
            assert_eq!(asset.width, Some(3072));
            assert_eq!(asset.height, Some(2048));
            assert_eq!(original.mime, "image/heic");
            assert_eq!(
                original.bytes,
                std::fs::read(fixture.path().join("Native HEIC.heic")).unwrap()
            );
        }
        original_formats.push(original.mime);
    }
    let exact = library
        .original_image(
            dated.id.clone(),
            dated.revision.clone(),
            stillharbour_core::library::ORIGINAL_IMAGE_POLICY,
        )
        .unwrap();
    assert_eq!(
        exact.bytes,
        std::fs::read(fixture.path().join("Coast/Generated-00.jpg")).unwrap()
    );
    let verified = fixture.verify_unchanged().unwrap();
    println!(
        "{}",
        serde_json::json!({"kind":"packaged-generated-library","indexedMedia":page.total,"previewFormats":["jpeg","png","webp"],"scanMs":scan_ms,"previewMs":preview_ms,"cachedPngBytes":bytes,"sourceEntriesUnchanged":verified,"originalFormats":original_formats,"originalJpegExact":true,"originalHeicExact":true,"originalHeicDimensions":[3072,2048],"bundledMetadataDate":dated.date,"offset":dated.offset})
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Packaged native probe requires the validated macOS adapter.");
}

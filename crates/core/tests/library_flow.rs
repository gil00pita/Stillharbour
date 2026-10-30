#![cfg(all(target_os = "macos", feature = "fixture-probes"))]
use std::{
    collections::HashSet,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use stillharbour_core::{
    catalog::Catalog,
    domain::ErrorCode,
    library::{Library, LibraryOptions, THUMBNAIL_POLICY, WorkerPaths, model::*},
    library_fixture::LibraryFixture,
};

fn open(data: &Path) -> (Arc<Catalog>, Arc<Library>) {
    let catalog = Arc::new(Catalog::open(data.join("fixture.stillharbour")).unwrap());
    let library = Library::new(
        catalog.clone(),
        LibraryOptions {
            data_directory: data.to_owned(),
            workers: WorkerPaths {
                executable: Path::new(env!("CARGO_BIN_EXE_stillharbour-media-worker")).to_owned(),
                exiftool: Some(
                    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/exiftool/exiftool"),
                ),
                working_directory: data.join("worker"),
            },
        },
    )
    .unwrap();
    (catalog, library)
}
// Fixture restart boundary; production still rejects genuinely open catalogs.
fn close(library: Arc<Library>, catalog: Arc<Catalog>) {
    library.cancel_all();
    drop(library);
    let start = Instant::now();
    let mut owner = catalog;
    loop {
        match Arc::try_unwrap(owner) {
            Ok(catalog) => {
                // Drop the last owner here so Catalog::drop joins its actor
                // before the caller reopens the fixture's lifetime lock.
                drop(catalog);
                return;
            }
            Err(shared) => {
                owner = shared;
                assert!(
                    start.elapsed() < Duration::from_secs(45),
                    "background catalog owners did not stop before fixture restart"
                );
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

#[test]
fn fixture_restart_waits_for_the_last_background_catalog_owner() {
    let data = tempfile::tempdir().unwrap();
    let (catalog, library) = open(data.path());
    let identity = catalog.summary().unwrap().id;
    // Model a preview worker paused after publishing its result, before it
    // releases its Arc. The test controls release, not a guessed worker delay.
    let background_owner = library.clone();
    let (started, start) = std::sync::mpsc::sync_channel(0);
    let (finished, finish) = std::sync::mpsc::sync_channel(1);
    let closer = std::thread::spawn(move || {
        started.send(()).unwrap();
        close(library, catalog);
        finished.send(()).unwrap();
    });
    start.recv_timeout(Duration::from_secs(5)).unwrap();
    let premature_open = finish
        .recv_timeout(Duration::from_millis(300))
        .ok()
        .map(|()| {
            Catalog::open(data.path().join("fixture.stillharbour"))
                .map(|_| ())
                .map_err(|error| error.code)
        });
    let still_locked = Catalog::open(data.path().join("fixture.stillharbour"))
        .err()
        .map(|error| error.code);
    drop(background_owner);
    closer.join().unwrap();
    assert_eq!(still_locked, Some(ErrorCode::CatalogInUse));
    assert_eq!(
        premature_open, None,
        "fixture restart raced a legitimate catalog owner"
    );
    let (catalog, _) = open(data.path());
    assert_eq!(catalog.summary().unwrap().id, identity);
}

fn filter() -> ScanFilter {
    ScanFilter {
        excluded_folders: vec!["Private".into()],
        ..Default::default()
    }
}
fn wait(library: &Arc<Library>, id: &str) -> ScanJob {
    let start = Instant::now();
    loop {
        let job = library.job_details(id.into()).unwrap().job;
        if !matches!(
            job.state.as_str(),
            "queued" | "running" | "cancel_requested"
        ) {
            // A completed scan now queues a separate derived-cache job. Fixture
            // maintenance/reopen assertions require both background owners gone.
            while Arc::strong_count(library) > 1 {
                assert!(
                    start.elapsed() < Duration::from_secs(45),
                    "background work stalled"
                );
                std::thread::sleep(Duration::from_millis(20));
            }
            return job;
        }
        assert!(
            start.elapsed() < Duration::from_secs(45),
            "scan stalled: {job:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn register(library: &Arc<Library>, fixture: &LibraryFixture) -> ScanJob {
    library
        .register(
            fixture.path().to_owned(),
            None,
            "Generated test library".into(),
            RootMode::Referenced,
            filter(),
        )
        .unwrap()
}
#[test]
fn first_import_reveals_only_published_previews_and_preserves_sources() {
    let fixture = LibraryFixture::generate().unwrap();
    let before = independent_manifest(fixture.path());
    let data = tempfile::tempdir().unwrap();
    let (catalog, library) = open(data.path());
    let scan = register(&library, &fixture);
    let query = AssetQuery {
        root_id: Some(scan.root_id.clone()),
        defer_pending_previews: Some(true),
        ..Default::default()
    };
    let initial = format!("thumbnail:initial:{}", scan.root_id);
    let start = Instant::now();
    let mut saw_ready = false;
    loop {
        let page = library.query(query.clone(), None, 240).unwrap();
        for asset in page.assets {
            assert_ne!(
                asset.preview_state, "pending",
                "An unprepared photo entered the initial collection"
            );
            if asset.preview_state == "ready" {
                let bytes = library
                    .cached_thumbnail(
                        ThumbnailKey {
                            asset_id: asset.id,
                            revision: asset.revision,
                            edge: 32,
                        },
                        THUMBNAIL_POLICY,
                    )
                    .unwrap();
                assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
                saw_ready = true;
            }
        }
        let status = library.status().unwrap();
        let job = library.job_details(initial.clone()).unwrap().job;
        let scanning = status.jobs.iter().any(|j| {
            j.root_id == scan.root_id
                && !matches!(j.kind, Some(JobKind::Thumbnails))
                && matches!(j.state.as_str(), "running" | "queued" | "cancel_requested")
        });
        if !scanning && matches!(job.state.as_str(), "completed" | "partial") {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(45),
            "Initial preparation did not finish: {job:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    // The last publication can fall between the final query and job completion.
    let ready = library
        .query(query, None, 240)
        .unwrap()
        .assets
        .into_iter()
        .filter(|a| a.preview_state == "ready")
        .count();
    assert!(saw_ready || ready > 0);
    assert!(
        !library.status().unwrap().roots[0]
            .preparing_previews
            .unwrap_or(false)
    );
    assert_eq!(before, independent_manifest(fixture.path()));
    fixture.verify_unchanged().unwrap();
    close(library, catalog);
}

#[test]
fn native_scan_keysets_notes_previews_offline_and_restart_preserve_sources() {
    let fixture = LibraryFixture::generate().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (catalog, library) = open(data.path());
    let catalog_id = catalog.summary().unwrap().id;
    std::fs::write(
        data.path().join("worker/.ExifTool_config"),
        b"die \"Unwanted configuration loaded\";",
    )
    .unwrap();
    let job = register(&library, &fixture);
    let job = wait(&library, &job.id);
    assert!(
        matches!(job.state.as_str(), "completed" | "partial"),
        "{job:?}"
    );
    let status = library.status().unwrap();
    assert_eq!(status.roots.len(), 1);
    assert!(status.metadata_available);
    let root = &status.roots[0];
    assert_eq!(root.mode, RootMode::Referenced);
    assert!(root.is_base);
    assert_eq!(
        library
            .register(
                fixture.path().join("Coast"),
                None,
                "overlap".into(),
                RootMode::Managed,
                filter()
            )
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    let alias = fixture
        .path()
        .with_file_name(
            fixture
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_ascii_uppercase(),
        )
        .join("Coast");
    if alias.exists() {
        assert_eq!(
            library
                .register(
                    alias,
                    None,
                    "Nested case alias".into(),
                    RootMode::Referenced,
                    filter()
                )
                .unwrap_err()
                .code,
            ErrorCode::Conflict
        );
    }
    let query = AssetQuery::default();
    let first = library.query(query.clone(), None, 5).unwrap();
    assert_eq!(first.total, 20);
    let mut all = first.assets.clone();
    let mut cursor = first.next_cursor.clone();
    while let Some(next) = cursor {
        let page = library.query(query.clone(), Some(next), 5).unwrap();
        all.extend(page.assets);
        cursor = page.next_cursor;
    }
    assert_eq!(all.len(), 20);
    let portrait = all
        .iter()
        .find(|asset| asset.name == "Generated-06.jpg")
        .unwrap();
    assert_eq!((portrait.width, portrait.height), (Some(450), Some(600)));
    let detail = library.detail(portrait.id.clone()).unwrap();
    assert_eq!(
        (detail.asset.width, detail.asset.height),
        (Some(450), Some(600))
    );
    assert_eq!(all.iter().map(|a| &a.id).collect::<HashSet<_>>().len(), 20);
    assert_eq!(
        all.iter()
            .filter(
                |a| (a.name.starts_with("Generated-") || a.name.starts_with("-Generated-"))
                    && a.metadata_state == "ready"
            )
            .count(),
        18
    );
    let different = AssetQuery {
        name: "Generated".into(),
        ..Default::default()
    };
    assert_eq!(
        library
            .query(different, first.next_cursor, 5)
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    let buckets = library.buckets(query.clone()).unwrap();
    let seek = library
        .seek(query.clone(), buckets[0].year.clone(), 5)
        .unwrap();
    assert_eq!(seek.assets[0].id, all[0].id);
    let token = library.freeze(query.clone()).unwrap();
    assert_eq!(token.count, 20);
    assert_eq!(
        library
            .annotate(
                AssetSelection::Token { id: token.id },
                AnnotationPatch {
                    favorite: Some(true),
                    rating: Some(4),
                    tags: Some(vec!["家族".into(), "Coast".into()])
                }
            )
            .unwrap(),
        20
    );
    assert_eq!(
        library
            .query(
                AssetQuery {
                    favorites_only: true,
                    tag: "家族".into(),
                    ..Default::default()
                },
                None,
                240
            )
            .unwrap()
            .total,
        20
    );
    let asset = all
        .iter()
        .find(|a| a.relative_path == "Coast/Generated-00.jpg")
        .unwrap();
    assert_eq!(asset.date, "1964-02-29T13:15:20.1234");
    assert_eq!(asset.offset.as_deref(), Some("+05:30"));
    assert_eq!(asset.date_source, "embedded_original");
    let detail = library.detail(asset.id.clone()).unwrap();
    assert_eq!(detail.asset.rating, 4);
    assert_eq!(detail.companions.len(), 1);
    let key = ThumbnailKey {
        asset_id: asset.id.clone(),
        revision: asset.revision.clone(),
        edge: 256,
    };
    let start = Instant::now();
    loop {
        let result = library.thumbnails(vec![key.clone()], "1".into()).unwrap();
        if result[0].state == "ready" {
            break;
        }
        assert_eq!(result[0].state, "pending", "{result:?}");
        assert!(start.elapsed() < Duration::from_secs(20));
        std::thread::sleep(Duration::from_millis(30));
    }
    let preview = library
        .cached_thumbnail(key.clone(), THUMBNAIL_POLICY)
        .unwrap();
    assert!(preview.starts_with(b"\x89PNG"));
    assert_eq!(
        library.thumbnails(vec![], "0".into()).unwrap_err().code,
        ErrorCode::Busy
    );
    assert!(
        library
            .cached_thumbnail(key.clone(), "unknown-policy")
            .is_err()
    );
    let holder = tempfile::tempdir().unwrap();
    let offline = holder.path().join("offline");
    std::fs::rename(fixture.path(), &offline).unwrap();
    assert_eq!(library.status().unwrap().roots[0].availability, "offline");
    assert_eq!(library.query(query.clone(), None, 240).unwrap().total, 20);
    assert_eq!(
        library
            .cached_thumbnail(key.clone(), THUMBNAIL_POLICY)
            .unwrap(),
        preview
    );
    let wrong = tempfile::tempdir().unwrap();
    assert!(
        library
            .relink(root.id.clone(), wrong.path().to_owned(), None)
            .is_err()
    );
    let relink = library
        .relink(root.id.clone(), offline.clone(), None)
        .unwrap();
    wait(&library, &relink.id);
    assert_eq!(
        library
            .query(query.clone(), None, 240)
            .unwrap()
            .assets
            .iter()
            .find(|a| a.id == asset.id)
            .unwrap()
            .rating,
        4
    );
    std::fs::rename(&offline, fixture.path()).unwrap();
    let relink = library
        .relink(root.id.clone(), fixture.path().to_owned(), None)
        .unwrap();
    wait(&library, &relink.id);
    let previous_jobs: HashSet<_> = library
        .status()
        .unwrap()
        .jobs
        .into_iter()
        .map(|j| j.id)
        .collect();
    std::fs::rename(fixture.path(), &offline).unwrap();
    assert_eq!(library.status().unwrap().roots[0].availability, "offline");
    std::fs::rename(&offline, fixture.path()).unwrap();
    let reconnected = library.status().unwrap();
    let reconciliation = reconnected
        .jobs
        .iter()
        .find(|job| !previous_jobs.contains(&job.id))
        .expect("Reconnect should start a fresh read-only scan");
    wait(&library, &reconciliation.id);
    assert!(fixture.verify_unchanged().unwrap() >= 26);
    close(library, catalog);
    let (catalog, library) = open(data.path());
    assert_eq!(catalog.summary().unwrap().id, catalog_id);
    assert_eq!(library.query(query, None, 240).unwrap().total, 20);
    assert_eq!(library.status().unwrap().roots[0].id, root.id);
    assert!(fixture.verify_unchanged().is_ok());
}

#[test]
fn filters_rescans_cancel_and_authoritative_missing_reconcile_without_mutation() {
    let fixture = LibraryFixture::generate().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (_, library) = open(data.path());
    let first = register(&library, &fixture);
    wait(&library, &first.id);
    let before = library.query(AssetQuery::default(), None, 240).unwrap();
    let original = before
        .assets
        .iter()
        .find(|a| a.name == "Generated-00.jpg")
        .unwrap()
        .clone();
    let scan = library
        .scan(
            first.root_id.clone(),
            ScanFilter {
                from_date: Some("2100-01-01".into()),
                ..filter()
            },
        )
        .unwrap();
    wait(&library, &scan.id);
    assert_eq!(
        library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .total,
        0,
        "date filters must apply to unchanged ready metadata"
    );
    let scan = library.scan(first.root_id.clone(), filter()).unwrap();
    wait(&library, &scan.id);
    assert_eq!(
        library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .total,
        20
    );
    assert!(fixture.verify_unchanged().is_ok());
    // An external actor renames an owned fixture, never application mutation code.
    let old = fixture.path().join("Coast/Generated-00.jpg");
    let renamed = fixture.path().join("Coast/Renamed.jpg");
    std::fs::rename(&old, &renamed).unwrap();
    let scan = library.scan(first.root_id.clone(), filter()).unwrap();
    wait(&library, &scan.id);
    let after = library.query(AssetQuery::default(), None, 240).unwrap();
    let renamed_asset = after
        .assets
        .iter()
        .find(|a| a.name == "Renamed.jpg")
        .unwrap();
    assert_eq!(renamed_asset.id, original.id);
    assert_eq!(renamed_asset.added_at, original.added_at);
    std::fs::rename(&renamed, &old).unwrap();
    let scan = library.scan(first.root_id.clone(), filter()).unwrap();
    library.cancel(scan.id.clone()).unwrap();
    assert_eq!(wait(&library, &scan.id).state, "canceled");
    assert_eq!(
        library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .total,
        20,
        "canceled scans must not infer missing"
    );
    let scan = library.scan(first.root_id.clone(), filter()).unwrap();
    wait(&library, &scan.id);
    let removed = fixture.path().join("Woodland/Generated-01.png");
    let holder = tempfile::tempdir().unwrap();
    let away = holder.path().join("away.png");
    std::fs::rename(&removed, &away).unwrap();
    let scan = library.scan(first.root_id.clone(), filter()).unwrap();
    wait(&library, &scan.id);
    assert_eq!(
        library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .total,
        19
    );
    let missing = library
        .query(
            AssetQuery {
                include_missing: true,
                ..Default::default()
            },
            None,
            240,
        )
        .unwrap();
    assert_eq!(missing.total, 20);
    assert!(
        missing
            .assets
            .iter()
            .any(|a| a.name == "Generated-01.png" && a.presence == "missing")
    );
    std::fs::rename(&away, &removed).unwrap();
    let scan = library.scan(first.root_id.clone(), filter()).unwrap();
    wait(&library, &scan.id);
    let scan = library
        .scan(
            first.root_id.clone(),
            ScanFilter {
                include_hidden_files: true,
                include_hidden_folders: false,
                ..filter()
            },
        )
        .unwrap();
    wait(&library, &scan.id);
    assert_eq!(
        library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .total,
        21
    );
    assert!(fixture.verify_unchanged().is_ok());
}

#[test]
fn native_paths_links_and_replaced_root_fail_closed() {
    use std::os::unix::{ffi::OsStringExt, fs::symlink};
    use stillharbour_core::{filesystem, library::native};
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.jpg"), b"outside source").unwrap();
    symlink(outside.path(), root.path().join("escape")).unwrap();
    let handle = native::open_root(root.path(), None).unwrap();
    let identity = filesystem::identity(&handle).unwrap();
    let volume = native::volume_token(&handle).unwrap();
    assert!(
        native::open_asset(
            root.path(),
            &identity,
            &volume,
            Path::new("escape/secret.jpg")
        )
        .is_err()
    );
    assert!(
        native::open_asset(root.path(), &identity, &volume, Path::new("../secret.jpg")).is_err()
    );
    assert!(native::open_root(outside.path(), Some(&identity)).is_err());
    let lossless =
        std::path::PathBuf::from(std::ffi::OsString::from_vec(b"invalid-\xff.jpg".to_vec()));
    assert_eq!(
        native::decode_path(&native::encode_path(&lossless)).unwrap(),
        lossless
    );
    assert_eq!(
        std::fs::read(outside.path().join("secret.jpg")).unwrap(),
        b"outside source"
    );
}

#[test]
fn migration_retains_a_consistent_v1_snapshot_and_catalog_identity() {
    use rusqlite::Connection;
    let data = tempfile::tempdir().unwrap();
    let path = data.path().join("prior.stillharbour");
    let db = Connection::open(&path).unwrap();
    db.execute_batch("PRAGMA application_id=1398032450; PRAGMA user_version=1; CREATE TABLE catalog_identity(singleton INTEGER PRIMARY KEY,uuid TEXT NOT NULL); INSERT INTO catalog_identity VALUES(1,'unchanged-catalog-identity'); CREATE TABLE job_events(sequence INTEGER PRIMARY KEY,job_id TEXT,state TEXT); INSERT INTO job_events VALUES(1,'old-job','recorded');").unwrap();
    drop(db);
    let catalog = Catalog::open(path).unwrap();
    assert_eq!(catalog.summary().unwrap().id, "unchanged-catalog-identity");
    assert_eq!(catalog.summary().unwrap().schema, 2);
    let snapshot = std::fs::read_dir(data.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("pre-migration-v1-")
        })
        .unwrap();
    let retained =
        Connection::open_with_flags(snapshot, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(
        retained
            .query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        retained
            .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        retained
            .query_row("SELECT state FROM job_events", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "recorded"
    );
}

#[test]
fn os_hidden_controls_deep_includes_and_denied_directory_preserve_inventory() {
    use std::os::unix::{ffi::OsStrExt, fs::PermissionsExt};
    let source = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (_, library) = open(data.path());
    std::fs::create_dir_all(source.path().join("Parent/Deep")).unwrap();
    std::fs::create_dir(source.path().join("HiddenFolder")).unwrap();
    for path in [
        "visible.png",
        "hidden-by-os.png",
        "HiddenFolder/child.png",
        "Parent/Deep/kept.png",
    ] {
        image::RgbImage::from_pixel(16, 12, image::Rgb([20, 90, 30]))
            .save(source.path().join(path))
            .unwrap();
    }
    for path in ["hidden-by-os.png", "HiddenFolder"] {
        let native =
            std::ffi::CString::new(source.path().join(path).as_os_str().as_bytes()).unwrap();
        // SAFETY: both paths are inside this test's generated TempDir.
        assert_eq!(
            unsafe { libc::chflags(native.as_ptr(), libc::UF_HIDDEN) },
            0
        );
    }
    let job = library
        .register(
            source.path().to_owned(),
            None,
            "OS hidden fixtures".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        )
        .unwrap();
    wait(&library, &job.id);
    assert_eq!(
        library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .total,
        2
    );
    for (files, folders, expected) in [
        (true, false, 3),
        (false, true, 3),
        (true, true, 4),
        (false, false, 2),
    ] {
        let job = library
            .scan(
                job.root_id.clone(),
                ScanFilter {
                    include_hidden_files: files,
                    include_hidden_folders: folders,
                    ..Default::default()
                },
            )
            .unwrap();
        wait(&library, &job.id);
        assert_eq!(
            library
                .query(AssetQuery::default(), None, 240)
                .unwrap()
                .total,
            expected
        );
    }
    let scan = library
        .scan(
            job.root_id.clone(),
            ScanFilter {
                include_globs: vec!["Parent/Deep/*.png".into()],
                ..Default::default()
            },
        )
        .unwrap();
    wait(&library, &scan.id);
    assert_eq!(
        library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .total,
        1
    );
    let scan = library
        .scan(job.root_id.clone(), ScanFilter::default())
        .unwrap();
    wait(&library, &scan.id);
    let denied = source.path().join("Parent");
    std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o000)).unwrap();
    let scan = library
        .scan(job.root_id.clone(), ScanFilter::default())
        .unwrap();
    let result = wait(&library, &scan.id);
    std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(result.state, "partial");
    assert!(result.errors > 0);
    assert_eq!(
        library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .total,
        2
    );
    assert!(
        std::fs::metadata(denied.join("Deep/kept.png"))
            .unwrap()
            .len()
            > 0
    );
}

#[test]
fn independent_roots_keep_locations_distinct_and_frozen_scope_excludes_later_results() {
    let first = LibraryFixture::generate().unwrap();
    let second = LibraryFixture::generate().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (_, library) = open(data.path());
    let a = register(&library, &first);
    wait(&library, &a.id);
    let b = register(&library, &second);
    wait(&library, &b.id);
    let combined = library.query(AssetQuery::default(), None, 240).unwrap();
    assert_eq!(combined.total, 40);
    assert_eq!(
        combined
            .assets
            .iter()
            .map(|a| &a.id)
            .collect::<HashSet<_>>()
            .len(),
        40
    );
    for id in [&a.root_id, &b.root_id] {
        assert_eq!(
            library
                .query(
                    AssetQuery {
                        root_id: Some(id.clone()),
                        ..Default::default()
                    },
                    None,
                    240
                )
                .unwrap()
                .total,
            20
        );
    }
    let frozen = library.freeze(AssetQuery::default()).unwrap();
    assert_eq!(frozen.count, 40);
    let late = tempfile::tempdir().unwrap();
    image::RgbImage::from_pixel(16, 12, image::Rgb([10, 20, 30]))
        .save(late.path().join("later.png"))
        .unwrap();
    let job = library
        .register(
            late.path().to_owned(),
            None,
            "Later fixture".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        )
        .unwrap();
    wait(&library, &job.id);
    assert_eq!(
        library
            .annotate(
                AssetSelection::Token { id: frozen.id },
                AnnotationPatch {
                    favorite: Some(true),
                    rating: None,
                    tags: None
                }
            )
            .unwrap(),
        40
    );
    assert_eq!(
        library
            .query(
                AssetQuery {
                    favorites_only: true,
                    ..Default::default()
                },
                None,
                240
            )
            .unwrap()
            .total,
        40
    );
    assert_eq!(
        library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .total,
        41
    );
    assert_eq!(
        library
            .status()
            .unwrap()
            .roots
            .iter()
            .filter(|root| root.is_base)
            .count(),
        1
    );
    assert!(first.verify_unchanged().is_ok());
    assert!(second.verify_unchanged().is_ok());
}

#[test]
fn case_aliases_cannot_register_or_scan_application_owned_data() {
    let owned = tempfile::tempdir().unwrap();
    let parent = owned.path().join("LibraryCase");
    std::fs::create_dir(&parent).unwrap();
    let data = parent.join("ManagedData");
    std::fs::create_dir(&data).unwrap();
    let (_, library) = open(&data);
    image::RgbImage::from_pixel(16, 12, image::Rgb([10, 20, 30]))
        .save(parent.join("original.png"))
        .unwrap();
    image::RgbImage::from_pixel(16, 12, image::Rgb([30, 20, 10]))
        .save(data.join("cache.png"))
        .unwrap();
    let managed_alias = parent.join("MANAGEDDATA");
    if managed_alias.exists() {
        assert_eq!(
            library
                .register(
                    managed_alias.join("thumbnails"),
                    None,
                    "Disallowed managed alias".into(),
                    RootMode::Referenced,
                    ScanFilter::default()
                )
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
    let root_alias = owned.path().join("LIBRARYCASE");
    let selected = if root_alias.exists() {
        root_alias
    } else {
        parent.clone()
    };
    let job = library
        .register(
            selected,
            None,
            "Owned exclusion fixture".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        )
        .unwrap();
    wait(&library, &job.id);
    let page = library.query(AssetQuery::default(), None, 240).unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.assets[0].name, "original.png");
    assert!(data.join("cache.png").is_file());
}

#[test]
fn photo_extension_preset_discovers_all_requested_suffixes_without_requiring_a_decoder() {
    use stillharbour_core::library::model::DEFAULT_PHOTO_EXTENSIONS;
    let source = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (_, library) = open(data.path());
    let mut baseline = Vec::new();
    for (i, ext) in DEFAULT_PHOTO_EXTENSIONS.iter().enumerate() {
        let name = format!(
            "generated-{i}.{}",
            if i % 2 == 0 {
                ext.to_ascii_uppercase()
            } else {
                ext.to_string()
            }
        );
        let bytes = format!("Generated undecodable media fixture {i}").into_bytes();
        std::fs::write(source.path().join(&name), &bytes).unwrap();
        baseline.push((name, bytes));
    }
    std::fs::write(
        source.path().join("ignored.pdf"),
        b"unchanged excluded fixture",
    )
    .unwrap();
    let job = library
        .register(
            source.path().to_owned(),
            None,
            "Generated photo suffixes".into(),
            RootMode::Managed,
            ScanFilter {
                extensions: DEFAULT_PHOTO_EXTENSIONS
                    .iter()
                    .map(|e| e.to_string())
                    .collect(),
                ..Default::default()
            },
        )
        .unwrap();
    wait(&library, &job.id);
    let page = library.query(AssetQuery::default(), None, 240).unwrap();
    assert_eq!(page.total, 33);
    for (name, bytes) in baseline {
        assert!(page.assets.iter().any(|asset| asset.name == name));
        assert_eq!(std::fs::read(source.path().join(name)).unwrap(), bytes);
    }
    assert_eq!(
        std::fs::read(source.path().join("ignored.pdf")).unwrap(),
        b"unchanged excluded fixture"
    );
}

#[test]
fn month_seeks_and_viewer_neighbors_follow_query_order_without_touching_sources() {
    let fixture = LibraryFixture::generate().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (_, library) = open(data.path());
    let job = register(&library, &fixture);
    wait(&library, &job.id);
    let all = library.query(AssetQuery::default(), None, 240).unwrap();
    let buckets = library.buckets(AssetQuery::default()).unwrap();
    assert_eq!(buckets.iter().map(|b| b.count).sum::<u32>(), all.total);
    for bucket in &buckets {
        assert_eq!(
            bucket.months.iter().map(|m| m.count).sum::<u32>(),
            bucket.count
        );
        for month in &bucket.months {
            let page = library
                .seek_date(AssetQuery::default(), month.period.clone(), 5)
                .unwrap();
            assert!(page.assets[0].date.starts_with(&month.period));
        }
    }
    let added_query = AssetQuery {
        sort: "added".into(),
        ..Default::default()
    };
    let added_buckets = library.buckets(added_query.clone()).unwrap();
    assert_eq!(
        added_buckets.iter().map(|b| b.count).sum::<u32>(),
        all.total
    );
    for bucket in added_buckets {
        for month in bucket.months {
            let page = library
                .seek_date(added_query.clone(), month.period.clone(), 5)
                .unwrap();
            assert!(page.assets[0].added_at.starts_with(&month.period));
        }
    }
    for period in [
        "2026-00",
        "2026-13",
        "2026-1",
        "2026/10",
        "💥-10",
        "2026-10-01",
    ] {
        assert_eq!(
            library
                .seek_date(AssetQuery::default(), period.into(), 5)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
    for sort in ["capture", "name", "size", "added", "modified"] {
        let query = AssetQuery {
            sort: sort.into(),
            name: "Generated".into(),
            ..Default::default()
        };
        let ordered = library.query(query.clone(), None, 240).unwrap().assets;
        for index in [0, ordered.len() / 2, ordered.len() - 1] {
            let neighbors = library
                .neighbors(query.clone(), ordered[index].id.clone())
                .unwrap();
            assert_eq!(
                neighbors.previous.as_ref().map(|a| &a.id),
                index.checked_sub(1).map(|i| &ordered[i].id)
            );
            assert_eq!(
                neighbors.next.as_ref().map(|a| &a.id),
                ordered.get(index + 1).map(|a| &a.id)
            );
        }
        let outside = all
            .assets
            .iter()
            .find(|a| !a.name.contains("Generated"))
            .unwrap();
        assert_eq!(
            library
                .neighbors(query, outside.id.clone())
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
    assert_eq!(
        library
            .neighbors(AssetQuery::default(), "not-an-id".into())
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    assert!(fixture.verify_unchanged().unwrap() >= 26);
}

#[test]
fn batch_tag_addition_merges_and_rolls_back_overflow_for_the_entire_explicit_scope() {
    let fixture = LibraryFixture::generate().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (_, library) = open(data.path());
    let job = register(&library, &fixture);
    wait(&library, &job.id);
    let all = library
        .query(AssetQuery::default(), None, 240)
        .unwrap()
        .assets;
    let first = all[0].id.clone();
    let second = all[1].id.clone();
    for (id, tags) in [
        (first.clone(), vec!["old".into(), "shared".into()]),
        (second.clone(), vec!["other".into()]),
    ] {
        library
            .annotate(
                AssetSelection::Ids { ids: vec![id] },
                AnnotationPatch {
                    favorite: None,
                    rating: None,
                    tags: Some(tags),
                },
            )
            .unwrap();
    }
    let ids = vec![first.clone(), second.clone()];
    assert_eq!(
        library
            .add_tags(
                AssetSelection::Ids { ids: ids.clone() },
                vec!["new".into(), "shared".into(), " new ".into()]
            )
            .unwrap(),
        2
    );
    assert_eq!(
        library.detail(first.clone()).unwrap().asset.tags,
        vec!["new", "old", "shared"]
    );
    assert_eq!(
        library.detail(second.clone()).unwrap().asset.tags,
        vec!["new", "other", "shared"]
    );
    library
        .annotate(
            AssetSelection::Ids {
                ids: vec![first.clone()],
            },
            AnnotationPatch {
                favorite: None,
                rating: None,
                tags: Some((0..64).map(|i| format!("tag-{i:02}")).collect()),
            },
        )
        .unwrap();
    assert_eq!(
        library
            .add_tags(AssetSelection::Ids { ids }, vec!["overflow".into()])
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    assert_eq!(library.detail(first).unwrap().asset.tags.len(), 64);
    assert_eq!(
        library.detail(second).unwrap().asset.tags,
        vec!["new", "other", "shared"]
    );
    let query = AssetQuery {
        name: "Generated".into(),
        ..Default::default()
    };
    let token = library.freeze(query).unwrap();
    // Reset the overflowing item's tags before checking frozen membership.
    library
        .annotate(
            AssetSelection::Ids {
                ids: all.iter().map(|a| a.id.clone()).collect(),
            },
            AnnotationPatch {
                favorite: None,
                rating: None,
                tags: Some(vec![]),
            },
        )
        .unwrap();
    library
        .add_tags(
            AssetSelection::Token { id: token.id },
            vec!["frozen".into()],
        )
        .unwrap();
    for asset in all {
        assert_eq!(
            library
                .detail(asset.id)
                .unwrap()
                .asset
                .tags
                .contains(&"frozen".into()),
            asset.name.contains("Generated")
        );
    }
    assert!(fixture.verify_unchanged().unwrap() >= 26);
}

#[test]
fn stable_asset_windows_page_both_directions_for_every_sort_and_filter() {
    let fixture = LibraryFixture::generate().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (_catalog, library) = open(data.path());
    let job = register(&library, &fixture);
    wait(&library, &job.id);
    let sample = library.query(AssetQuery::default(), None, 240).unwrap();
    let ids: Vec<_> = sample.assets.iter().take(3).map(|a| a.id.clone()).collect();
    // Owned synthetic observations only; the generated source files are unchanged.
    let db = rusqlite::Connection::open(data.path().join("fixture.stillharbour")).unwrap();
    for (index, id) in ids.iter().enumerate() {
        db.execute(
            "UPDATE assets SET bytes=? WHERE id=?",
            rusqlite::params![9_007_199_254_740_993i64 + index as i64, id],
        )
        .unwrap();
    }
    drop(db);
    for sort in ["capture", "added", "modified", "name", "size"] {
        let query = AssetQuery {
            sort: sort.into(),
            name: "Generated".into(),
            ..Default::default()
        };
        let ordered = library.query(query.clone(), None, 240).unwrap().assets;
        for index in [0, ordered.len() / 2, ordered.len() - 1] {
            let at = library
                .seek_asset(
                    query.clone(),
                    ordered[index].id.clone(),
                    AssetSeekPosition::At,
                    3,
                )
                .unwrap();
            assert_eq!(
                at.assets.iter().map(|a| &a.id).collect::<Vec<_>>(),
                ordered[index..(index + 3).min(ordered.len())]
                    .iter()
                    .map(|a| &a.id)
                    .collect::<Vec<_>>()
            );
            let before = library
                .seek_asset(
                    query.clone(),
                    ordered[index].id.clone(),
                    AssetSeekPosition::Before,
                    3,
                )
                .unwrap();
            assert_eq!(
                before.assets.iter().map(|a| &a.id).collect::<Vec<_>>(),
                ordered[index.saturating_sub(3)..index]
                    .iter()
                    .map(|a| &a.id)
                    .collect::<Vec<_>>()
            );
            if let Some(cursor) = before.next_cursor {
                let next = library.query(query.clone(), Some(cursor), 3).unwrap();
                assert_eq!(next.assets[0].id, ordered[index].id);
            }
        }
    }
    assert!(
        library
            .seek_asset(
                AssetQuery::default(),
                "invalid".into(),
                AssetSeekPosition::At,
                240
            )
            .is_err()
    );
    assert!(
        library
            .seek_asset(
                AssetQuery::default(),
                sample.assets[0].id.clone(),
                AssetSeekPosition::Before,
                241
            )
            .is_err()
    );
    assert!(fixture.verify_unchanged().unwrap() >= 26);
}

fn ready_preview(library: &Arc<Library>, asset: &AssetSummary, generation: &str) -> ThumbnailKey {
    let key = ThumbnailKey {
        asset_id: asset.id.clone(),
        revision: asset.revision.clone(),
        edge: 256,
    };
    let start = Instant::now();
    loop {
        let response = library
            .thumbnails(vec![key.clone()], generation.into())
            .unwrap();
        if response[0].state == "ready" {
            return key;
        }
        assert_eq!(response[0].state, "pending", "{response:?}");
        assert!(start.elapsed() < Duration::from_secs(20));
        std::thread::sleep(Duration::from_millis(20));
    }
}
#[test]
fn reviewed_removal_clears_only_its_indexed_cache_and_promotes_another_root_without_touching_sources()
 {
    let first = LibraryFixture::generate().unwrap();
    let second = LibraryFixture::generate().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (catalog, library) = open(data.path());
    let job = register(&library, &first);
    wait(&library, &job.id);
    let root = job.root_id;
    let job = register(&library, &second);
    wait(&library, &job.id);
    let remaining = job.root_id;
    let first_asset = library
        .query(
            AssetQuery {
                root_id: Some(root.clone()),
                ..Default::default()
            },
            None,
            240,
        )
        .unwrap()
        .assets
        .into_iter()
        .find(|a| a.relative_path == "Coast/Generated-00.jpg")
        .unwrap();
    let second_asset = library
        .query(
            AssetQuery {
                root_id: Some(remaining.clone()),
                ..Default::default()
            },
            None,
            240,
        )
        .unwrap()
        .assets
        .into_iter()
        .find(|a| a.relative_path == "Coast/Generated-00.jpg")
        .unwrap();
    let first_key = ready_preview(&library, &first_asset, "1");
    let second_key = ready_preview(&library, &second_asset, "2");
    let token = library.freeze(AssetQuery::default()).unwrap();
    let identity = catalog.summary().unwrap().id;
    let target = MaintenanceTarget::Library {
        root_id: root.clone(),
    };
    let stale = library.prepare_maintenance(target.clone()).unwrap();
    library
        .annotate(
            AssetSelection::Ids {
                ids: vec![first_asset.id.clone()],
            },
            AnnotationPatch {
                favorite: Some(true),
                rating: None,
                tags: None,
            },
        )
        .unwrap();
    assert_eq!(
        library.apply_maintenance(stale.id).unwrap_err().code,
        ErrorCode::Conflict
    );
    assert!(library.root_exists(root.clone()).unwrap());
    let preview = library.prepare_maintenance(target).unwrap();
    assert!(
        preview.cached_previews > 1,
        "tiny and clear previews must both be reviewed"
    );
    assert!(preview.asset_count >= 20);
    assert!(library.apply_maintenance("invented".into()).is_err());
    let result = library.apply_maintenance(preview.id.clone()).unwrap();
    assert_eq!(result.removed_root_id, Some(root.clone()));
    assert_eq!(result.cleared_entries, preview.cached_previews);
    assert!(library.apply_maintenance(preview.id).is_err());
    assert!(!library.root_exists(root.clone()).unwrap());
    assert!(
        library
            .cached_thumbnail(first_key, THUMBNAIL_POLICY)
            .is_err()
    );
    assert!(
        library
            .cached_thumbnail(second_key, THUMBNAIL_POLICY)
            .is_ok()
    );
    assert!(
        library
            .annotate(
                AssetSelection::Token { id: token.id },
                AnnotationPatch {
                    favorite: Some(true),
                    rating: None,
                    tags: None
                }
            )
            .is_err()
    );
    let status = library.status().unwrap();
    assert_eq!(status.roots.len(), 1);
    assert_eq!(status.roots[0].id, remaining);
    assert!(status.roots[0].is_base);
    assert!(status.jobs.iter().all(|j| j.root_id != root));
    assert_eq!(catalog.summary().unwrap().id, identity);
    let db = rusqlite::Connection::open(data.path().join("fixture.stillharbour")).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, u32>(
            0
        ))
        .unwrap(),
        0
    );
    let preview = library
        .prepare_maintenance(MaintenanceTarget::Library { root_id: remaining })
        .unwrap();
    library.apply_maintenance(preview.id).unwrap();
    assert!(library.status().unwrap().roots.is_empty());
    close(library, catalog);
    let (_, reopened) = open(data.path());
    assert!(reopened.status().unwrap().roots.is_empty());
    first.verify_unchanged().unwrap();
    second.verify_unchanged().unwrap();
}
#[test]
fn cache_clear_retains_notes_and_sources_rebuilds_previews_and_keeps_unindexed_files() {
    let fixture = LibraryFixture::generate().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (_, library) = open(data.path());
    let job = register(&library, &fixture);
    wait(&library, &job.id);
    let assets = library
        .query(AssetQuery::default(), None, 240)
        .unwrap()
        .assets;
    let asset = assets
        .iter()
        .find(|a| a.relative_path == "Coast/Generated-00.jpg")
        .unwrap();
    let key = ready_preview(&library, asset, "1");
    library
        .annotate(
            AssetSelection::Ids {
                ids: vec![asset.id.clone()],
            },
            AnnotationPatch {
                favorite: Some(true),
                rating: Some(5),
                tags: Some(vec!["keep".into()]),
            },
        )
        .unwrap();
    let unknown = data.path().join("thumbnails/unindexed.txt");
    std::fs::write(&unknown, b"retain unknown content").unwrap();
    let plan = library
        .prepare_maintenance(MaintenanceTarget::PreviewCache)
        .unwrap();
    assert!(plan.cached_previews > 1);
    let expected_previews = plan.cached_previews;
    let result = library.apply_maintenance(plan.id).unwrap();
    assert!(result.removed_root_id.is_none());
    assert_eq!(result.cleared_entries, expected_previews);
    assert!(library.cached_thumbnail(key, THUMBNAIL_POLICY).is_err());
    let detail = library.detail(asset.id.clone()).unwrap();
    assert!(detail.asset.favorite);
    assert_eq!(detail.asset.rating, 5);
    assert_eq!(detail.asset.tags, ["keep"]);
    let rebuilt = ready_preview(&library, asset, "2");
    assert!(library.cached_thumbnail(rebuilt, THUMBNAIL_POLICY).is_ok());
    assert_eq!(std::fs::read(unknown).unwrap(), b"retain unknown content");
    fixture.verify_unchanged().unwrap();
}
#[test]
fn cleanup_failure_keeps_root_and_index_for_retry_and_offline_removal_never_opens_originals() {
    let fixture = LibraryFixture::generate().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (_, library) = open(data.path());
    let job = register(&library, &fixture);
    wait(&library, &job.id);
    let asset = library
        .query(AssetQuery::default(), None, 240)
        .unwrap()
        .assets
        .into_iter()
        .find(|a| a.relative_path == "Coast/Generated-00.jpg")
        .unwrap();
    let key = ready_preview(&library, &asset, "1");
    let cache_key = blake3::hash(
        format!(
            "{THUMBNAIL_POLICY}:{}:{}:{}",
            key.asset_id, key.revision, key.edge
        )
        .as_bytes(),
    )
    .to_hex()
    .to_string();
    let path = data
        .path()
        .join("thumbnails")
        .join(format!("{cache_key}.png"));
    let original_cache = std::fs::read(&path).unwrap();
    std::fs::write(&path, b"unexpected").unwrap();
    let target = MaintenanceTarget::Library {
        root_id: job.root_id.clone(),
    };
    let plan = library.prepare_maintenance(target.clone()).unwrap();
    let expected_previews = plan.cached_previews;
    assert_eq!(
        library.apply_maintenance(plan.id).unwrap_err().code,
        ErrorCode::Conflict
    );
    assert!(library.root_exists(job.root_id.clone()).unwrap());
    std::fs::write(&path, original_cache).unwrap();
    // Fault injection is confined to this owned temporary catalog. File cleanup
    // may finish before a catalog commit fails; all records/index must roll back.
    let db = rusqlite::Connection::open(data.path().join("fixture.stillharbour")).unwrap();
    db.execute_batch("CREATE TRIGGER fixture_refuse_removal BEFORE DELETE ON roots BEGIN SELECT RAISE(ABORT,'fixture database failure'); END;").unwrap();
    let plan = library.prepare_maintenance(target.clone()).unwrap();
    assert!(library.apply_maintenance(plan.id).is_err());
    assert!(library.root_exists(job.root_id.clone()).unwrap());
    assert_eq!(
        library
            .prepare_maintenance(target.clone())
            .unwrap()
            .cached_previews,
        expected_previews
    );
    assert!(!path.exists());
    db.execute_batch("DROP TRIGGER fixture_refuse_removal")
        .unwrap();
    let moved = fixture.path().with_file_name(format!(
        "{}-offline",
        fixture.path().file_name().unwrap().to_string_lossy()
    ));
    std::fs::rename(fixture.path(), &moved).unwrap();
    let plan = library.prepare_maintenance(target).unwrap();
    library.apply_maintenance(plan.id).unwrap();
    std::fs::rename(moved, fixture.path()).unwrap();
    assert!(!library.root_exists(job.root_id).unwrap());
    fixture.verify_unchanged().unwrap();
}

#[test]
fn directory_explorer_preserves_immediate_scope_empty_folders_files_cursors_and_native_names() {
    let data = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let root = source.path().join("source");
    std::fs::create_dir(&root).unwrap();
    for path in ["Trips", "Trips/2026", "Empty"] {
        std::fs::create_dir(root.join(path)).unwrap();
    }
    let fixture = LibraryFixture::generate().unwrap();
    let jpeg = std::fs::read(fixture.path().join("Coast/Generated-00.jpg")).unwrap();
    std::fs::write(root.join("root.jpg"), &jpeg).unwrap();
    std::fs::write(root.join("Trips/trip.jpg"), &jpeg).unwrap();
    std::fs::write(root.join("Trips/2026/nested.jpg"), &jpeg).unwrap();
    std::fs::write(root.join("note%_.txt"), b"Generated read-only note").unwrap();
    let (catalog, library) = open(data.path());
    let identity = catalog.summary().unwrap().id;
    let job = library
        .register(
            root.clone(),
            None,
            "Explorer fixture".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        )
        .unwrap();
    wait(&library, &job.id);
    // APFS rejects these native names. Seed only the owned catalog to verify
    // distinct lossless identities without claiming native filename support.
    let db = rusqlite::Connection::open(data.path().join("fixture.stillharbour")).unwrap();
    for byte in [0xfe, 0xff] {
        db.execute("INSERT INTO directories(root_id,native_path,parent_native,seen_scan,state) VALUES(?,?,?,?,?)",rusqlite::params![job.root_id,vec![b'N',byte],Vec::<u8>::new(),job.id,"incomplete"]).unwrap();
    }
    let listing = library
        .browse_directory(job.root_id.clone(), None, String::new(), None, 240)
        .unwrap();
    assert_eq!(listing.total, 6);
    assert!(listing.ancestors.is_empty());
    let folders: Vec<_> = listing
        .entries
        .iter()
        .filter_map(|e| {
            if let DirectoryEntry::Folder { directory } = e {
                Some(directory)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(folders.len(), 4);
    let same_display: Vec<_> = folders
        .iter()
        .filter(|d| d.name.contains('\u{fffd}'))
        .collect();
    assert_eq!(same_display.len(), 2);
    assert_ne!(same_display[0].id, same_display[1].id);
    let trips = folders.iter().find(|d| d.name == "Trips").unwrap();
    let empty = folders.iter().find(|d| d.name == "Empty").unwrap();
    let contents = library
        .browse_directory(
            job.root_id.clone(),
            Some(trips.id.clone()),
            String::new(),
            None,
            240,
        )
        .unwrap();
    assert_eq!(contents.total, 2);
    assert_eq!(contents.ancestors[0].name, "Explorer fixture");
    let scoped = AssetQuery {
        root_id: Some(job.root_id.clone()),
        directory_id: Some(trips.id.clone()),
        sort: "name".into(),
        ..Default::default()
    };
    let photos = library.query(scoped.clone(), None, 240).unwrap();
    assert_eq!(photos.total, 1);
    assert_eq!(photos.assets[0].name, "trip.jpg");
    let neighbors = library
        .neighbors(scoped, photos.assets[0].id.clone())
        .unwrap();
    assert!(neighbors.next.is_none() && neighbors.previous.is_none());
    assert_eq!(
        library
            .browse_directory(
                job.root_id.clone(),
                Some(empty.id.clone()),
                String::new(),
                None,
                240
            )
            .unwrap()
            .total,
        0
    );
    assert!(listing.entries.iter().any(
        |entry| matches!(entry,DirectoryEntry::File{asset,media:false} if asset.name=="note%_.txt")
    ));
    let filtered = library
        .browse_directory(job.root_id.clone(), None, "%_".into(), None, 240)
        .unwrap();
    assert_eq!(filtered.total, 5);
    assert_eq!(
        filtered
            .entries
            .iter()
            .filter(|e| matches!(e, DirectoryEntry::File { .. }))
            .count(),
        1
    );
    let first = library
        .browse_directory(job.root_id.clone(), None, String::new(), None, 2)
        .unwrap();
    assert!(first.previous_cursor.is_none());
    let next = library
        .browse_directory(
            job.root_id.clone(),
            None,
            String::new(),
            first.next_cursor.clone(),
            2,
        )
        .unwrap();
    let back = library
        .browse_directory(
            job.root_id.clone(),
            None,
            String::new(),
            next.previous_cursor.clone(),
            2,
        )
        .unwrap();
    assert_eq!(
        serde_json::to_value(&first.entries).unwrap(),
        serde_json::to_value(back.entries).unwrap()
    );
    assert!(
        library
            .browse_directory(
                job.root_id.clone(),
                Some(trips.id.clone()),
                String::new(),
                first.next_cursor.clone(),
                2
            )
            .is_err()
    );
    assert!(
        library
            .browse_directory(
                job.root_id.clone(),
                None,
                "other".into(),
                first.next_cursor.clone(),
                2
            )
            .is_err()
    );
    for id in ["not-native", "d0", "d787878"] {
        assert!(
            library
                .browse_directory(
                    job.root_id.clone(),
                    Some(id.into()),
                    String::new(),
                    None,
                    240
                )
                .is_err()
        );
    }
    assert!(
        library
            .browse_directory(job.root_id.clone(), None, String::new(), None, 241)
            .is_err()
    );
    assert!(
        library
            .browse_directory("unknown-root".into(), None, String::new(), None, 240)
            .is_err()
    );
    let moved = source.path().join("offline");
    std::fs::rename(&root, &moved).unwrap();
    let offline = library.status().unwrap();
    assert_eq!(offline.roots[0].availability, "offline");
    assert_eq!(
        library
            .browse_directory(job.root_id, None, String::new(), None, 240)
            .unwrap()
            .total,
        listing.total
    );
    std::fs::rename(moved, &root).unwrap();
    assert_eq!(std::fs::read(root.join("root.jpg")).unwrap(), jpeg);
    assert_eq!(std::fs::read(root.join("Trips/trip.jpg")).unwrap(), jpeg);
    assert_eq!(
        std::fs::read(root.join("Trips/2026/nested.jpg")).unwrap(),
        jpeg
    );
    assert_eq!(
        std::fs::read(root.join("note%_.txt")).unwrap(),
        b"Generated read-only note"
    );
    assert_eq!(catalog.summary().unwrap().id, identity);
    fixture.verify_unchanged().unwrap();
}

#[test]
fn compatible_m1_positive_previews_remain_available_in_catalog_offline_state() {
    let fixture = LibraryFixture::generate().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (_catalog, library) = open(data.path());
    let job = register(&library, &fixture);
    wait(&library, &job.id);
    let asset = library
        .query(AssetQuery::default(), None, 240)
        .unwrap()
        .assets
        .into_iter()
        .find(|a| a.relative_path == "Coast/Generated-00.jpg")
        .unwrap();
    let key = ready_preview(&library, &asset, "1");
    let expected = library
        .cached_thumbnail(key.clone(), THUMBNAIL_POLICY)
        .unwrap();
    let hash = |policy: &str| {
        blake3::hash(format!("{policy}:{}:{}:{}", key.asset_id, key.revision, key.edge).as_bytes())
            .to_hex()
            .to_string()
    };
    let legacy = hash("m1-v1-oriented-assumed-srgb-no-icc");
    // Construct an old-policy derived cache fixture; no source is moved/rewritten.
    std::fs::write(
        data.path().join("thumbnails").join(format!("{legacy}.png")),
        &expected,
    )
    .unwrap();
    let db = rusqlite::Connection::open(data.path().join("fixture.stillharbour")).unwrap();
    db.execute(
        "UPDATE thumbnail_cache SET key=? WHERE key=?",
        rusqlite::params![legacy, hash(THUMBNAIL_POLICY)],
    )
    .unwrap();
    db.execute(
        "UPDATE roots SET availability='offline' WHERE id=?",
        [&job.root_id],
    )
    .unwrap();
    drop(db);
    assert_eq!(
        library.thumbnails(vec![key.clone()], "2".into()).unwrap()[0].state,
        "ready"
    );
    assert_eq!(
        library.cached_thumbnail(key, THUMBNAIL_POLICY).unwrap(),
        expected
    );
    assert_eq!(fixture.verify_unchanged().unwrap(), 32);
}

#[test]
fn tiny_previews_are_a_separate_durable_job_with_warm_resume_and_no_inventory_reconciliation() {
    let fixture = LibraryFixture::generate().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (_catalog, library) = open(data.path());
    let scan = register(&library, &fixture);
    wait(&library, &scan.id);
    let start = Instant::now();
    let auto = loop {
        if let Some(job) = library
            .status()
            .unwrap()
            .jobs
            .into_iter()
            .find(|job| matches!(job.kind, Some(JobKind::Thumbnails)))
        {
            break job;
        }
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(20));
    };
    let finished = wait(&library, &auto.id);
    assert!(
        matches!(finished.state.as_str(), "completed" | "partial"),
        "{finished:?}"
    );
    let root = library.status().unwrap().roots.remove(0);
    let last_scan = root.last_scan.clone();
    let total = library
        .query(AssetQuery::default(), None, 64)
        .unwrap()
        .total;
    // Wait for the terminal job to release its queue slot, then start a warm pass.
    let warm = loop {
        let next = library.generate_thumbnails(root.id.clone()).unwrap();
        if next.id != auto.id {
            break next;
        }
        assert!(start.elapsed() < Duration::from_secs(45));
        std::thread::sleep(Duration::from_millis(20));
    };
    let warm = wait(&library, &warm.id);
    assert!(warm.skipped > 0, "{warm:?}");
    assert_eq!(warm.indexed, 0, "successful tiny previews should be reused");
    assert_eq!(library.status().unwrap().roots[0].last_scan, last_scan);
    assert_eq!(
        library
            .query(AssetQuery::default(), None, 64)
            .unwrap()
            .total,
        total
    );
    let asset = library
        .query(AssetQuery::default(), None, 64)
        .unwrap()
        .assets
        .into_iter()
        .find(|asset| asset.extension == "png")
        .unwrap();
    let tiny = library
        .thumbnails(
            vec![ThumbnailKey {
                asset_id: asset.id,
                revision: asset.revision,
                edge: 32,
            }],
            "999".into(),
        )
        .unwrap();
    assert_eq!(tiny[0].state, "ready");
    let clear = ThumbnailKey {
        asset_id: tiny[0].asset_id.clone(),
        revision: tiny[0].revision.clone(),
        edge: 512,
    };
    let start = Instant::now();
    while library
        .thumbnails(vec![clear.clone()], "1000".into())
        .unwrap()[0]
        .state
        != "ready"
    {
        assert!(start.elapsed() < Duration::from_secs(15));
        std::thread::sleep(Duration::from_millis(20));
    }
    let high = ThumbnailKey {
        edge: 1024,
        ..clear.clone()
    };
    let start = Instant::now();
    loop {
        let result = library
            .thumbnails(vec![high.clone()], "1001".into())
            .unwrap();
        assert_eq!(
            result[0].fallback_edge,
            Some(512),
            "Keep the smaller cache through actual image load"
        );
        if result[0].state == "ready" {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(15));
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        !library
            .cached_thumbnail(high, stillharbour_core::library::THUMBNAIL_POLICY)
            .unwrap()
            .is_empty()
    );
    // Catalog-offline state must not enqueue a new decode or hide an indexed
    // smaller rendition. This is a state fixture, not a physical NAS test.
    let external = rusqlite::Connection::open(data.path().join("fixture.stillharbour")).unwrap();
    external.busy_timeout(Duration::from_secs(5)).unwrap();
    external
        .execute(
            "UPDATE roots SET availability='offline' WHERE id=?",
            [root.id.clone()],
        )
        .unwrap();
    let offline = library
        .thumbnails(
            vec![ThumbnailKey {
                edge: 2048,
                ..clear
            }],
            "1002".into(),
        )
        .unwrap();
    assert_eq!(offline[0].state, "unavailable");
    assert_eq!(offline[0].fallback_edge, Some(1024));
    fixture.verify_unchanged().unwrap();
}

#[test]
fn synthetic_performance_roots_stay_offline_without_reconciling_catalog_observations() {
    let data = tempfile::tempdir().unwrap();
    let (catalog, library) = open(data.path());
    let seeded =
        stillharbour_core::library::performance_probe::seed_catalog(&catalog, data.path(), 10_000)
            .unwrap();
    let ids = seeded["rootIds"].as_array().unwrap();
    let status = library.status().unwrap();
    for id in ids {
        let root = status
            .roots
            .iter()
            .find(|root| root.id == id.as_str().unwrap())
            .unwrap();
        assert_eq!(
            root.availability, "offline",
            "Synthetic roots must not reconnect to empty directories and scan away seeded rows"
        );
    }
    assert!(
        status.jobs.is_empty(),
        "Read-only synthetic browsing must not start a recovery scan"
    );
    assert_eq!(
        library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .total,
        10_000
    );
    let keys = library
        .query(AssetQuery::default(), None, 64)
        .unwrap()
        .assets
        .into_iter()
        .map(|asset| ThumbnailKey {
            asset_id: asset.id,
            revision: asset.revision,
            edge: 1024,
        })
        .collect();
    let before = catalog.probe_call_count();
    let previews = library.thumbnails(keys, "1".into()).unwrap();
    assert!(previews.iter().all(|item| item.state == "unavailable"));
    let calls = catalog.probe_call_count() - before;
    println!("preview_batch_catalog_calls={calls}");
    assert!(
        calls <= 4,
        "A64-key cache-status batch used{calls} catalog calls; do not query every policy/tier/asset separately"
    );
}

fn independent_manifest(root: &Path) -> serde_json::Value {
    let output = std::process::Command::new("python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/performance/manifest.py"))
        .arg(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).unwrap()
}
fn eventually(mut condition: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(45);
    while !condition() {
        assert!(Instant::now() < until, "Live reconciliation did not finish");
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn change_journal_survives_restart_and_missing_folders_leave_normal_explorer() {
    let fixture = LibraryFixture::generate().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (catalog, library) = open(data.path());
    let first = register(&library, &fixture);
    wait(&library, &first.id);
    assert!(
        library.changes(None, 100).unwrap().changes.is_empty(),
        "Initial scan is a baseline"
    );
    let original = library
        .query(AssetQuery::default(), None, 240)
        .unwrap()
        .assets
        .into_iter()
        .find(|a| a.name == "Generated-00.jpg")
        .unwrap();
    library
        .annotate(
            AssetSelection::Ids {
                ids: vec![original.id.clone()],
            },
            AnnotationPatch {
                favorite: Some(true),
                rating: None,
                tags: None,
            },
        )
        .unwrap();
    let old = fixture.path().join("Coast/Generated-00.jpg");
    let moved = fixture.path().join("Woodland/Live-renamed.jpg");
    std::fs::rename(&old, &moved).unwrap(); // External actor, owned fixture only.
    let expected = independent_manifest(fixture.path());
    let scan = library.scan(first.root_id.clone(), filter()).unwrap();
    wait(&library, &scan.id);
    assert_eq!(expected, independent_manifest(fixture.path()));
    let changes = library.changes(None, 100).unwrap().changes;
    let change = changes
        .iter()
        .find(|c| c.asset_id.as_ref() == Some(&original.id))
        .unwrap();
    assert_eq!(change.kind, LibraryChangeKind::Moved);
    assert_eq!(
        change.previous_path.as_deref(),
        Some("Coast/Generated-00.jpg")
    );
    assert_eq!(change.path.as_deref(), Some("Woodland/Live-renamed.jpg"));
    assert!(library.detail(original.id.clone()).unwrap().asset.favorite);
    let count = changes.len();
    let scan = library.scan(first.root_id.clone(), filter()).unwrap();
    wait(&library, &scan.id);
    assert_eq!(
        count,
        library.changes(None, 100).unwrap().changes.len(),
        "Unchanged scans must not manufacture changes"
    );
    let holder = tempfile::tempdir().unwrap();
    let coast = fixture.path().join("Coast");
    let away = holder.path().join("Coast");
    std::fs::rename(&coast, &away).unwrap();
    let expected = independent_manifest(fixture.path());
    let scan = library.scan(first.root_id.clone(), filter()).unwrap();
    wait(&library, &scan.id);
    assert_eq!(expected, independent_manifest(fixture.path()));
    let page = library
        .browse_directory(first.root_id.clone(), None, String::new(), None, 240)
        .unwrap();
    assert!(
        !page
            .entries
            .iter()
            .any(|e| matches!(e,DirectoryEntry::Folder{directory} if directory.name=="Coast"))
    );
    let removed = library.changes(None, 100).unwrap();
    assert!(
        removed
            .changes
            .iter()
            .any(|c| c.kind == LibraryChangeKind::Removed && c.entry_kind == "folder")
    );
    let one = library.changes(None, 1).unwrap();
    let older = library.changes(one.next_before, 1).unwrap();
    assert_ne!(one.changes[0].sequence, older.changes[0].sequence);
    assert!(library.changes(Some("-1".into()), 25).is_err());
    assert!(library.changes(None, 101).is_err());
    std::fs::rename(&away, &coast).unwrap();
    std::fs::rename(&moved, &old).unwrap();
    let expected = independent_manifest(fixture.path());
    let scan = library.scan(first.root_id, filter()).unwrap();
    wait(&library, &scan.id);
    assert_eq!(expected, independent_manifest(fixture.path()));
    fixture.verify_unchanged().unwrap();
    let recorded = library.changes(None, 100).unwrap().changes.len();
    library.cancel_all();
    eventually(|| {
        library
            .status()
            .unwrap()
            .jobs
            .iter()
            .all(|j| !matches!(j.state.as_str(), "queued" | "running" | "cancel_requested"))
    });
    close(library, catalog);
    let (_, library) = open(data.path());
    assert_eq!(recorded, library.changes(None, 100).unwrap().changes.len());
}

#[test]
fn native_notifications_reconcile_external_add_move_remove_and_preserve_offline_inventory() {
    let fixture = LibraryFixture::generate().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (_, library) = open(data.path());
    let first = register(&library, &fixture);
    wait(&library, &first.id);
    library.start_monitoring().unwrap();
    library.start_monitoring().unwrap();
    eventually(|| {
        library
            .status()
            .unwrap()
            .monitoring
            .unwrap_or_default()
            .iter()
            .any(|m| m.mode == "watching")
    });
    let created = fixture.path().join("Live-added.jpg");
    std::fs::copy(fixture.path().join("Coast/Generated-00.jpg"), &created).unwrap();
    let expected = independent_manifest(fixture.path());
    eventually(|| {
        library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .assets
            .iter()
            .any(|a| a.name == "Live-added.jpg")
    });
    assert_eq!(expected, independent_manifest(fixture.path()));
    let moved = fixture.path().join("Woodland/Live-moved.jpg");
    std::fs::rename(&created, &moved).unwrap();
    let expected = independent_manifest(fixture.path());
    eventually(|| {
        library.changes(None, 100).unwrap().changes.iter().any(|c| {
            c.kind == LibraryChangeKind::Moved
                && c.path.as_deref() == Some("Woodland/Live-moved.jpg")
        })
    });
    assert_eq!(expected, independent_manifest(fixture.path()));
    let holder = tempfile::tempdir().unwrap();
    let gone = holder.path().join("removed.jpg");
    std::fs::rename(&moved, &gone).unwrap();
    let expected = independent_manifest(fixture.path());
    eventually(|| {
        !library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .assets
            .iter()
            .any(|a| a.name == "Live-moved.jpg")
    });
    assert_eq!(expected, independent_manifest(fixture.path()));
    fixture.verify_unchanged().unwrap();
    let total = library
        .query(AssetQuery::default(), None, 240)
        .unwrap()
        .total;
    let recorded = library.changes(None, 100).unwrap().changes.len();
    let offline = holder.path().join("offline-root");
    std::fs::rename(fixture.path(), &offline).unwrap();
    eventually(|| library.status().unwrap().roots[0].availability == "offline");
    assert_eq!(
        total,
        library
            .query(AssetQuery::default(), None, 240)
            .unwrap()
            .total
    );
    assert_eq!(
        recorded,
        library.changes(None, 100).unwrap().changes.len(),
        "Offline is not mass deletion"
    );
    std::fs::rename(&offline, fixture.path()).unwrap();
    eventually(|| library.status().unwrap().roots[0].availability == "online");
    library.cancel_all();
    fixture.verify_unchanged().unwrap();
}

#[test]
fn original_image_returns_exact_full_size_bytes_and_rejects_stale_or_unavailable_sources() {
    let source = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    assert_ne!(source.path(), data.path());
    let path = source.path().join("Full-size-generated.jpg");
    image::RgbImage::from_fn(3072, 2048, |x, y| {
        image::Rgb([(x % 251) as u8, (y % 251) as u8, 70])
    })
    .save(&path)
    .unwrap();
    for extension in ["png", "webp"] {
        image::RgbImage::from_pixel(128, 96, image::Rgb([20, 100, 180]))
            .save(source.path().join(format!("Static-generated.{extension}")))
            .unwrap();
    }
    let before = independent_manifest(source.path());
    let (catalog, library) = open(data.path());
    let scan = library
        .register(
            source.path().to_owned(),
            None,
            "Generated original".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        )
        .unwrap();
    wait(&library, &scan.id);
    let asset = library
        .query(
            AssetQuery {
                name: "Full-size-generated".into(),
                ..Default::default()
            },
            None,
            10,
        )
        .unwrap()
        .assets
        .remove(0);
    let policy = stillharbour_core::library::ORIGINAL_IMAGE_POLICY;
    let original = library
        .original_image(asset.id.clone(), asset.revision.clone(), policy)
        .unwrap();
    assert_eq!(original.mime, "image/jpeg");
    assert_eq!(original.bytes, std::fs::read(&path).unwrap());
    assert_eq!(
        image::load_from_memory(&original.bytes).unwrap().width(),
        3072
    );
    for (extension, mime) in [("png", "image/png"), ("webp", "image/webp")] {
        let name = format!("Static-generated.{extension}");
        let image = library
            .query(
                AssetQuery {
                    name: name.clone(),
                    ..Default::default()
                },
                None,
                10,
            )
            .unwrap()
            .assets
            .remove(0);
        let response = library
            .original_image(image.id, image.revision, policy)
            .unwrap();
        assert_eq!(response.mime, mime);
        assert_eq!(
            response.bytes,
            std::fs::read(source.path().join(name)).unwrap()
        );
    }
    assert_eq!(independent_manifest(source.path()), before);
    assert_eq!(
        library
            .original_image(asset.id.clone(), "stale".into(), policy)
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
    assert_eq!(
        library
            .original_image(
                "../Full-size-generated.jpg".into(),
                asset.revision.clone(),
                policy
            )
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        library
            .original_image(asset.id.clone(), asset.revision.clone(), "unapproved")
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    // An external fixture deletion is distinct from all read-only phases.
    std::fs::remove_file(&path).unwrap();
    assert!(
        library
            .original_image(asset.id, asset.revision, policy)
            .is_err()
    );
    close(library, catalog);
}

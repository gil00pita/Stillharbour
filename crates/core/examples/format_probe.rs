#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{
        path::PathBuf,
        sync::Arc,
        time::{Duration, Instant},
    };
    use stillharbour_core::{
        catalog::Catalog,
        library::{Library, LibraryOptions, THUMBNAIL_POLICY, WorkerPaths, model::*},
    };
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 2
        || args.len() > 3
        || args[0] != "--run"
        || (args.len() == 3 && args[2] != "--packaged")
    {
        return Err("Use --run UUID [--packaged]; no library paths accepted".into());
    }
    let run = uuid::Uuid::parse_str(&args[1])?.to_string();
    let namespace = PathBuf::from(".context/formats");
    if std::fs::symlink_metadata(".context")?
        .file_type()
        .is_symlink()
        || std::fs::symlink_metadata(&namespace)?
            .file_type()
            .is_symlink()
    {
        return Err("Fixture namespace must not be a link".into());
    }
    let namespace = namespace.canonicalize()?;
    let requested = namespace.join(&run);
    if std::fs::symlink_metadata(&requested)?
        .file_type()
        .is_symlink()
    {
        return Err("Fixture owner must not be a link".into());
    }
    let owned = requested.canonicalize()?;
    let marker: serde_json::Value =
        serde_json::from_slice(&std::fs::read(owned.join("ownership.json"))?)?;
    if marker["run"] != run || marker["kind"] != "m5-format-fixture" {
        return Err("Fixture ownership mismatch".into());
    }
    let source_run =
        uuid::Uuid::parse_str(marker["sourceRun"].as_str().unwrap_or(&run))?.to_string();
    let source_owner = owned
        .parent()
        .ok_or("Invalid fixture namespace")?
        .join(&source_run)
        .canonicalize()?;
    if source_owner.parent() != owned.parent()
        || std::fs::symlink_metadata(owned.parent().ok_or("Invalid namespace")?.join(&source_run))?
            .file_type()
            .is_symlink()
        || std::fs::symlink_metadata(source_owner.join("sources"))?
            .file_type()
            .is_symlink()
    {
        return Err("Source fixture escaped its owned namespace".into());
    }
    let source_marker: serde_json::Value =
        serde_json::from_slice(&std::fs::read(source_owner.join("ownership.json"))?)?;
    if source_marker["run"] != source_run || source_marker["kind"] != "m5-format-fixture" {
        return Err("Source fixture ownership mismatch".into());
    }
    let fixtures: serde_json::Value =
        serde_json::from_slice(&std::fs::read(owned.join("fixtures.json"))?)?;
    let entries = fixtures.as_array().ok_or("Invalid fixture list")?;
    if entries.len() > 64 {
        return Err("Fixture count exceeds its bound".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for entry in entries {
        let id = entry["id"].as_str().ok_or("Missing fixture identity")?;
        let file = entry["file"].as_str().ok_or("Missing fixture filename")?;
        if id.len() > 64
            || id.is_empty()
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            || !ids.insert(id)
            || std::path::Path::new(file).components().count() != 1
            || file == "."
            || file == ".."
        {
            return Err("Invalid fixture identity or filename".into());
        }
    }
    let packaged = args.len() == 3;
    let executable = PathBuf::from(if packaged {
        "target/release/bundle/macos/Stillharbour.app/Contents/MacOS/stillharbour"
    } else {
        "target/release/stillharbour"
    })
    .canonicalize()?;
    let exiftool = PathBuf::from(if packaged {
        "target/release/bundle/macos/Stillharbour.app/Contents/Resources/helpers/exiftool/exiftool"
    } else {
        "vendor/exiftool/exiftool"
    })
    .canonicalize()?;
    let data = owned.join(format!("data-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&data)?;
    let dbpath = data.join("Fixture.stillharbour");
    let catalog = Arc::new(Catalog::open(dbpath.clone())?);
    let library = Library::new(
        catalog.clone(),
        LibraryOptions {
            data_directory: data.clone(),
            workers: WorkerPaths {
                executable,
                exiftool: Some(exiftool),
                working_directory: data.join("worker"),
            },
        },
    )?;
    let job = library.register(
        source_owner.join("sources"),
        None,
        "Owned M5 format fixtures".into(),
        RootMode::Referenced,
        ScanFilter::default(),
    )?;
    let started = Instant::now();
    while matches!(
        library.job_details(job.id.clone())?.job.state.as_str(),
        "running" | "cancel_requested"
    ) {
        if started.elapsed() > Duration::from_secs(180) {
            return Err("Owned scan deadline reached".into());
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    // Inventory completion now starts a separate tiny-preview job. Keep its
    // writes out of the controlled legacy-failure injection and clear-preview
    // observations; never race that fixture with a background catalog writer.
    while Arc::strong_count(&library) > 1 {
        if started.elapsed() > Duration::from_secs(180) {
            return Err("Owned thumbnail job deadline reached".into());
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    // Model an existing catalog's legacy HEIC failure, after the owned scan.
    let db = rusqlite::Connection::open(dbpath)?;
    db.execute("UPDATE assets SET preview_state='unavailable',preview_error='Legacy decoder does not support HEIC' WHERE extension='heic' AND name='generated.heic'",[])?;
    drop(db);
    let page = library.query(AssetQuery::default(), None, 240)?;
    let retry = page
        .assets
        .iter()
        .find(|a| a.name == "generated.heic")
        .is_some_and(|a| a.preview_state == "pending");
    let mut results = Vec::new();
    for fixture in fixtures.as_array().ok_or("Invalid fixture list")? {
        let name = fixture["file"].as_str().ok_or("Invalid fixture identity")?;
        let Some(asset) = page.assets.iter().find(|a| a.name == name) else {
            return Err("Fixture not indexed".into());
        };
        let key = ThumbnailKey {
            asset_id: asset.id.clone(),
            revision: asset.revision.clone(),
            edge: 512,
        };
        stillharbour_core::performance::enable();
        let started = Instant::now();
        let outcome = loop {
            let item = library.thumbnails(vec![key.clone()], "1".into())?.remove(0);
            if item.state != "pending" {
                break item;
            }
            if started.elapsed() > Duration::from_secs(20) {
                return Err("Owned preview deadline reached".into());
            }
            std::thread::sleep(Duration::from_millis(25));
        };
        let expected = fixture["expected"]
            .as_str()
            .ok_or("Invalid fixture expectation")?;
        let mut entry = serde_json::json!({"id":fixture["id"],"expected":expected,"state":outcome.state,"status":if outcome.state==expected {"passed"} else {"failed"},"elapsedMs":started.elapsed().as_secs_f64()*1000.0});
        if outcome.state == "ready" {
            let png = library.cached_thumbnail(key, THUMBNAIL_POLICY)?;
            let image = image::load_from_memory(&png)?;
            if image.width() > 512 || image.height() > 512 {
                return Err("Preview dimension violation".into());
            }
            std::io::Write::write_all(
                &mut std::fs::File::create_new(owned.join(format!(
                    "{}-preview.png",
                    fixture["id"].as_str().ok_or("Invalid fixture id")?
                )))?,
                &png,
            )?;
            if fixture["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("generated"))
            {
                let colors = [
                    [220u8, 30, 20],
                    [20, 180, 40],
                    [20, 40, 210],
                    [230, 190, 20],
                ];
                let rgb = image.to_rgb8();
                let mut valid = true;
                for (index, color) in colors.into_iter().enumerate() {
                    let x = rgb.width() * (if index % 2 == 0 { 1 } else { 3 }) / 4;
                    let y = rgb.height() * (if index < 2 { 1 } else { 3 }) / 4;
                    if rgb
                        .get_pixel(x, y)
                        .0
                        .into_iter()
                        .zip(color)
                        .any(|(a, b)| a.abs_diff(b) > 14)
                    {
                        valid = false;
                    }
                }
                entry["colorPatchesMatch"] = valid.into();
                if !valid {
                    entry["status"] = "failed".into();
                }
            }
            entry["width"] = image.width().into();
            entry["height"] = image.height().into();
            entry["bytes"] = png.len().into();
        } else {
            entry["reason"] = outcome.error.into();
        }
        let trace = stillharbour_core::performance::snapshot();
        entry["observedChildPeakRssBytes"] = trace
            .events
            .iter()
            .filter(|e| matches!(e.metric, stillharbour_core::performance::Metric::WorkerRss))
            .map(|e| e.bytes)
            .max()
            .into();
        entry["traceDropped"] = trace.dropped.into();
        results.push(entry);
    }
    println!(
        "{}",
        serde_json::json!({"results":results,"legacyFailureRetry":retry,"schema":catalog.summary()?.schema,"policy":THUMBNAIL_POLICY,"productionBackupAvailable":false})
    );
    if !retry {
        return Err("Legacy failure was not retryable".into());
    }
    Ok(())
}
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("No native ImageIO evidence is available on this platform.");
    std::process::exit(2);
}

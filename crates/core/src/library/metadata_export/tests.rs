use super::*;
use crate::{
    catalog::Catalog,
    library::{
        LibraryOptions, WorkerPaths,
        model::{RootMode, ScanFilter},
    },
};
use std::sync::Arc;
struct Fixture {
    owned: tempfile::TempDir,
    library: Arc<Library>,
    root: String,
    asset: String,
    output: PathBuf,
    selection: (FileIdentity, String),
    companion: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let owned = tempfile::Builder::new()
            .prefix("stillharbour-m5-export-")
            .tempdir()
            .unwrap();
        let root_path = owned.path().join("source");
        let data = owned.path().join("data");
        let output = owned.path().join("output");
        for folder in [&root_path, &data, &output] {
            std::fs::create_dir(folder).unwrap();
        }
        std::fs::write(
            root_path.join("Photo.jpg"),
            b"generated original photo bytes",
        )
        .unwrap();
        let companion = root_path.join("Photo.xmp");
        std::fs::write(&companion,br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><r:RDF xmlns:r="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><r:Description xmlns:p="urn:private-edit-history" p:Recipe="retain-exactly"/></r:RDF></x:xmpmeta>"#).unwrap();
        std::fs::write(output.join("destination-only"), b"retain destination").unwrap();
        let catalog = Arc::new(Catalog::open(data.join("Catalog.stillharbour")).unwrap());
        let handle = native::open_root(&root_path, None).unwrap();
        let identity = filesystem::identity(&handle).unwrap();
        let volume = native::volume_token(&handle).unwrap();
        let root = store::add_root(
            &catalog,
            root_path.clone(),
            identity,
            volume,
            "Generated library".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        )
        .unwrap();
        let asset = uuid::Uuid::new_v4().to_string();
        let id = asset.clone();
        let root_id = root.clone();
        catalog.call(move|db|{for (name,is_media,asset_id) in [("Photo.jpg",true,id),("Photo.xmp",false,uuid::Uuid::new_v4().to_string())]{let mut file=File::open(root_path.join(name))?;let identity=filesystem::identity(&file)?;let revision=native::revision(&file)?;let bytes=i64::try_from(file.metadata()?.len()).unwrap();let _=hash(&mut file,1024*1024,&AtomicBool::new(false))?;
   db.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan,tags,raw_metadata,rating) VALUES(?1,?2,?3,?4,x'','',?4,'Photo',?5,?6,?7,?8,'2026-11-02','2026-11-02','unavailable','ready','unavailable','present',?9,'generated','[\"Family\"]',?10,4)",params![asset_id,root_id,native::encode_path(Path::new(name)),name,if is_media{"jpg"}else{"xmp"},identity.file.to_string(),bytes,revision,is_media,r#"[{"Keywords":["Observed"],"GPSLatitude":1,"GPSLongitude":2,"AuthorizationURL":"https://example.invalid/?token=fixture-secret"}]"#])?;}
   db.execute("INSERT INTO job_events(job_id,state) VALUES('generated','fixture-secret-do-not-export')",[])?;Ok(())}).unwrap();
        let handle = native::open_root(&output, None).unwrap();
        let selection = (
            filesystem::identity(&handle).unwrap(),
            native::volume_token(&handle).unwrap(),
        );
        let library = Library::new(
            catalog,
            LibraryOptions {
                data_directory: data.clone(),
                workers: WorkerPaths {
                    executable: std::env::current_exe().unwrap(),
                    exiftool: None,
                    working_directory: data.join("workers"),
                },
            },
        )
        .unwrap();
        Self {
            owned,
            library,
            root,
            asset,
            output,
            selection,
            companion,
        }
    }
    fn review(&self, kind: MetadataExportKind) -> Result<MetadataExportReview> {
        self.library.metadata_export_preview(
            kind,
            (kind == MetadataExportKind::Xmp).then(|| AssetSelection::Ids {
                ids: vec![self.asset.clone()],
            }),
            if kind == MetadataExportKind::Xmp {
                1
            } else {
                0
            },
            "generated-selection".into(),
            self.output.clone(),
            self.selection.clone(),
        )
    }
    fn save(&self, review: MetadataExportReview) -> Result<MetadataExportSaved> {
        self.library.metadata_export_save(
            review.id,
            review.digest,
            "generated-selection".into(),
            self.output.clone(),
            self.selection.clone(),
        )
    }
    fn audit(&self) {
        assert_eq!(
            std::fs::read(self.owned.path().join("source/Photo.jpg")).unwrap(),
            b"generated original photo bytes"
        );
        assert_eq!(
            std::fs::read(self.output.join("destination-only")).unwrap(),
            b"retain destination"
        );
    }
}
#[test]
fn portable_catalog_reopens_independently_with_organization_but_without_runtime_authority_or_secret_artifacts()
 {
    let f = Fixture::new();
    let review = f.review(MetadataExportKind::Catalog).unwrap();
    let path = f.output.join(&review.files[0].name);
    f.save(review).unwrap();
    #[cfg(feature = "fixture-probes")]
    {
        let inspected = crate::catalog::inspect_portable_export_fixture(&path).unwrap();
        assert_eq!(inspected["nativeAuthorities"], 0);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for work in
            std::fs::read_dir(f.library.options.data_directory.join("metadata-exports")).unwrap()
        {
            assert_eq!(
                work.unwrap().metadata().unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }
    let copy = Catalog::open(path).unwrap();
    assert_eq!(
        copy.summary().unwrap().id,
        f.library.catalog.summary().unwrap().id
    );
    let id = f.asset.clone();
    copy.call(move |db| {
        assert_eq!(
            db.query_row("SELECT tags FROM assets WHERE id=?", [id], |r| r
                .get::<_, String>(0))?,
            "[\"Family\"]"
        );
        assert_eq!(
            db.query_row("SELECT availability FROM roots", [], |r| r
                .get::<_, String>(0))?,
            "needs_relink"
        );
        for table in [
            "job_events",
            "catalog_metadata_plans",
            "backup_snapshot_profiles",
            "backup_snapshot_schedules",
            "backup_sync_plans",
            "thumbnail_cache",
            "selections",
        ] {
            assert_eq!(
                db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))?,
                0
            );
        }
        assert!(
            !db.query_row("SELECT raw_metadata FROM assets LIMIT 1", [], |r| r
                .get::<_, String>(0))?
                .contains("fixture-secret")
        );
        Ok(())
    })
    .unwrap();
    f.audit();
}
#[test]
fn xmp_export_retains_unknown_packets_and_detects_external_change_before_any_publication() {
    let f = Fixture::new();
    let original = std::fs::read(&f.companion).unwrap();
    let review = f.review(MetadataExportKind::Xmp).unwrap();
    std::fs::write(&f.companion, b"external edit").unwrap();
    assert!(f.save(review).is_err());
    assert_eq!(std::fs::read_dir(&f.output).unwrap().count(), 1);
    std::fs::write(&f.companion, &original).unwrap();
    // Reconcile only this generated companion's new observed revision.
    let revision = native::revision(&File::open(&f.companion).unwrap()).unwrap();
    let root = f.root.clone();
    f.library
        .catalog
        .call(move |db| {
            db.execute(
                "UPDATE assets SET revision=? WHERE root_id=? AND extension='xmp'",
                params![revision, root],
            )?;
            Ok(())
        })
        .unwrap();
    let review = f.review(MetadataExportKind::Xmp).unwrap();
    let names = review
        .files
        .iter()
        .map(|v| v.name.clone())
        .collect::<Vec<_>>();
    f.save(review).unwrap();
    let merged = names
        .iter()
        .find(|n| n.ends_with(".xmp") && !n.contains("original"))
        .unwrap();
    assert!(
        std::fs::read_to_string(f.output.join(merged))
            .unwrap()
            .contains("retain-exactly")
    );
    let retained = names.iter().find(|n| n.contains("original")).unwrap();
    assert_eq!(std::fs::read(f.output.join(retained)).unwrap(), original);
    assert_eq!(std::fs::read(&f.companion).unwrap(), original);
    f.audit();
}
#[test]
fn late_destination_conflict_and_repeated_approval_retain_all_existing_versions() {
    let f = Fixture::new();
    let review = f.review(MetadataExportKind::Catalog).unwrap();
    let name = review.files[0].name.clone();
    std::fs::write(f.output.join(&name), b"new destination occupant").unwrap();
    assert!(f.save(review.clone()).is_err());
    assert_eq!(
        std::fs::read(f.output.join(&name)).unwrap(),
        b"new destination occupant"
    );
    assert!(f.save(review).is_err());
    f.audit();
}
#[test]
fn changed_catalog_staged_bytes_or_native_session_invalidate_the_exact_export_review() {
    let f = Fixture::new();
    let review = f.review(MetadataExportKind::Catalog).unwrap();
    let work = f.library.export_work(&review.id).unwrap();
    std::fs::write(work.join(&review.files[0].name), b"tampered owned export").unwrap();
    assert!(f.save(review).is_err());
    let review = f.review(MetadataExportKind::Catalog).unwrap();
    let id = f.asset.clone();
    f.library
        .catalog
        .call(move |db| {
            db.execute("UPDATE assets SET favorite=1 WHERE id=?", [id])?;
            db.execute("UPDATE library_state SET revision=revision+1", [])?;
            Ok(())
        })
        .unwrap();
    assert!(f.save(review).is_err());
    assert_eq!(std::fs::read_dir(&f.output).unwrap().count(), 1);
    f.audit();
}

#[test]
fn fresh_companion_inventory_covers_unindexed_additions_and_refuses_ambiguous_variants() {
    let f = Fixture::new();
    let review = f.review(MetadataExportKind::Xmp).unwrap();
    let acr = f.owned.path().join("source/Photo.acr");
    std::fs::write(&acr, b"unindexed camera editing instructions").unwrap();
    assert!(f.save(review).is_err());
    assert_eq!(std::fs::read_dir(&f.output).unwrap().count(), 1);
    let review = f.review(MetadataExportKind::Xmp).unwrap();
    let artifact = review
        .files
        .iter()
        .find(|v| v.name.ends_with(".acr"))
        .unwrap()
        .name
        .clone();
    f.save(review).unwrap();
    assert_eq!(
        std::fs::read(f.output.join(artifact)).unwrap(),
        b"unindexed camera editing instructions"
    );
    std::fs::write(
        f.owned.path().join("source/Photo.raw"),
        b"generated second variant",
    )
    .unwrap();
    assert!(f.review(MetadataExportKind::Xmp).is_err());
    f.audit();
}
#[test]
#[cfg(unix)]
fn linked_companion_is_never_followed_and_cancelled_output_is_retained_without_publication() {
    let f = Fixture::new();
    let private = f.owned.path().join("outside-generated-private");
    std::fs::write(&private, b"generated private neighbor").unwrap();
    let link = f.owned.path().join("source/Photo.json");
    std::os::unix::fs::symlink(&private, &link).unwrap();
    assert!(f.review(MetadataExportKind::Xmp).is_err());
    assert_eq!(
        std::fs::read(&private).unwrap(),
        b"generated private neighbor"
    );
    let directory = review_export::output_chain(&f.output.canonicalize().unwrap()).unwrap();
    let stage = format!(".stillharbour-review-{}.partial", uuid::Uuid::new_v4());
    let filename = format!("stillharbour-catalog-{}.stillharbour", uuid::Uuid::new_v4());
    let mut source = File::open(&f.companion).unwrap();
    let result = review_export::save_owned_file(
        &f.output,
        &directory,
        &stage,
        &filename,
        &mut source,
        &AtomicBool::new(true),
    );
    assert!(result.is_err());
    assert!(!f.output.join(filename).exists());
    assert!(f.output.join(stage).exists());
    f.audit();
}

#[test]
fn actual_export_process_exit_keeps_catalog_sources_neighbor_and_unambiguous_completion_state() {
    use std::time::{Duration, Instant};
    for phase in ["approved", "published", "file_verified"] {
        let owner = tempfile::Builder::new()
            .prefix("stillharbour-m5-export-driver-")
            .tempdir()
            .unwrap();
        let token = uuid::Uuid::new_v4().to_string();
        std::fs::write(owner.path().join("owner"), &token).unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "library::metadata_export::tests::owned_export_exit_child",
                "--nocapture",
            ])
            .env("STILLHARBOUR_M5_EXPORT_DRIVER", owner.path())
            .env("STILLHARBOUR_M5_EXPORT_TOKEN", &token)
            .env("STILLHARBOUR_M5_EXPORT_EXIT", phase)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            assert!(Instant::now() < deadline, "Owned export child timed out");
            std::thread::sleep(Duration::from_millis(25));
        };
        assert_eq!(status.code(), Some(92));
        let receipt: serde_json::Value =
            serde_json::from_slice(&std::fs::read(owner.path().join("receipt.json")).unwrap())
                .unwrap();
        let root = PathBuf::from(receipt["root"].as_str().unwrap())
            .canonicalize()
            .unwrap();
        assert!(
            root.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("stillharbour-m5-export-")
        );
        assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        assert_eq!(
            std::fs::read_to_string(root.join("m5-owner")).unwrap(),
            token
        );
        let output = root.join("output");
        assert_eq!(
            std::fs::read(output.join("destination-only")).unwrap(),
            b"retain destination"
        );
        assert_eq!(
            std::fs::read(root.join("source/Photo.jpg")).unwrap(),
            b"generated original photo bytes"
        );
        let catalog = Catalog::open(root.join("data/Catalog.stillharbour")).unwrap();
        let count = catalog
            .call(|db| {
                Ok(db.query_row(
                    "SELECT count(*) FROM catalog_export_journal WHERE phase='completed'",
                    [],
                    |r| r.get::<_, i64>(0),
                )?)
            })
            .unwrap();
        assert_eq!(count, 0);
        let published = output.join(receipt["filename"].as_str().unwrap());
        if phase == "approved" {
            assert!(!published.exists());
        } else {
            let copy = Catalog::open(published).unwrap();
            assert_eq!(
                copy.summary().unwrap().schema,
                crate::catalog::SCHEMA_VERSION
            );
            drop(copy);
        }
        drop(catalog);
        assert_eq!(
            std::fs::read_to_string(root.join("m5-owner")).unwrap(),
            token
        );
        std::fs::remove_dir_all(&root).unwrap();
    }
}
#[test]
fn owned_export_exit_child() {
    let Ok(driver) = std::env::var("STILLHARBOUR_M5_EXPORT_DRIVER") else {
        return;
    };
    let driver = PathBuf::from(driver);
    assert!(
        driver
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("stillharbour-m5-export-driver-")
    );
    let token = std::env::var("STILLHARBOUR_M5_EXPORT_TOKEN").unwrap();
    assert_eq!(
        std::fs::read_to_string(driver.join("owner")).unwrap(),
        token
    );
    let f = Fixture::new();
    std::fs::write(f.owned.path().join("m5-owner"), &token).unwrap();
    let review = f.review(MetadataExportKind::Catalog).unwrap();
    std::fs::write(
        driver.join("receipt.json"),
        serde_json::to_vec(
            &serde_json::json!({"root":f.owned.path(),"filename":review.files[0].name}),
        )
        .unwrap(),
    )
    .unwrap();
    f.save(review).unwrap();
    panic!("The requested process-exit boundary was not reached");
}

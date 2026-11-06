use super::*;
use crate::{
    catalog::Catalog,
    library::{
        LibraryOptions, WorkerPaths,
        model::{RootMode, ScanFilter},
    },
};
use std::time::{Duration, Instant};

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn append_review_freezes_all_regular_files_without_creating_output_or_authority() {
    let f = Fixture::new();
    for (name, bytes) in [
        ("movie.mp4", b"video".as_slice()),
        (".hidden", b"opaque"),
        ("movie.xmp", b"sidecar"),
    ] {
        std::fs::write(f.source.join(name), bytes).unwrap();
    }
    let run = f.start(CompareMode::Verified);
    let run = f.wait(&run.id);
    let plan = f
        .library
        .backup_append_review(run.id.clone(), vec![])
        .unwrap();
    assert_eq!(plan.copy_files, 3);
    assert_eq!(plan.copy_bytes, "18");
    assert!(!plan.execution_available);
    assert_eq!(
        plan.digest,
        f.library
            .backup_append_review(run.id.clone(), vec![])
            .unwrap()
            .digest
    );
    assert!(!f.dataset().exists());
    assert_eq!(std::fs::read_dir(&f.destination).unwrap().count(), 0);
    assert_eq!(std::fs::read(f.source.join("movie.mp4")).unwrap(), b"video");
    assert!(
        f.library
            .backup_append_review("foreign".into(), vec![])
            .is_err()
    );
    let next = f.start(CompareMode::Verified);
    f.wait(&next.id);
    assert!(f.library.backup_append_review(run.id, vec![]).is_err());
    assert!(!f.dataset().exists());
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn append_review_retains_existing_destination_and_marks_uncopied_differences() {
    let f = Fixture::new();
    let dataset = f.dataset();
    std::fs::create_dir_all(&dataset).unwrap();
    std::fs::write(f.source.join("new.mp4"), b"new").unwrap();
    std::fs::write(f.source.join("different.jpg"), b"source").unwrap();
    std::fs::write(dataset.join("different.jpg"), b"edited").unwrap();
    std::fs::write(dataset.join("unique.bin"), b"destination only").unwrap();
    let run = f.start(CompareMode::Verified);
    let run = f.wait(&run.id);
    assert!(
        f.library
            .backup_append_review(run.id.clone(), vec![])
            .is_err()
    );
    let rows = f
        .library
        .backup_comparison_page(run.id.clone(), 0)
        .unwrap()
        .entries;
    let decision = rows
        .iter()
        .find(|r| r.display_path == "different.jpg")
        .unwrap()
        .id
        .clone();
    let plan = f
        .library
        .backup_append_review(run.id, vec![decision])
        .unwrap();
    assert_eq!(plan.copy_files, 1);
    assert_eq!(plan.retained_destination_files, 2);
    assert_eq!(plan.uncopied_source_revisions, 1);
    assert!(!dataset.join("new.mp4").exists());
    assert_eq!(
        std::fs::read(dataset.join("different.jpg")).unwrap(),
        b"edited"
    );
    assert_eq!(
        std::fs::read(dataset.join("unique.bin")).unwrap(),
        b"destination only"
    );
    assert_eq!(
        std::fs::read(f.source.join("different.jpg")).unwrap(),
        b"source"
    );
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn append_review_rejects_late_arrival_and_changed_source_without_writes() {
    for change in ["source", "dataset", "new_file", "type"] {
        let f = Fixture::new();
        std::fs::write(f.source.join("photo.jpg"), b"original").unwrap();
        let run = f.start(CompareMode::Verified);
        let run = f.wait(&run.id);
        match change {
            "source" => std::fs::write(f.source.join("photo.jpg"), b"changed!").unwrap(),
            "dataset" => std::fs::create_dir_all(f.dataset()).unwrap(),
            "new_file" => std::fs::write(f.source.join("late.xmp"), b"unique").unwrap(),
            "type" => {
                std::fs::remove_file(f.source.join("photo.jpg")).unwrap();
                std::fs::create_dir(f.source.join("photo.jpg")).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(
            f.library.backup_append_review(run.id, vec![]).is_err(),
            "{change}"
        );
        assert!(!f.dataset().join("photo.jpg").exists());
    }
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn append_review_rejects_changed_existing_destination_quick_and_revoked_scopes() {
    let f = Fixture::new();
    let dataset = f.dataset();
    std::fs::create_dir_all(&dataset).unwrap();
    std::fs::write(f.source.join("photo.jpg"), b"same").unwrap();
    std::fs::write(dataset.join("photo.jpg"), b"same").unwrap();
    let quick = f.start(CompareMode::Quick);
    let quick = f.wait(&quick.id);
    assert!(f.library.backup_append_review(quick.id, vec![]).is_err());
    let run = f.start(CompareMode::Verified);
    let run = f.wait(&run.id);
    std::fs::write(dataset.join("photo.jpg"), b"edit").unwrap();
    assert!(f.library.backup_append_review(run.id, vec![]).is_err());
    assert_eq!(std::fs::read(dataset.join("photo.jpg")).unwrap(), b"edit");
    let run = f.start(CompareMode::Verified);
    let run = f.wait(&run.id);
    let lease = Registry::shared()
        .try_acquire(vec![Scope::new(
            DirectoryChain::open(&f.source).unwrap(),
            Access::Write,
        )])
        .unwrap();
    assert_eq!(
        f.library
            .backup_append_review(run.id.clone(), vec![])
            .unwrap_err()
            .code,
        ErrorCode::Busy
    );
    drop(lease);
    f.library.backup_compare_cancel(run.id.clone()).unwrap();
    assert!(f.library.backup_append_review(run.id, vec![]).is_err());
    assert_eq!(std::fs::read(f.source.join("photo.jpg")).unwrap(), b"same");
}

#[test]
fn comparison_directory_link_counts_do_not_replace_native_object_identity() {
    let a = FileIdentity {
        volume: 2,
        file: 3,
        links: 1,
    };
    assert!(same_directory(
        &a,
        &FileIdentity {
            links: 8,
            ..a.clone()
        }
    ));
    assert!(!same_directory(
        &a,
        &FileIdentity {
            file: 4,
            ..a.clone()
        }
    ));
    assert!(!same_directory(
        &a,
        &FileIdentity {
            volume: 5,
            ..a.clone()
        }
    ));
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn comparison_profiles_reopen_with_native_bindings_and_no_approval() {
    let f = Fixture::new();
    std::fs::write(f.source.join("original.mp4"), b"preserved fixture bytes").unwrap();
    let handle = native::open_root(&f.destination, None).unwrap();
    let profile = f
        .library
        .backup_profile_save(
            f.destination.clone(),
            (
                filesystem::identity(&handle).unwrap(),
                native::volume_token(&handle).unwrap(),
            ),
            "Generated comparison".into(),
        )
        .unwrap();
    let path = f
        .library
        .options
        .data_directory
        .join("Fixture.stillharbour");
    let options = f.library.options.clone();
    let Fixture {
        library,
        _owned,
        source,
        destination,
        ..
    } = f;
    drop(library);
    let reopened = Library::new(Arc::new(Catalog::open(path).unwrap()), options).unwrap();
    let profiles = reopened.backup_profiles().unwrap();
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].id, profile.id);
    let run = reopened
        .backup_profile_compare(profile.id.clone(), CompareMode::Verified)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let result = reopened.backup_comparison(run.id.clone()).unwrap();
        if result.state != ComparisonState::Comparing {
            assert_eq!(
                result.state,
                ComparisonState::NeedsReview,
                "{}",
                result.message
            );
            assert_eq!(result.profile_id.as_deref(), Some(profile.id.as_str()));
            assert!(!result.capabilities.execution_available);
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        std::fs::read(source.join("original.mp4")).unwrap(),
        b"preserved fixture bytes"
    );
    assert_eq!(std::fs::read_dir(destination).unwrap().count(), 0);
    drop(reopened);
    drop(_owned);
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn comparison_profile_never_follows_a_changed_base_or_relinked_source() {
    let f = Fixture::new();
    let destination = native::open_root(&f.destination, None).unwrap();
    let expected = (
        filesystem::identity(&destination).unwrap(),
        native::volume_token(&destination).unwrap(),
    );
    assert!(
        f.library
            .backup_profile_save(f.destination.clone(), expected.clone(), "\n".into())
            .is_err()
    );
    let profile = f
        .library
        .backup_profile_save(f.destination.clone(), expected, "Pinned source".into())
        .unwrap();
    let other_path = f.source.parent().unwrap().join("other-source");
    std::fs::create_dir(&other_path).unwrap();
    let other_file = native::open_root(&other_path, None).unwrap();
    let other = store::add_root(
        &f.library.catalog,
        other_path.clone(),
        filesystem::identity(&other_file).unwrap(),
        native::volume_token(&other_file).unwrap(),
        "New base".into(),
        RootMode::Referenced,
        ScanFilter::default(),
    )
    .unwrap();
    let root_id = f.root.clone();
    let new_id = other.clone();
    f.library
        .catalog
        .call(move |db| {
            db.execute("UPDATE roots SET is_base=0 WHERE id=?", [root_id])?;
            db.execute("UPDATE roots SET is_base=1 WHERE id=?", [new_id])?;
            Ok(())
        })
        .unwrap();
    let run = f
        .library
        .backup_profile_compare(profile.id.clone(), CompareMode::Verified)
        .unwrap();
    assert_eq!(run.source_root_id, f.root);
    assert_eq!(f.wait(&run.id).state, ComparisonState::NeedsReview);
    let id = f.root.clone();
    let native_path = native::encode_path(&other_path);
    f.library
        .catalog
        .call(move |db| {
            db.execute(
                "UPDATE roots SET native_path=? WHERE id=?",
                rusqlite::params![native_path, id],
            )?;
            Ok(())
        })
        .unwrap();
    assert!(
        f.library
            .backup_profile_compare(profile.id.clone(), CompareMode::Verified)
            .is_err()
    );
    let id = profile.id.clone();
    f.library
        .catalog
        .call(move |db| {
            assert!(
                db.execute(
                    "UPDATE backup_sync_profiles SET record='{}' WHERE id=?",
                    [&id]
                )
                .is_err()
            );
            assert!(
                db.execute("DELETE FROM backup_sync_profiles WHERE id=?", [&id])
                    .is_err()
            );
            Ok(())
        })
        .unwrap();
    assert!(
        f.library
            .backup_profile_compare(uuid::Uuid::new_v4().to_string(), CompareMode::Verified)
            .is_err()
    );
    assert_eq!(
        f.library.backup_profiles().unwrap()[0].source_root_id,
        f.root
    );
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn comparison_profile_refuses_a_reused_destination_path_without_writing() {
    let f = Fixture::new();
    let handle = native::open_root(&f.destination, None).unwrap();
    let profile = f
        .library
        .backup_profile_save(
            f.destination.clone(),
            (
                filesystem::identity(&handle).unwrap(),
                native::volume_token(&handle).unwrap(),
            ),
            "Destination identity".into(),
        )
        .unwrap();
    drop(handle);
    let boundary = f._owned.path().canonicalize().unwrap();
    let retained = f.destination.with_file_name("retained-destination");
    assert!(
        f.destination.starts_with(&boundary)
            && retained.starts_with(&boundary)
            && !retained.exists()
    );
    std::fs::rename(&f.destination, &retained).unwrap();
    std::fs::create_dir(&f.destination).unwrap();
    std::fs::write(
        f.destination.join("neighbor"),
        b"retain replacement neighbor",
    )
    .unwrap();
    assert!(
        f.library
            .backup_profile_compare(profile.id, CompareMode::Verified)
            .is_err()
    );
    assert_eq!(
        std::fs::read(f.destination.join("neighbor")).unwrap(),
        b"retain replacement neighbor"
    );
    assert_eq!(std::fs::read_dir(retained).unwrap().count(), 0);
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn comparison_profiles_bound_admission_without_replacing_saved_bindings() {
    let f = Fixture::new();
    let file = native::open_root(&f.destination, None).unwrap();
    let expected = (
        filesystem::identity(&file).unwrap(),
        native::volume_token(&file).unwrap(),
    );
    let mut ids = Vec::new();
    for index in 0..32 {
        ids.push(
            f.library
                .backup_profile_save(
                    f.destination.clone(),
                    expected.clone(),
                    format!("Profile {index}"),
                )
                .unwrap()
                .id,
        );
    }
    assert!(
        f.library
            .backup_profile_save(f.destination.clone(), expected, "Not admitted".into())
            .is_err()
    );
    assert_eq!(
        f.library
            .backup_profiles()
            .unwrap()
            .iter()
            .map(|p| p.id.clone())
            .collect::<Vec<_>>(),
        ids
    );
    assert_eq!(std::fs::read_dir(&f.destination).unwrap().count(), 0);
}

#[test]
fn comparison_fills_short_reads_and_obeys_cancellation() {
    struct Short(std::io::Cursor<Vec<u8>>);
    impl std::io::Read for Short {
        fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
            let n = bytes.len().min(3);
            std::io::Read::read(&mut self.0, &mut bytes[..n])
        }
    }
    let mut file = Short(std::io::Cursor::new(b"complete bytes".to_vec()));
    let mut buffer = [0; 64];
    let mut count = 0;
    let cancel = AtomicBool::new(false);
    assert_eq!(
        fill(&mut file, &mut buffer, &cancel, &mut count).unwrap(),
        14
    );
    assert_eq!(&buffer[..14], b"complete bytes");
    assert_eq!(count, 14);
    cancel.store(true, Ordering::Release);
    assert!(fill(&mut file, &mut buffer, &cancel, &mut count).is_err());
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn comparison_blocks_native_aliases_and_file_directory_conflicts_without_reading_them() {
    let f = Fixture::new();
    let dataset = f.dataset();
    assert!(dataset.starts_with(f._owned.path().canonicalize().unwrap()));
    std::fs::create_dir_all(dataset.join("parent")).unwrap();
    for (path, bytes) in [
        (f.source.join("Clip.MP4"), b"source bytes".as_slice()),
        (dataset.join("clip.mp4"), b"destination bytes"),
        (f.source.join("parent"), b"source parent file"),
        (dataset.join("parent/child.xmp"), b"destination child"),
    ] {
        std::fs::write(path, bytes).unwrap();
    }
    let started = f.start(CompareMode::Verified);
    let result = f.wait(&started.id);
    assert_eq!(
        result.state,
        ComparisonState::Incomplete,
        "{}",
        result.message
    );
    assert_eq!(result.policy_version, 3);
    assert!(result.plan_digest.is_none());
    assert_eq!(result.bytes_read, "0");
    assert_eq!(result.summary.as_ref().unwrap().blocked, 4);
    let page = f.library.backup_comparison_page(started.id, 0).unwrap();
    assert_eq!(page.entries.len(), 4);
    assert!(
        page.entries
            .iter()
            .all(|r| r.category == ComparisonCategory::Blocked
                && r.proposal == SyncProposal::Blocked)
    );
    let parent = page
        .entries
        .iter()
        .find(|r| r.display_path == "parent")
        .unwrap();
    assert_eq!(parent.source_kind, Some(ComparisonEntryKind::File));
    assert_eq!(
        parent.destination_kind,
        Some(ComparisonEntryKind::Directory)
    );
    assert_eq!(
        std::fs::read(f.source.join("Clip.MP4")).unwrap(),
        b"source bytes"
    );
    assert_eq!(
        std::fs::read(dataset.join("clip.mp4")).unwrap(),
        b"destination bytes"
    );
    assert_eq!(
        std::fs::read(f.source.join("parent")).unwrap(),
        b"source parent file"
    );
    assert_eq!(
        std::fs::read(dataset.join("parent/child.xmp")).unwrap(),
        b"destination child"
    );
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn comparison_filters_the_whole_report_with_filtered_offsets_and_summary() {
    let f = Fixture::new();
    let dataset = f.dataset();
    std::fs::create_dir_all(&dataset).unwrap();
    for n in 0..205 {
        std::fs::write(f.source.join(format!("source-{n:03}.bin")), b"source").unwrap();
        std::fs::write(dataset.join(format!("destination-{n:03}.xmp")), b"retain").unwrap();
    }
    let run = f.start(CompareMode::Quick);
    let result = f.wait(&run.id);
    assert_eq!(result.state, ComparisonState::NeedsReview);
    let summary = result.summary.unwrap();
    assert_eq!(summary.source_only, 205);
    assert_eq!(summary.destination_only, 205);
    let page = f
        .library
        .backup_comparison_page_filtered(run.id.clone(), 0, Some(ComparisonCategory::SourceOnly))
        .unwrap();
    assert_eq!(page.total, Some(205));
    assert_eq!(page.entries.len(), 200);
    assert_eq!(page.next_offset, Some(200));
    assert!(
        page.entries
            .iter()
            .all(|r| r.category == ComparisonCategory::SourceOnly)
    );
    let last = f
        .library
        .backup_comparison_page_filtered(run.id.clone(), 200, Some(ComparisonCategory::SourceOnly))
        .unwrap();
    assert_eq!(last.entries.len(), 5);
    assert_eq!(last.next_offset, None);
    assert_ne!(page.entries[0].id, last.entries[0].id);
    let empty = f
        .library
        .backup_comparison_page_filtered(run.id.clone(), 0, Some(ComparisonCategory::Blocked))
        .unwrap();
    assert_eq!(empty.total, Some(0));
    assert!(empty.entries.is_empty());
    assert!(
        f.library
            .backup_comparison_page_filtered(run.id, 206, Some(ComparisonCategory::SourceOnly))
            .is_err()
    );
    assert_eq!(std::fs::read_dir(f.source).unwrap().count(), 205);
    assert_eq!(std::fs::read_dir(dataset).unwrap().count(), 205);
}

#[cfg(any(windows, target_os = "macos"))]
struct Fixture {
    _owned: tempfile::TempDir,
    library: Arc<Library>,
    source: PathBuf,
    destination: PathBuf,
    root: String,
}
#[cfg(any(windows, target_os = "macos"))]
impl Fixture {
    fn new() -> Self {
        let owned = tempfile::tempdir().unwrap();
        let parent = owned.path().canonicalize().unwrap();
        let source = parent.join("source");
        let destination = parent.join("destination");
        let data = parent.join("data");
        for p in [&source, &destination, &data] {
            std::fs::create_dir(p).unwrap();
        }
        let catalog = Arc::new(Catalog::open(data.join("Fixture.stillharbour")).unwrap());
        let library = Library::new(
            catalog.clone(),
            LibraryOptions {
                data_directory: data.clone(),
                workers: WorkerPaths {
                    executable: std::env::current_exe().unwrap(),
                    exiftool: None,
                    working_directory: data.join("worker"),
                },
            },
        )
        .unwrap();
        let file = native::open_root(&source, None).unwrap();
        let root = store::add_root(
            &catalog,
            source.clone(),
            filesystem::identity(&file).unwrap(),
            native::volume_token(&file).unwrap(),
            "Generated base".into(),
            RootMode::Referenced,
            ScanFilter {
                extensions: vec!["notindexed".into()],
                ..Default::default()
            },
        )
        .unwrap();
        Self {
            _owned: owned,
            library,
            source,
            destination,
            root,
        }
    }
    fn dataset(&self) -> PathBuf {
        self.destination
            .join(".stillharbour-copy")
            .join(self.library.catalog.summary().unwrap().id)
            .join(&self.root)
    }
    fn start(&self, mode: CompareMode) -> ComparisonRun {
        let file = native::open_root(&self.destination, None).unwrap();
        self.library
            .backup_compare_start(
                self.destination.clone(),
                (
                    filesystem::identity(&file).unwrap(),
                    native::volume_token(&file).unwrap(),
                ),
                mode,
            )
            .unwrap()
    }
    fn wait(&self, id: &str) -> ComparisonRun {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let run = self.library.backup_comparison(id.into()).unwrap();
            if run.state != ComparisonState::Comparing {
                return run;
            }
            assert!(Instant::now() < deadline, "comparison deadline");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn comparison_inventories_hidden_unknown_companions_and_never_creates_a_dataset() {
    let f = Fixture::new();
    for (name, bytes) in [
        (".hidden.xmp", b"xmp".as_slice()),
        ("unknown.bin", b"unknown"),
        ("broken.jpg", b"not an image"),
    ] {
        std::fs::write(f.source.join(name), bytes).unwrap();
    }
    std::fs::write(
        f.destination.join("destination-only.txt"),
        b"retain outside dataset",
    )
    .unwrap();
    let run = f.start(CompareMode::Verified);
    let result = f.wait(&run.id);
    assert_eq!(
        result.state,
        ComparisonState::NeedsReview,
        "{}",
        result.message
    );
    assert_eq!(result.source_root_id, f.root);
    assert_eq!(result.entries, 3);
    assert_eq!(result.bytes_read, "0");
    assert!(result.plan_digest.is_some());
    assert!(!result.capabilities.execution_available);
    assert!(!f.dataset().exists());
    assert_eq!(
        std::fs::read(f.destination.join("destination-only.txt")).unwrap(),
        b"retain outside dataset"
    );
    assert!(
        f.library
            .backup_comparison_page(run.id, 0)
            .unwrap()
            .entries
            .iter()
            .all(|e| e.category == ComparisonCategory::SourceOnly)
    );
    assert_eq!(
        std::fs::read(f.source.join("broken.jpg")).unwrap(),
        b"not an image"
    );
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn comparison_quick_is_unverified_and_full_bytes_detect_restored_time_edits() {
    let f = Fixture::new();
    let dataset = f.dataset();
    std::fs::create_dir_all(&dataset).unwrap();
    for (name, a, b) in [
        ("same.bin", b"same".as_slice(), b"same".as_slice()),
        ("edited.bin", b"left", b"edit"),
    ] {
        std::fs::write(f.source.join(name), a).unwrap();
        std::fs::write(dataset.join(name), b).unwrap();
        let time = std::fs::metadata(f.source.join(name))
            .unwrap()
            .modified()
            .unwrap();
        File::options()
            .write(true)
            .open(dataset.join(name))
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(time))
            .unwrap();
    }
    std::fs::write(dataset.join("keep.bin"), b"unique destination").unwrap();
    let quick = f.start(CompareMode::Quick);
    assert_eq!(f.wait(&quick.id).state, ComparisonState::NeedsReview);
    let rows = f
        .library
        .backup_comparison_page(quick.id, 0)
        .unwrap()
        .entries;
    assert_eq!(
        rows.iter()
            .find(|r| r.display_path == "edited.bin")
            .unwrap()
            .category,
        ComparisonCategory::AppearsUnchangedUnverified
    );
    let verified = f.start(CompareMode::Verified);
    let report = f.wait(&verified.id);
    assert_eq!(
        report.state,
        ComparisonState::NeedsReview,
        "{}",
        report.message
    );
    let rows = f
        .library
        .backup_comparison_page(verified.id.clone(), 0)
        .unwrap()
        .entries;
    assert_eq!(
        rows.iter()
            .find(|r| r.display_path == "same.bin")
            .unwrap()
            .category,
        ComparisonCategory::VerifiedEqual
    );
    assert_eq!(
        rows.iter()
            .find(|r| r.display_path == "edited.bin")
            .unwrap()
            .category,
        ComparisonCategory::DifferentContent
    );
    assert_eq!(
        rows.iter()
            .find(|r| r.display_path == "keep.bin")
            .unwrap()
            .proposal,
        SyncProposal::KeepDestination
    );
    assert_eq!(
        std::fs::read(dataset.join("keep.bin")).unwrap(),
        b"unique destination"
    );
    assert_eq!(std::fs::read(f.source.join("edited.bin")).unwrap(), b"left");
    assert!(f.library.backup_compare_cancel("foreign".into()).is_err());
    assert!(f.library.backup_comparison_page(verified.id, 1000).is_err());
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn comparison_cancellation_rejects_overlap_and_retains_both_trees() {
    let f = Fixture::new();
    let dataset = f.dataset();
    std::fs::create_dir_all(&dataset).unwrap();
    let bytes = vec![19; 8 * 1024 * 1024];
    std::fs::write(f.source.join("large.bin"), &bytes).unwrap();
    std::fs::write(dataset.join("large.bin"), &bytes).unwrap();
    let run = f.start(CompareMode::Verified);
    let file = native::open_root(&f.destination, None).unwrap();
    assert_eq!(
        f.library
            .backup_compare_start(
                f.destination.clone(),
                (
                    filesystem::identity(&file).unwrap(),
                    native::volume_token(&file).unwrap()
                ),
                CompareMode::Quick
            )
            .unwrap_err()
            .code,
        ErrorCode::Busy
    );
    assert!(f.library.backup_compare_cancel("foreign".into()).is_err());
    f.library.backup_compare_cancel(run.id.clone()).unwrap();
    let stopped = f.wait(&run.id);
    assert_eq!(
        stopped.state,
        ComparisonState::Canceled,
        "{}",
        stopped.message
    );
    assert!(stopped.plan_digest.is_none());
    assert_eq!(std::fs::read(f.source.join("large.bin")).unwrap(), bytes);
    assert_eq!(std::fs::read(dataset.join("large.bin")).unwrap(), bytes);
    let next = f.start(CompareMode::Quick);
    assert_eq!(f.wait(&next.id).state, ComparisonState::NeedsReview);
    assert!(f.library.backup_comparison(run.id).is_err());
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn comparison_rejects_overlaps_changed_identity_and_owned_writer_leases() {
    let f = Fixture::new();
    let file = native::open_root(&f.source, None).unwrap();
    let identity = filesystem::identity(&file).unwrap();
    let volume = native::volume_token(&file).unwrap();
    assert!(
        f.library
            .backup_compare_start(f.source.clone(), (identity, volume), CompareMode::Verified)
            .is_err()
    );
    let file = native::open_root(&f.destination, None).unwrap();
    let mut identity = filesystem::identity(&file).unwrap();
    identity.file += 1;
    assert!(
        f.library
            .backup_compare_start(
                f.destination.clone(),
                (identity, native::volume_token(&file).unwrap()),
                CompareMode::Verified
            )
            .is_err()
    );
    let lease = Registry::shared()
        .try_acquire(vec![Scope::new(
            DirectoryChain::open(&f.source).unwrap(),
            Access::Write,
        )])
        .unwrap();
    let file = native::open_root(&f.destination, None).unwrap();
    assert!(
        f.library
            .backup_compare_start(
                f.destination.clone(),
                (
                    filesystem::identity(&file).unwrap(),
                    native::volume_token(&file).unwrap()
                ),
                CompareMode::Verified
            )
            .is_err()
    );
    drop(lease);
    let run = f.start(CompareMode::Verified);
    assert_eq!(f.wait(&run.id).state, ComparisonState::NeedsReview);
}

#[test]
#[cfg(target_os = "macos")]
fn comparison_does_not_follow_links_and_marks_inventory_incomplete() {
    use std::os::unix::fs::symlink;
    let owned = tempfile::tempdir().unwrap();
    let path = owned.path().canonicalize().unwrap();
    std::fs::write(path.join("preserved"), b"valuable").unwrap();
    symlink(path.join("preserved"), path.join("link")).unwrap();
    let file = native::open_root(&path, None).unwrap();
    let root = Binding {
        path,
        identity: filesystem::identity(&file).unwrap(),
        volume: native::volume_token(&file).unwrap(),
    };
    let mut inventory = Inventory::default();
    inventory
        .walk(&root, Path::new(""), &AtomicBool::new(false), 0)
        .unwrap();
    assert!(inventory.incomplete);
    assert_eq!(inventory.files.len(), 1);
    assert_eq!(inventory.issues.len(), 1);
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn comparison_history_detects_destination_edits_without_reusing_equality_or_recovery() {
    let f = Fixture::new();
    let dataset = f.dataset();
    std::fs::create_dir_all(&dataset).unwrap();
    std::fs::write(f.source.join("photo.jpg"), b"left").unwrap();
    std::fs::write(f.source.join("photo.xmp"), b"unique sidecar").unwrap();
    std::fs::write(dataset.join("photo.jpg"), b"left").unwrap();
    let first = f.start(CompareMode::Verified);
    let first = f.wait(&first.id);
    assert!(first.baseline_id.is_none());
    assert_eq!(first.state, ComparisonState::NeedsReview);
    std::fs::write(dataset.join("photo.jpg"), b"edit").unwrap();
    let quick = f.start(CompareMode::Quick);
    let quick = f.wait(&quick.id);
    assert_eq!(quick.baseline_id.as_deref(), Some(first.id.as_str()));
    let full = f.start(CompareMode::Verified);
    let full = f.wait(&full.id);
    assert_eq!(full.baseline_id.as_deref(), Some(first.id.as_str()));
    let rows = f
        .library
        .backup_comparison_page(full.id.clone(), 0)
        .unwrap()
        .entries;
    let row = rows.iter().find(|r| r.display_path == "photo.jpg").unwrap();
    assert_eq!(
        row.category,
        ComparisonCategory::DestinationEditedSinceLastRun
    );
    assert_eq!(row.proposal, SyncProposal::ReviewDifference);
    assert_eq!(row.companions, vec!["photo.xmp"]);
    assert!(!full.capabilities.execution_available);
    assert!(!full.capabilities.independent_recovery_verified);
    f.library
        .catalog
        .call(|db| {
            assert_eq!(
                db.query_row("SELECT count(*) FROM backup_comparison_history WHERE scope!='utility-report-v1'", [], |r| {
                    r.get::<_, u32>(0)
                })?,
                2
            );
            assert!(
                db.execute("DELETE FROM backup_comparison_history", [])
                    .is_err()
            );
            assert!(
                db.execute("UPDATE backup_comparison_history SET scope='foreign'", [])
                    .is_err()
            );
            Ok(())
        })
        .unwrap();
    assert_eq!(std::fs::read(f.source.join("photo.jpg")).unwrap(), b"left");
    assert_eq!(std::fs::read(dataset.join("photo.jpg")).unwrap(), b"edit");
    assert_eq!(
        std::fs::read(f.source.join("photo.xmp")).unwrap(),
        b"unique sidecar"
    );
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn saved_reports_survive_new_runs_without_retaining_execution_authority() {
    let f = Fixture::new();
    std::fs::write(f.source.join("generated.bin"), b"original").unwrap();
    let first = f.start(CompareMode::Quick);
    f.wait(&first.id);
    let original = f
        .library
        .backup_comparison_page(first.id.clone(), 0)
        .unwrap();
    let second = f.start(CompareMode::Verified);
    f.wait(&second.id);
    let history = f.library.backup_comparison_history().unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].id, second.id);
    assert!(history.iter().all(|run| run.plan_digest.is_none()
        && run.review_plan.is_none()
        && !run.capabilities.execution_available));
    assert!(f.library.backup_comparison(first.id.clone()).is_err());
    let retained = f
        .library
        .backup_comparison_page(first.id.clone(), 0)
        .unwrap();
    assert_eq!(retained.entries.len(), original.entries.len());
    assert_eq!(
        retained.entries[0].display_path,
        original.entries[0].display_path
    );
    assert!(f.library.backup_compare_cancel(first.id.clone()).is_err());
    let catalog_path = f._owned.path().join("data/Fixture.stillharbour");
    let dataset = f.dataset();
    drop(f.library);
    // Reopening the database reads only saved observations; it does not reopen the source.
    let catalog = crate::catalog::Catalog::open(catalog_path).unwrap();
    assert_eq!(reports::list(&catalog).unwrap().len(), 2);
    assert_eq!(
        std::fs::read(f.source.join("generated.bin")).unwrap(),
        b"original"
    );
    assert!(!dataset.exists());
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn display_report_limits_and_imported_authority_do_not_affect_byte_observations() {
    let f = Fixture::new();
    let run = f.start(CompareMode::Quick);
    let run = f.wait(&run.id);
    for _ in 1..32 {
        let mut version = run.clone();
        version.id = uuid::Uuid::new_v4().to_string();
        reports::save(&f.library.catalog, &version, &[]).unwrap();
    }
    let mut unsaved = run.clone();
    unsaved.id = uuid::Uuid::new_v4().to_string();
    assert!(
        reports::save(&f.library.catalog, &unsaved, &[])
            .unwrap()
            .contains("history is full")
    );
    assert_eq!(reports::list(&f.library.catalog).unwrap().len(), 32);
    assert!(reports::load(&f.library.catalog, &unsaved.id).is_err());
    // Display reports cannot exhaust the separate byte-observation allowance.
    assert!(
        history::save(
            &f.library.catalog,
            &uuid::Uuid::new_v4().to_string(),
            "a".repeat(64),
            BTreeMap::new()
        )
        .unwrap()
        .contains("Saved complete")
    );
    let mut forged = serde_json::json!({ "version": 1, "run": run, "rows": [] });
    let id = uuid::Uuid::new_v4().to_string();
    forged["run"]["id"] = id.clone().into();
    forged["run"]["planDigest"] = "untrusted authority".into();
    forged["run"]["capabilities"]["executionAvailable"] = true.into();
    let text = forged.to_string();
    let key = format!("report:{id}");
    f.library.catalog.call(move |db| {
        db.execute("INSERT INTO backup_comparison_history(id,scope,record) VALUES(?,'utility-report-v1',?)", rusqlite::params![key, text])?;
        Ok(())
    }).unwrap();
    let (loaded, _) = reports::load(&f.library.catalog, &id).unwrap();
    assert!(loaded.plan_digest.is_none());
    assert!(!loaded.capabilities.execution_available);
    forged["run"]["catalogId"] = uuid::Uuid::new_v4().to_string().into();
    let id = uuid::Uuid::new_v4().to_string();
    forged["run"]["id"] = id.clone().into();
    let text = forged.to_string();
    let key = format!("report:{id}");
    f.library.catalog.call(move |db| {
        db.execute("INSERT INTO backup_comparison_history(id,scope,record) VALUES(?,'utility-report-v1',?)", rusqlite::params![key, text])?;
        Ok(())
    }).unwrap();
    assert!(reports::load(&f.library.catalog, &id).is_err());
    assert!(reports::list(&f.library.catalog).is_err());
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn comparison_rebinding_retains_prior_profile_and_requires_review_and_fresh_comparison() {
    let f = Fixture::new();
    let handle = native::open_root(&f.destination, None).unwrap();
    let expected = (
        filesystem::identity(&handle).unwrap(),
        native::volume_token(&handle).unwrap(),
    );
    let prior = f
        .library
        .backup_profile_save(f.destination.clone(), expected.clone(), "Original".into())
        .unwrap();
    assert!(
        f.library
            .backup_profile_rebind(
                prior.id.clone(),
                f.destination.clone(),
                expected.clone(),
                "New binding".into(),
                false
            )
            .is_err()
    );
    let next = f
        .library
        .backup_profile_rebind(
            prior.id.clone(),
            f.destination.clone(),
            expected,
            "New binding".into(),
            true,
        )
        .unwrap();
    assert_ne!(prior.id, next.id);
    assert_eq!(next.supersedes.as_deref(), Some(prior.id.as_str()));
    assert_eq!(f.library.backup_profiles().unwrap().len(), 2);
    assert!(f.library.backup_comparison_latest().unwrap().is_none());
    let run = f
        .library
        .backup_profile_compare(next.id.clone(), CompareMode::Verified)
        .unwrap();
    let result = f.wait(&run.id);
    assert_eq!(result.profile_id.as_deref(), Some(next.id.as_str()));
    assert!(!result.capabilities.execution_available);
    assert_eq!(std::fs::read_dir(f.destination).unwrap().count(), 0);
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn comparison_inventory_failures_and_late_changes_never_prove_absence_or_keep_a_digest() {
    let f = Fixture::new();
    std::fs::write(f.source.join("fixture.bin"), b"original").unwrap();
    let handle = native::open_root(&f.source, None).unwrap();
    let root = Binding {
        path: f.source.clone(),
        identity: filesystem::identity(&handle).unwrap(),
        volume: native::volume_token(&handle).unwrap(),
    };
    let mut inventory = Inventory::default();
    inventory
        .walk(&root, Path::new(""), &AtomicBool::new(false), 0)
        .unwrap();
    let observed_time = std::fs::metadata(f.source.join("fixture.bin"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(inventory.files.len(), 1);
    std::fs::write(f.source.join("fixture.bin"), b"modified").unwrap();
    // Preserve the same-length scenario but make its native revision change
    // deterministic: Windows writes can share a filesystem clock tick. This
    // test exercises revision invalidation, not universal content-change proof.
    std::fs::OpenOptions::new()
        .write(true)
        .open(f.source.join("fixture.bin"))
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(observed_time + Duration::from_secs(2)))
        .unwrap();
    assert!(inventory.validate(&root).is_err());
    let mut denied = Inventory::default();
    denied
        .walk(
            &root,
            Path::new("unavailable-directory"),
            &AtomicBool::new(false),
            0,
        )
        .unwrap();
    assert!(denied.incomplete);
    assert!(!denied.issues.is_empty());
    assert_eq!(
        classify(true, false, !denied.incomplete, None, false).0,
        ComparisonCategory::Uncertain
    );
    let mut bounded = Inventory {
        visited: MAX_ENTRIES,
        ..Default::default()
    };
    bounded
        .walk(&root, Path::new(""), &AtomicBool::new(false), 0)
        .unwrap();
    assert!(bounded.incomplete);
    assert!(bounded.files.is_empty());
    let dataset = f.dataset();
    assert!(dataset.starts_with(f._owned.path().canonicalize().unwrap()));
    std::fs::create_dir_all(dataset.parent().unwrap()).unwrap();
    std::fs::write(&dataset, b"namespace occupant").unwrap();
    let run = f.start(CompareMode::Verified);
    let report = f.wait(&run.id);
    assert_eq!(report.state, ComparisonState::Failed);
    assert!(report.plan_digest.is_none());
    assert_eq!(std::fs::read(dataset).unwrap(), b"namespace occupant");
    assert_eq!(
        std::fs::read(f.source.join("fixture.bin")).unwrap(),
        b"modified"
    );
}

#[test]
#[cfg(any(windows, target_os = "macos"))]
fn comparison_preview_refuses_revoked_source_before_starting_a_decoder() {
    let f = Fixture::new();
    std::fs::write(f.source.join("original.png"), b"preserved undecoded source").unwrap();
    let run = f.start(CompareMode::Verified);
    assert_eq!(f.wait(&run.id).state, ComparisonState::NeedsReview);
    let row = f
        .library
        .backup_comparison_page(run.id.clone(), 0)
        .unwrap()
        .entries
        .remove(0);
    let id = f.root.clone();
    f.library
        .catalog
        .call(move |db| {
            db.execute("UPDATE roots SET availability='offline' WHERE id=?", [id])?;
            Ok(())
        })
        .unwrap();
    assert!(
        f.library
            .backup_comparison_preview(run.id.clone(), row.id, ComparisonSide::Source)
            .is_err()
    );
    assert_eq!(
        std::fs::read(f.source.join("original.png")).unwrap(),
        b"preserved undecoded source"
    );
}

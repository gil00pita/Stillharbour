//! Metadata-only direct-directory inventory, independent of gallery filters.
//! Private native locators are bound to opaque evidence IDs, never UI paths.
use super::{Library, native, organization, store};
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem::{self, DirectoryChain},
    operations::{
        BundleEntryKind, BundleInventory, BundleInventoryEntry, BundleRelation,
        MAX_BUNDLE_DIRECTORY_ENTRIES, validate_id,
    },
    organization::PreviewSource,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{OsStr, OsString},
    fs::File,
    path::{Component, Path},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NativeBundleEntry {
    pub view: BundleInventoryEntry,
    pub relative_native: Vec<u8>,
    pub identity: Option<(u64, u64)>,
    pub revision: Option<String>,
}

pub(crate) struct InventoryObservation {
    pub view: BundleInventory,
    pub source: PreviewSource,
    pub entries: Vec<NativeBundleEntry>,
}

#[derive(Clone)]
struct IndexedEntry {
    id: String,
    presence: String,
}

fn changed() -> AppError {
    AppError::new(
        ErrorCode::Conflict,
        "The source folder or catalog changed. Build a fresh preview and review all filenames again.",
    )
}

pub(crate) fn relation(name: &OsStr, source: &OsStr) -> BundleRelation {
    use BundleRelation::*;
    if name == source {
        return Primary;
    }
    let mut double = OsString::from("._");
    double.push(source);
    if name == double {
        return AppleDouble;
    }
    let name = Path::new(name);
    let source = Path::new(source);
    if name.file_stem() == source.file_name()
        && name
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|ext| ext.eq_ignore_ascii_case("xmp") || ext.eq_ignore_ascii_case("aae"))
    {
        return FilenameSidecar;
    }
    if name.file_stem().is_some() && name.file_stem() == source.file_stem() {
        return SameStem;
    }
    Additional
}

fn open_child(parent: &File, name: &OsStr, directory: bool) -> Result<File> {
    #[cfg(windows)]
    {
        if directory {
            filesystem::windows::child(parent, name, true)
        } else {
            filesystem::windows::metadata_child(parent, name)
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::CString::new(name.as_bytes()).map_err(|_| changed())?;
        #[cfg(target_os = "macos")]
        let file = if directory {
            native::child_file(parent, &name, true)?
        } else {
            use std::os::fd::{AsRawFd, FromRawFd};
            // SAFETY: live borrowed parent descriptor and owned C string; this
            // requests an event/metadata descriptor, with no create/write flags.
            let fd = unsafe {
                libc::openat(
                    parent.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_EVTONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            // SAFETY: successful open transfers one fresh descriptor exactly once.
            unsafe { File::from_raw_fd(fd) }
        };
        #[cfg(not(target_os = "macos"))]
        let file = native::child_file(parent, &name, directory)?;
        #[cfg(target_os = "macos")]
        {
            use std::os::macos::fs::MetadataExt;
            if file.metadata()?.st_flags() & 0x40000000 != 0 {
                return Err(AppError::new(
                    ErrorCode::Unsupported,
                    "Cloud placeholders are not downloaded for companion review.",
                ));
            }
        }
        Ok(file)
    }
}

struct NameEntry {
    name: OsString,
    kind: BundleEntryKind,
    bytes: Option<u64>,
    file_id: Option<u64>,
}

#[cfg(windows)]
fn names(parent: &File) -> Result<(Vec<NameEntry>, bool)> {
    use windows_sys::Win32::Storage::FileSystem::*;
    let mut directory = filesystem::windows::Directory::new(parent)?;
    let mut entries = Vec::new();
    while let Some(entry) = directory.next()? {
        if entry.name == "." || entry.name == ".." {
            continue;
        }
        if entries.len() == MAX_BUNDLE_DIRECTORY_ENTRIES {
            return Ok((entries, true));
        }
        let kind = if entry.attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            BundleEntryKind::LinkOrSpecial
        } else if entry.attributes
            & (FILE_ATTRIBUTE_OFFLINE
                | FILE_ATTRIBUTE_RECALL_ON_OPEN
                | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
            != 0
        {
            BundleEntryKind::Placeholder
        } else if entry.attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            BundleEntryKind::Directory
        } else {
            BundleEntryKind::File
        };
        entries.push(NameEntry {
            name: entry.name,
            bytes: (kind == BundleEntryKind::File).then_some(entry.bytes),
            kind,
            file_id: Some(entry.file_id),
        });
    }
    Ok((entries, false))
}

#[cfg(unix)]
fn names(parent: &File) -> Result<(Vec<NameEntry>, bool)> {
    use rustix::fs::{AtFlags, Dir, FileType, statat};
    use std::os::unix::ffi::OsStrExt;
    let directory = Dir::read_from(parent).map_err(std::io::Error::from)?;
    let mut entries = Vec::new();
    for entry in directory {
        let entry = entry.map_err(std::io::Error::from)?;
        if matches!(entry.file_name().to_bytes(), b"." | b"..") {
            continue;
        }
        if entries.len() == MAX_BUNDLE_DIRECTORY_ENTRIES {
            return Ok((entries, true));
        }
        let name = OsStr::from_bytes(entry.file_name().to_bytes()).to_owned();
        let stat = match statat(parent, entry.file_name(), AtFlags::SYMLINK_NOFOLLOW) {
            Ok(stat) => stat,
            Err(_) => {
                entries.push(NameEntry {
                    name,
                    kind: BundleEntryKind::Unreadable,
                    bytes: None,
                    file_id: None,
                });
                continue;
            }
        };
        let native_kind = FileType::from_raw_mode(stat.st_mode);
        #[cfg(target_os = "macos")]
        let placeholder = stat.st_flags & 0x40000000 != 0;
        #[cfg(not(target_os = "macos"))]
        let placeholder = false;
        let kind = if placeholder {
            BundleEntryKind::Placeholder
        } else if native_kind == FileType::RegularFile {
            BundleEntryKind::File
        } else if native_kind == FileType::Directory {
            BundleEntryKind::Directory
        } else {
            BundleEntryKind::LinkOrSpecial
        };
        entries.push(NameEntry {
            name,
            bytes: (kind == BundleEntryKind::File)
                .then(|| u64::try_from(stat.st_size).ok())
                .flatten(),
            kind,
            file_id: Some(stat.st_ino),
        });
    }
    Ok((entries, false))
}

impl Library {
    pub fn organization_bundle_inventory(
        &self,
        asset_id: String,
        source_revision: String,
    ) -> Result<BundleInventory> {
        Ok(self
            .observe_bundle_inventory(asset_id, source_revision)?
            .view)
    }

    pub(crate) fn observe_bundle_inventory(
        &self,
        asset_id: String,
        source_revision: String,
    ) -> Result<InventoryObservation> {
        validate_id(&asset_id)?;
        if source_revision.is_empty() || source_revision.len() > 128 {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Invalid source revision.",
            ));
        }
        let _admission = self.destination_gate.try_lock().map_err(|_| {
            AppError::new(
                ErrorCode::Busy,
                "Another folder review is running. Try again after it finishes.",
            )
        })?;
        let id = asset_id.clone();
        let (catalog_revision, mut sources) = self
            .catalog
            .call(move |db| organization::source_snapshot(db, vec![id]))?;
        let source = sources.pop().ok_or_else(changed)?;
        if !source.available || source.revision != source_revision {
            return Err(changed());
        }
        let root = store::root(&self.catalog, source.root_id.clone())?;
        let relative = native::decode_path(&source.native_path)?;
        if relative.is_absolute()
            || relative.components().count() > 65
            || relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(changed());
        }
        let source_name = relative.file_name().ok_or_else(changed)?;
        let folder = relative.parent().ok_or_else(changed)?.to_owned();
        if root
            .path
            .components()
            .chain(folder.components())
            .any(|part| {
                part.as_os_str()
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .ends_with(".photoslibrary")
            })
        {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "Companion review does not enter Apple Photos library packages.",
            ));
        }
        let canonical_root = root.path.canonicalize()?;
        if canonical_root.components().any(|part| {
            part.as_os_str()
                .to_string_lossy()
                .to_ascii_lowercase()
                .ends_with(".photoslibrary")
        }) {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "Companion review does not enter Apple Photos library packages.",
            ));
        }
        let chain = DirectoryChain::open(&canonical_root)?;
        if chain.identity().volume != root.identity.volume
            || chain.identity().file != root.identity.file
            || native::volume_token(chain.leaf())? != root.volume
        {
            return Err(changed());
        }
        let mut held = vec![chain.leaf().try_clone()?];
        for part in folder.components() {
            let child = open_child(held.last().ok_or_else(changed)?, part.as_os_str(), true)?;
            if filesystem::identity(&child)?.volume != root.identity.volume {
                return Err(changed());
            }
            held.push(child);
        }
        let parent = held.last().ok_or_else(changed)?;
        let source_file = open_child(parent, source_name, false)?;
        if !source_file.metadata()?.is_file() || native::revision(&source_file)? != source_revision
        {
            return Err(changed());
        }
        let directory_revision = native::revision(parent)?;
        let folder_native = native::encode_path(&folder);
        let prefix_bytes = folder_native.len()
            + if folder.as_os_str().is_empty() {
                0
            } else {
                native::encode_path(Path::new(std::path::MAIN_SEPARATOR_STR)).len()
            };
        let root_id = source.root_id.clone();
        let (indexed, indexed_limit_exceeded) = self.catalog.call(move |db| {
            let mut query = db.prepare("SELECT id,substr(relative_native,?3,1025),presence FROM assets WHERE root_id=?1 AND folder_native=?2 LIMIT ?4")?;
            let values = query.query_map(rusqlite::params![root_id, folder_native, (prefix_bytes + 1) as i64, (MAX_BUNDLE_DIRECTORY_ENTRIES + 1) as i64], |r| Ok((r.get::<_, Vec<u8>>(1)?, IndexedEntry { id: r.get(0)?, presence: r.get(2)? })))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            let too_many = values.len() > MAX_BUNDLE_DIRECTORY_ENTRIES || values.iter().any(|(name, _)| name.len() > 1024);
            Ok((values.into_iter().take(MAX_BUNDLE_DIRECTORY_ENTRIES).collect::<BTreeMap<_, _>>(), too_many))
        })?;
        let _resources = self.reserve_inventory(root.identity.volume)?;
        let (mut observed_names, mut truncated) = names(parent)?;
        observed_names.sort_by_key(|entry| native::encode_path(Path::new(&entry.name)));
        let mut entries = Vec::with_capacity(observed_names.len());
        let mut seen = BTreeSet::new();
        let mut metadata_complete = true;
        let mut locator_bytes = 0usize;
        for entry in observed_names {
            let relative_native = native::encode_path(&folder.join(&entry.name));
            locator_bytes = locator_bytes.saturating_add(relative_native.len());
            if locator_bytes > 8 * 1024 * 1024 {
                truncated = true;
                break;
            }
            let name_native = native::encode_path(Path::new(&entry.name));
            seen.insert(name_native.clone());
            let indexed_entry = indexed.get(&name_native);
            let mut kind = entry.kind;
            let mut error = None;
            let mut identity = entry.file_id.map(|id| (root.identity.volume, id));
            let mut revision = None;
            if kind == BundleEntryKind::File {
                match open_child(parent, &entry.name, false).and_then(|file| {
                    Ok((
                        filesystem::identity(&file)?,
                        native::revision(&file)?,
                        file.metadata()?.is_file() && entry.bytes == Some(file.metadata()?.len()),
                    ))
                }) {
                    Ok((actual, current, true))
                        if entry.file_id == Some(actual.file)
                            && actual.volume == root.identity.volume =>
                    {
                        identity = Some((actual.volume, actual.file));
                        revision = Some(current);
                    }
                    _ => {
                        kind = BundleEntryKind::Unreadable;
                        error = Some(
                            "File metadata could not be verified; it is not selectable.".into(),
                        );
                        metadata_complete = false;
                    }
                }
            } else if kind == BundleEntryKind::Unreadable {
                metadata_complete = false;
            }
            let relation = relation(&entry.name, source_name);
            let name_representable = entry.name.to_str().is_some();
            let id_bytes = serde_json::to_vec(&(
                "bundle-entry-v1",
                &source.root_id,
                &relative_native,
                &identity,
                &revision,
                &kind,
            ))
            .map_err(|e| {
                AppError::new(
                    ErrorCode::Catalog,
                    "Could not fingerprint an inventory entry.",
                )
                .caused_by(e)
            })?;
            let id = blake3::hash(&id_bytes).to_hex().to_string();
            let selectable = kind == BundleEntryKind::File
                && entry
                    .name
                    .to_str()
                    .is_some_and(crate::organization::portable_filename)
                && relation != BundleRelation::Primary;
            entries.push(NativeBundleEntry {
                view: BundleInventoryEntry {
                    id,
                    name: entry.name.to_string_lossy().into_owned(),
                    kind,
                    relation,
                    bytes: entry.bytes.map(|v| v.to_string()),
                    catalog_asset_id: indexed_entry.map(|e| e.id.clone()),
                    catalog_presence: indexed_entry.map(|e| e.presence.clone()),
                    name_representable,
                    selectable,
                    error,
                },
                relative_native,
                identity,
                revision,
            });
        }
        let mut unobserved_candidates = Vec::new();
        for (path, _) in indexed.iter().filter(|(path, _)| !seen.contains(*path)) {
            let decoded = native::decode_path(path)?;
            if decoded
                .file_name()
                .is_some_and(|name| relation(name, source_name) != BundleRelation::Additional)
            {
                unobserved_candidates.push(decoded.to_string_lossy().into_owned());
            }
        }
        if native::revision(parent)? != directory_revision
            || native::revision(&source_file)? != source_revision
        {
            return Err(changed());
        }
        let fresh_root = DirectoryChain::open(&canonical_root)?;
        if fresh_root.identity().volume != root.identity.volume
            || fresh_root.identity().file != root.identity.file
            || native::volume_token(fresh_root.leaf())? != root.volume
        {
            return Err(changed());
        }
        let mut fresh_parent = fresh_root.leaf().try_clone()?;
        for part in folder.components() {
            fresh_parent = open_child(&fresh_parent, part.as_os_str(), true)?;
        }
        if filesystem::identity(&fresh_parent)? != filesystem::identity(parent)? {
            return Err(changed());
        }
        let fresh_source = open_child(&fresh_parent, source_name, false)?;
        if native::revision(&fresh_source)? != source_revision
            || filesystem::identity(&fresh_source)? != filesystem::identity(&source_file)?
        {
            return Err(changed());
        }
        let id = asset_id.clone();
        let (current_revision, current_sources) = self
            .catalog
            .call(move |db| organization::source_snapshot(db, vec![id]))?;
        if current_revision != catalog_revision
            || serde_json::to_vec(&current_sources).ok()
                != serde_json::to_vec(&vec![source.clone()]).ok()
        {
            return Err(changed());
        }
        let evidence = serde_json::to_vec(&(
            "bundle-directory-v1",
            &source,
            &catalog_revision,
            &directory_revision,
            &entries,
            truncated,
            indexed_limit_exceeded,
            &unobserved_candidates,
        ))
        .map_err(|e| {
            AppError::new(
                ErrorCode::Catalog,
                "Could not fingerprint the companion inventory.",
            )
            .caused_by(e)
        })?;
        let view = BundleInventory {
            asset_id,
            source_revision,
            catalog_revision,
            fingerprint: blake3::hash(&evidence).to_hex().to_string(),
            entries: entries.iter().map(|e| e.view.clone()).collect(),
            inventory_complete: !truncated
                && !indexed_limit_exceeded
                && metadata_complete
                && unobserved_candidates.is_empty(),
            truncated,
            indexed_limit_exceeded,
            unobserved_candidates,
            execution_available: false,
        };
        Ok(InventoryObservation {
            view,
            source,
            entries,
        })
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::{
        catalog::Catalog,
        library::{LibraryOptions, WorkerPaths, model::RootMode},
    };
    use std::{path::PathBuf, sync::Arc};

    pub struct Fixture {
        pub run: tempfile::TempDir,
        pub source: PathBuf,
        pub catalog: Arc<Catalog>,
        pub library: Arc<Library>,
        pub asset_id: String,
        pub revision: String,
    }
    pub fn fixture() -> Fixture {
        let run = tempfile::tempdir().unwrap();
        let base = run.path().canonicalize().unwrap();
        let source = base.join("source");
        let data = base.join("data");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&data).unwrap();
        image::RgbImage::from_pixel(16, 8, image::Rgb([10, 20, 30]))
            .save(source.join("Photo.JPG"))
            .unwrap();
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
        let handle = native::open_root(&source, None).unwrap();
        let identity = filesystem::identity(&handle).unwrap();
        let volume = native::volume_token(&handle).unwrap();
        let root_id = store::add_root(
            &catalog,
            source.clone(),
            identity.clone(),
            volume.clone(),
            "Owned metadata fixture".into(),
            RootMode::Referenced,
            crate::library::model::ScanFilter::default(),
        )
        .unwrap();
        let file = native::open_asset(&source, &identity, &volume, Path::new("Photo.JPG")).unwrap();
        let revision = native::revision(&file).unwrap();
        let file_id = filesystem::identity(&file).unwrap();
        let bytes = file.metadata().unwrap().len();
        drop(file);
        drop(handle);
        let asset_id = uuid::Uuid::new_v4().to_string();
        let id = asset_id.clone();
        let rev = revision.clone();
        catalog.call(move |db| {
            db.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan) VALUES(?1,?2,?3,'Photo.JPG',?4,'','Photo.JPG','Photo','jpg',?5,?6,?7,'now','2026-10-31T00:00:00','filesystem_modified','ready','pending','present',1,'owned-fixture')",
                rusqlite::params![id, root_id, native::encode_path(Path::new("Photo.JPG")), native::encode_path(Path::new("")), format!("{}:{}",file_id.volume,file_id.file), bytes as i64, rev])?;
            db.execute("UPDATE library_state SET revision=revision+1", [])?;
            Ok(())
        }).unwrap();
        Fixture {
            run,
            source,
            catalog,
            library,
            asset_id,
            revision,
        }
    }

    #[test]
    fn bundle_inventory_includes_unindexed_hidden_and_filename_sidecars_without_source_writes() {
        let fixture = fixture();
        for name in [
            "Photo.xmp",
            "Photo.JPG.AAE",
            "._Photo.JPG",
            ".hidden.bin",
            "Other.bin",
        ] {
            std::fs::write(fixture.source.join(name), name.as_bytes()).unwrap();
        }
        let before = std::fs::read(fixture.source.join("Photo.JPG")).unwrap();
        let inventory = fixture
            .library
            .organization_bundle_inventory(fixture.asset_id.clone(), fixture.revision.clone())
            .unwrap();
        assert!(inventory.inventory_complete);
        assert_eq!(inventory.entries.len(), 6);
        assert!(!inventory.execution_available);
        for (name, expected) in [
            ("Photo.xmp", BundleRelation::SameStem),
            ("Photo.JPG.AAE", BundleRelation::FilenameSidecar),
            ("._Photo.JPG", BundleRelation::AppleDouble),
            (".hidden.bin", BundleRelation::Additional),
        ] {
            let entry = inventory
                .entries
                .iter()
                .find(|entry| entry.name == name)
                .unwrap();
            assert_eq!(entry.relation, expected);
            assert!(entry.catalog_asset_id.is_none());
            assert!(entry.selectable);
        }
        assert_eq!(
            std::fs::read(fixture.source.join("Photo.JPG")).unwrap(),
            before
        );
        assert_eq!(
            fixture
                .library
                .organization_bundle_inventory(fixture.asset_id.clone(), fixture.revision.clone())
                .unwrap()
                .fingerprint,
            inventory.fingerprint
        );
        assert_eq!(std::fs::read_dir(&fixture.source).unwrap().count(), 6);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn companion_metadata_descriptor_cannot_read_contents() {
        use std::io::Read;
        let fixture = fixture();
        let parent = native::open_root(&fixture.source, None).unwrap();
        let mut file = open_child(&parent, OsStr::new("Photo.JPG"), false).unwrap();
        assert!(file.metadata().unwrap().is_file());
        assert!(file.read(&mut [0u8; 1]).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn non_unicode_companions_keep_distinct_native_ids_but_cannot_be_selected() {
        use std::os::windows::ffi::OsStringExt;
        let fixture = fixture();
        for unit in [0xd800, 0xd801] {
            let name = OsString::from_wide(&[0x61, unit, 0x2e, 0x78]);
            std::fs::write(fixture.source.join(name), b"preserved").unwrap();
        }
        let inventory = fixture
            .library
            .organization_bundle_inventory(fixture.asset_id.clone(), fixture.revision.clone())
            .unwrap();
        assert!(inventory.inventory_complete);
        let names: Vec<_> = inventory
            .entries
            .iter()
            .filter(|entry| !entry.name_representable)
            .collect();
        assert_eq!(names.len(), 2);
        assert_ne!(names[0].id, names[1].id);
        assert!(names.iter().all(|entry| !entry.selectable));
    }

    #[test]
    fn bundle_inventory_binds_sidecar_changes_even_with_same_size_and_restored_mtime() {
        let fixture = fixture();
        let sidecar = fixture.source.join("Photo.xmp");
        std::fs::write(&sidecar, b"first").unwrap();
        let before = fixture
            .library
            .organization_bundle_inventory(fixture.asset_id.clone(), fixture.revision.clone())
            .unwrap();
        let modified = std::fs::metadata(&sidecar).unwrap().modified().unwrap();
        std::fs::write(&sidecar, b"other").unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&sidecar)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(modified))
            .unwrap();
        let after = fixture
            .library
            .organization_bundle_inventory(fixture.asset_id.clone(), fixture.revision.clone())
            .unwrap();
        assert_ne!(before.fingerprint, after.fingerprint);
        assert_ne!(
            before
                .entries
                .iter()
                .find(|e| e.name == "Photo.xmp")
                .unwrap()
                .id,
            after
                .entries
                .iter()
                .find(|e| e.name == "Photo.xmp")
                .unwrap()
                .id
        );
    }

    #[test]
    fn bundle_inventory_never_treats_a_replaced_root_as_an_empty_folder() {
        let fixture = fixture();
        let base = fixture.run.path().canonicalize().unwrap();
        let moved = base.join("moved-source");
        assert!(fixture.source.starts_with(&base));
        assert!(moved.starts_with(&base));
        std::fs::rename(&fixture.source, &moved).unwrap();
        std::fs::create_dir(&fixture.source).unwrap();
        assert!(
            fixture
                .library
                .organization_bundle_inventory(fixture.asset_id.clone(), fixture.revision.clone())
                .is_err()
        );
        assert_eq!(fixture.library.status().unwrap().roots.len(), 1);
        std::fs::remove_dir(&fixture.source).unwrap();
        std::fs::rename(&moved, &fixture.source).unwrap();
        assert_eq!(
            fixture.library.status().unwrap().roots[0].availability,
            "online"
        );
        assert!(
            fixture
                .library
                .organization_bundle_inventory(fixture.asset_id.clone(), fixture.revision.clone())
                .unwrap()
                .inventory_complete
        );
    }

    #[test]
    fn unobserved_indexed_companions_block_complete_inventory_without_erasing_history() {
        let fixture = fixture();
        let root = fixture.library.status().unwrap().roots[0].id.clone();
        std::fs::write(
            fixture.source.join("Photo.xmp"),
            b"retained unique metadata",
        )
        .unwrap();
        fixture.catalog.call(move |db| {
            db.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan) VALUES(?1,?2,?3,'Photo.xmp',?4,'','Photo.xmp','Photo','xmp','retained-observation',24,'old-observation','now','2026-10-31T00:00:00','filesystem_modified','unsupported','unavailable','present',0,'owned-fixture')",
                rusqlite::params![uuid::Uuid::new_v4().to_string(),root,native::encode_path(Path::new("Photo.xmp")),native::encode_path(Path::new(""))])?;
            db.execute("UPDATE library_state SET revision=revision+1",[])?;
            Ok(())
        }).unwrap();
        let retained = fixture.run.path().join("retained-sidecar.xmp");
        assert!(retained.starts_with(fixture.run.path()));
        std::fs::rename(fixture.source.join("Photo.xmp"), &retained).unwrap();
        let inventory = fixture
            .library
            .organization_bundle_inventory(fixture.asset_id.clone(), fixture.revision.clone())
            .unwrap();
        assert!(!inventory.inventory_complete);
        assert_eq!(inventory.unobserved_candidates, ["Photo.xmp"]);
        assert_eq!(
            fixture
                .catalog
                .call(|db| Ok(db.query_row(
                    "SELECT presence FROM assets WHERE name='Photo.xmp'",
                    [],
                    |r| r.get::<_, String>(0)
                )?))
                .unwrap(),
            "present"
        );
        assert_eq!(
            std::fs::read(retained).unwrap(),
            b"retained unique metadata"
        );
    }

    #[test]
    fn bundle_directory_collector_has_an_exact_native_entry_bound() {
        let owned = tempfile::tempdir().unwrap();
        for index in 0..MAX_BUNDLE_DIRECTORY_ENTRIES {
            std::fs::write(owned.path().join(format!("{index:04}.bin")), b"").unwrap();
        }
        let parent = native::open_root(owned.path(), None).unwrap();
        let (entries, truncated) = names(&parent).unwrap();
        assert_eq!(entries.len(), MAX_BUNDLE_DIRECTORY_ENTRIES);
        assert!(!truncated);
        drop(parent);
        std::fs::write(owned.path().join("one-more.bin"), b"").unwrap();
        let parent = native::open_root(owned.path(), None).unwrap();
        let (entries, truncated) = names(&parent).unwrap();
        assert_eq!(entries.len(), MAX_BUNDLE_DIRECTORY_ENTRIES);
        assert!(truncated);
    }
}

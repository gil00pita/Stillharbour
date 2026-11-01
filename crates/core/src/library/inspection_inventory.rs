//! Read-only handle-relative inventory. This never enrolls unknown files as photos.
use super::{filters::CompiledFilter, inspection_sources::NativeSources, native};
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem,
    inspection::{InspectionScope, InventoryCoverage, model::*},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::File,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};

pub(crate) struct Known {
    pub id: String,
    pub relative: PathBuf,
    pub media: bool,
}
pub(crate) struct Inventory {
    pub sources: NativeSources,
    pub findings: Vec<InspectionFinding>,
    pub media: Vec<bool>,
    pub coverage: InventoryCoverage,
    pub issues: Vec<InspectionIssue>,
    visited: u32,
    data_identity: filesystem::FileIdentity,
}
pub(super) struct Entry {
    pub(super) name: OsString,
    pub(super) directory: bool,
    pub(super) blocked: bool,
    pub(super) hidden: bool,
    pub(super) bytes: u64,
}
pub(super) fn child(parent: &File, name: &std::ffi::OsStr, directory: bool) -> Result<File> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::CString::new(name.as_bytes())
            .map_err(|_| crate::inspection::invalid("Invalid native component."))?;
        native::child_file(parent, &name, directory)
    }
    #[cfg(windows)]
    {
        filesystem::windows::child(parent, name, directory)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (parent, name, directory);
        Err(AppError::new(
            ErrorCode::Unsupported,
            "Native inventory is unavailable on this platform.",
        ))
    }
}
pub(super) fn each(parent: &File, mut visitor: impl FnMut(Entry) -> Result<()>) -> Result<()> {
    #[cfg(unix)]
    {
        use rustix::fs::{AtFlags, Dir, statat};
        use std::os::unix::ffi::OsStrExt;
        for entry in
            Dir::read_from(parent).map_err(|error| AppError::from(std::io::Error::from(error)))?
        {
            let entry = entry.map_err(|error| AppError::from(std::io::Error::from(error)))?;
            let name = entry.file_name();
            if matches!(name.to_bytes(), b"." | b"..") {
                continue;
            }
            let stat = statat(parent, name, AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|error| AppError::from(std::io::Error::from(error)))?;
            let mode = stat.st_mode as u32 & libc::S_IFMT as u32;
            let blocked = mode != libc::S_IFREG as u32 && mode != libc::S_IFDIR as u32;
            let hidden = name.to_bytes().starts_with(b".");
            #[cfg(target_os = "macos")]
            let (blocked, hidden) = (
                blocked || stat.st_flags & 0x40000000 != 0,
                hidden || stat.st_flags & libc::UF_HIDDEN != 0,
            );
            visitor(Entry {
                name: std::ffi::OsStr::from_bytes(name.to_bytes()).to_owned(),
                directory: mode == libc::S_IFDIR as u32,
                blocked,
                hidden,
                bytes: u64::try_from(stat.st_size).map_err(|_| {
                    AppError::new(ErrorCode::Unsupported, "Invalid native file length.")
                })?,
            })?;
        }
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Storage::FileSystem::*;
        let mut entries = filesystem::windows::Directory::new(parent)?;
        while let Some(entry) = entries.next()? {
            if entry.name == "." || entry.name == ".." {
                continue;
            }
            let hidden = entry.attributes & FILE_ATTRIBUTE_HIDDEN != 0
                || entry.name.as_encoded_bytes().starts_with(b".");
            visitor(Entry {
                name: entry.name,
                directory: entry.attributes & FILE_ATTRIBUTE_DIRECTORY != 0,
                blocked: entry.attributes
                    & (FILE_ATTRIBUTE_REPARSE_POINT
                        | FILE_ATTRIBUTE_OFFLINE
                        | FILE_ATTRIBUTE_RECALL_ON_OPEN
                        | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
                    != 0,
                hidden,
                bytes: entry.bytes,
            })?;
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (parent, &mut visitor);
        return Err(AppError::new(
            ErrorCode::Unsupported,
            "Native inventory is unavailable.",
        ));
    }
    Ok(())
}
fn observation_id(root: &str, path: &Path) -> String {
    let mut hash = blake3::Hasher::new();
    hash.update(b"inspection-observation-v1");
    hash.update(root.as_bytes());
    hash.update(&native::encode_path(path));
    hash.finalize().to_hex().to_string()
}
type CompanionIndex = BTreeMap<Vec<u8>, BTreeSet<Vec<u8>>>;
fn comparison_key(name: &std::ffi::OsStr) -> Vec<u8> {
    match name.to_str() {
        Some(name) => {
            let mut value = b"utf8:".to_vec();
            value.extend(name.to_ascii_lowercase().bytes());
            value
        }
        None => {
            let mut value = b"native:".to_vec();
            value.extend(native::encode_path(Path::new(name)));
            value
        }
    }
}
fn companion_index(known: &[Known]) -> CompanionIndex {
    let mut result: CompanionIndex = BTreeMap::new();
    for file in known.iter().filter(|file| file.media) {
        let parent = native::encode_path(file.relative.parent().unwrap_or(Path::new("")));
        let names = result.entry(parent).or_default();
        for name in [file.relative.file_name(), file.relative.file_stem()]
            .into_iter()
            .flatten()
        {
            names.insert(comparison_key(name));
        }
    }
    result
}
fn companion(path: &Path, index: &CompanionIndex) -> bool {
    let Some(names) = index.get(&native::encode_path(path.parent().unwrap_or(Path::new(""))))
    else {
        return false;
    };
    let Some(name) = path.file_name() else {
        return false;
    };
    if name
        .to_str()
        .and_then(|name| name.strip_prefix("._"))
        .is_some_and(|name| names.contains(&comparison_key(std::ffi::OsStr::new(name))))
    {
        return true;
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !["xmp", "aae", "acr", "mov"].contains(&extension.as_str())
        && !super::model::DEFAULT_PHOTO_EXTENSIONS.contains(&extension.as_str())
    {
        return false;
    }
    path.file_stem()
        .is_some_and(|name| names.contains(&comparison_key(name)))
}
fn rebound_directory(sources: &NativeSources, root: usize, relative: &Path) -> Result<File> {
    let binding = &sources.roots[root];
    let mut parent = native::open_root(&binding.path, Some(&binding.identity))?;
    for component in relative.components() {
        parent = child(&parent, component.as_os_str(), true)?;
    }
    Ok(parent)
}

impl Inventory {
    pub(crate) fn collect(
        sources: NativeSources,
        scope: InspectionScope,
        data_directory: &Path,
        policy: Option<&NamePolicy>,
    ) -> Result<Self> {
        let data_identity =
            filesystem::identity(&filesystem::open_directory_readonly(data_directory)?)?;
        let mut result = Self {
            sources,
            findings: Vec::new(),
            media: Vec::new(),
            coverage: InventoryCoverage {
                enumeration_complete: true,
                ..InventoryCoverage::default()
            },
            issues: Vec::new(),
            visited: 0,
            data_identity,
        };
        for root in 0..result.sources.roots.len() {
            if result.sources.limited {
                break;
            }
            if result.sources.roots[root].identity.volume == result.data_identity.volume
                && result.sources.roots[root].identity.file == result.data_identity.file
            {
                return Err(crate::inspection::invalid(
                    "Application data is excluded from inspection sources.",
                ));
            }
            let id = result.sources.roots[root].view.id.clone();
            let known = result.sources.catalog.call(move |db| {
                let mut statement = db.prepare("SELECT id,relative_native,is_media FROM assets WHERE root_id=? ORDER BY sequence LIMIT 100001")?;
                let mut query = statement.query([id])?;
                let mut known = Vec::new();
                let mut budget = 0usize;
                while let Some(row) = query.next()? {
                    let id: String = row.get(0)?;
                    let path: Vec<u8> = row.get(1)?;
                    budget = budget.saturating_add(id.len()).saturating_add(path.len().saturating_mul(4)).saturating_add(256);
                    if budget > 64 * 1024 * 1024 { return Err(AppError::new(ErrorCode::Unsupported,"Indexed inspection paths exceed the retained metadata budget.")); }
                    known.push(Known { id, relative:native::decode_path(&path)?, media:row.get(2)? });
                }
                Ok(known)
            })?;
            if known.len() > crate::inspection::candidates::MAX_ITEMS {
                result.coverage.enumeration_complete = false;
                result.issue(
                    root,
                    Path::new(""),
                    IssueEntryKind::Directory,
                    crate::inspection::invalid("The indexed scope exceeds 100,000 members."),
                );
                continue;
            }
            let companions = companion_index(&known);
            let lookup = known
                .iter()
                .map(|file| (native::encode_path(&file.relative), file))
                .collect::<BTreeMap<_, _>>();
            let mut filter = result.sources.roots[root].view.filter.clone();
            // Inspection scope overrides gallery extension/capture-date rules;
            // explicit hidden/directory/glob/size exclusions remain in effect.
            filter.extensions.clear();
            filter.from_date = None;
            filter.through_date = None;
            let filter = CompiledFilter::new(filter)?;
            let parent = native::open_root(
                &result.sources.roots[root].path,
                Some(&result.sources.roots[root].identity),
            )?;
            let mut seen = BTreeSet::new();
            result.walk(
                root,
                parent,
                PathBuf::new(),
                0,
                scope,
                data_directory,
                &filter,
                &companions,
                &lookup,
                &mut seen,
                policy,
            )?;
            // Infer absence only after complete enumeration and a fresh native
            // not-found result under the still-authorized root. Filters never
            // turn a successful open into a missing observation.
            if result.coverage.enumeration_complete {
                for file in known
                    .iter()
                    .filter(|file| file.media && !seen.contains(&file.id))
                {
                    let binding = &result.sources.roots[root];
                    if let Err(error) = native::open_asset(
                        &binding.path,
                        &binding.identity,
                        &binding.volume,
                        &file.relative,
                    ) && error.io_kind == Some(std::io::ErrorKind::NotFound)
                    {
                        result.issue(root, &file.relative, IssueEntryKind::File, error);
                    }
                }
            }
        }
        result.sources.check_all_roots()?;
        Ok(result)
    }
    #[allow(clippy::too_many_arguments)]
    fn walk(
        &mut self,
        root: usize,
        parent: File,
        relative: PathBuf,
        depth: u32,
        scope: InspectionScope,
        data_directory: &Path,
        filter: &CompiledFilter,
        companions: &CompanionIndex,
        lookup: &BTreeMap<Vec<u8>, &Known>,
        seen: &mut BTreeSet<String>,
        policy: Option<&NamePolicy>,
    ) -> Result<()> {
        if self.sources.cancel.load(Ordering::Acquire) {
            self.coverage.enumeration_complete = false;
            return Ok(());
        }
        if depth > 64 {
            self.coverage.incomplete_directories += 1;
            self.coverage.enumeration_complete = false;
            self.issue(
                root,
                &relative,
                IssueEntryKind::Directory,
                AppError::new(
                    ErrorCode::Unsupported,
                    "Inventory stopped at its 64-level directory bound.",
                ),
            );
            return Ok(());
        }
        self.sources.validate_root(root)?;
        if filesystem::identity(&parent)?
            != filesystem::identity(&rebound_directory(&self.sources, root, &relative)?)?
        {
            return Err(crate::inspection::changed(
                "A directory changed membership before inspection.",
            ));
        }
        let revision = native::revision(&parent)?;
        let result = each(&parent, |entry| {
            self.visited = self.visited.saturating_add(1);
            if self.sources.limited || self.visited > 1_000_000 {
                self.sources.limited = true;
                return Err(AppError::new(
                    ErrorCode::Unsupported,
                    "Inventory reached its membership or enumeration bound.",
                ));
            }
            if self.sources.cancel.load(Ordering::Acquire) {
                self.coverage.enumeration_complete = false;
                return Err(AppError::new(
                    ErrorCode::Busy,
                    "Inventory canceled before the next entry.",
                ));
            }
            let path = relative.join(&entry.name);
            if entry.blocked {
                self.coverage.inaccessible_entries += 1;
                self.coverage.enumeration_complete = false;
                self.issue(root,&path,if entry.directory {IssueEntryKind::Directory}else{IssueEntryKind::Unknown},AppError::new(ErrorCode::Unsupported,"A link, special entry or cloud placeholder was not traversed or downloaded."));
                return Ok(());
            }
            if entry.directory {
                if filter.directory_reason(&path, entry.hidden).is_some()
                    || self.sources.roots[root]
                        .path
                        .join(&path)
                        .starts_with(data_directory)
                {
                    self.coverage.excluded_entries += 1;
                    return Ok(());
                }
                match child(&parent, &entry.name, true) {
                    Ok(directory) => {
                        let identity = filesystem::identity(&directory)?;
                        if identity.volume == self.data_identity.volume
                            && identity.file == self.data_identity.file
                        {
                            self.coverage.excluded_entries += 1;
                        } else {
                            self.walk(
                                root,
                                directory,
                                path,
                                depth + 1,
                                scope,
                                data_directory,
                                filter,
                                companions,
                                lookup,
                                seen,
                                policy,
                            )?;
                        }
                    }
                    Err(error) => {
                        self.coverage.incomplete_directories += 1;
                        self.coverage.enumeration_complete = false;
                        self.issue(root, &path, IssueEntryKind::Directory, error);
                    }
                }
                return Ok(());
            }
            if filter
                .file_reason(&path, entry.hidden, entry.bytes, None)
                .is_some()
            {
                self.coverage.excluded_entries += 1;
                return Ok(());
            }
            let registered = lookup.get(&native::encode_path(&path)).copied();
            let media = registered.is_some_and(|file| file.media) || companion(&path, companions);
            if scope == InspectionScope::IndexedMediaCompanions && !media {
                return Ok(());
            }
            let id = registered
                .map(|file| file.id.clone())
                .unwrap_or_else(|| observation_id(&self.sources.roots[root].view.id, &path));
            if !seen.insert(id.clone()) {
                return Err(crate::inspection::invalid(
                    "Inventory produced repeated membership.",
                ));
            }
            match self.sources.capture(root, path.clone(), id) {
                Ok(()) => {
                    let snapshot = self.sources.entries.last().ok_or_else(|| {
                        crate::inspection::invalid("Captured inventory is missing.")
                    })?;
                    self.findings.push(snapshot.finding(
                        &self.sources.roots[root],
                        registered.map(|file| file.id.clone()),
                        registered.is_some_and(|file| file.media),
                        policy,
                    )?);
                    self.media.push(media);
                }
                Err(error) => {
                    self.coverage.inaccessible_entries += 1;
                    self.coverage.enumeration_complete = false;
                    self.issue(root, &path, IssueEntryKind::File, error);
                }
            }
            Ok(())
        });
        if let Err(error) = result {
            self.coverage.incomplete_directories += 1;
            self.coverage.enumeration_complete = false;
            self.issue(root, &relative, IssueEntryKind::Directory, error);
        }
        if filesystem::identity(&parent)?
            != filesystem::identity(&rebound_directory(&self.sources, root, &relative)?)?
            || native::revision(&parent)? != revision
        {
            self.coverage.incomplete_directories += 1;
            self.coverage.enumeration_complete = false;
            self.issue(
                root,
                &relative,
                IssueEntryKind::Directory,
                crate::inspection::changed(
                    "A directory changed during inventory; coverage is incomplete.",
                ),
            );
        }
        Ok(())
    }
    fn issue(&mut self, root: usize, path: &Path, entry_kind: IssueEntryKind, error: AppError) {
        let inventory = if error.io_kind == Some(std::io::ErrorKind::NotFound) {
            match self.sources.validate_root(root) {
                Ok(()) => InventoryState::Missing,
                Err(root_error) => super::inspection_sources::inventory_state(&root_error),
            }
        } else {
            super::inspection_sources::inventory_state(&error)
        };
        self.issues.push(InspectionIssue {
            root_id: self.sources.roots[root].view.id.clone(),
            display_path: self.sources.roots[root]
                .path
                .join(path)
                .to_string_lossy()
                .into_owned(),
            entry_kind,
            inventory,
            error,
        });
    }
    pub(crate) fn sizes(&self) -> Vec<crate::inspection::storage::FileSize> {
        self.sources
            .entries
            .iter()
            .zip(&self.media)
            .map(|(entry, &media)| entry.size(&self.sources.roots[entry.root], media))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        catalog::Catalog,
        jobs::resources::ResourceScheduler,
        library::{
            model::{RootMode, ScanFilter},
            store,
        },
    };
    use std::sync::{Arc, atomic::AtomicBool};
    #[test]
    fn native_inspection_inventory_preserves_unknown_files_and_excludes_owned_data_in_both_scopes()
    {
        let run = tempfile::tempdir().unwrap();
        let source = run.path().join("source");
        std::fs::create_dir(&source).unwrap();
        let data = source.join("application-data");
        std::fs::create_dir(&data).unwrap();
        for (name, bytes) in [
            ("a.bin", 100),
            ("b.bin", 200),
            ("c.bin", 300),
            ("Photo.JPG", 400),
            ("Photo.xmp", 20),
        ] {
            let path = source.join(name);
            assert!(path.starts_with(run.path()));
            std::fs::write(path, vec![11; bytes]).unwrap();
        }
        let catalog = Arc::new(Catalog::open(data.join("Fixture.stillharbour")).unwrap());
        let root = native::open_root(&source, None).unwrap();
        let filter = ScanFilter {
            extensions: vec!["jpg".into()],
            ..ScanFilter::default()
        };
        let root_id = store::add_root(
            &catalog,
            source.clone(),
            filesystem::identity(&root).unwrap(),
            native::volume_token(&root).unwrap(),
            "Generated inventory".into(),
            RootMode::Referenced,
            filter,
        )
        .unwrap();
        let original = std::fs::File::open(source.join("Photo.JPG")).unwrap();
        let identity = filesystem::identity(&original).unwrap();
        store::discover_batch(
            &catalog,
            root_id.clone(),
            uuid::Uuid::new_v4().to_string(),
            vec![store::Discovery {
                relative: "Photo.JPG".into(),
                revision: native::revision(&original).unwrap(),
                native_file: format!("{}:{}", identity.volume, identity.file),
                links: identity.links,
                bytes: 400,
                modified: None,
                born: None,
                detected: None,
                media: true,
                placeholder: false,
            }],
        )
        .unwrap();
        drop(original);
        for (scope, expected, count) in [
            (InspectionScope::AllEligibleFiles, "1020", 5),
            (InspectionScope::IndexedMediaCompanions, "420", 2),
        ] {
            let sources = NativeSources::new(
                catalog.clone(),
                vec![root_id.clone()],
                ResourceScheduler::default(),
                Arc::new(AtomicBool::new(false)),
            )
            .unwrap();
            let inventory =
                Inventory::collect(sources, scope, &data.canonicalize().unwrap(), None).unwrap();
            assert_eq!(inventory.findings.len(), count);
            assert!(inventory.coverage.enumeration_complete);
            assert!(inventory.coverage.excluded_entries > 0);
            assert!(inventory.issues.is_empty());
            assert!(
                inventory
                    .findings
                    .iter()
                    .filter(|finding| finding.name == "Photo.xmp")
                    .all(|finding| !finding.is_media)
            );
            let totals = crate::inspection::storage::aggregate(
                scope,
                &inventory.sizes(),
                inventory.coverage.clone(),
            )
            .unwrap();
            assert_eq!(
                totals
                    .folders
                    .iter()
                    .find(|folder| folder.id == "scope")
                    .unwrap()
                    .logical_bytes,
                expected
            );
            assert!(
                inventory
                    .findings
                    .iter()
                    .all(|finding| !finding.display_path.contains("application-data"))
            );
        }
        for (name, bytes) in [
            ("a.bin", 100),
            ("b.bin", 200),
            ("c.bin", 300),
            ("Photo.JPG", 400),
            ("Photo.xmp", 20),
        ] {
            assert_eq!(std::fs::read(source.join(name)).unwrap(), vec![11; bytes]);
        }
    }
}

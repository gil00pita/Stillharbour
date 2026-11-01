//! Fresh native read snapshots behind the inspection domain's reader port.
//! Paths originate only in catalog/native inventory, never renderer requests.
use super::{
    native,
    store::{self, RootRecord},
};
use crate::{
    catalog::Catalog,
    domain::{AppError, ErrorCode, Result},
    filesystem::{self, DirectoryChain, FileIdentity},
    inspection::{
        candidates::{Item, Sources},
        changed, invalid,
        rules::Properties,
    },
    jobs::{
        operation_locks::{Access, Lease, Registry, Scope},
        resources::{Class, ResourceScheduler, Work},
    },
};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

fn unavailable_root(availability: &str) -> AppError {
    let mut error = AppError::new(
        ErrorCode::Io,
        "The registered inspection root is offline or inaccessible; retained inventory is not an empty folder.",
    );
    error.io_kind = Some(if availability == "offline" {
        std::io::ErrorKind::NotConnected
    } else {
        std::io::ErrorKind::PermissionDenied
    });
    error
}
fn root_failure(mut error: AppError) -> AppError {
    if error.io_kind == Some(std::io::ErrorKind::NotFound) {
        error.io_kind = Some(std::io::ErrorKind::NotConnected);
        error.message =
            "The inspection root is unavailable; retained inventory is not empty.".into();
    }
    error
}
pub(super) fn inventory_state(error: &AppError) -> crate::inspection::model::InventoryState {
    use crate::inspection::model::InventoryState;
    match (error.code.clone(), error.io_kind) {
        (_, Some(std::io::ErrorKind::NotConnected)) => InventoryState::Offline,
        (ErrorCode::Conflict, _) => InventoryState::ChangedDuringRead,
        (ErrorCode::Unsupported, _) => InventoryState::UnsupportedEntry,
        (_, Some(std::io::ErrorKind::NotFound)) => InventoryState::Missing,
        _ => InventoryState::Inaccessible,
    }
}
pub(crate) struct Snapshot {
    pub id: String,
    pub root: usize,
    pub relative: PathBuf,
    pub identity: FileIdentity,
    pub revision: String,
    pub bytes: u64,
    pub properties: Properties,
    pub allocated: Option<u64>,
}
pub(crate) struct NativeSources {
    pub catalog: Arc<Catalog>,
    pub roots: Vec<RootRecord>,
    pub entries: Vec<Snapshot>,
    pub cancel: Arc<AtomicBool>,
    pub bytes_read: Arc<AtomicU64>,
    pub limited: bool,
    resources: ResourceScheduler,
    member_ids: BTreeSet<String>,
    metadata_bytes: usize,
    _lease: Lease,
}
pub(crate) struct Reader {
    pub file: File,
    volume: u64,
    cancel: Arc<AtomicBool>,
    bytes_read: Arc<AtomicU64>,
    resources: ResourceScheduler,
}
impl Read for Reader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let _admission = self
            .resources
            .acquire(self.volume, Class::Background, Work::Metadata, &self.cancel)
            .map_err(|error| std::io::Error::other(error.message))?;
        let count = self.file.read(buffer)?;
        self.bytes_read.fetch_add(count as u64, Ordering::Relaxed);
        Ok(count)
    }
}
impl Seek for Reader {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        self.file.seek(position)
    }
}
impl NativeSources {
    pub(crate) fn decode(
        &self,
        index: usize,
        paths: &super::workers::WorkerPaths,
        mode: crate::inspection::decoder::DecodeMode,
    ) -> Result<crate::inspection::decoder::DecodeReport> {
        use crate::{
            inspection::decoder::{self, DecodeMode, FormatHint},
            jobs::paced_io::{IoStep, PacedIo},
        };
        let entry = &self.entries[index];
        let limit = if mode == DecodeMode::Quick {
            decoder::QUICK_LIMIT
        } else {
            decoder::INPUT_LIMIT
        };
        if mode != DecodeMode::Quick && entry.bytes > limit {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "Image exceeds the bounded 32 MiB decode input policy.",
            ));
        }
        let mut reader = self.open(index)?;
        let mut bytes = Vec::new();
        let mut buffer = [0; 64 * 1024];
        let expected = entry.bytes.min(limit);
        while (bytes.len() as u64) < expected {
            let length = buffer.len().min((expected - bytes.len() as u64) as usize);
            match PacedIo::shared().read(&mut reader, &mut buffer[..length], &self.cancel)? {
                IoStep::Canceled => {
                    return Err(AppError::new(ErrorCode::Busy, "Image inspection canceled."));
                }
                IoStep::Completed(0) => {
                    return Err(changed("Image length changed during inspection."));
                }
                IoStep::Completed(count) => bytes.extend_from_slice(&buffer[..count]),
            }
        }
        self.validate(index, &reader)?;
        let extension = entry
            .relative
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let hint = if matches!(
            extension.as_str(),
            "raw"
                | "crw"
                | "cr2"
                | "cr3"
                | "nef"
                | "nrw"
                | "arw"
                | "sr2"
                | "srf"
                | "dng"
                | "orf"
                | "rw2"
                | "raf"
                | "pef"
                | "srw"
                | "x3f"
                | "3fr"
                | "fff"
                | "iiq"
                | "rwl"
        ) {
            FormatHint::Raw
        } else if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp") {
            FormatHint::Image
        } else {
            FormatHint::Unknown
        };
        let result = {
            let _admission = self.resources.acquire(
                reader.volume,
                Class::Background,
                Work::Rendition,
                &self.cancel,
            )?;
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(AppError::from)?;
            runtime.block_on(super::workers::inspect(
                paths,
                bytes,
                mode,
                hint,
                entry.bytes <= limit,
                self.cancel.clone(),
            ))
        };
        // A successful decoder result never substitutes for fresh source identity.
        self.validate(index, &reader)?;
        result
    }
    pub(crate) fn new(
        catalog: Arc<Catalog>,
        root_ids: Vec<String>,
        resources: ResourceScheduler,
        cancel: Arc<AtomicBool>,
    ) -> Result<Self> {
        if root_ids.is_empty() || root_ids.len() > 64 {
            return Err(invalid("Choose 1–64 registered source roots."));
        }
        let mut roots = Vec::new();
        let mut scopes = Vec::new();
        for id in root_ids {
            crate::operations::validate_id(&id)?;
            if roots.iter().any(|root: &RootRecord| root.view.id == id) {
                return Err(invalid(
                    "Inspection root membership contains a repeated ID.",
                ));
            }
            let root = store::root(&catalog, id)?;
            if root.view.availability != "online" {
                return Err(unavailable_root(&root.view.availability));
            }
            let file = native::open_root(&root.path, Some(&root.identity)).map_err(root_failure)?;
            if native::volume_token(&file)? != root.volume {
                return Err(changed("The inspection volume changed identity."));
            }
            // Resolve picker/temporary-directory alias spelling for the exact
            // component opener. Authorization still requires the stored native
            // object below and fresh original-path validation on every read.
            let chain = DirectoryChain::open(&root.path.canonicalize()?)?;
            if filesystem::identity(chain.leaf())?.file != root.identity.file
                || filesystem::identity(chain.leaf())?.volume != root.identity.volume
            {
                return Err(changed("The inspection root changed during admission."));
            }
            scopes.push(Scope::new(chain, Access::Read));
            roots.push(root);
        }
        let lease = Registry::shared().try_acquire(scopes)?;
        Ok(Self {
            catalog,
            roots,
            entries: Vec::new(),
            cancel,
            bytes_read: Arc::new(AtomicU64::new(0)),
            limited: false,
            resources,
            member_ids: BTreeSet::new(),
            metadata_bytes: 0,
            _lease: lease,
        })
    }
    pub(crate) fn validate_root(&self, index: usize) -> Result<()> {
        let root = self
            .roots
            .get(index)
            .ok_or_else(|| invalid("Unknown inspection root."))?;
        let current = store::root(&self.catalog, root.view.id.clone())?;
        if current.view.availability != "online" {
            return Err(unavailable_root(&current.view.availability));
        }
        if current.identity.volume != root.identity.volume
            || current.identity.file != root.identity.file
            || current.volume != root.volume
            || current.path != root.path
            || current.view.last_scan != root.view.last_scan
        {
            return Err(changed(
                "The registered inspection scope changed or became unavailable. Review its inventory again.",
            ));
        }
        let file = native::open_root(&root.path, Some(&root.identity)).map_err(root_failure)?;
        if native::volume_token(&file)? != root.volume {
            return Err(changed("The registered inspection volume changed."));
        }
        Ok(())
    }
    pub(crate) fn capture(
        &mut self,
        root_index: usize,
        relative: PathBuf,
        id: String,
    ) -> Result<()> {
        let retained = native::encode_path(&relative)
            .len()
            .saturating_mul(2)
            .saturating_add(id.len())
            .saturating_add(256);
        if self.entries.len() >= crate::inspection::candidates::MAX_ITEMS
            || self.metadata_bytes.saturating_add(retained) > 64 * 1024 * 1024
        {
            self.limited = true;
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "Inspection membership reached its file/name-memory bound. This inventory is incomplete.",
            ));
        }
        if self.member_ids.contains(&id) {
            return Err(invalid(
                "Inspection membership contains a repeated file ID.",
            ));
        }
        self.validate_root(root_index)?;
        let root = &self.roots[root_index];
        let file = native::open_asset(&root.path, &root.identity, &root.volume, &relative)?;
        let metadata = file.metadata()?;
        let identity = filesystem::identity(&file)?;
        if identity.file == 0 {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "Stable native object identity is unavailable.",
            ));
        }
        let properties = Properties {
            filename: relative
                .file_stem()
                .and_then(|name| name.to_str())
                .map(str::to_owned),
            extension: relative
                .extension()
                .and_then(|name| name.to_str())
                .map(str::to_owned),
            bytes: metadata.len(),
            modified_millis: filesystem_millis(metadata.modified()),
            created_millis: filesystem_millis(metadata.created()),
            readonly: Some(metadata.permissions().readonly()),
        };
        #[cfg(unix)]
        let allocated = {
            use std::os::unix::fs::MetadataExt;
            metadata.blocks().checked_mul(512)
        };
        #[cfg(not(unix))]
        let allocated = None;
        self.member_ids.insert(id.clone());
        self.metadata_bytes += retained;
        self.entries.push(Snapshot {
            id,
            root: root_index,
            relative,
            identity,
            revision: native::revision(&file)?,
            bytes: metadata.len(),
            properties,
            allocated,
        });
        Ok(())
    }
    pub(crate) fn items(&self) -> Vec<Item> {
        self.entries
            .iter()
            .map(|entry| Item {
                id: entry.id.clone(),
                object: Some(format!("{}:{}", entry.identity.volume, entry.identity.file)),
                links: entry.identity.links,
                available: true,
                properties: entry.properties.clone(),
            })
            .collect()
    }
    pub(crate) fn check_all_roots(&self) -> Result<()> {
        for index in 0..self.roots.len() {
            self.validate_root(index)?;
        }
        Ok(())
    }
}
fn filesystem_millis(time: std::io::Result<std::time::SystemTime>) -> Option<i64> {
    let nanos = match time.ok()?.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => i128::try_from(duration.as_nanos()).ok()?,
        Err(error) => -i128::try_from(error.duration().as_nanos()).ok()?,
    };
    i64::try_from(nanos.div_euclid(1_000_000)).ok()
}

impl Snapshot {
    pub(crate) fn object_key(&self) -> String {
        format!("{}:{}", self.identity.volume, self.identity.file)
    }
    pub(crate) fn folder_id(root_id: &str, relative: &std::path::Path) -> String {
        if relative.as_os_str().is_empty() {
            return root_id.into();
        }
        let mut hash = blake3::Hasher::new();
        hash.update(b"inspection-folder-v1");
        hash.update(root_id.as_bytes());
        hash.update(&native::encode_path(relative));
        hash.finalize().to_hex().to_string()
    }
    pub(crate) fn finding(
        &self,
        root: &RootRecord,
        asset_id: Option<String>,
        media: bool,
        policy: Option<&crate::inspection::model::NamePolicy>,
    ) -> Result<crate::inspection::model::InspectionFinding> {
        use crate::inspection::model::*;
        let parent = self.relative.parent().unwrap_or(std::path::Path::new(""));
        let name = self
            .relative
            .file_name()
            .ok_or_else(|| invalid("Inspection filename is missing."))?
            .to_string_lossy()
            .into_owned();
        let format_time = |time: Option<i64>| {
            time.and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
                .map(|time| time.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        };
        Ok(InspectionFinding {
            id: self.id.clone(),
            asset_id,
            root_id: root.view.id.clone(),
            folder_id: Self::folder_id(&root.view.id, parent),
            name,
            display_path: root
                .path
                .join(&self.relative)
                .to_string_lossy()
                .into_owned(),
            bytes: self.bytes.to_string(),
            reported_allocated_bytes: self.allocated.map(|bytes| bytes.to_string()),
            object_id: Some(
                blake3::hash(self.object_key().as_bytes())
                    .to_hex()
                    .to_string(),
            ),
            links: self.identity.links.to_string(),
            is_media: media,
            inventory: InventoryState::Present,
            content: ContentState::NotChecked,
            checksum: ChecksumState::NotChecked,
            comparison_name: policy.and_then(|policy| {
                crate::inspection::rules::name_key(self.properties.filename.as_deref(), policy)
            }),
            filesystem_modified: format_time(self.properties.modified_millis),
            filesystem_created: format_time(self.properties.created_millis),
            readonly: self.properties.readonly,
            baseline_id: None,
            decoder_policy: None,
            pixel_signature: None,
            pixel_digest: None,
            width: None,
            height: None,
            format: None,
            coverage: "Native regular-file metadata; content not checked".into(),
            error: None,
        })
    }
    pub(crate) fn size(
        &self,
        root: &RootRecord,
        media_scope: bool,
    ) -> crate::inspection::storage::FileSize {
        let mut relative = PathBuf::new();
        let mut folders = Vec::new();
        if let Some(parent) = self.relative.parent() {
            for part in parent.components() {
                relative.push(part);
                folders.push((
                    Self::folder_id(&root.view.id, &relative),
                    part.as_os_str().to_string_lossy().into_owned(),
                ));
            }
        }
        crate::inspection::storage::FileSize {
            id: self.id.clone(),
            root_id: root.view.id.clone(),
            root_label: root.view.label.clone(),
            folders,
            object: Some(self.object_key()),
            logical: self.bytes,
            allocated: self.allocated,
            in_media_scope: media_scope,
        }
    }
}
impl Sources for NativeSources {
    type Reader = Reader;
    fn open(&self, index: usize) -> Result<Reader> {
        let entry = self
            .entries
            .get(index)
            .ok_or_else(|| invalid("Unknown inspection member."))?;
        self.validate_root(entry.root)?;
        let root = &self.roots[entry.root];
        let file = native::open_asset(&root.path, &root.identity, &root.volume, &entry.relative)?;
        let reader = Reader {
            file,
            volume: entry.identity.volume,
            cancel: self.cancel.clone(),
            bytes_read: self.bytes_read.clone(),
            resources: self.resources.clone(),
        };
        self.validate(index, &reader)?;
        Ok(reader)
    }
    fn validate(&self, index: usize, reader: &Reader) -> Result<()> {
        let entry = self
            .entries
            .get(index)
            .ok_or_else(|| invalid("Unknown inspection member."))?;
        self.validate_root(entry.root)?;
        let root = &self.roots[entry.root];
        let current =
            native::open_asset(&root.path, &root.identity, &root.volume, &entry.relative)?;
        for file in [&reader.file, &current] {
            if filesystem::identity(file)? != entry.identity
                || native::revision(file)? != entry.revision
                || file.metadata()?.len() != entry.bytes
            {
                return Err(changed(
                    "The inspected file changed native identity, revision, length or directory membership.",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::model::{RootMode, ScanFilter};
    fn fixture() -> (tempfile::TempDir, NativeSources) {
        let run = tempfile::tempdir().unwrap();
        let source = run.path().join("source");
        std::fs::create_dir(&source).unwrap();
        let file = source.join("Photo.JPG");
        assert!(file.starts_with(run.path()));
        std::fs::write(file, b"generated content").unwrap();
        let catalog = Arc::new(Catalog::open(run.path().join("fixture.stillharbour")).unwrap());
        let root = native::open_root(&source, None).unwrap();
        let id = store::add_root(
            &catalog,
            source,
            filesystem::identity(&root).unwrap(),
            native::volume_token(&root).unwrap(),
            "Generated inspection".into(),
            RootMode::Referenced,
            ScanFilter::default(),
        )
        .unwrap();
        let mut sources = NativeSources::new(
            catalog,
            vec![id],
            ResourceScheduler::default(),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
        sources
            .capture(0, "Photo.JPG".into(), "generated".into())
            .unwrap();
        (run, sources)
    }
    #[test]
    fn native_inspection_rejects_same_size_restored_mtime_changes_and_replaced_objects() {
        for replace in [false, true] {
            let (run, mut sources) = fixture();
            let path = run.path().join("source/Photo.JPG");
            assert!(path.starts_with(run.path()));
            let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
            // Windows held directory scopes deny some external name changes.
            // Model the between-review interval without relaxing those handles:
            // retain only old facts, close the scope, then reacquire after edit.
            let snapshot = sources.entries.pop().unwrap();
            let catalog = sources.catalog.clone();
            let root_id = sources.roots[0].view.id.clone();
            drop(sources);
            if replace {
                std::fs::rename(&path, run.path().join("retained-original")).unwrap();
            }
            let bytes = if replace {
                b"generated content"
            } else {
                b"Generated content"
            };
            std::fs::write(&path, bytes).unwrap();
            File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(modified)
                .unwrap();
            let mut sources = NativeSources::new(
                catalog,
                vec![root_id],
                ResourceScheduler::default(),
                Arc::new(AtomicBool::new(false)),
            )
            .unwrap();
            sources.entries.push(snapshot);
            assert!(sources.open(0).is_err());
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
        }
    }
    #[test]
    fn native_inspection_hashes_without_source_changes_and_rejects_root_revocation() {
        let (run, sources) = fixture();
        let before = sources.entries[0].revision.clone();
        let io = crate::jobs::paced_io::PacedIo::new(1024 * 1024 * 1024).unwrap();
        let mut reader = sources.open(0).unwrap();
        let outcome = crate::inspection::hashing::full(
            &mut reader,
            sources.entries[0].bytes,
            crate::inspection::model::ContentAlgorithm::Blake3,
            &io,
            &sources.cancel,
            &mut crate::inspection::hashing::Stats::default(),
        )
        .unwrap();
        assert!(matches!(
            outcome,
            crate::jobs::paced_io::IoStep::Completed(_)
        ));
        sources.validate(0, &reader).unwrap();
        assert_eq!(native::revision(&reader.file).unwrap(), before);
        assert_eq!(
            std::fs::read(run.path().join("source/Photo.JPG")).unwrap(),
            b"generated content"
        );
        let id = sources.roots[0].view.id.clone();
        sources
            .catalog
            .call(move |db| {
                db.execute("UPDATE roots SET availability='offline' WHERE id=?", [id])?;
                Ok(())
            })
            .unwrap();
        let error = sources.validate(0, &reader).unwrap_err();
        assert_eq!(
            inventory_state(&error),
            crate::inspection::model::InventoryState::Offline
        );
    }
    #[test]
    fn inventory_errors_distinguish_offline_missing_access_and_changed_without_string_matching() {
        use crate::inspection::model::InventoryState;
        assert_eq!(
            inventory_state(&unavailable_root("offline")),
            InventoryState::Offline
        );
        assert_eq!(
            inventory_state(&unavailable_root("inaccessible")),
            InventoryState::Inaccessible
        );
        let missing = AppError::from(std::io::Error::from(std::io::ErrorKind::NotFound));
        assert_eq!(inventory_state(&missing), InventoryState::Missing);
        assert_eq!(
            inventory_state(&root_failure(missing)),
            InventoryState::Offline
        );
        assert_eq!(
            inventory_state(&changed("Generated stale bytes")),
            InventoryState::ChangedDuringRead
        );
    }
}

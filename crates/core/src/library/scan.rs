use super::{
    filters::CompiledFilter,
    model::ScanJob,
    native,
    store::{self, Discovery, RootRecord},
    workers,
};
#[cfg(any(unix, windows))]
use crate::filesystem;
use crate::{
    catalog::Catalog,
    domain::{AppError, ErrorCode, Result},
    jobs::resources::{Class, ResourceScheduler, Work},
};
#[cfg(any(unix, windows))]
use std::io::{Read, Seek, SeekFrom};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub struct ScanContext {
    pub catalog: Arc<Catalog>,
    pub root: RootRecord,
    pub job: ScanJob,
    pub filter: CompiledFilter,
    pub paths: workers::WorkerPaths,
    #[cfg(any(unix, windows))]
    pub data_directory: PathBuf,
    #[cfg(any(unix, windows))]
    pub data_identity: filesystem::FileIdentity,
    pub canceled: Arc<AtomicBool>,
    pub resources: ResourceScheduler,
    pub discovered: u32,
    pub indexed: u32,
    pub skipped: u32,
    pub errors: u32,
    pub batch: Vec<Discovery>,
}
impl ScanContext {
    fn progress(&self) -> Result<()> {
        store::progress(
            &self.catalog,
            self.job.id.clone(),
            self.discovered,
            self.indexed,
            self.skipped,
            self.errors,
            "Indexing files; originals unchanged".into(),
        )
    }
    fn issue(&mut self, path: &Path, error: AppError) -> Result<()> {
        self.errors = self.errors.saturating_add(1);
        store::issue(
            &self.catalog,
            self.job.id.clone(),
            path.to_string_lossy().into_owned(),
            format!("{:?}", error.code),
            error.message,
        )
    }
    #[cfg(any(unix, windows))]
    fn excluded(&mut self, path: &Path) -> Result<()> {
        self.skipped = self.skipped.saturating_add(1);
        store::exclude_scope(
            &self.catalog,
            self.root.view.id.clone(),
            path.to_owned(),
            self.job.id.clone(),
        )
    }
    fn flush(&mut self, runtime: &tokio::runtime::Runtime) -> Result<()> {
        if self.batch.is_empty() {
            return self.progress();
        }
        let entries = std::mem::take(&mut self.batch);
        let ids = store::discover_batch(
            &self.catalog,
            self.root.view.id.clone(),
            self.job.id.clone(),
            entries.clone(),
        )?;
        self.progress()?;
        for ((id, revision), entry) in ids.into_iter().zip(entries) {
            if self.canceled.load(Ordering::Relaxed) {
                break;
            }
            if !entry.media || entry.placeholder {
                continue;
            }
            let current = store::asset(&self.catalog, id.clone())?;
            if current.metadata_state == "ready" {
                if self
                    .filter
                    .file_reason(
                        &entry.relative,
                        false,
                        entry.bytes as u64,
                        current.date.get(..10),
                    )
                    .is_some()
                {
                    store::exclude_scope(
                        &self.catalog,
                        self.root.view.id.clone(),
                        entry.relative.clone(),
                        self.job.id.clone(),
                    )?;
                }
                self.indexed = self.indexed.saturating_add(1);
                continue;
            }
            let operation = (|| -> Result<_> {
                let file = native::open_asset(
                    &self.root.path,
                    &self.root.identity,
                    &self.root.volume,
                    &entry.relative,
                )?;
                let _lease = self.resources.acquire(
                    crate::filesystem::identity(&file)?.volume,
                    Class::Background,
                    Work::Metadata,
                    &self.canceled,
                )?;
                if native::revision(&file)? != revision {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        "The original changed before metadata extraction.",
                    ));
                }
                let bytes = workers::read_input_cancellable(file, &self.canceled)?;
                let result = runtime.block_on(workers::metadata(
                    &self.paths,
                    bytes,
                    self.canceled.clone(),
                ))?;
                let current = native::open_asset(
                    &self.root.path,
                    &self.root.identity,
                    &self.root.volume,
                    &entry.relative,
                )?;
                if native::revision(&current)? != revision {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        "Source changed during metadata extraction. Rescan the folder.",
                    ));
                }
                Ok(result)
            })();
            match operation {
                Ok(metadata) => {
                    let date = metadata
                        .date
                        .as_deref()
                        .or(entry.modified.as_deref())
                        .or(entry.born.as_deref())
                        .and_then(|d| d.get(..10));
                    let excluded = self
                        .filter
                        .file_reason(&entry.relative, false, entry.bytes as u64, date)
                        .is_some();
                    store::metadata(&self.catalog, id, revision, Some(metadata), None, excluded)?;
                    self.indexed = self.indexed.saturating_add(1);
                }
                Err(error) => {
                    if self.canceled.load(Ordering::Relaxed) {
                        break;
                    }
                    let date = entry
                        .modified
                        .as_deref()
                        .or(entry.born.as_deref())
                        .and_then(|d| d.get(..10));
                    let excluded = self
                        .filter
                        .file_reason(&entry.relative, false, entry.bytes as u64, date)
                        .is_some();
                    store::metadata(
                        &self.catalog,
                        id,
                        revision,
                        None,
                        Some(error.message.clone()),
                        excluded,
                    )?;
                    self.issue(&entry.relative, error)?;
                }
            }
            self.progress()?;
        }
        Ok(())
    }

    pub fn run(mut self) -> Result<()> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let root = native::open_root(&self.root.path, Some(&self.root.identity))?;
        if native::volume_token(&root)? != self.root.volume {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "The root volume changed. Relink it before scanning.",
            ));
        }
        self.walk(root, PathBuf::new(), 0, &runtime)?;
        self.flush(&runtime)?;
        // An open directory handle is not proof that its volume/root still exists
        // at the authorised location after traversal. Revalidate before absence.
        let current = native::open_root(&self.root.path, Some(&self.root.identity))?;
        if native::volume_token(&current)? != self.root.volume {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "Root identity changed during scanning; inventory retained.",
            ));
        }
        let state = if self.canceled.load(Ordering::Relaxed) {
            "canceled"
        } else if self.errors > 0 {
            "partial"
        } else {
            "completed"
        };
        let message = match state {
            "canceled" => "Scan canceled; partial results retained",
            "partial" => "Scan finished with issues; previous inaccessible inventory retained",
            _ => "Scan complete; originals unchanged",
        };
        store::finish_job(&self.catalog, self.job.id, state.into(), message.into())
    }

    #[cfg(unix)]
    fn walk(
        &mut self,
        parent: File,
        relative: PathBuf,
        depth: u32,
        runtime: &tokio::runtime::Runtime,
    ) -> Result<()> {
        use rustix::fs::{AtFlags, Dir, statat};
        use std::os::unix::ffi::OsStrExt;
        if self.canceled.load(Ordering::Relaxed) {
            return Ok(());
        }
        if depth > 64 {
            self.issue(
                &relative,
                AppError::new(
                    ErrorCode::Unsupported,
                    "Directory exceeds the 64-level traversal limit.",
                ),
            )?;
            return store::directory(
                &self.catalog,
                self.root.view.id.clone(),
                relative,
                self.job.id.clone(),
                "incomplete".into(),
            );
        }
        let iterator = match Dir::read_from(&parent) {
            Ok(d) => d,
            Err(e) => {
                self.issue(&relative, AppError::from(std::io::Error::from(e)))?;
                return store::directory(
                    &self.catalog,
                    self.root.view.id.clone(),
                    relative,
                    self.job.id.clone(),
                    "incomplete".into(),
                );
            }
        };
        store::directory(
            &self.catalog,
            self.root.view.id.clone(),
            relative.clone(),
            self.job.id.clone(),
            "incomplete".into(),
        )?;
        let mut complete = true;
        for entry in iterator {
            if self.canceled.load(Ordering::Relaxed) {
                complete = false;
                break;
            }
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    self.issue(&relative, AppError::from(std::io::Error::from(e)))?;
                    complete = false;
                    break;
                }
            };
            let name = entry.file_name();
            if name.to_bytes() == b"." || name.to_bytes() == b".." {
                continue;
            }
            let relative_entry = relative.join(std::ffi::OsStr::from_bytes(name.to_bytes()));
            let stat = match statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(s) => s,
                Err(e) => {
                    self.issue(&relative_entry, AppError::from(std::io::Error::from(e)))?;
                    store::uncertain(
                        &self.catalog,
                        self.root.view.id.clone(),
                        relative_entry,
                        self.job.id.clone(),
                    )?;
                    continue;
                }
            };
            let mode = stat.st_mode as u32 & libc::S_IFMT as u32;
            let hidden_name = name.to_bytes().first() == Some(&b'.');
            #[cfg(target_os = "macos")]
            let (hidden, placeholder) = (
                hidden_name || stat.st_flags & libc::UF_HIDDEN != 0,
                stat.st_flags & 0x40000000 != 0,
            );
            #[cfg(not(target_os = "macos"))]
            let (hidden, placeholder) = (hidden_name, false);
            if mode == libc::S_IFDIR as u32 {
                let app_path = self.root.path.join(&relative_entry);
                if self
                    .filter
                    .directory_reason(&relative_entry, hidden)
                    .is_some()
                    || app_path.starts_with(&self.data_directory)
                    || (stat.st_dev as u64 == self.data_identity.volume
                        && stat.st_ino as u64 == self.data_identity.file)
                {
                    self.excluded(&relative_entry)?;
                    store::directory(
                        &self.catalog,
                        self.root.view.id.clone(),
                        relative_entry,
                        self.job.id.clone(),
                        "excluded".into(),
                    )?;
                    continue;
                }
                if placeholder {
                    self.issue(
                        &relative_entry,
                        AppError::new(
                            ErrorCode::Unsupported,
                            "Cloud folder placeholder was not downloaded.",
                        ),
                    )?;
                    store::directory(
                        &self.catalog,
                        self.root.view.id.clone(),
                        relative_entry,
                        self.job.id.clone(),
                        "incomplete".into(),
                    )?;
                    continue;
                }
                match native::child_file(&parent, name, true) {
                    Ok(child) => self.walk(child, relative_entry, depth + 1, runtime)?,
                    Err(error) => {
                        self.issue(&relative_entry, error)?;
                        store::directory(
                            &self.catalog,
                            self.root.view.id.clone(),
                            relative_entry,
                            self.job.id.clone(),
                            "incomplete".into(),
                        )?;
                    }
                }
                continue;
            }
            if mode != libc::S_IFREG as u32 {
                self.skipped = self.skipped.saturating_add(1);
                store::uncertain(
                    &self.catalog,
                    self.root.view.id.clone(),
                    relative_entry.clone(),
                    self.job.id.clone(),
                )?;
                store::directory(
                    &self.catalog,
                    self.root.view.id.clone(),
                    relative_entry,
                    self.job.id.clone(),
                    "blocked".into(),
                )?;
                continue;
            }
            let size = match u64::try_from(stat.st_size) {
                Ok(n) => n,
                Err(_) => {
                    self.issue(
                        &relative_entry,
                        AppError::new(ErrorCode::Io, "The filesystem returned an invalid size."),
                    )?;
                    continue;
                }
            };
            if self
                .filter
                .file_reason(&relative_entry, hidden, size, None)
                .is_some()
            {
                self.excluded(&relative_entry)?;
                continue;
            }
            if placeholder {
                let revision = blake3::hash(
                    format!("placeholder:{}:{}:{}", stat.st_dev, stat.st_ino, size).as_bytes(),
                )
                .to_hex()
                .to_string();
                let extension = relative_entry
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_ascii_lowercase();
                self.batch.push(Discovery {
                    relative: relative_entry,
                    revision,
                    native_file: format!("{}:{}", stat.st_dev, stat.st_ino),
                    links: stat.st_nlink as u64,
                    bytes: i64::try_from(size).map_err(|_| {
                        AppError::new(
                            ErrorCode::Unsupported,
                            "Placeholder exceeds catalog size range.",
                        )
                    })?,
                    modified: chrono::DateTime::<chrono::Utc>::from_timestamp(stat.st_mtime, 0)
                        .map(|d| d.to_rfc3339()),
                    born: None,
                    detected: None,
                    media: is_media_extension(&extension),
                    placeholder: true,
                });
                self.discovered = self.discovered.saturating_add(1);
                if self.batch.len() >= 32 {
                    self.flush(runtime)?;
                }
                continue;
            }
            let result = (|| -> Result<Discovery> {
                let mut file = native::child_file(&parent, name, false)?;
                let metadata = file.metadata()?;
                if !metadata.is_file() {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        "Entry changed during enumeration.",
                    ));
                }
                let identity = filesystem::identity(&file)?;
                let revision = native::revision(&file)?;
                let mut header = [0u8; 32];
                let n = if placeholder {
                    0
                } else {
                    file.read(&mut header)?
                };
                file.seek(SeekFrom::Start(0))?;
                let detected = image::guess_format(&header[..n])
                    .ok()
                    .map(|f| format!("{f:?}").to_ascii_lowercase());
                let extension = relative_entry
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_ascii_lowercase();
                let media = detected.is_some() || is_media_extension(&extension);
                Ok(Discovery {
                    relative: relative_entry.clone(),
                    revision,
                    native_file: format!("{}:{}", identity.volume, identity.file),
                    links: identity.links,
                    bytes: i64::try_from(metadata.len()).map_err(|_| {
                        AppError::new(ErrorCode::Unsupported, "File size exceeds catalog range.")
                    })?,
                    modified: native::unix_time(metadata.modified().ok()),
                    born: native::unix_time(metadata.created().ok()),
                    detected,
                    media,
                    placeholder,
                })
            })();
            match result {
                Ok(entry) => {
                    self.batch.push(entry);
                    self.discovered = self.discovered.saturating_add(1)
                }
                Err(error) => {
                    self.issue(&relative_entry, error)?;
                    store::uncertain(
                        &self.catalog,
                        self.root.view.id.clone(),
                        relative_entry,
                        self.job.id.clone(),
                    )?;
                }
            }
            if self.batch.len() >= 32 {
                self.flush(runtime)?;
            }
        }
        self.flush(runtime)?;
        if complete {
            store::directory(
                &self.catalog,
                self.root.view.id.clone(),
                relative,
                self.job.id.clone(),
                "complete".into(),
            )?;
        }
        Ok(())
    }
    #[cfg(not(any(unix, windows)))]
    fn walk(
        &mut self,
        _parent: File,
        _relative: PathBuf,
        _depth: u32,
        _runtime: &tokio::runtime::Runtime,
    ) -> Result<()> {
        Err(AppError::new(
            ErrorCode::Unsupported,
            "Handle-anchored traversal needs the Windows native implementation and tests.",
        ))
    }
}

#[cfg(any(unix, windows))]
fn is_media_extension(extension: &str) -> bool {
    super::model::DEFAULT_PHOTO_EXTENSIONS.contains(&extension)
        || matches!(
            extension,
            "mp4" | "mov" | "m4v" | "avi" | "mts" | "m2ts" | "mkv"
        )
}

#[cfg(windows)]
#[path = "scan_windows.rs"]
mod windows;

#[cfg(all(test, unix))]
mod tests {
    #[test]
    fn all_default_photo_formats_remain_media_when_headers_are_unsupported() {
        for extension in super::super::model::DEFAULT_PHOTO_EXTENSIONS {
            assert!(super::is_media_extension(extension), "{extension}");
        }
        assert!(super::is_media_extension("mov"));
        assert!(!super::is_media_extension("xmp"));
    }
}

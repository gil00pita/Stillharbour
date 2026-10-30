//! Windows enumeration adapter for the existing read-only scan orchestration.
use super::super::store::Discovery;
use super::*;
use windows_sys::Win32::Storage::FileSystem::*;

impl ScanContext {
    pub(super) fn walk(
        &mut self,
        parent: File,
        relative: PathBuf,
        depth: u32,
        runtime: &tokio::runtime::Runtime,
    ) -> Result<()> {
        if self.canceled.load(Ordering::Relaxed) {
            return Ok(());
        }
        store::directory(
            &self.catalog,
            self.root.view.id.clone(),
            relative.clone(),
            self.job.id.clone(),
            "incomplete".into(),
        )?;
        if depth > 64 {
            self.issue(
                &relative,
                AppError::new(
                    ErrorCode::Unsupported,
                    "Directory exceeds the 64-level traversal limit.",
                ),
            )?;
            return Ok(());
        }
        let mut directory = match filesystem::windows::Directory::new(&parent) {
            Ok(directory) => directory,
            Err(error) => {
                self.issue(&relative, error)?;
                return Ok(());
            }
        };
        let mut complete = true;
        loop {
            if self.canceled.load(Ordering::Relaxed) {
                complete = false;
                break;
            }
            let entry = match directory.next() {
                Ok(Some(entry)) => entry,
                Ok(None) => break,
                Err(error) => {
                    self.issue(&relative, error)?;
                    complete = false;
                    break;
                }
            };
            if entry.name == "." || entry.name == ".." {
                continue;
            }
            let path = relative.join(&entry.name);
            let is_directory = entry.attributes & FILE_ATTRIBUTE_DIRECTORY != 0;
            let hidden = entry.attributes & FILE_ATTRIBUTE_HIDDEN != 0
                || entry.name.to_string_lossy().starts_with('.');
            if entry.attributes
                & (FILE_ATTRIBUTE_REPARSE_POINT
                    | FILE_ATTRIBUTE_OFFLINE
                    | FILE_ATTRIBUTE_RECALL_ON_OPEN
                    | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
                != 0
            {
                self.skipped = self.skipped.saturating_add(1);
                self.issue(&path, AppError::new(ErrorCode::Unsupported, "A link, reparse point or cloud placeholder was not traversed or downloaded."))?;
                store::uncertain(
                    &self.catalog,
                    self.root.view.id.clone(),
                    path.clone(),
                    self.job.id.clone(),
                )?;
                store::directory(
                    &self.catalog,
                    self.root.view.id.clone(),
                    path,
                    self.job.id.clone(),
                    "blocked".into(),
                )?;
                continue;
            }
            if is_directory {
                let managed = self.root.identity.volume == self.data_identity.volume
                    && entry.file_id == self.data_identity.file;
                if managed
                    || self.root.path.join(&path).starts_with(&self.data_directory)
                    || self.filter.directory_reason(&path, hidden).is_some()
                {
                    self.excluded(&path)?;
                    store::directory(
                        &self.catalog,
                        self.root.view.id.clone(),
                        path,
                        self.job.id.clone(),
                        "excluded".into(),
                    )?;
                    continue;
                }
                match filesystem::windows::child(&parent, &entry.name, true) {
                    Ok(child) => {
                        let identity = filesystem::identity(&child)?;
                        if identity.volume != self.root.identity.volume
                            || identity.file != entry.file_id
                        {
                            self.issue(
                                &path,
                                AppError::new(
                                    ErrorCode::Conflict,
                                    "Directory identity changed during enumeration.",
                                ),
                            )?;
                            store::directory(
                                &self.catalog,
                                self.root.view.id.clone(),
                                path,
                                self.job.id.clone(),
                                "incomplete".into(),
                            )?;
                        } else {
                            self.walk(child, path, depth + 1, runtime)?;
                        }
                    }
                    Err(error) => {
                        self.issue(&path, error)?;
                        store::directory(
                            &self.catalog,
                            self.root.view.id.clone(),
                            path,
                            self.job.id.clone(),
                            "incomplete".into(),
                        )?;
                    }
                }
                continue;
            }
            if self
                .filter
                .file_reason(&path, hidden, entry.bytes, None)
                .is_some()
            {
                self.excluded(&path)?;
                continue;
            }
            let result = (|| -> Result<Discovery> {
                let mut file = filesystem::windows::child(&parent, &entry.name, false)?;
                let identity = filesystem::identity(&file)?;
                if identity.volume != self.root.identity.volume || identity.file != entry.file_id {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        "File identity changed during enumeration.",
                    ));
                }
                let metadata = file.metadata()?;
                let revision = native::revision(&file)?;
                let mut header = [0u8; 32];
                let n = file.read(&mut header)?;
                file.seek(SeekFrom::Start(0))?;
                let detected = image::guess_format(&header[..n])
                    .ok()
                    .map(|f| format!("{f:?}").to_ascii_lowercase());
                let extension = path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_ascii_lowercase();
                let media = detected.is_some() || is_media_extension(&extension);
                Ok(Discovery {
                    relative: path.clone(),
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
                    placeholder: false,
                })
            })();
            match result {
                Ok(entry) => {
                    self.batch.push(entry);
                    self.discovered = self.discovered.saturating_add(1);
                }
                Err(error) => {
                    self.issue(&path, error)?;
                    store::uncertain(
                        &self.catalog,
                        self.root.view.id.clone(),
                        path,
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
}

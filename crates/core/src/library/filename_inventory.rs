//! Advisory inventory only. No file content, presence reconciliation or queued paths.
use super::*;

impl ScanContext {
    #[cfg(windows)]
    pub(super) fn count_names(&self, parent: &File, relative: PathBuf, depth: u32) -> Result<u32> {
        use windows_sys::Win32::Storage::FileSystem::*;
        self.boundary()?;
        if depth > 64 || self.canceled.load(Ordering::Relaxed) {
            return Err(AppError::new(ErrorCode::Busy, "Inventory incomplete"));
        }
        let mut directory = filesystem::windows::Directory::new(parent)?;
        let mut count = 0u32;
        while let Some(entry) = directory.next()? {
            self.boundary()?;
            if self.canceled.load(Ordering::Relaxed) {
                return Err(AppError::new(ErrorCode::Busy, "Inventory stopped"));
            }
            if entry.name == "." || entry.name == ".." {
                continue;
            }
            let path = relative.join(&entry.name);
            if entry.attributes
                & (FILE_ATTRIBUTE_REPARSE_POINT
                    | FILE_ATTRIBUTE_OFFLINE
                    | FILE_ATTRIBUTE_RECALL_ON_OPEN
                    | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
                != 0
            {
                continue;
            }
            let hidden = entry.attributes & FILE_ATTRIBUTE_HIDDEN != 0
                || entry.name.to_string_lossy().starts_with('.');
            if entry.attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
                if self.filter.directory_reason(&path, hidden).is_some()
                    || self.root.path.join(&path).starts_with(&self.data_directory)
                    || (self.root.identity.volume == self.data_identity.volume
                        && entry.file_id == self.data_identity.file)
                {
                    continue;
                }
                let child = filesystem::windows::child(parent, &entry.name, true)?;
                let identity = filesystem::identity(&child)?;
                if identity.volume != self.root.identity.volume || identity.file != entry.file_id {
                    return Err(AppError::new(
                        ErrorCode::Conflict,
                        "Directory changed during inventory",
                    ));
                }
                count = count.saturating_add(self.count_names(&child, path, depth + 1)?);
            } else if self
                .filter
                .file_reason(&path, hidden, entry.bytes, None)
                .is_none()
            {
                count = count.saturating_add(1);
            }
        }
        Ok(count)
    }

    #[cfg(unix)]
    pub(super) fn count_names(&self, parent: &File, relative: PathBuf, depth: u32) -> Result<u32> {
        use rustix::fs::{AtFlags, Dir, statat};
        use std::os::unix::ffi::OsStrExt;
        self.boundary()?;
        if depth > 64 || self.canceled.load(Ordering::Relaxed) {
            return Err(AppError::new(ErrorCode::Busy, "Inventory incomplete"));
        }
        let mut count = 0u32;
        for entry in Dir::read_from(parent).map_err(std::io::Error::from)? {
            self.boundary()?;
            if self.canceled.load(Ordering::Relaxed) {
                return Err(AppError::new(ErrorCode::Busy, "Inventory stopped"));
            }
            let entry = entry.map_err(std::io::Error::from)?;
            let name = entry.file_name();
            if name.to_bytes() == b"." || name.to_bytes() == b".." {
                continue;
            }
            let path = relative.join(std::ffi::OsStr::from_bytes(name.to_bytes()));
            let stat =
                statat(parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(std::io::Error::from)?;
            let mode = stat.st_mode as u32 & libc::S_IFMT as u32;
            let hidden_name = name.to_bytes().first() == Some(&b'.');
            #[cfg(target_os = "macos")]
            let (hidden, placeholder) = (
                hidden_name || stat.st_flags & libc::UF_HIDDEN != 0,
                stat.st_flags & 0x40000000 != 0,
            );
            #[cfg(not(target_os = "macos"))]
            let (hidden, placeholder) = (hidden_name, false);
            if placeholder {
                continue;
            }
            if mode == libc::S_IFDIR as u32 {
                if self.filter.directory_reason(&path, hidden).is_some()
                    || self.root.path.join(&path).starts_with(&self.data_directory)
                    || (stat.st_dev as u64 == self.data_identity.volume
                        && stat.st_ino as u64 == self.data_identity.file)
                {
                    continue;
                }
                let child = native::child_file(parent, name, true)?;
                count = count.saturating_add(self.count_names(&child, path, depth + 1)?);
            } else if mode == libc::S_IFREG as u32
                && self
                    .filter
                    .file_reason(&path, hidden, stat.st_size as u64, None)
                    .is_none()
            {
                count = count.saturating_add(1);
            }
        }
        Ok(count)
    }
}

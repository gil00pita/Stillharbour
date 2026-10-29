//! Deletion authority is an open, backend-owned cache directory, never an
//! original path. Only catalog-indexed keys are accepted; no recursive sweep.
use crate::domain::{AppError, ErrorCode, Result};
use std::path::Path;
#[cfg(unix)]
use std::{
    ffi::CString,
    fs::File,
    io::Read,
    os::fd::{AsRawFd, FromRawFd},
};

pub struct CacheCleanup {
    #[cfg(unix)]
    directory: File,
}
impl CacheCleanup {
    pub fn open(data: &Path) -> Result<Self> {
        #[cfg(unix)]
        {
            let parent = crate::filesystem::open_readonly(data)?;
            let name = c"thumbnails";
            // SAFETY: live owned directory descriptor and terminated literal.
            let fd = unsafe {
                libc::openat(
                    parent.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            // SAFETY: successful openat returns an exclusively owned descriptor.
            Ok(Self {
                directory: unsafe { File::from_raw_fd(fd) },
            })
        }
        #[cfg(not(unix))]
        {
            let _ = data;
            Ok(Self {})
        }
    }
    pub fn remove(&self, key: &str) -> Result<()> {
        if key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Invalid owned cache key.",
            ));
        }
        #[cfg(unix)]
        {
            let name = CString::new(format!("{key}.png"))
                .map_err(|_| AppError::new(ErrorCode::InvalidInput, "Invalid cache key."))?;
            // O_NONBLOCK also prevents a substituted FIFO from blocking cleanup.
            // SAFETY: valid descriptor, C string and defined open flags.
            let fd = unsafe {
                libc::openat(
                    self.directory.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
                )
            };
            if fd < 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::NotFound {
                    return Ok(());
                }
                return Err(error.into());
            }
            // SAFETY: successful openat returns an exclusively owned descriptor.
            let mut file = unsafe { File::from_raw_fd(fd) };
            let metadata = file.metadata()?;
            let identity = crate::filesystem::identity(&file)?;
            let mut signature = [0u8; 8];
            if !metadata.is_file()
                || metadata.len()
                    > crate::rendition::png_budget(crate::rendition::VIEWER_EDGE)? as u64
                || identity.links != 1
                || file.read_exact(&mut signature).is_err()
                || signature != *b"\x89PNG\r\n\x1a\n"
            {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "An unexpected cache file was retained. The library has not been removed.",
                ));
            }
            // Check for normal concurrent replacement; unlinkat never follows a
            // final link. This is not a hostile-writer authorization engine.
            let mut current = std::mem::MaybeUninit::<libc::stat>::uninit();
            // SAFETY: output points to valid storage; name/descriptor remain live.
            if unsafe {
                libc::fstatat(
                    self.directory.as_raw_fd(),
                    name.as_ptr(),
                    current.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            } != 0
            {
                return Err(std::io::Error::last_os_error().into());
            }
            // SAFETY: fstatat initialized the stat on success.
            let current = unsafe { current.assume_init() };
            if current.st_dev as u64 != identity.volume
                || current.st_ino != identity.file
                || current.st_nlink != 1
            {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "The cache file changed; review cleanup again.",
                ));
            }
            // SAFETY: descriptor-relative unlink of one validated indexed name,
            // without AT_REMOVEDIR, cannot recursively remove a directory.
            if unsafe { libc::unlinkat(self.directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            Err(AppError::new(
                ErrorCode::Unsupported,
                "Cache cleanup requires native platform validation.",
            ))
        }
    }
    pub fn sync(&self) -> Result<()> {
        #[cfg(unix)]
        {
            self.directory.sync_all()?;
            Ok(())
        }
        #[cfg(not(unix))]
        {
            Err(AppError::new(
                ErrorCode::Unsupported,
                "Cache cleanup requires native platform validation.",
            ))
        }
    }
}
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn only_indexed_regular_cache_names_are_removed_without_following_links() {
        let data = tempfile::tempdir().unwrap();
        let cache = data.path().join("thumbnails");
        std::fs::create_dir(&cache).unwrap();
        let cleanup = CacheCleanup::open(data.path()).unwrap();
        let key = "a".repeat(64);
        let path = cache.join(format!("{key}.png"));
        assert!(cleanup.remove("../original").is_err());
        let original = data.path().join("original.png");
        std::fs::write(&original, b"\x89PNG\r\n\x1a\noriginal").unwrap();
        std::os::unix::fs::symlink(&original, &path).unwrap();
        assert!(cleanup.remove(&key).is_err());
        std::fs::remove_file(&path).unwrap();
        std::fs::hard_link(&original, &path).unwrap();
        assert!(cleanup.remove(&key).is_err());
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, b"unexpected").unwrap();
        assert!(cleanup.remove(&key).is_err());
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, b"\x89PNG\r\n\x1a\nowned").unwrap();
        cleanup.remove(&key).unwrap();
        cleanup.remove(&key).unwrap();
        cleanup.sync().unwrap();
        assert_eq!(
            std::fs::read(original).unwrap(),
            b"\x89PNG\r\n\x1a\noriginal"
        );
    }
    #[test]
    fn directory_replacement_cannot_redirect_cleanup_into_another_folder() {
        let data = tempfile::tempdir().unwrap();
        let cache = data.path().join("thumbnails");
        std::fs::create_dir(&cache).unwrap();
        let cleanup = CacheCleanup::open(data.path()).unwrap();
        let moved = data.path().join("old-cache");
        std::fs::rename(&cache, &moved).unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), &cache).unwrap();
        let name = format!("{}.png", "b".repeat(64));
        std::fs::write(outside.path().join(&name), b"\x89PNG\r\n\x1a\noriginal").unwrap();
        cleanup.remove(&"b".repeat(64)).unwrap();
        assert!(outside.path().join(name).exists());
        assert!(CacheCleanup::open(data.path()).is_err());
    }
}

//! Source access is anchored to open directory handles. No child symlink is followed.
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem::{self, FileIdentity},
};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

pub fn encode_path(path: &Path) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes().to_vec()
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        path.as_os_str()
            .encode_wide()
            .flat_map(u16::to_le_bytes)
            .collect()
    }
}
pub fn decode_path(bytes: &[u8]) -> Result<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(std::ffi::OsString::from_vec(bytes.to_vec()).into())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        if bytes.len() % 2 != 0 {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Invalid stored native path.",
            ));
        }
        let wide: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|x| u16::from_le_bytes([x[0], x[1]]))
            .collect();
        Ok(std::ffi::OsString::from_wide(&wide).into())
    }
}
pub fn open_root(path: &Path, expected: Option<&FileIdentity>) -> Result<File> {
    let file = filesystem::open_readonly(path)?;
    if !file.metadata()?.is_dir() {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Choose a directory.",
        ));
    }
    let actual = filesystem::identity(&file)?;
    if expected.is_some_and(|e| e.volume != actual.volume || e.file != actual.file) {
        return Err(AppError::new(
            ErrorCode::Conflict,
            "Folder identity changed. Reconnect or explicitly relink it.",
        ));
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::macos::fs::MetadataExt;
        if file.metadata()?.st_flags() & 0x40000000 != 0 {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "This folder is an undownloaded cloud placeholder.",
            ));
        }
    }
    Ok(file)
}

pub fn volume_token(file: &File) -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        use std::os::fd::AsRawFd;
        let mut attributes = libc::attrlist {
            bitmapcount: 5,
            reserved: 0,
            commonattr: 0,
            volattr: libc::ATTR_VOL_INFO | libc::ATTR_VOL_UUID,
            dirattr: 0,
            fileattr: 0,
            forkattr: 0,
        };
        let mut result = [0u8; 20];
        // SAFETY: the file descriptor is borrowed/live; attrlist selects a single
        // fixed 16-byte UUID following the four-byte returned buffer length.
        let status = unsafe {
            libc::fgetattrlist(
                file.as_raw_fd(),
                (&mut attributes as *mut libc::attrlist).cast(),
                result.as_mut_ptr().cast(),
                result.len(),
                0,
            )
        };
        if status != 0
            || u32::from_ne_bytes(result[..4].try_into().expect("four bytes")) != 20
            || result[4..].iter().all(|b| *b == 0)
        {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "This volume cannot provide the stable identity required for indexing.",
            )
            .caused_by(std::io::Error::last_os_error()));
        }
        Ok(result[4..].iter().map(|b| format!("{b:02x}")).collect())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = file;
        Err(AppError::new(
            ErrorCode::Unsupported,
            "Stable volume identity requires native platform validation.",
        ))
    }
}

pub fn revision(file: &File) -> Result<String> {
    let m = file.metadata()?;
    let id = filesystem::identity(file)?;
    #[cfg(unix)]
    let changed = {
        use std::os::unix::fs::MetadataExt;
        format!("{}:{}", m.ctime(), m.ctime_nsec())
    };
    #[cfg(not(unix))]
    let changed = String::new();
    let bytes = format!(
        "{}:{}:{}:{:?}:{}",
        id.volume,
        id.file,
        m.len(),
        m.modified()?,
        changed
    );
    Ok(blake3::hash(bytes.as_bytes()).to_hex().to_string())
}

#[cfg(unix)]
pub fn child_file(parent: &File, name: &std::ffi::CStr, directory: bool) -> Result<File> {
    use rustix::fs::{Mode, OFlags, openat};
    let mut flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK;
    if directory {
        flags |= OFlags::DIRECTORY;
    }
    let fd = openat(parent, name, flags, Mode::empty())
        .map_err(|e| AppError::from(std::io::Error::from(e)))?;
    Ok(File::from(fd))
}

/// Re-open a recorded relative path from a freshly validated root, checking each
/// ancestor through openat. The renderer never supplies these path components.
pub fn open_asset(
    root: &Path,
    expected: &FileIdentity,
    volume: &str,
    relative: &Path,
) -> Result<File> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid catalog relative path.",
        ));
    }
    #[cfg(unix)]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let mut parent = open_root(root, Some(expected))?;
        if volume_token(&parent)? != volume {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "The library volume has changed. Relink before reading originals.",
            ));
        }
        let parts: Vec<_> = relative.components().collect();
        if parts.is_empty() {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Missing asset path.",
            ));
        }
        for (index, part) in parts.iter().enumerate() {
            let name = CString::new(part.as_os_str().as_bytes())
                .map_err(|_| AppError::new(ErrorCode::InvalidInput, "Invalid stored path."))?;
            parent = child_file(&parent, &name, index + 1 < parts.len())?;
        }
        let m = parent.metadata()?;
        if !m.is_file() {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "The original is not a regular file.",
            ));
        }
        #[cfg(target_os = "macos")]
        {
            use std::os::macos::fs::MetadataExt;
            if m.st_flags() & 0x40000000 != 0 {
                return Err(AppError::new(
                    ErrorCode::Unsupported,
                    "The original is an undownloaded cloud placeholder.",
                ));
            }
        }
        Ok(parent)
    }
    #[cfg(not(unix))]
    {
        let _ = (root, expected, volume);
        Err(AppError::new(
            ErrorCode::Unsupported,
            "Safe Windows root traversal requires its native gate.",
        ))
    }
}

pub fn unix_time(time: Option<std::time::SystemTime>) -> Option<String> {
    time.map(|time| {
        chrono::DateTime::<chrono::Utc>::from(time)
            .format("%Y-%m-%dT%H:%M:%S%.fZ")
            .to_string()
    })
}

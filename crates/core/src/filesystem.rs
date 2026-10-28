//! Native feasibility primitives, not a production path-authorisation engine.
//! Canonical paths alone cannot defend against ancestor replacement races.
use crate::domain::{AppError, ErrorCode, Result};
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileIdentity {
    pub volume: u64,
    pub file: u64,
    pub links: u64,
}

#[cfg(unix)]
pub fn identity(file: &File) -> Result<FileIdentity> {
    use std::os::unix::fs::MetadataExt;
    let metadata = file.metadata()?;
    Ok(FileIdentity {
        volume: metadata.dev(),
        file: metadata.ino(),
        links: metadata.nlink(),
    })
}

#[cfg(not(unix))]
pub fn identity(_file: &File) -> Result<FileIdentity> {
    Err(AppError::new(
        ErrorCode::Unsupported,
        "Native file identity has not passed the Windows gate.",
    ))
}

pub fn open_readonly(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    Ok(options.open(path)?)
}

/// Atomic same-volume publication on macOS. Never falls back to ordinary rename.
#[cfg(target_os = "macos")]
pub fn publish_noreplace(staged: &Path, destination: &Path) -> Result<()> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let from = CString::new(staged.as_os_str().as_bytes())
        .map_err(|_| AppError::new(ErrorCode::InvalidInput, "Invalid native path."))?;
    let to = CString::new(destination.as_os_str().as_bytes())
        .map_err(|_| AppError::new(ErrorCode::InvalidInput, "Invalid native path."))?;
    // SAFETY: owned C strings remain alive; RENAME_EXCL asks the kernel to refuse replacement.
    if unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) } == 0 {
        Ok(())
    } else {
        let cause = std::io::Error::last_os_error();
        let code = if cause.kind() == std::io::ErrorKind::AlreadyExists {
            ErrorCode::Conflict
        } else {
            ErrorCode::Io
        };
        Err(AppError::new(
            code,
            "Publication failed; existing destinations are preserved.",
        )
        .caused_by(cause))
    }
}

#[cfg(not(target_os = "macos"))]
pub fn publish_noreplace(_staged: &Path, _destination: &Path) -> Result<()> {
    Err(AppError::new(
        ErrorCode::Unsupported,
        "No-replace publication requires native platform verification.",
    ))
}

pub fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(AppError::new(
            ErrorCode::Unsupported,
            "Directory durability is unverified on this platform.",
        ))
    }
}

#[derive(Debug)]
pub struct VolumeProbe {
    pub available_bytes: u64,
    pub total_bytes: u64,
    pub native_name_max: Option<i64>,
    /// These properties must be probed on each selected volume, not inferred from the OS.
    pub case_sensitive: Option<bool>,
    pub independent_failure_domain: Option<bool>,
}

pub fn volume_probe(path: &Path) -> Result<VolumeProbe> {
    #[cfg(unix)]
    let name_max = {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let path = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| AppError::new(ErrorCode::InvalidInput, "Invalid native path."))?;
        // SAFETY: valid terminated path and defined pathconf query. -1 means unknown/error.
        let value = unsafe { libc::pathconf(path.as_ptr(), libc::_PC_NAME_MAX) };
        (value >= 0).then_some(value)
    };
    #[cfg(not(unix))]
    let name_max = None;
    Ok(VolumeProbe {
        available_bytes: fs2::available_space(path)?,
        total_bytes: fs2::total_space(path)?,
        native_name_max: name_max,
        case_sensitive: None,
        independent_failure_domain: None,
    })
}

pub fn exact_bytes(left: &mut File, right: &mut File) -> Result<bool> {
    use std::io::{Read, Seek, SeekFrom};
    left.seek(SeekFrom::Start(0))?;
    right.seek(SeekFrom::Start(0))?;
    if left.metadata()?.len() != right.metadata()?.len() {
        return Ok(false);
    }
    let mut a = [0u8; 65536];
    let mut b = [0u8; 65536];
    loop {
        let n = left.read(&mut a)?;
        if n == 0 {
            return Ok(right.read(&mut b[..1])? == 0);
        }
        right.read_exact(&mut b[..n])?;
        if a[..n] != b[..n] {
            return Ok(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    #[cfg(not(unix))]
    fn unverified_native_primitives_fail_closed_without_changing_files() {
        let data = tempfile::tempdir().unwrap();
        let source = data.path().join("source");
        let destination = data.path().join("destination");
        fs::write(&source, b"source preserved").unwrap();
        fs::write(&destination, b"existing destination preserved").unwrap();
        let handle = open_readonly(&source).unwrap();
        assert_eq!(identity(&handle).unwrap_err().code, ErrorCode::Unsupported);
        assert_eq!(
            publish_noreplace(&source, &destination).unwrap_err().code,
            ErrorCode::Unsupported
        );
        assert_eq!(
            sync_directory(data.path()).unwrap_err().code,
            ErrorCode::Unsupported
        );
        assert_eq!(fs::read(&source).unwrap(), b"source preserved");
        assert_eq!(
            fs::read(&destination).unwrap(),
            b"existing destination preserved"
        );
    }
    #[test]
    #[cfg(target_os = "macos")]
    fn late_arrival_cannot_be_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("stage");
        let target = temp.path().join("target");
        fs::write(&source, b"incoming").unwrap();
        fs::write(&target, b"unique existing").unwrap();
        assert_eq!(
            publish_noreplace(&source, &target).unwrap_err().code,
            ErrorCode::Conflict
        );
        assert_eq!(fs::read(&target).unwrap(), b"unique existing");
        assert_eq!(fs::read(&source).unwrap(), b"incoming");
    }

    #[test]
    #[cfg(unix)]
    fn identity_detects_hardlinks_and_replacement_without_lossy_paths() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("-quotes\"%\n港📷");
        let second = temp.path().join("link");
        fs::write(&first, b"original").unwrap();
        let handle = open_readonly(&first).unwrap();
        fs::hard_link(&first, &second).unwrap();
        assert_eq!(
            identity(&handle).unwrap(),
            identity(&File::open(&second).unwrap()).unwrap()
        );
        assert_eq!(identity(&handle).unwrap().links, 2);
        let probe = volume_probe(temp.path()).unwrap();
        assert!(probe.total_bytes >= probe.available_bytes);
        assert_eq!(probe.independent_failure_domain, None);
    }

    #[test]
    #[cfg(unix)]
    fn final_symlink_is_refused() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("file");
        let link = temp.path().join("link");
        fs::write(&file, b"preserve").unwrap();
        std::os::unix::fs::symlink(&file, &link).unwrap();
        assert!(open_readonly(&link).is_err());
        assert_eq!(fs::read(&file).unwrap(), b"preserve");
    }
}

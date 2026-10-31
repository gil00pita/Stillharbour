//! Windows local NTFS publication of an application-created SQLite snapshot.
//! Not an original-file operation or a replacement primitive.
use super::{DirectoryChain, FileIdentity, identity, windows};
use crate::domain::{AppError, ErrorCode, Result};
use std::{
    fs::{File, OpenOptions},
    mem::{offset_of, size_of},
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle},
    path::Path,
};
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::{
    Wdk::Storage::FileSystem::{
        FILE_RENAME_INFORMATION, FileRenameInformation, NtSetInformationFile,
    },
    Win32::{Foundation::RtlNtStatusToDosError, System::IO::IO_STATUS_BLOCK},
};

fn conflict(message: &str) -> AppError {
    AppError::new(ErrorCode::Conflict, message)
}

pub(crate) fn publish(staged: &Path, destination: &Path, expected: &FileIdentity) -> Result<File> {
    let parent = destination
        .parent()
        .ok_or_else(|| conflict("Snapshot requires a local parent."))?;
    if staged.parent() != Some(parent) {
        return Err(conflict(
            "Snapshot staging must use the destination directory.",
        ));
    }
    let parent_path = parent.canonicalize()?;
    let directory = DirectoryChain::open_for_owned_publication(&parent_path)?;
    let name = destination
        .file_name()
        .ok_or_else(|| conflict("Snapshot requires a filename."))?;
    let wide: Vec<u16> = name.encode_wide().collect();
    if wide.is_empty()
        || wide.len() > 255
        || wide
            .iter()
            .any(|c| matches!(*c, 0 | 47 | 92 | 58 | 34 | 42 | 60 | 62 | 63))
    {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid snapshot filename.",
        ));
    }
    // Close the tempfile writer first. Retain this no-delete-sharing handle
    // through publication and flushing; no original can be substituted by path.
    let source = windows::checked(
        OpenOptions::new()
            .access_mode(FILE_GENERIC_READ | FILE_GENERIC_WRITE | DELETE)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(
                FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_OPEN_NO_RECALL | FILE_FLAG_WRITE_THROUGH,
            )
            .open(
                parent_path.join(
                    staged
                        .file_name()
                        .ok_or_else(|| conflict("Invalid snapshot stage."))?,
                ),
            )?,
    )?;
    let actual = identity(&source)?;
    if !source.metadata()?.is_file()
        || actual.volume != expected.volume
        || actual.file != expected.file
        || actual.links != 1
        || actual.volume != directory.identity().volume
    {
        return Err(conflict(
            "The owned snapshot stage changed identity or link count.",
        ));
    }
    source.sync_all()?;
    let header = offset_of!(FILE_RENAME_INFORMATION, FileName);
    let length = header + (wide.len() + 1) * 2;
    let mut buffer = vec![
        0u64;
        length
            .div_ceil(8)
            .max(size_of::<FILE_RENAME_INFORMATION>().div_ceil(8))
    ];
    // SAFETY: SDK layout, 8-byte aligned buffer, bounded trailing UTF-16 name,
    // borrowed live source/parent handles. Zero ReplaceIfExists forbids clobber.
    let result = unsafe {
        let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
        (*info).RootDirectory = directory.leaf().as_raw_handle();
        (*info).FileNameLength = (wide.len() * 2) as u32;
        std::ptr::copy_nonoverlapping(wide.as_ptr(), (*info).FileName.as_mut_ptr(), wide.len());
        let mut io = IO_STATUS_BLOCK::default();
        NtSetInformationFile(
            source.as_raw_handle(),
            &mut io,
            info.cast(),
            length as u32,
            FileRenameInformation,
        )
    };
    if result != 0 {
        // SAFETY: status conversion has no pointer arguments or side effects.
        let error =
            std::io::Error::from_raw_os_error(unsafe { RtlNtStatusToDosError(result) } as i32);
        let code = if matches!(error.raw_os_error(), Some(80 | 183)) {
            ErrorCode::Conflict
        } else {
            ErrorCode::Io
        };
        return Err(AppError::new(
            code,
            "Snapshot publication failed; existing files are retained.",
        )
        .caused_by(error));
    }
    // Windows flushes file data and metadata through FlushFileBuffers. Do not
    // invent a generic directory-fsync implementation or a power-loss proof.
    source.sync_all()?;
    drop(source);
    let published = windows::child(directory.leaf(), name, false)?;
    let published_id = identity(&published)?;
    let current_parent = DirectoryChain::open(&parent_path)?;
    if published_id.volume != expected.volume
        || published_id.file != expected.file
        || published_id.links != 1
        || current_parent.identity() != directory.identity()
    {
        return Err(conflict(
            "Snapshot publication identity could not be revalidated.",
        ));
    }
    Ok(published)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owned_snapshot_publication_is_relative_flushed_and_noreplace() {
        let owned = tempfile::tempdir().unwrap();
        let destination = owned.path().join("retained.stillharbour");
        let stage = tempfile::NamedTempFile::new_in(owned.path()).unwrap();
        std::fs::write(stage.path(), b"owned snapshot").unwrap();
        let expected = identity(stage.as_file()).unwrap();
        let stage = stage.into_temp_path().keep().unwrap();
        let published = publish(&stage, &destination, &expected).unwrap();
        assert_eq!(identity(&published).unwrap(), expected);
        assert_eq!(std::fs::read(&destination).unwrap(), b"owned snapshot");
        assert!(!stage.exists());
        drop(published);
        let late = tempfile::NamedTempFile::new_in(owned.path()).unwrap();
        let expected = identity(late.as_file()).unwrap();
        let late = late.into_temp_path().keep().unwrap();
        assert_eq!(
            publish(&late, &destination, &expected).unwrap_err().code,
            ErrorCode::Conflict
        );
        assert_eq!(std::fs::read(&destination).unwrap(), b"owned snapshot");
        assert!(late.exists());
        let mut wrong = expected;
        wrong.file ^= 1;
        assert_eq!(
            publish(&late, &owned.path().join("wrong"), &wrong)
                .unwrap_err()
                .code,
            ErrorCode::Conflict
        );
        assert!(!owned.path().join("wrong").exists());
    }
}

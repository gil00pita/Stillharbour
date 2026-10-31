//! Owned-output primitives, currently reachable only through disposable executor
//! fixtures. No existing original/destination replacement or deletion API.
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem::{self, FileIdentity},
    organization::portable_filename,
};
use std::fs::File;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum EntryKind {
    File,
    Directory,
    Unsupported,
}
pub(super) struct Entry {
    pub object: super::Object,
    pub kind: EntryKind,
}
fn conflict(message: &str) -> AppError {
    AppError::new(ErrorCode::Conflict, message)
}
fn name(value: &str) -> Result<()> {
    if !portable_filename(value) {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid owned-output filename.",
        ));
    }
    Ok(())
}

/// Inspect only this held directory; never recurse into unknown entries.
pub(super) fn verify_entries(parent: &File, expected: &[(super::Object, EntryKind)]) -> Result<()> {
    let mut remaining: std::collections::BTreeMap<_, _> = expected
        .iter()
        .map(|(object, kind)| ((object.volume, object.file), *kind))
        .collect();
    if remaining.len() != expected.len() {
        return Err(conflict("Repeated prepared namespace object."));
    }
    let mut check = |leaf: &std::ffi::OsStr| -> Result<()> {
        let leaf = leaf
            .to_str()
            .ok_or_else(|| conflict("Unknown native name in prepared namespace."))?;
        let entry = lookup(parent, leaf)?
            .ok_or_else(|| conflict("A prepared namespace entry disappeared during inspection."))?;
        if remaining.remove(&(entry.object.volume, entry.object.file)) != Some(entry.kind)
            || entry.kind == EntryKind::Unsupported
        {
            return Err(conflict(
                "Unexpected, repeated or changed prepared namespace entry. Retain it for review.",
            ));
        }
        Ok(())
    };
    #[cfg(windows)]
    {
        let mut directory = filesystem::windows::Directory::new(parent)?;
        while let Some(entry) = directory.next()? {
            if entry.name != "." && entry.name != ".." {
                check(&entry.name)?;
            }
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        for entry in rustix::fs::Dir::read_from(parent).map_err(std::io::Error::from)? {
            let entry = entry.map_err(std::io::Error::from)?;
            if !matches!(entry.file_name().to_bytes(), b"." | b"..") {
                check(std::ffi::OsStr::from_bytes(entry.file_name().to_bytes()))?;
            }
        }
    }
    if !remaining.is_empty() {
        return Err(conflict("Prepared namespace inventory is incomplete."));
    }
    Ok(())
}

pub(super) fn lookup(parent: &File, leaf: &str) -> Result<Option<Entry>> {
    name(leaf)?;
    #[cfg(windows)]
    {
        use windows_sys::Win32::Storage::FileSystem::*;
        let volume = filesystem::identity(parent)?.volume;
        Ok(
            filesystem::windows::destination_entry(parent, std::ffi::OsStr::new(leaf))?.map(
                |entry| Entry {
                    object: super::Object {
                        volume,
                        file: entry.file_id,
                    },
                    kind: if entry.attributes
                        & (FILE_ATTRIBUTE_REPARSE_POINT
                            | FILE_ATTRIBUTE_OFFLINE
                            | FILE_ATTRIBUTE_RECALL_ON_OPEN
                            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
                        != 0
                    {
                        EntryKind::Unsupported
                    } else if entry.attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
                        EntryKind::Directory
                    } else {
                        EntryKind::File
                    },
                },
            ),
        )
    }
    #[cfg(unix)]
    {
        use rustix::fs::{AtFlags, FileType, statat};
        let stat = match statat(parent, leaf, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(stat) => stat,
            Err(rustix::io::Errno::NOENT) => return Ok(None),
            Err(error) => return Err(std::io::Error::from(error).into()),
        };
        #[cfg(target_os = "macos")]
        let volume = stat.st_dev as u64;
        #[cfg(not(target_os = "macos"))]
        let volume = stat.st_dev;
        let mut kind = match FileType::from_raw_mode(stat.st_mode) {
            FileType::Directory => EntryKind::Directory,
            FileType::RegularFile => EntryKind::File,
            _ => EntryKind::Unsupported,
        };
        #[cfg(target_os = "macos")]
        if stat.st_flags & 0x40000000 != 0 {
            kind = EntryKind::Unsupported;
        }
        Ok(Some(Entry {
            object: super::Object {
                volume,
                file: stat.st_ino,
            },
            kind,
        }))
    }
}

#[cfg(windows)]
fn open(parent: &File, leaf: &str, directory: bool, create: bool) -> Result<File> {
    use std::{
        mem::size_of,
        os::windows::{
            fs::MetadataExt,
            io::{AsRawHandle, FromRawHandle},
        },
    };
    use windows_sys::{
        Wdk::{Foundation::OBJECT_ATTRIBUTES, Storage::FileSystem as nt},
        Win32::{
            Foundation::{
                INVALID_HANDLE_VALUE, OBJ_CASE_INSENSITIVE, RtlNtStatusToDosError, UNICODE_STRING,
            },
            Storage::FileSystem::*,
            System::IO::IO_STATUS_BLOCK,
        },
    };
    name(leaf)?;
    let mut wide: Vec<u16> = leaf.encode_utf16().collect();
    let string = UNICODE_STRING {
        Length: (wide.len() * 2) as u16,
        MaximumLength: (wide.len() * 2) as u16,
        Buffer: wide.as_mut_ptr(),
    };
    let attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        RootDirectory: parent.as_raw_handle(),
        ObjectName: &string,
        Attributes: OBJ_CASE_INSENSITIVE,
        SecurityDescriptor: std::ptr::null(),
        SecurityQualityOfService: std::ptr::null(),
    };
    let mut handle = std::ptr::null_mut();
    let mut io = IO_STATUS_BLOCK::default();
    let access = if directory {
        FILE_LIST_DIRECTORY
            | FILE_TRAVERSE
            | FILE_READ_ATTRIBUTES
            | FILE_WRITE_ATTRIBUTES
            | SYNCHRONIZE
    } else {
        FILE_GENERIC_READ | FILE_GENERIC_WRITE | DELETE
    };
    // SAFETY: bounded owned UTF-16 component and SDK structures remain alive;
    // RootDirectory is borrowed, and FILE_CREATE refuses any existing object.
    let status = unsafe {
        nt::NtCreateFile(
            &mut handle,
            access,
            &attributes,
            &mut io,
            std::ptr::null(),
            if directory {
                FILE_ATTRIBUTE_DIRECTORY
            } else {
                FILE_ATTRIBUTE_NORMAL
            },
            if directory {
                FILE_SHARE_READ | FILE_SHARE_WRITE
            } else {
                FILE_SHARE_READ
            },
            if create {
                nt::FILE_CREATE
            } else {
                nt::FILE_OPEN
            },
            nt::FILE_SYNCHRONOUS_IO_NONALERT
                | nt::FILE_OPEN_REPARSE_POINT
                | if directory {
                    nt::FILE_DIRECTORY_FILE
                } else {
                    nt::FILE_NON_DIRECTORY_FILE | nt::FILE_OPEN_NO_RECALL
                },
            std::ptr::null(),
            0,
        )
    };
    if status != 0 {
        if status > 0 && !handle.is_null() && handle != INVALID_HANDLE_VALUE {
            // SAFETY: a synchronous successful informational return may still
            // transfer a fresh valid handle; retain no unverified capability.
            drop(unsafe { File::from_raw_handle(handle) });
        }
        // SAFETY: status conversion has no pointer arguments.
        let error =
            std::io::Error::from_raw_os_error(unsafe { RtlNtStatusToDosError(status) } as i32);
        return Err(AppError::new(
            if matches!(error.raw_os_error(), Some(80 | 183)) {
                ErrorCode::Conflict
            } else {
                ErrorCode::Io
            },
            "Owned output could not be opened; existing entries are retained.",
        )
        .caused_by(format!("NTSTATUS {status:#x}: {error}")));
    }
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(conflict("Native output creation returned no valid handle."));
    }
    // SAFETY: successful call transfers exactly one fresh handle.
    let file = unsafe { File::from_raw_handle(handle) };
    let metadata = file.metadata()?;
    if metadata.is_dir() != directory
        || metadata.file_attributes()
            & (FILE_ATTRIBUTE_REPARSE_POINT
                | FILE_ATTRIBUTE_OFFLINE
                | FILE_ATTRIBUTE_RECALL_ON_OPEN
                | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
            != 0
    {
        return Err(conflict("The owned output has an unsupported type."));
    }
    Ok(file)
}

pub(super) fn create_directory(parent: &File, leaf: &str) -> Result<File> {
    name(leaf)?;
    #[cfg(windows)]
    {
        open(parent, leaf, true, true)
    }
    #[cfg(unix)]
    {
        use rustix::fs::{Mode, OFlags, mkdirat, openat};
        mkdirat(parent, leaf, Mode::RUSR | Mode::WUSR | Mode::XUSR)
            .map_err(std::io::Error::from)?;
        parent.sync_all()?;
        Ok(File::from(
            openat(
                parent,
                leaf,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(std::io::Error::from)?,
        ))
    }
}
pub(super) fn create_file(parent: &File, leaf: &str) -> Result<File> {
    name(leaf)?;
    #[cfg(windows)]
    {
        open(parent, leaf, false, true)
    }
    #[cfg(unix)]
    {
        use rustix::fs::{Mode, OFlags, openat};
        let file = File::from(
            openat(
                parent,
                leaf,
                OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            )
            .map_err(std::io::Error::from)?,
        );
        parent.sync_all()?;
        Ok(file)
    }
}
pub(super) fn reopen_stage(parent: &File, leaf: &str, expected: &FileIdentity) -> Result<File> {
    name(leaf)?;
    #[cfg(windows)]
    let file = open(parent, leaf, false, false)?;
    #[cfg(unix)]
    let file = {
        use rustix::fs::{Mode, OFlags, openat};
        File::from(
            openat(
                parent,
                leaf,
                OFlags::RDWR | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(std::io::Error::from)?,
        )
    };
    if filesystem::identity(&file)? != *expected
        || expected.links != 1
        || !file.metadata()?.is_file()
    {
        return Err(conflict(
            "The owned stage changed identity, type or link count.",
        ));
    }
    Ok(file)
}
pub(super) fn read(parent: &File, leaf: &str) -> Result<File> {
    name(leaf)?;
    #[cfg(windows)]
    {
        filesystem::windows::child(parent, std::ffi::OsStr::new(leaf), false)
    }
    #[cfg(unix)]
    {
        let name =
            std::ffi::CString::new(leaf).map_err(|_| conflict("Invalid owned output name."))?;
        crate::library::native::child_file(parent, &name, false)
    }
}

pub(super) fn maybe_read(parent: &File, leaf: &str) -> Result<Option<File>> {
    name(leaf)?;
    #[cfg(windows)]
    {
        if filesystem::windows::destination_entry(parent, std::ffi::OsStr::new(leaf))?.is_none() {
            return Ok(None);
        }
    }
    #[cfg(unix)]
    {
        use rustix::fs::{AtFlags, statat};
        match statat(parent, leaf, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(_) => {}
            Err(rustix::io::Errno::NOENT) => return Ok(None),
            Err(error) => return Err(std::io::Error::from(error).into()),
        }
    }
    read(parent, leaf).map(Some)
}

pub(super) fn entry_matches(parent: &File, leaf: &str, expected: &FileIdentity) -> Result<bool> {
    name(leaf)?;
    #[cfg(windows)]
    {
        use windows_sys::Win32::Storage::FileSystem::*;
        let entry = filesystem::windows::destination_entry(parent, std::ffi::OsStr::new(leaf))?;
        Ok(entry.is_some_and(|entry| {
            entry.file_id == expected.file
                && entry.attributes
                    & (FILE_ATTRIBUTE_DIRECTORY
                        | FILE_ATTRIBUTE_REPARSE_POINT
                        | FILE_ATTRIBUTE_OFFLINE
                        | FILE_ATTRIBUTE_RECALL_ON_OPEN
                        | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
                    == 0
        }) && filesystem::identity(parent)?.volume == expected.volume)
    }
    #[cfg(unix)]
    {
        use rustix::fs::{AtFlags, FileType, statat};
        let stat = match statat(parent, leaf, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(stat) => stat,
            Err(rustix::io::Errno::NOENT) => return Ok(false),
            Err(error) => return Err(std::io::Error::from(error).into()),
        };
        #[cfg(target_os = "macos")]
        let volume = stat.st_dev as u64;
        #[cfg(not(target_os = "macos"))]
        let volume = stat.st_dev;
        Ok(volume == expected.volume
            && stat.st_ino == expected.file
            && FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile)
    }
}

pub(super) fn publish_new(
    parent: &File,
    stage: &str,
    file: File,
    target: &str,
    expected: &FileIdentity,
) -> Result<File> {
    name(stage)?;
    name(target)?;
    if filesystem::identity(&file)? != *expected || expected.links != 1 {
        return Err(conflict("The owned stage changed before publication."));
    }
    file.sync_all()?;
    #[cfg(windows)]
    {
        use std::{
            mem::{offset_of, size_of},
            os::windows::io::AsRawHandle,
        };
        use windows_sys::{
            Wdk::Storage::FileSystem::{
                FILE_RENAME_INFORMATION, FileRenameInformation, NtSetInformationFile,
            },
            Win32::{Foundation::RtlNtStatusToDosError, System::IO::IO_STATUS_BLOCK},
        };
        let wide: Vec<u16> = target.encode_utf16().collect();
        let header = offset_of!(FILE_RENAME_INFORMATION, FileName);
        let length = header + (wide.len() + 1) * 2;
        let mut buffer = vec![
            0u64;
            length
                .div_ceil(8)
                .max(size_of::<FILE_RENAME_INFORMATION>().div_ceil(8))
        ];
        // SAFETY: aligned SDK layout and bounded name; zero replacement flags
        // preserve every existing target. The source is the held owned handle.
        let status = unsafe {
            let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
            (*info).RootDirectory = parent.as_raw_handle();
            (*info).FileNameLength = (wide.len() * 2) as u32;
            std::ptr::copy_nonoverlapping(wide.as_ptr(), (*info).FileName.as_mut_ptr(), wide.len());
            let mut io = IO_STATUS_BLOCK::default();
            NtSetInformationFile(
                file.as_raw_handle(),
                &mut io,
                info.cast(),
                length as u32,
                FileRenameInformation,
            )
        };
        if status != 0 {
            // SAFETY: status conversion has no pointer arguments.
            let error =
                std::io::Error::from_raw_os_error(unsafe { RtlNtStatusToDosError(status) } as i32);
            return Err(AppError::new(
                if matches!(error.raw_os_error(), Some(80 | 183)) {
                    ErrorCode::Conflict
                } else {
                    ErrorCode::Io
                },
                "Owned publication failed; existing files are retained.",
            )
            .caused_by(error));
        }
        file.sync_all()?;
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::fd::AsRawFd;
        let from = std::ffi::CString::new(stage).map_err(|_| conflict("Invalid stage."))?;
        let to = std::ffi::CString::new(target).map_err(|_| conflict("Invalid target."))?;
        // SAFETY: live held directory and bounded names. RENAME_EXCL prevents
        // replacing any target; post-publication identity is checked below.
        if unsafe {
            libc::renameatx_np(
                parent.as_raw_fd(),
                from.as_ptr(),
                parent.as_raw_fd(),
                to.as_ptr(),
                libc::RENAME_EXCL,
            )
        } != 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        parent.sync_all()?;
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        return Err(AppError::new(
            ErrorCode::Unsupported,
            "Owned copy publication requires native target verification.",
        ));
    }
    drop(file);
    let published = read(parent, target)?;
    if filesystem::identity(&published)? != *expected {
        return Err(conflict(
            "Published output identity changed; inspect retained evidence.",
        ));
    }
    Ok(published)
}

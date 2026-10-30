//! Local NTFS read adapter. Directory enumeration and child opens use held
//! handles; names never become absolute execution paths. No mutation primitive.
use super::FileIdentity;
use crate::domain::{AppError, ErrorCode, Result};
use std::{
    ffi::{OsStr, OsString},
    fs::{File, OpenOptions},
    mem::{offset_of, size_of},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::{MetadataExt, OpenOptionsExt},
        io::{AsRawHandle, FromRawHandle},
    },
    path::Path,
};
use windows_sys::Win32::{
    Foundation::{ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE},
    Storage::FileSystem::*,
};

fn unsupported(message: &str) -> AppError {
    AppError::new(ErrorCode::Unsupported, message)
}
fn os_error() -> AppError {
    std::io::Error::last_os_error().into()
}

pub(super) fn checked(file: File) -> Result<File> {
    let attributes = file.metadata()?.file_attributes();
    if attributes
        & (FILE_ATTRIBUTE_REPARSE_POINT
            | FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        != 0
    {
        return Err(unsupported(
            "Links, reparse points and cloud placeholders are not read by the Windows scanner.",
        ));
    }
    Ok(file)
}

pub fn open_root(path: &Path) -> Result<File> {
    let file = checked(
        OpenOptions::new()
            .access_mode(FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | SYNCHRONIZE)
            // Refuse concurrent directory write/delete handles, including reparse
            // conversion handles. This is not a snapshot of child entries.
            .share_mode(FILE_SHARE_READ)
            .custom_flags(
                FILE_FLAG_BACKUP_SEMANTICS
                    | FILE_FLAG_OPEN_REPARSE_POINT
                    | FILE_FLAG_OPEN_NO_RECALL,
            )
            .open(path)?,
    )?;
    if !file.metadata()?.is_dir() {
        return Err(unsupported("Choose an ordinary local NTFS directory."));
    }
    identity(&file)?;
    Ok(file)
}

pub fn identity(file: &File) -> Result<FileIdentity> {
    let mut fs_name = [0u16; 32];
    let mut serial = 0;
    // SAFETY: handle is borrowed/live; all writable buffers have stated lengths.
    let result = unsafe {
        GetVolumeInformationByHandleW(
            file.as_raw_handle(),
            std::ptr::null_mut(),
            0,
            &mut serial,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            fs_name.as_mut_ptr(),
            fs_name.len() as u32,
        )
    };
    if result == 0 {
        return Err(os_error());
    }
    let end = fs_name
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(fs_name.len());
    if String::from_utf16_lossy(&fs_name[..end]) != "NTFS" {
        return Err(unsupported(
            "Windows scanning currently requires a local NTFS volume. This filesystem's identity has not been validated.",
        ));
    }
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: correctly sized SDK struct, live borrowed handle.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(os_error());
    }
    let id = (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
    if id == 0 || info.dwVolumeSerialNumber != serial {
        return Err(unsupported(
            "The filesystem did not return a consistent NTFS identity.",
        ));
    }
    Ok(FileIdentity {
        volume: u64::from(serial),
        file: id,
        links: u64::from(info.nNumberOfLinks),
    })
}

pub fn volume_token(file: &File) -> Result<String> {
    identity(file)?;
    let mut path = vec![0u16; 32768];
    // SAFETY: live borrowed handle and a bounded writable UTF-16 buffer.
    let n = unsafe {
        GetFinalPathNameByHandleW(
            file.as_raw_handle(),
            path.as_mut_ptr(),
            path.len() as u32,
            VOLUME_NAME_GUID,
        )
    } as usize;
    if n == 0 || n >= path.len() {
        return Err(unsupported(
            "A stable local volume GUID is required. Network volumes are not supported by this Windows scanner.",
        ));
    }
    let prefix: Vec<u16> = r"\\?\Volume{".encode_utf16().collect();
    let start = prefix.len();
    if !path[..n].starts_with(&prefix)
        || n < start + 38
        || path[start + 36] != u16::from(b'}')
        || path[start + 37] != u16::from(b'\\')
    {
        return Err(unsupported(
            "A local native volume GUID is required for scanning.",
        ));
    }
    // Only the GUID is text; an unrelated native path suffix may contain exact
    // non-Unicode UTF-16 and must not be lossy-converted or rejected here.
    let text = String::from_utf16(&path[start..start + 36])
        .map_err(|_| unsupported("Invalid native volume identity."))?;
    let guid =
        uuid::Uuid::parse_str(&text).map_err(|_| unsupported("Invalid native volume GUID."))?;
    Ok(format!("windows-ntfs-v1:{guid}"))
}

pub fn change_time(file: &File) -> Result<i64> {
    let mut info = FILE_BASIC_INFO::default();
    // SAFETY: SDK layout and exact length, borrowed/live file handle.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileBasicInfo,
            (&mut info as *mut FILE_BASIC_INFO).cast(),
            size_of::<FILE_BASIC_INFO>() as u32,
        )
    } == 0
    {
        return Err(os_error());
    }
    Ok(info.ChangeTime)
}

fn by_id(parent: &File, id: u64, directory: bool) -> Result<File> {
    let descriptor = FILE_ID_DESCRIPTOR {
        dwSize: size_of::<FILE_ID_DESCRIPTOR>() as u32,
        Type: FileIdType,
        Anonymous: FILE_ID_DESCRIPTOR_0 { FileId: id as i64 },
    };
    let access = if directory {
        FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | SYNCHRONIZE
    } else {
        FILE_GENERIC_READ
    };
    // SAFETY: a correctly sized SDK descriptor, live volume-hint handle and no
    // mutable buffers. OpenFileById opens exactly this NTFS object without a
    // pathname; OPEN_REPARSE_POINT refuses redirection to a reparse target.
    let handle = unsafe {
        OpenFileById(
            parent.as_raw_handle(),
            &descriptor,
            access,
            FILE_SHARE_READ,
            std::ptr::null(),
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_OPEN_NO_RECALL,
        )
    };
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        return Err(os_error());
    }
    // SAFETY: successful call transfers a fresh owned handle exactly once.
    let file = checked(unsafe { File::from_raw_handle(handle) })?;
    if file.metadata()?.is_dir() != directory || identity(&file)?.file != id {
        return Err(AppError::new(
            ErrorCode::Conflict,
            "The native entry changed identity or type.",
        ));
    }
    Ok(file)
}

fn entry_id(parent: &File, name: &OsStr) -> Result<u64> {
    let mut entries = Directory::new(parent)?;
    while let Some(entry) = entries.next()? {
        if entry.name == name {
            return Ok(entry.file_id);
        }
    }
    Err(AppError::new(
        ErrorCode::Conflict,
        "The entry is no longer in its recorded directory.",
    ))
}

pub fn child(parent: &File, name: &OsStr, directory: bool) -> Result<File> {
    let wide: Vec<u16> = name.encode_wide().collect();
    if wide.is_empty()
        || wide.len() > 255
        || wide.iter().any(|c| matches!(*c, 0 | 47 | 92 | 58))
        || name == OsStr::new(".")
        || name == OsStr::new("..")
    {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid native child component.",
        ));
    }
    // Bind the exact native name to its ID using the held parent's enumeration,
    // then check membership again after acquiring a no-delete file handle.
    // This deliberately favors preservation over large-directory lookup speed.
    let id = entry_id(parent, name)?;
    let file = by_id(parent, id, directory)?;
    if entry_id(parent, name)? != id {
        return Err(AppError::new(
            ErrorCode::Conflict,
            "Entry membership changed while opening.",
        ));
    }
    Ok(file)
}

pub struct Entry {
    pub name: OsString,
    pub attributes: u32,
    pub bytes: u64,
    pub file_id: u64,
}
pub struct Directory {
    file: File,
    buffer: Vec<u64>,
    offset: Option<usize>,
    started: bool,
    ended: bool,
}

impl Directory {
    pub fn new(file: &File) -> Result<Self> {
        Ok(Self {
            file: by_id(file, identity(file)?.file, true)?,
            buffer: vec![0; 8192],
            offset: None,
            started: false,
            ended: false,
        })
    }
    pub fn next(&mut self) -> Result<Option<Entry>> {
        if self.ended {
            return Ok(None);
        }
        if self.offset.is_none() {
            self.buffer.fill(0);
            let class = if self.started {
                FileIdBothDirectoryInfo
            } else {
                FileIdBothDirectoryRestartInfo
            };
            self.started = true;
            // SAFETY: buffer is 64 KiB, aligned to 8 as required by the SDK;
            // handle remains borrowed/live for this bounded enumeration request.
            if unsafe {
                GetFileInformationByHandleEx(
                    self.file.as_raw_handle(),
                    class,
                    self.buffer.as_mut_ptr().cast(),
                    (self.buffer.len() * 8) as u32,
                )
            } == 0
            {
                if std::io::Error::last_os_error().raw_os_error()
                    == Some(ERROR_NO_MORE_FILES as i32)
                {
                    self.ended = true;
                    return Ok(None);
                }
                self.ended = true;
                return Err(os_error());
            }
            self.offset = Some(0);
        }
        let offset = self.offset.take().expect("enumeration offset");
        let length = self.buffer.len() * 8;
        let header = offset_of!(FILE_ID_BOTH_DIR_INFO, FileName);
        if !offset.is_multiple_of(8) || offset + header > length {
            self.ended = true;
            return Err(unsupported("Invalid native directory response."));
        }
        // SAFETY: aligned offset and header bounds checked; read only the fixed
        // prefix through byte slices, then independently bound the variable name.
        let bytes =
            unsafe { std::slice::from_raw_parts(self.buffer.as_ptr().cast::<u8>(), length) };
        let u32_at = |at: usize| {
            u32::from_le_bytes(
                bytes[offset + at..offset + at + 4]
                    .try_into()
                    .expect("bounded header"),
            )
        };
        let next = u32_at(offset_of!(FILE_ID_BOTH_DIR_INFO, NextEntryOffset)) as usize;
        let name_bytes = u32_at(offset_of!(FILE_ID_BOTH_DIR_INFO, FileNameLength)) as usize;
        if name_bytes == 0
            || name_bytes > 510
            || !name_bytes.is_multiple_of(2)
            || offset + header + name_bytes > length
            || (next != 0
                && (next < header + name_bytes
                    || !next.is_multiple_of(8)
                    || offset + next >= length))
        {
            self.ended = true;
            return Err(unsupported("Invalid native directory name bounds."));
        }
        self.offset = (next != 0).then_some(offset + next);
        let name = bytes[offset + header..offset + header + name_bytes]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect::<Vec<_>>();
        let i64_at = |at: usize| {
            i64::from_le_bytes(
                bytes[offset + at..offset + at + 8]
                    .try_into()
                    .expect("bounded header"),
            )
        };
        let size = i64_at(offset_of!(FILE_ID_BOTH_DIR_INFO, EndOfFile));
        if size < 0 {
            self.ended = true;
            return Err(unsupported("Invalid native file size."));
        }
        Ok(Some(Entry {
            name: OsString::from_wide(&name),
            attributes: u32_at(offset_of!(FILE_ID_BOTH_DIR_INFO, FileAttributes)),
            bytes: size as u64,
            file_id: i64_at(offset_of!(FILE_ID_BOTH_DIR_INFO, FileId)) as u64,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn volume_guid_does_not_lossily_decode_native_root_names() {
        let fixture = tempfile::tempdir().unwrap();
        let root_path = fixture
            .path()
            .join(OsString::from_wide(&[0x61, 0xd800, 0x62]));
        std::fs::create_dir(&root_path).unwrap();
        let root = open_root(&root_path).unwrap();
        assert!(volume_token(&root).unwrap().starts_with("windows-ntfs-v1:"));
    }
    #[test]
    fn native_child_opens_are_relative_and_reject_escape_components() {
        let fixture = tempfile::tempdir().unwrap();
        std::fs::create_dir(fixture.path().join("nested")).unwrap();
        std::fs::write(fixture.path().join("nested/photo.png"), b"preserved").unwrap();
        let root = open_root(fixture.path()).unwrap();
        let result = child(&root, OsStr::new("nested"), true);
        assert!(result.is_ok(), "{result:?}");
        let nested = result.unwrap();
        assert!(child(&nested, OsStr::new("photo.png"), false).is_ok());
        for name in ["..", ".", "a/b", "a\\b", "C:", "file:stream"] {
            assert_eq!(
                child(&root, OsStr::new(name), false).unwrap_err().code,
                ErrorCode::InvalidInput
            );
        }
    }

    #[test]
    fn native_directory_batches_preserve_names_ids_and_lock_parent_mutation() {
        use std::collections::BTreeSet;
        let fixture = tempfile::tempdir().unwrap();
        let mut expected = BTreeSet::new();
        for index in 0..600 {
            let name = format!("photo-{index:04}-{}.png", "x".repeat(60));
            std::fs::write(fixture.path().join(&name), b"preserved").unwrap();
            expected.insert(OsString::from(name));
        }
        let unusual = OsString::from_wide(&[0x61, 0xd800, 0x62]);
        std::fs::write(fixture.path().join(&unusual), b"native UTF-16").unwrap();
        expected.insert(unusual.clone());
        let root = open_root(fixture.path()).unwrap();
        assert!(volume_token(&root).unwrap().starts_with("windows-ntfs-v1:"));
        assert!(
            OpenOptions::new()
                .access_mode(FILE_GENERIC_WRITE)
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
                .open(fixture.path())
                .is_err()
        );
        let mut directory = Directory::new(&root).unwrap();
        let mut actual = BTreeSet::new();
        while let Some(entry) = directory.next().unwrap() {
            if entry.name == "." || entry.name == ".." {
                continue;
            }
            let file = child(&root, &entry.name, false).unwrap();
            assert_eq!(identity(&file).unwrap().file, entry.file_id);
            assert!(actual.insert(entry.name));
        }
        assert_eq!(actual, expected);
        assert!(directory.next().unwrap().is_none());
        assert!(child(&root, &unusual, false).is_ok());
    }

    #[test]
    fn native_asset_open_refuses_junction_escape_and_wrong_volume() {
        let fixture = tempfile::tempdir().unwrap();
        let source = fixture.path().join("source");
        let target = fixture.path().join("outside");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("photo.png"), b"outside preserved").unwrap();
        let junction = source.join("escape");
        assert!(std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:STILLHARBOUR_TEST_JUNCTION -Target $env:STILLHARBOUR_TEST_TARGET | Out-Null"])
            .env("STILLHARBOUR_TEST_JUNCTION", &junction).env("STILLHARBOUR_TEST_TARGET", &target).status().unwrap().success());
        let root = open_root(&source).unwrap();
        let id = identity(&root).unwrap();
        let volume = volume_token(&root).unwrap();
        assert!(child(&root, OsStr::new("escape"), true).is_err());
        assert!(
            crate::library::native::open_asset(
                &source,
                &id,
                &volume,
                Path::new("escape/photo.png")
            )
            .is_err()
        );
        assert_eq!(
            crate::library::native::open_asset(
                &source,
                &id,
                "wrong-volume",
                Path::new("photo.png")
            )
            .unwrap_err()
            .code,
            ErrorCode::Conflict
        );
        assert_eq!(
            std::fs::read(target.join("photo.png")).unwrap(),
            b"outside preserved"
        );
    }
}

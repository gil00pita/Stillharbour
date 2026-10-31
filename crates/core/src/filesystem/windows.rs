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
use windows_sys::Wdk::Storage::FileSystem::{
    FILE_ID_BOTH_DIR_INFORMATION, FileIdBothDirectoryInformation, NtQueryDirectoryFile,
};
use windows_sys::Win32::{
    Foundation::{
        ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE, RtlNtStatusToDosError, STATUS_NO_MORE_FILES,
        STATUS_NO_SUCH_FILE, UNICODE_STRING,
    },
    Storage::FileSystem::*,
    System::IO::IO_STATUS_BLOCK,
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
    by_id_shared(parent, id, directory, FILE_SHARE_READ)
}

fn by_id_shared(parent: &File, id: u64, directory: bool, sharing: u32) -> Result<File> {
    by_id_access(parent, id, directory, sharing, false)
}

fn by_id_access(
    parent: &File,
    id: u64,
    directory: bool,
    sharing: u32,
    attributes_only: bool,
) -> Result<File> {
    let descriptor = FILE_ID_DESCRIPTOR {
        dwSize: size_of::<FILE_ID_DESCRIPTOR>() as u32,
        Type: FileIdType,
        Anonymous: FILE_ID_DESCRIPTOR_0 { FileId: id as i64 },
    };
    let access = if attributes_only {
        FILE_READ_ATTRIBUTES | SYNCHRONIZE
    } else if directory {
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
            sharing,
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
    // Native-only names can contain DOS search metacharacters. Enumerate those
    // literally rather than interpreting untrusted names as search expressions.
    let wide: Vec<u16> = name.encode_wide().collect();
    if wide.iter().any(|c| matches!(*c, 34 | 42 | 60 | 62 | 63)) {
        return enumerated_entry_id(parent, name);
    }
    let entry = destination_entry(parent, name)?.ok_or_else(|| {
        AppError::new(
            ErrorCode::Conflict,
            "The entry is no longer in its recorded directory.",
        )
    })?;
    exact_entry_id(entry, name)
}

/// Native lookup for an already validated portable target component. A returned
/// case/short-name alias is an occupied name, never authority to open that alias.
pub(crate) fn destination_entry(parent: &File, name: &OsStr) -> Result<Option<Entry>> {
    validate_component(name)?;
    let mut wide: Vec<u16> = name.encode_wide().collect();
    if wide.iter().any(|c| matches!(*c, 34 | 42 | 60 | 62 | 63)) {
        return Err(unsupported(
            "Target lookup does not accept search expressions.",
        ));
    }
    // A fresh handle is essential: Windows captures the search expression on
    // its first query and ignores later FileName arguments on that handle.
    let directory = by_id(parent, identity(parent)?.file, true)?;
    let query = UNICODE_STRING {
        Length: (wide.len() * 2) as u16,
        MaximumLength: (wide.len() * 2) as u16,
        Buffer: wide.as_mut_ptr(),
    };
    let mut buffer = [0u64; 128]; // aligned, enough for one 255-unit NTFS name
    let mut io = IO_STATUS_BLOCK::default();
    // SAFETY: synchronous owned directory handle; no event/APC; all input and
    // output buffers remain live with exact bounded lengths through completion.
    let status = unsafe {
        NtQueryDirectoryFile(
            directory.as_raw_handle(),
            std::ptr::null_mut(),
            None,
            std::ptr::null(),
            &mut io,
            buffer.as_mut_ptr().cast(),
            size_of_val(&buffer) as u32,
            FileIdBothDirectoryInformation,
            true,
            &query,
            true,
        )
    };
    if status != 0 {
        if matches!(status, STATUS_NO_SUCH_FILE | STATUS_NO_MORE_FILES) {
            return Ok(None);
        }
        // NTSTATUS is not GetLastError; retain the actual native failure.
        return Err(std::io::Error::from_raw_os_error(unsafe {
            RtlNtStatusToDosError(status) as i32
        })
        .into());
    }
    parse_lookup_entry(&buffer, io.Information).map(Some)
}

#[cfg(test)]
fn lookup_id(buffer: &[u64], length: usize, name: &OsStr) -> Result<u64> {
    exact_entry_id(parse_lookup_entry(buffer, length)?, name)
}

fn exact_entry_id(entry: Entry, name: &OsStr) -> Result<u64> {
    if entry.name != name {
        return Err(AppError::new(
            ErrorCode::Conflict,
            "Native entry name changed during lookup.",
        ));
    }
    Ok(entry.file_id)
}

fn parse_lookup_entry(buffer: &[u64], length: usize) -> Result<Entry> {
    let header = offset_of!(FILE_ID_BOTH_DIR_INFORMATION, FileName);
    if length < header || length > size_of_val(buffer) {
        return Err(unsupported("Invalid native lookup response."));
    }
    // SAFETY: buffer is initialized and the returned byte length is bounded.
    let bytes = unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), length) };
    let at = offset_of!(FILE_ID_BOTH_DIR_INFORMATION, FileNameLength);
    let name_bytes = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    if name_bytes == 0
        || name_bytes > 510
        || !name_bytes.is_multiple_of(2)
        || header + name_bytes > length
        || bytes[..4] != [0, 0, 0, 0]
    {
        return Err(unsupported("Invalid native lookup name bounds."));
    }
    let returned = bytes[header..header + name_bytes]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect::<Vec<_>>();
    let at = offset_of!(FILE_ID_BOTH_DIR_INFORMATION, FileId);
    let id = u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
    if id == 0 {
        return Err(unsupported("Invalid native lookup identity."));
    }
    let at = offset_of!(FILE_ID_BOTH_DIR_INFORMATION, FileAttributes);
    let attributes = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let at = offset_of!(FILE_ID_BOTH_DIR_INFORMATION, EndOfFile);
    let size = i64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
    if size < 0 {
        return Err(unsupported("Invalid native lookup file size."));
    }
    Ok(Entry {
        name: OsString::from_wide(&returned),
        file_id: id,
        attributes,
        bytes: size as u64,
    })
}

fn enumerated_entry_id(parent: &File, name: &OsStr) -> Result<u64> {
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
    validate_component(name)?;
    // Bind the exact native name to its ID using the held parent's enumeration,
    // then check membership again after acquiring a no-delete file handle.
    // Exact-name queries avoid re-enumerating every sibling for ordinary names.
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

pub(crate) fn metadata_child(parent: &File, name: &OsStr) -> Result<File> {
    validate_component(name)?;
    let id = entry_id(parent, name)?;
    let file = by_id_access(parent, id, false, FILE_SHARE_READ, true)?;
    if entry_id(parent, name)? != id {
        return Err(AppError::new(
            ErrorCode::Conflict,
            "Entry membership changed during metadata review.",
        ));
    }
    Ok(file)
}

// Only an app-owned publication namespace may admit directory writers. Keep
// delete sharing refused so the directory/ancestor bindings cannot be renamed.
pub(crate) fn owned_directory_child(parent: &File, name: &OsStr) -> Result<File> {
    validate_component(name)?;
    let id = entry_id(parent, name)?;
    let file = by_id_shared(parent, id, true, FILE_SHARE_READ | FILE_SHARE_WRITE)?;
    if entry_id(parent, name)? != id {
        return Err(AppError::new(
            ErrorCode::Conflict,
            "Owned directory membership changed while opening.",
        ));
    }
    Ok(file)
}

fn validate_component(name: &OsStr) -> Result<()> {
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
    Ok(())
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
    fn metadata_only_children_cannot_read_source_contents() {
        use std::io::Read;
        let fixture = tempfile::tempdir().unwrap();
        std::fs::write(fixture.path().join("Photo.xmp"), b"preserved metadata").unwrap();
        let root = open_root(fixture.path()).unwrap();
        let mut file = metadata_child(&root, OsStr::new("Photo.xmp")).unwrap();
        assert_eq!(file.metadata().unwrap().len(), 18);
        assert!(file.read(&mut [0u8; 1]).is_err());
        assert!(metadata_child(&root, OsStr::new("PHOTO.XMP")).is_err());
        drop(file);
        assert_eq!(
            std::fs::read(fixture.path().join("Photo.xmp")).unwrap(),
            b"preserved metadata"
        );
    }

    #[test]
    fn exact_lookup_is_fresh_literal_and_preserves_locked_sources() {
        let fixture = tempfile::tempdir().unwrap();
        for name in ["long-photo-name.png", "other.png", "photo😀.png"] {
            std::fs::write(fixture.path().join(name), name.as_bytes()).unwrap();
        }
        let unusual = OsString::from_wide(&[0x61, 0xd800, 0x62]);
        std::fs::write(fixture.path().join(&unusual), b"exact UTF-16").unwrap();
        let root = open_root(fixture.path()).unwrap();
        for name in [
            OsStr::new("other.png"),
            OsStr::new("long-photo-name.png"),
            &unusual,
            OsStr::new("photo😀.png"),
        ] {
            let id = entry_id(&root, name).unwrap();
            assert_eq!(id, enumerated_entry_id(&root, name).unwrap());
            let file = child(&root, name, false).unwrap();
            assert_eq!(identity(&file).unwrap().file, id);
            assert!(
                std::fs::rename(fixture.path().join(name), fixture.path().join("moved")).is_err()
            );
        }
        // Native search may match case/short aliases; neither is an exact name.
        assert_eq!(
            entry_id(&root, OsStr::new("OTHER.PNG")).unwrap_err().code,
            ErrorCode::Conflict
        );
        assert!(entry_id(&root, OsStr::new("*.png")).is_err());
        assert!(entry_id(&root, OsStr::new("other?.png")).is_err());
        assert_eq!(
            entry_id(&root, OsStr::new("missing.png")).unwrap_err().code,
            ErrorCode::Conflict
        );
        let old = entry_id(&root, OsStr::new("other.png")).unwrap();
        // Keep the removed object alive so NTFS cannot reuse its ID in this test.
        let old_handle = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .open(fixture.path().join("other.png"))
            .unwrap();
        std::fs::remove_file(fixture.path().join("other.png")).unwrap();
        assert!(entry_id(&root, OsStr::new("other.png")).is_err());
        std::fs::write(fixture.path().join("other.png"), b"replacement").unwrap();
        assert_ne!(entry_id(&root, OsStr::new("other.png")).unwrap(), old);
        drop(old_handle);
        assert_eq!(
            std::fs::read(fixture.path().join("long-photo-name.png")).unwrap(),
            b"long-photo-name.png"
        );
    }

    #[test]
    fn exact_lookup_rejects_malformed_native_responses() {
        let mut buffer = [0u64; 128];
        let header = offset_of!(FILE_ID_BOTH_DIR_INFORMATION, FileName);
        for length in [0, header - 1, 1025] {
            assert!(lookup_id(&buffer, length, OsStr::new("a")).is_err());
        }
        let bytes =
            unsafe { std::slice::from_raw_parts_mut(buffer.as_mut_ptr().cast::<u8>(), 1024) };
        let at = offset_of!(FILE_ID_BOTH_DIR_INFORMATION, FileNameLength);
        bytes[at..at + 4].copy_from_slice(&2u32.to_le_bytes());
        bytes[header..header + 2].copy_from_slice(&u16::from(b'a').to_le_bytes());
        let at = offset_of!(FILE_ID_BOTH_DIR_INFORMATION, FileId);
        bytes[at..at + 8].copy_from_slice(&7u64.to_le_bytes());
        assert_eq!(lookup_id(&buffer, header + 2, OsStr::new("a")).unwrap(), 7);
        assert!(lookup_id(&buffer, header + 1, OsStr::new("a")).is_err());
        assert!(lookup_id(&buffer, header + 2, OsStr::new("b")).is_err());
        for invalid in [0u32, 1, 512, u32::MAX] {
            let mut malformed = buffer;
            let bytes = unsafe {
                std::slice::from_raw_parts_mut(malformed.as_mut_ptr().cast::<u8>(), 1024)
            };
            let at = offset_of!(FILE_ID_BOTH_DIR_INFORMATION, FileNameLength);
            bytes[at..at + 4].copy_from_slice(&invalid.to_le_bytes());
            assert!(lookup_id(&malformed, 1024, OsStr::new("a")).is_err());
        }
        let mut zero_id = buffer;
        zero_id[offset_of!(FILE_ID_BOTH_DIR_INFORMATION, FileId) / 8] = 0;
        assert!(lookup_id(&zero_id, header + 2, OsStr::new("a")).is_err());
        buffer[0] = 8;
        assert!(lookup_id(&buffer, header + 2, OsStr::new("a")).is_err());
    }

    #[test]
    #[ignore = "owned flat-directory lookup comparison; run explicitly without concurrent builds"]
    fn flat_directory_lookup_comparison() {
        use std::time::Instant;
        for count in [256, 2048] {
            let fixture = tempfile::tempdir().unwrap();
            for index in 0..count {
                std::fs::write(
                    fixture.path().join(format!("photo-{index:05}.png")),
                    b"preserved",
                )
                .unwrap();
            }
            let root = open_root(fixture.path()).unwrap();
            let names: Vec<_> = (0..128)
                .map(|i| OsString::from(format!("photo-{:05}.png", i * (count - 1) / 127)))
                .collect();
            let start = Instant::now();
            let baseline: Vec<_> = names
                .iter()
                .map(|n| enumerated_entry_id(&root, n).unwrap())
                .collect();
            let old = start.elapsed();
            let start = Instant::now();
            let exact: Vec<_> = names.iter().map(|n| entry_id(&root, n).unwrap()).collect();
            let new = start.elapsed();
            assert_eq!(baseline, exact);
            for name in &names {
                assert_eq!(
                    std::fs::read(fixture.path().join(name)).unwrap(),
                    b"preserved"
                );
            }
            println!(
                "entries={count} lookups=128 enumerated_us={} exact_us={} (warm lookup only, not end-to-end scan)",
                old.as_micros(),
                new.as_micros()
            );
        }
    }
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

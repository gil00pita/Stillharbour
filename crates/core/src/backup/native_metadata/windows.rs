//! Win32 handle metadata. References: GetSecurityInfo, FILE_STREAM_INFO and
//! FILE_BASIC_INFO; only owner/group/DACL and descriptive times are policy v1.
use super::*;
use std::{
    mem::size_of,
    os::windows::io::{AsRawHandle, FromRawHandle},
};
use windows_sys::{
    Wdk::Storage::FileSystem as nt,
    Win32::{
        Foundation::*,
        Security::{Authorization::*, *},
        Storage::FileSystem::*,
        System::{IO::IO_STATUS_BLOCK, SystemServices::ACCESS_SYSTEM_SECURITY},
    },
};

const ATTRIBUTES: u32 = FILE_ATTRIBUTE_READONLY
    | FILE_ATTRIBUTE_HIDDEN
    | FILE_ATTRIBUTE_SYSTEM
    | FILE_ATTRIBUTE_ARCHIVE
    | FILE_ATTRIBUTE_NORMAL
    | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED;
struct Descriptor(PSECURITY_DESCRIPTOR);
impl Drop for Descriptor {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: GetSecurityInfo transfers this LocalAlloc allocation.
            unsafe {
                LocalFree(self.0);
            }
        }
    }
}
fn error() -> AppError {
    AppError::new(
        ErrorCode::Io,
        "Native essential metadata inspection/restoration failed; no original is changed.",
    )
    .caused_by(std::io::Error::last_os_error())
}
pub(super) fn difference(a: &[u8], b: &[u8]) -> String {
    fn component(bytes: &[u8], field: usize, sid: bool) -> Option<&[u8]> {
        let offset = u32::from_le_bytes(bytes.get(field..field + 4)?.try_into().ok()?) as usize;
        if offset == 0 {
            return Some(&[]);
        }
        let header = bytes.get(offset..offset + 8)?;
        let length = if sid {
            8 + usize::from(header[1]) * 4
        } else {
            u16::from_le_bytes([header[2], header[3]]) as usize
        };
        bytes.get(offset..offset + length)
    }
    format!(
        "security_control={},owner={},group={},dacl={}",
        a.get(2..4) != b.get(2..4),
        component(a, 4, true) != component(b, 4, true),
        component(a, 8, true) != component(b, 8, true),
        component(a, 16, false) != component(b, 16, false)
    )
}
pub(super) fn label_difference(a: &[u8], b: &[u8]) -> String {
    fn label(bytes: &[u8]) -> (&'static str, bool) {
        let Some(offset) = bytes
            .get(12..16)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_le_bytes)
            .map(|n| n as usize)
        else {
            return ("invalid", false);
        };
        if offset == 0 {
            return ("absent", false);
        }
        let Some(header) = bytes.get(offset..offset + 8) else {
            return ("invalid", true);
        };
        if u16::from_le_bytes([header[4], header[5]]) == 0 {
            return ("empty", true);
        }
        let Some(rid) = bytes
            .get(offset + 24..offset + 28)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_le_bytes)
        else {
            return ("invalid", true);
        };
        (
            match rid {
                0 => "untrusted",
                4096 => "low",
                8192 => "medium",
                8448 => "medium_plus",
                12288 => "high",
                16384 => "system",
                20480 => "protected",
                _ => "other",
            },
            true,
        )
    }
    let (expected, ep) = label(a);
    let (actual, ap) = label(b);
    format!(
        "mandatory_label_expected={expected},actual={actual},presence_differs={},descriptor_control_differs={}",
        ep != ap,
        a.get(2..4) != b.get(2..4)
    )
}
fn no_extra(file: &File) -> Result<()> {
    let mut ea = nt::FILE_EA_INFORMATION::default();
    let mut status = IO_STATUS_BLOCK::default();
    // SAFETY: borrowed file handle, initialized fixed SDK buffers and exact sizes.
    if unsafe {
        nt::NtQueryInformationFile(
            file.as_raw_handle(),
            &mut status,
            (&mut ea as *mut nt::FILE_EA_INFORMATION).cast(),
            size_of::<nt::FILE_EA_INFORMATION>() as u32,
            nt::FileEaInformation,
        )
    } != 0
    {
        return Err(unsupported(
            "This filesystem cannot prove absence of Windows extended attributes.",
        ));
    }
    if ea.EaSize != 0 {
        return Err(unsupported(
            "Windows extended attributes require a full-fidelity adapter; replacement is unavailable.",
        ));
    }
    let mut buffer = vec![0u64; 8192];
    // SAFETY: 64 KiB writable buffer has required 8-byte alignment.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileStreamInfo,
            buffer.as_mut_ptr().cast(),
            65536,
        )
    } == 0
    {
        if std::io::Error::last_os_error().raw_os_error() == Some(ERROR_HANDLE_EOF as i32)
            && file.metadata()?.len() == 0
        {
            return Ok(());
        }
        return Err(unsupported(
            "This filesystem cannot enumerate every native data stream; replacement is unavailable.",
        ));
    }
    // SAFETY: view of our live initialized, correctly aligned allocation.
    let bytes = unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), 65536) };
    let mut offset = 0usize;
    let mut unnamed = false;
    loop {
        let header = bytes
            .get(offset..offset + 24)
            .ok_or_else(|| invalid("Malformed native stream enumeration."))?;
        let next = u32::from_le_bytes(
            header[0..4]
                .try_into()
                .map_err(|_| invalid("Invalid native stream offset."))?,
        ) as usize;
        let count = u32::from_le_bytes(
            header[4..8]
                .try_into()
                .map_err(|_| invalid("Invalid native stream name size."))?,
        ) as usize;
        let name = bytes
            .get(offset + 24..offset + 24 + count)
            .filter(|_| count.is_multiple_of(2) && count <= 65500)
            .ok_or_else(|| invalid("Invalid native stream name."))?;
        let expected = "::$DATA"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        if name != expected || unnamed {
            return Err(unsupported(
                "A named or repeated Windows data stream is present. It must be preserved by a separate full-fidelity adapter; replacement is unavailable.",
            ));
        }
        unnamed = true;
        if next == 0 {
            break;
        }
        if next < 24 + count || !next.is_multiple_of(8) {
            return Err(invalid("Invalid native stream linkage."));
        }
        offset = offset
            .checked_add(next)
            .filter(|o| *o < 65536)
            .ok_or_else(|| invalid("Native stream enumeration exceeds its bound."))?;
    }
    if !unnamed {
        return Err(unsupported(
            "The ordinary native data stream was not proven.",
        ));
    }
    Ok(())
}
pub(super) fn validate_descriptor(bytes: &[u8]) -> Result<()> {
    if bytes.len() < 20
        || bytes.len() > 65536
        || bytes[0] != 1
        || u16::from_le_bytes([bytes[2], bytes[3]]) & 0x8000 == 0
    {
        return Err(invalid(
            "Invalid self-relative Windows security descriptor.",
        ));
    }
    let control = u16::from_le_bytes([bytes[2], bytes[3]]);
    if bytes[1] != 0 || control & 0x0004 == 0 || control & !0x950f != 0 {
        return Err(unsupported(
            "This security descriptor uses unsupported control/audit metadata.",
        ));
    }
    for (field, sid) in [(4, true), (8, true), (12, false), (16, false)] {
        let offset = u32::from_le_bytes(
            bytes[field..field + 4]
                .try_into()
                .map_err(|_| invalid("Invalid security descriptor offset."))?,
        ) as usize;
        if field == 12 && offset != 0 {
            return Err(unsupported(
                "System audit ACL recovery is outside this metadata policy.",
            ));
        }
        if offset == 0 {
            if sid {
                return Err(invalid(
                    "Essential Windows ownership identifiers are missing.",
                ));
            }
            continue;
        }
        if offset < 20 || !offset.is_multiple_of(4) {
            return Err(invalid("Invalid relative security descriptor alignment."));
        }
        let header = bytes
            .get(offset..offset + 8)
            .ok_or_else(|| invalid("Security descriptor component exceeds its bound."))?;
        if sid {
            if header[0] != 1
                || header[1] > 15
                || offset + 8 + usize::from(header[1]) * 4 > bytes.len()
            {
                return Err(invalid("Invalid bounded Windows security identifier."));
            }
        } else {
            let length = u16::from_le_bytes([header[2], header[3]]) as usize;
            let count = u16::from_le_bytes([header[4], header[5]]) as usize;
            if !matches!(header[0], 2 | 4) || length < 8 || offset + length > bytes.len() {
                return Err(invalid("Invalid bounded Windows ACL."));
            }
            let mut cursor = offset + 8;
            for _ in 0..count {
                let ace = bytes
                    .get(cursor..cursor + 4)
                    .ok_or_else(|| invalid("Invalid ACL entry header."))?;
                let size = u16::from_le_bytes([ace[2], ace[3]]) as usize;
                if size < 4 || !size.is_multiple_of(4) || cursor + size > offset + length {
                    return Err(invalid("Invalid bounded Windows ACL entry."));
                }
                // Only ordinary allow/deny ACEs are supported in policy v1.
                // Object/callback/conditional ACEs require another adapter.
                // Validate their complete SID extent before passing imported
                // bytes to any Win32 descriptor API.
                if !matches!(ace[0], 0 | 1) {
                    return Err(unsupported(
                        "Object, callback or other advanced Windows ACL entries require another fidelity adapter.",
                    ));
                }
                if size < 16 {
                    return Err(invalid("Truncated ordinary Windows ACL entry."));
                }
                let sid = bytes
                    .get(cursor + 8..cursor + 16)
                    .ok_or_else(|| invalid("Truncated ACL security identifier."))?;
                if sid[0] != 1 || sid[1] > 15 || 16 + usize::from(sid[1]) * 4 != size {
                    return Err(invalid("Invalid bounded ACL security identifier."));
                }
                cursor += size;
            }
        }
    }
    Ok(())
}
fn descriptor(file: &File, information: u32) -> Result<Vec<u8>> {
    let mut descriptor = Descriptor(std::ptr::null_mut());
    // SAFETY: live handle and owned output descriptor pointer; all optional
    // component outputs are null. No privilege is enabled or changed.
    let status = unsafe {
        GetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            information,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut descriptor.0,
        )
    };
    if status != 0 || descriptor.0.is_null() {
        return Err(AppError::new(
            ErrorCode::Unsupported,
            "Requested native security metadata is unavailable under the current read rights.",
        )
        .caused_by(std::io::Error::from_raw_os_error(status as i32)));
    }
    // SAFETY: system-owned descriptor was returned successfully above.
    let length = unsafe { GetSecurityDescriptorLength(descriptor.0) } as usize;
    if !(20..=65536).contains(&length) {
        return Err(invalid("Native security descriptor exceeds its bound."));
    }
    // SAFETY: documented allocation length, copied before LocalFree.
    Ok(unsafe { std::slice::from_raw_parts(descriptor.0.cast::<u8>(), length) }.to_vec())
}
pub(super) fn validate_sacl(bytes: &[u8]) -> Result<()> {
    if bytes.len() < 20
        || bytes.len() > 65536
        || bytes[0] != 1
        || u16::from_le_bytes([bytes[2], bytes[3]]) & 0x8000 == 0
        || bytes[4..12].iter().any(|b| *b != 0)
        || bytes[16..20].iter().any(|b| *b != 0)
    {
        return Err(invalid(
            "Invalid bounded mandatory-label/audit absence descriptor.",
        ));
    }
    let offset = u32::from_le_bytes(
        bytes[12..16]
            .try_into()
            .map_err(|_| invalid("Invalid native label offset."))?,
    ) as usize;
    if offset == 0 {
        return Ok(());
    }
    if offset < 20 || !offset.is_multiple_of(4) {
        return Err(invalid("Invalid mandatory-label ACL alignment."));
    }
    let header = bytes
        .get(offset..offset + 8)
        .ok_or_else(|| invalid("Truncated mandatory-label ACL."))?;
    let size = u16::from_le_bytes([header[2], header[3]]) as usize;
    let count = u16::from_le_bytes([header[4], header[5]]) as usize;
    if !matches!(header[0], 2 | 4) || size < 8 || offset + size > bytes.len() {
        return Err(invalid("Invalid bounded mandatory-label ACL."));
    }
    let mut cursor = offset + 8;
    for _ in 0..count {
        let ace = bytes
            .get(cursor..cursor + 16)
            .ok_or_else(|| invalid("Truncated mandatory-label entry."))?;
        let length = u16::from_le_bytes([ace[2], ace[3]]) as usize;
        // Label ACE (SYSTEM_MANDATORY_LABEL_ACE_TYPE=17) has mask + bounded SID.
        // Other SACL/audit ACEs remain outside this no-audit-content policy.
        if ace[0] != 17 {
            return Err(unsupported(
                "Native auditing/advanced SACL content requires another fidelity adapter.",
            ));
        }
        if length < 16
            || !length.is_multiple_of(4)
            || cursor + length > offset + size
            || ace[8] != 1
            || ace[9] > 15
            || 16 + usize::from(ace[9]) * 4 != length
        {
            return Err(invalid(
                "Invalid bounded mandatory-label security identifier.",
            ));
        }
        if ace[9] != 1
            || ace[10..16] != [0, 0, 0, 0, 0, 16]
            || u32::from_le_bytes(
                ace[4..8]
                    .try_into()
                    .map_err(|_| invalid("Invalid mandatory-label mask."))?,
            ) & !7
                != 0
        {
            return Err(unsupported(
                "Unsupported mandatory-label authority or policy mask.",
            ));
        }
        cursor += length;
    }
    Ok(())
}
fn canonical_label(bytes: Vec<u8>) -> Result<Vec<u8>> {
    validate_sacl(&bytes)?;
    let offset = u32::from_le_bytes(
        bytes[12..16]
            .try_into()
            .map_err(|_| invalid("Invalid label offset."))?,
    ) as usize;
    if offset == 0 || u16::from_le_bytes([bytes[offset + 4], bytes[offset + 5]]) == 0 {
        // LABEL_SECURITY_INFORMATION is a partial descriptor. NULL/empty SACL
        // encodings both contain NO mandatory-label ACE; SACL presence/control
        // here must not impersonate audit/security state (captured separately).
        let mut absent = vec![0u8; 20];
        absent[0] = 1;
        absent[2..4].copy_from_slice(&0x8000u16.to_le_bytes());
        return Ok(absent);
    }
    Ok(bytes)
}
fn audit(file: &File) -> Option<Vec<u8>> {
    // Request read-only audit rights on the SAME held object. Failure remains
    // unknown; privileges are never enabled merely to certify replacement.
    // SAFETY: native reopen returns one owned handle or INVALID_HANDLE_VALUE.
    let handle = unsafe {
        ReOpenFile(
            file.as_raw_handle(),
            READ_CONTROL | ACCESS_SYSTEM_SECURITY | SYNCHRONIZE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            FILE_FLAG_OPEN_REPARSE_POINT,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return None;
    }
    // SAFETY: ReOpenFile transferred this fresh valid handle.
    let target = unsafe { File::from_raw_handle(handle) };
    if filesystem::identity(&target).ok() != filesystem::identity(file).ok() {
        return None;
    }
    let descriptor = descriptor(&target, SACL_SECURITY_INFORMATION).ok()?;
    validate_sacl(&descriptor).ok()?;
    Some(descriptor)
}
pub(super) fn inspect(file: &File) -> Result<NativeMetadata> {
    no_extra(file)?;
    let mut info = FILE_BASIC_INFO::default();
    // SAFETY: borrowed handle and exactly sized writable SDK structure.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileBasicInfo,
            (&mut info as *mut FILE_BASIC_INFO).cast(),
            size_of::<FILE_BASIC_INFO>() as u32,
        )
    } == 0
    {
        return Err(error());
    }
    if info.FileAttributes & !ATTRIBUTES != 0 {
        return Err(unsupported(
            "This Windows attribute set needs a separate fidelity adapter (for example encrypted, compressed or sparse data).",
        ));
    }
    let security_descriptor = descriptor(
        file,
        OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
    )?;
    validate_descriptor(&security_descriptor)?;
    let mandatory_label = canonical_label(descriptor(file, LABEL_SECURITY_INFORMATION)?)?;
    Ok(NativeMetadata::Windows {
        attributes: info.FileAttributes,
        created: info.CreationTime.to_string(),
        modified: info.LastWriteTime.to_string(),
        security_descriptor,
        mandatory_label,
        audit_descriptor: audit(file),
    })
}
pub(super) fn restore(file: &File, metadata: &NativeMetadata) -> Result<()> {
    let NativeMetadata::Windows {
        attributes,
        created,
        modified,
        security_descriptor,
        mandatory_label,
        audit_descriptor,
    } = metadata
    else {
        return Err(unsupported("Metadata belongs to another native OS."));
    };
    no_extra(file)?;
    validate_descriptor(security_descriptor)?;
    // Reopen the SAME held object for metadata rights, never by pathname.
    // SAFETY: ReOpenFile returns a fresh owned handle or INVALID_HANDLE_VALUE.
    let handle = unsafe {
        ReOpenFile(
            file.as_raw_handle(),
            READ_CONTROL
                | WRITE_DAC
                | WRITE_OWNER
                | FILE_WRITE_ATTRIBUTES
                | SYNCHRONIZE
                | if audit_descriptor.is_some() {
                    ACCESS_SYSTEM_SECURITY
                } else {
                    0
                },
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            FILE_FLAG_OPEN_REPARSE_POINT,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(error());
    }
    // SAFETY: the successful ReOpenFile call transfers this fresh handle.
    let target = unsafe { File::from_raw_handle(handle) };
    if filesystem::identity(&target)? != filesystem::identity(file)? {
        return Err(changed());
    }
    let mut aligned = vec![0u64; security_descriptor.len().div_ceil(8)];
    // SAFETY: our initialized allocation has sufficient capacity and alignment.
    unsafe {
        std::ptr::copy_nonoverlapping(
            security_descriptor.as_ptr(),
            aligned.as_mut_ptr().cast::<u8>(),
            security_descriptor.len(),
        );
    }
    let base = aligned.as_mut_ptr().cast::<u8>();
    let component = |field: usize| -> Result<*mut std::ffi::c_void> {
        let offset = u32::from_le_bytes(
            security_descriptor[field..field + 4]
                .try_into()
                .map_err(|_| invalid("Invalid security component offset."))?,
        ) as usize;
        if offset == 0 {
            Ok(std::ptr::null_mut())
        } else {
            // SAFETY: validate_descriptor bounded every component in this owned buffer.
            Ok(unsafe { base.add(offset) }.cast())
        }
    };
    let control = u16::from_le_bytes([security_descriptor[2], security_descriptor[3]]);
    let protection = if control & SE_DACL_PROTECTED != 0 {
        PROTECTED_DACL_SECURITY_INFORMATION
    } else {
        UNPROTECTED_DACL_SECURITY_INFORMATION
    };
    // SAFETY: bounded owner/group SID and DACL components in our aligned,
    // manually validated descriptor; target is a newly created owned file.
    if unsafe {
        SetSecurityInfo(
            target.as_raw_handle(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION
                | GROUP_SECURITY_INFORMATION
                | DACL_SECURITY_INFORMATION
                | protection,
            component(4)?,
            component(8)?,
            component(16)?.cast(),
            std::ptr::null(),
        )
    } != 0
    {
        return Err(error());
    }
    // SetSecurityInfo applies the destination-parent inheritance model even
    // when replaying inherited source ACEs. After setting the required control
    // state above, use the native handle setter to restore the complete exact
    // captured DACL without performing a parent-tree propagation operation.
    // Nt/ZwSetSecurityObject accepts a bounded self-relative descriptor. Every
    // SID/ACE extent was validated before any call and target is NEW/owned.
    // Request the provider's inheritance bookkeeping when the captured object
    // used that model. This is an ordinary FILE (no child objects), so no parent
    // or descendant tree can be changed by this request.
    if control & SE_DACL_AUTO_INHERITED != 0 {
        let requested = control | SE_DACL_AUTO_INHERIT_REQ;
        // SAFETY: the aligned descriptor owns at least its 20-byte header.
        unsafe {
            std::ptr::copy_nonoverlapping(
                requested.to_le_bytes().as_ptr(),
                aligned.as_mut_ptr().cast::<u8>().add(2),
                2,
            );
        }
    }
    // SAFETY: live metadata-rights handle and aligned validated descriptor.
    let status = unsafe {
        nt::NtSetSecurityObject(
            target.as_raw_handle(),
            DACL_SECURITY_INFORMATION,
            aligned.as_mut_ptr().cast(),
        )
    };
    if status != 0 {
        return Err(AppError::new(
            ErrorCode::Io,
            "Exact new-output DACL restoration failed; all originals remain retained.",
        )
        .caused_by(format!("Native security status {status:#x}")));
    }
    for (descriptor, information) in [(mandatory_label, LABEL_SECURITY_INFORMATION)]
        .into_iter()
        .chain(
            audit_descriptor
                .as_ref()
                .map(|d| (d, SACL_SECURITY_INFORMATION)),
        )
    {
        validate_sacl(descriptor)?;
        let mut aligned = vec![0u64; descriptor.len().div_ceil(8)];
        // SAFETY: bounded live descriptor and sufficiently sized aligned buffer.
        unsafe {
            std::ptr::copy_nonoverlapping(
                descriptor.as_ptr(),
                aligned.as_mut_ptr().cast::<u8>(),
                descriptor.len(),
            );
        }
        let offset = u32::from_le_bytes(
            descriptor[12..16]
                .try_into()
                .map_err(|_| invalid("Invalid security ACL offset."))?,
        ) as usize;
        let empty = ACL {
            AclRevision: 2,
            AclSize: size_of::<ACL>() as u16,
            ..Default::default()
        };
        let sacl = if offset == 0 {
            if information == LABEL_SECURITY_INFORMATION {
                &empty as *const ACL
            } else {
                std::ptr::null()
            }
        } else {
            // SAFETY: validate_sacl bounded this ACL in our aligned live buffer.
            unsafe { aligned.as_ptr().cast::<u8>().add(offset).cast::<ACL>() }
        };
        // SAFETY: validated bounded label/empty-audit ACL on NEW owned output.
        if unsafe {
            SetSecurityInfo(
                target.as_raw_handle(),
                SE_FILE_OBJECT,
                information,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
                sacl,
            )
        } != 0
        {
            return Err(error());
        }
    }
    let info = FILE_BASIC_INFO {
        CreationTime: created
            .parse()
            .map_err(|_| invalid("Invalid native creation time."))?,
        LastWriteTime: modified
            .parse()
            .map_err(|_| invalid("Invalid native modification time."))?,
        LastAccessTime: 0,
        ChangeTime: 0,
        FileAttributes: *attributes,
    };
    // SAFETY: exact SDK structure; only descriptive times/attributes are set on
    // this asserted NEW output. Zero access/change times mean leave unchanged.
    if unsafe {
        SetFileInformationByHandle(
            target.as_raw_handle(),
            FileBasicInfo,
            (&info as *const FILE_BASIC_INFO).cast(),
            size_of::<FILE_BASIC_INFO>() as u32,
        )
    } == 0
    {
        return Err(error());
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn test_low_label(file: &File) -> Result<()> {
    let text = "S:(ML;;NW;;;LW)"
        .encode_utf16()
        .chain([0])
        .collect::<Vec<_>>();
    let mut value = Descriptor(std::ptr::null_mut());
    let mut length = 0u32;
    // SAFETY: static bounded SDDL for a generated fixture, owned descriptor output.
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            text.as_ptr(),
            1,
            &mut value.0,
            &mut length,
        )
    } == 0
    {
        return Err(error());
    }
    if !(20..=65536).contains(&length) {
        return Err(invalid("Invalid fixture label descriptor."));
    }
    // SAFETY: successful system allocation has the documented descriptor length.
    let bytes = unsafe { std::slice::from_raw_parts(value.0.cast::<u8>(), length as usize) };
    validate_sacl(bytes)?;
    let offset = u32::from_le_bytes(
        bytes[12..16]
            .try_into()
            .map_err(|_| invalid("Invalid fixture label offset."))?,
    ) as usize;
    if offset == 0 {
        return Err(invalid("Fixture label is missing."));
    }
    // SAFETY: reopen only the held generated test file for metadata rights.
    let handle = unsafe {
        ReOpenFile(
            file.as_raw_handle(),
            READ_CONTROL | WRITE_OWNER | SYNCHRONIZE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            FILE_FLAG_OPEN_REPARSE_POINT,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(error());
    }
    // SAFETY: successful reopen transferred the fresh valid handle.
    let target = unsafe { File::from_raw_handle(handle) };
    // SAFETY: bounded system-owned label ACL and generated test output only.
    if unsafe {
        SetSecurityInfo(
            target.as_raw_handle(),
            SE_FILE_OBJECT,
            LABEL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
            value.0.cast::<u8>().add(offset).cast(),
        )
    } != 0
    {
        return Err(error());
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn test_private_directory(path: &std::path::Path) -> Result<()> {
    use std::os::windows::fs::OpenOptionsExt;
    let file = std::fs::OpenOptions::new()
        .access_mode(READ_CONTROL | WRITE_DAC | WRITE_OWNER | SYNCHRONIZE)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let text = "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;OW)"
        .encode_utf16()
        .chain([0])
        .collect::<Vec<_>>();
    let mut value = Descriptor(std::ptr::null_mut());
    let mut length = 0u32;
    // SAFETY: fixed trusted SDDL and newly created generated test directory.
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            text.as_ptr(),
            1,
            &mut value.0,
            &mut length,
        )
    } == 0
    {
        return Err(error());
    }
    if !(20..=65536).contains(&length) {
        return Err(invalid("Invalid fixture DACL."));
    }
    // SAFETY: system allocation length from successful SDDL parsing.
    let bytes = unsafe { std::slice::from_raw_parts(value.0.cast::<u8>(), length as usize) };
    let offset = u32::from_le_bytes(
        bytes[16..20]
            .try_into()
            .map_err(|_| invalid("Invalid fixture DACL offset."))?,
    ) as usize;
    if offset < 20 || offset + 8 > bytes.len() {
        return Err(invalid("Missing fixture DACL."));
    }
    // SAFETY: static system-parsed DACL on this generated test directory only.
    if unsafe {
        SetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            value.0.cast::<u8>().add(offset).cast(),
            std::ptr::null(),
        )
    } != 0
    {
        return Err(error());
    }
    Ok(())
}

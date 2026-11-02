//! Read-only volume extents + physical bus classification through fixed Win32
//! storage controls. File-backed, virtual, unknown and pooled buses fail closed.
use super::*;
use crate::{
    domain::{AppError, ErrorCode},
    filesystem,
};
use std::{
    mem::{offset_of, size_of},
    os::windows::io::{AsRawHandle, FromRawHandle},
};
use windows_sys::Win32::{
    Foundation::INVALID_HANDLE_VALUE,
    Storage::FileSystem::*,
    System::{IO::DeviceIoControl, Ioctl::*},
};

fn volume_name(file: &File) -> Option<Vec<u16>> {
    let mut buffer = vec![0u16; 32768];
    // SAFETY: live file and bounded writable UTF-16 buffer. Kernel supplies the
    // actual volume GUID; no user path is used to construct a device handle.
    let count = unsafe {
        GetFinalPathNameByHandleW(
            file.as_raw_handle(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            VOLUME_NAME_GUID,
        )
    } as usize;
    if count == 0 || count >= buffer.len() {
        return None;
    }
    let close = buffer[..count]
        .iter()
        .position(|unit| *unit == b'}' as u16)?;
    let prefix = String::from_utf16(&buffer[..=close]).ok()?;
    let guid = prefix.strip_prefix(r"\\?\Volume{")?.strip_suffix('}')?;
    uuid::Uuid::parse_str(guid).ok()?;
    Some(buffer[..=close].iter().copied().chain([0]).collect())
}
fn device(name: &[u16]) -> Option<File> {
    // SAFETY: fixed/kernel-derived NUL-terminated name. Zero desired access and
    // OPEN_EXISTING permit only control queries, never disk content mutations.
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        None
    } else {
        // SAFETY: successful CreateFileW transferred one owned handle.
        Some(unsafe { File::from_raw_handle(handle) })
    }
}
fn physical(disk: u32) -> Option<String> {
    if disk > 1023 {
        return None;
    }
    let name: Vec<u16> = format!(r"\\.\PhysicalDrive{disk}")
        .encode_utf16()
        .chain([0])
        .collect();
    let file = device(&name)?;
    let query = STORAGE_PROPERTY_QUERY {
        PropertyId: StorageDeviceProperty,
        QueryType: PropertyStandardQuery,
        ..Default::default()
    };
    let mut bytes = vec![0u64; 8192];
    let mut returned = 0;
    // SAFETY: fixed read-only property query, aligned initialized output buffer
    // and exact sizes. No passthrough command or renderer bytes are accepted.
    if unsafe {
        DeviceIoControl(
            file.as_raw_handle(),
            IOCTL_STORAGE_QUERY_PROPERTY,
            (&query as *const STORAGE_PROPERTY_QUERY).cast(),
            size_of::<STORAGE_PROPERTY_QUERY>() as u32,
            bytes.as_mut_ptr().cast(),
            65536,
            &mut returned,
            std::ptr::null_mut(),
        )
    } == 0
        || (returned as usize) < size_of::<STORAGE_DEVICE_DESCRIPTOR>()
        || returned > 65536
    {
        return None;
    }
    // SAFETY: the validated returned length covers this integer-only SDK record.
    let descriptor = unsafe { &*bytes.as_ptr().cast::<STORAGE_DEVICE_DESCRIPTOR>() };
    if descriptor.Size < size_of::<STORAGE_DEVICE_DESCRIPTOR>() as u32
        || descriptor.Size > returned
        || ![BusTypeAta, BusTypeSata, BusTypeNvme, BusTypeUsb].contains(&descriptor.BusType)
    {
        return None;
    }
    let offset = descriptor.SerialNumberOffset as usize;
    if offset < offset_of!(STORAGE_DEVICE_DESCRIPTOR, RawDeviceProperties)
        || offset >= descriptor.Size as usize
    {
        return None;
    }
    // SAFETY: initialized storage; only the bounded returned descriptor bytes
    // are examined. Native hardware IDs never enter logs or catalog manifests.
    let raw = unsafe {
        std::slice::from_raw_parts(bytes.as_ptr().cast::<u8>(), descriptor.Size as usize)
    };
    let model_offset = descriptor.ProductIdOffset as usize;
    if model_offset < offset_of!(STORAGE_DEVICE_DESCRIPTOR, RawDeviceProperties)
        || model_offset >= raw.len()
    {
        return None;
    }
    let model = &raw[model_offset..];
    let end = model.iter().position(|b| *b == 0)?;
    if end == 0 || end > 256 || !model[..end].iter().all(u8::is_ascii) {
        return None;
    }
    let model = String::from_utf8_lossy(&model[..end]).to_ascii_uppercase();
    if ["VIRTUAL", "VMWARE", "VBOX", "QEMU", "PARALLELS", "RAMDISK"]
        .iter()
        .any(|word| model.contains(word))
    {
        return None;
    }
    let serial = &raw[offset..];
    let end = serial.iter().position(|b| *b == 0)?;
    let serial = serial[..end].trim_ascii();
    if serial.is_empty() || serial.len() > 256 || !serial.iter().all(|b| (0x21..=0x7e).contains(b))
    {
        return None;
    }
    // Two LUNs advertising the same physical serial are not independent merely
    // because Windows assigns distinct disk numbers. Missing serials fail closed.
    Some(format!(
        "windows-physical-device:{}",
        blake3::hash(serial).to_hex()
    ))
}
pub(super) fn local(file: &File) -> Result<Option<BTreeSet<String>>> {
    let identity = filesystem::identity(file)?;
    let Some(name) = volume_name(file) else {
        return Ok(None);
    };
    let Some(volume) = device(&name) else {
        return Ok(None);
    };
    let mut bytes = vec![0u64; 8192];
    let mut returned = 0;
    // SAFETY: read-only volume control, aligned initialized output allocation.
    if unsafe {
        DeviceIoControl(
            volume.as_raw_handle(),
            IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS,
            std::ptr::null(),
            0,
            bytes.as_mut_ptr().cast(),
            65536,
            &mut returned,
            std::ptr::null_mut(),
        )
    } == 0
        || (returned as usize) < size_of::<VOLUME_DISK_EXTENTS>()
        || returned > 65536
    {
        return Ok(None);
    }
    // SAFETY: validated length covers the fixed prefix. Validate the flexible
    // array count and extent sizes before reading any additional element.
    let count =
        unsafe { (*bytes.as_ptr().cast::<VOLUME_DISK_EXTENTS>()).NumberOfDiskExtents } as usize;
    let offset = offset_of!(VOLUME_DISK_EXTENTS, Extents);
    if count == 0 || count > 16 || offset + count * size_of::<DISK_EXTENT>() > returned as usize {
        return Ok(None);
    }
    let mut disks = BTreeSet::new();
    for index in 0..count {
        // SAFETY: initialized allocation and flexible-array bounds checked above.
        let extent = unsafe {
            &*bytes
                .as_ptr()
                .cast::<u8>()
                .add(offset + index * size_of::<DISK_EXTENT>())
                .cast::<DISK_EXTENT>()
        };
        if extent.StartingOffset < 0 || extent.ExtentLength <= 0 {
            return Ok(None);
        }
        let Some(device) = physical(extent.DiskNumber) else {
            return Ok(None);
        };
        disks.insert(device);
    }
    if filesystem::identity(file)? != identity || volume_name(file).as_ref() != Some(&name) {
        return Err(AppError::new(
            ErrorCode::Conflict,
            "Native storage membership changed during recovery preflight.",
        ));
    }
    Ok(Some(disks))
}

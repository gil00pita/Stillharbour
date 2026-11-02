//! Fresh Darwin physical-media classification. Volume names/UUIDs and APFS
//! synthesized disks are never physical failure-domain identities.
//! Apple IOStorageProtocolCharacteristics.h defines the protocol/location
//! properties; Apple bless's APFS utilities use the same IOKit parent graph.
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem,
};
use std::{
    collections::{BTreeSet, VecDeque},
    ffi::{CStr, CString, c_char, c_void},
    fs::File,
    mem::MaybeUninit,
    os::fd::AsRawFd,
};

type Cf = *const c_void;
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOBSDNameMatching(port: u32, options: u32, name: *const c_char) -> Cf;
    fn IOServiceGetMatchingService(port: u32, matching: Cf) -> u32;
    fn IORegistryEntryGetParentIterator(
        entry: u32,
        plane: *const c_char,
        iterator: *mut u32,
    ) -> i32;
    fn IOIteratorNext(iterator: u32) -> u32;
    fn IOObjectRelease(object: u32) -> i32;
    fn IOObjectConformsTo(object: u32, class: *const c_char) -> u32;
    fn IORegistryEntryGetRegistryEntryID(entry: u32, id: *mut u64) -> i32;
    fn IORegistryEntryCreateCFProperty(entry: u32, key: Cf, allocator: Cf, options: u32) -> Cf;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithCString(allocator: Cf, text: *const c_char, encoding: u32) -> Cf;
    fn CFStringGetCString(text: Cf, buffer: *mut c_char, size: isize, encoding: u32) -> u8;
    fn CFGetTypeID(value: Cf) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFDictionaryGetTypeID() -> usize;
    fn CFDictionaryGetValue(dictionary: Cf, key: Cf) -> Cf;
    fn CFRelease(value: Cf);
}
const UTF8: u32 = 0x0800_0100;
struct Object(u32);
impl Drop for Object {
    fn drop(&mut self) {
        if self.0 != 0 {
            // SAFETY: this wrapper owns one IOKit object reference.
            unsafe {
                IOObjectRelease(self.0);
            }
        }
    }
}
struct OwnedCf(Cf);
impl Drop for OwnedCf {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: Create/Copy transferred one owned CoreFoundation reference.
            unsafe {
                CFRelease(self.0);
            }
        }
    }
}
fn key(value: &CStr) -> Option<OwnedCf> {
    // SAFETY: static/bounded live NUL-terminated C string; null default allocator.
    let value = unsafe { CFStringCreateWithCString(std::ptr::null(), value.as_ptr(), UTF8) };
    (!value.is_null()).then_some(OwnedCf(value))
}
fn text(value: Cf) -> Option<String> {
    if value.is_null() {
        return None;
    }
    // SAFETY: borrowed live CoreFoundation object and SDK type identity calls.
    if unsafe { CFGetTypeID(value) != CFStringGetTypeID() } {
        return None;
    }
    let mut buffer = [0u8; 128];
    // SAFETY: correctly bounded writable buffer; CFString checks its capacity.
    if unsafe {
        CFStringGetCString(
            value,
            buffer.as_mut_ptr().cast(),
            buffer.len() as isize,
            UTF8,
        )
    } == 0
    {
        return None;
    }
    let end = buffer.iter().position(|b| *b == 0)?;
    std::str::from_utf8(&buffer[..end]).ok().map(String::from)
}
fn identifier(object: u32) -> Option<u64> {
    let mut id = 0; // SAFETY: borrowed IOKit object and writable scalar buffer.
    if unsafe { IORegistryEntryGetRegistryEntryID(object, &mut id) } != 0 || id == 0 {
        None
    } else {
        Some(id)
    }
}
fn match_media(name: &CStr) -> Option<Object> {
    // SAFETY: bounded kernel-provided BSD name; IOBSDNameMatching creates an
    // owned dictionary consumed by IOServiceGetMatchingService, even on error.
    let matching = unsafe { IOBSDNameMatching(0, 0, name.as_ptr()) };
    if matching.is_null() {
        return None;
    }
    // SAFETY: one matching dictionary reference is transferred to IOKit.
    let service = unsafe { IOServiceGetMatchingService(0, matching) };
    (service != 0).then_some(Object(service))
}
fn physical_protocol(protocol: &str, location: &str) -> bool {
    matches!(location, "Internal" | "External" | "Internal/External")
        && (matches!(protocol, "ATA" | "SATA" | "USB" | "PCI-Express")
            || protocol == "Apple Fabric" && location == "Internal")
}
fn dictionary_text(object: u32, property: &CStr, field: &CStr) -> Option<String> {
    let property = key(property)?;
    // SAFETY: live IOKit node/key; returned CF property is owned.
    let dictionary = OwnedCf(unsafe {
        IORegistryEntryCreateCFProperty(object, property.0, std::ptr::null(), 0)
    });
    if dictionary.0.is_null() {
        return None;
    }
    // SAFETY: live system CF object is checked before dictionary access.
    if unsafe { CFGetTypeID(dictionary.0) != CFDictionaryGetTypeID() } {
        return None;
    }
    let field = key(field)?;
    // SAFETY: checked live dictionary and key, borrowed value converted now.
    text(unsafe { CFDictionaryGetValue(dictionary.0, field.0) })
}
fn verified_physical(object: u32) -> Option<String> {
    let property = key(c"Protocol Characteristics")?;
    // SAFETY: live IOKit object and CoreFoundation key; returned property owned.
    let dictionary = OwnedCf(unsafe {
        IORegistryEntryCreateCFProperty(object, property.0, std::ptr::null(), 0)
    });
    if dictionary.0.is_null() {
        return None;
    }
    // SAFETY: system-created live CF object; dictionary APIs follow type check.
    if unsafe { CFGetTypeID(dictionary.0) != CFDictionaryGetTypeID() } {
        return None;
    }
    let protocol = key(c"Physical Interconnect")?;
    let location = key(c"Physical Interconnect Location")?;
    // SAFETY: checked dictionary and live CFString keys. Borrowed values live
    // until dictionary's owning reference is released after conversion.
    let (protocol, location) = unsafe {
        (
            CFDictionaryGetValue(dictionary.0, protocol.0),
            CFDictionaryGetValue(dictionary.0, location.0),
        )
    };
    let protocol = text(protocol)?;
    let location = text(location)?;
    if !physical_protocol(&protocol, &location) {
        return None;
    }
    let vendor = dictionary_text(object, c"Device Characteristics", c"Vendor Name")?;
    let product = dictionary_text(object, c"Device Characteristics", c"Product Name")?;
    let label = format!("{vendor} {product}").to_ascii_lowercase();
    if vendor.trim().is_empty()
        || product.trim().is_empty()
        || ["virtual", "vmware", "qemu", "vbox", "disk image"]
            .iter()
            .any(|marker| label.contains(marker))
    {
        return None;
    }
    Some(protocol)
}
fn parents(entry: u32) -> Option<Vec<Object>> {
    let mut iterator = 0;
    // SAFETY: borrowed node, static IOKit plane and writable owned reference.
    if unsafe { IORegistryEntryGetParentIterator(entry, c"IOService".as_ptr(), &mut iterator) } != 0
        || iterator == 0
    {
        return None;
    }
    let iterator = Object(iterator);
    let mut result = Vec::new();
    loop {
        // SAFETY: live iterator; a nonzero result transfers one object reference.
        let object = unsafe { IOIteratorNext(iterator.0) };
        if object == 0 {
            break;
        }
        let object = Object(object);
        if result.len() >= 64 {
            return None;
        }
        result.push(object);
    }
    Some(result)
}
fn hardware_ids(entry: u32, class: &CStr) -> Option<BTreeSet<String>> {
    let mut queue = parents(entry)?
        .into_iter()
        .map(|p| (p, 0usize))
        .collect::<VecDeque<_>>();
    if queue.is_empty() {
        return None;
    }
    let mut visited = BTreeSet::new();
    let mut result = BTreeSet::new();
    while let Some((object, depth)) = queue.pop_front() {
        if depth > 64 || visited.len() >= 1024 || queue.len() > 1024 {
            return None;
        }
        let id = identifier(object.0)?;
        if !visited.insert(id) {
            continue;
        }
        // SAFETY: borrowed object and static hardware class string.
        if unsafe { IOObjectConformsTo(object.0, class.as_ptr()) } != 0 {
            result.insert(format!("darwin-hardware-{id:016x}"));
            continue;
        }
        let parents = parents(object.0)?;
        if parents.is_empty() {
            return None;
        }
        queue.extend(parents.into_iter().map(|p| (p, depth + 1)));
    }
    (!result.is_empty()).then_some(result)
}
fn mount(file: &File) -> Result<libc::statfs> {
    let mut info = MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: borrowed native fd, correctly sized writable libc structure.
    if unsafe { libc::fstatfs(file.as_raw_fd(), info.as_mut_ptr()) } != 0 {
        return Err(AppError::new(
            ErrorCode::Io,
            "Could not inspect the held native volume failure domain.",
        )
        .caused_by(std::io::Error::last_os_error()));
    }
    // SAFETY: successful fstatfs initialized the structure.
    Ok(unsafe { info.assume_init() })
}
fn name(bytes: &[c_char]) -> Option<Vec<u8>> {
    let end = bytes.iter().position(|b| *b == 0)?;
    Some(bytes[..end].iter().map(|b| *b as u8).collect())
}

pub(super) fn local(file: &File) -> Result<Option<BTreeSet<String>>> {
    let identity = filesystem::identity(file)?;
    let before = mount(file)?;
    let Some(kind) = name(&before.f_fstypename) else {
        return Ok(None);
    };
    if !matches!(kind.as_slice(), b"apfs" | b"hfs") {
        return Ok(None);
    }
    let Some(device) = name(&before.f_mntfromname) else {
        return Ok(None);
    };
    let Some(bsd) = device
        .strip_prefix(b"/dev/")
        .filter(|v| v.starts_with(b"disk") && v.iter().all(u8::is_ascii_alphanumeric))
    else {
        return Ok(None);
    };
    let Some(bsd) = CString::new(bsd).ok() else {
        return Ok(None);
    };
    let Some(media) = match_media(&bsd) else {
        return Ok(None);
    };
    // SAFETY: borrowed live IOKit service and static NUL-terminated class name.
    if unsafe { IOObjectConformsTo(media.0, c"IOMedia".as_ptr()) } == 0 {
        return Ok(None);
    }
    let Some(media_id) = identifier(media.0) else {
        return Ok(None);
    };
    let mut pending = VecDeque::from([(media, 0usize)]);
    let mut visited = BTreeSet::new();
    let mut devices = BTreeSet::new();
    while let Some((entry, depth)) = pending.pop_front() {
        if depth > 64 || visited.len() >= 1024 || pending.len() > 1024 {
            return Ok(None);
        }
        let Some(id) = identifier(entry.0) else {
            return Ok(None);
        };
        if !visited.insert(id) {
            continue;
        }
        // SAFETY: borrowed IOKit reference and static checked class name.
        if unsafe { IOObjectConformsTo(entry.0, c"IOBlockStorageDevice".as_ptr()) } != 0 {
            let Some(protocol) = verified_physical(entry.0) else {
                return Ok(None);
            };
            let Some(serial) =
                dictionary_text(entry.0, c"Device Characteristics", c"Serial Number")
                    .filter(|v| !v.trim().is_empty())
            else {
                return Ok(None);
            };
            devices.insert(format!(
                "darwin-physical-serial-{}",
                blake3::hash(serial.trim().as_bytes()).to_hex()
            ));
            let class = match protocol.as_str() {
                "USB" => Some(c"IOUSBHostDevice"),
                "PCI-Express" => Some(c"IOPCIDevice"),
                "Apple Fabric" => Some(c"AppleARMIODevice"),
                _ => None,
            };
            if let Some(class) = class {
                let Some(hardware) = hardware_ids(entry.0, class) else {
                    return Ok(None);
                };
                devices.extend(hardware);
            }
            // Stop at each physical block device. APFS volume/container IDs and
            // bus/controller parents never become independent-disk identities.
            continue;
        }
        let mut iterator = 0;
        // SAFETY: borrowed registry node, static plane and writable reference.
        if unsafe {
            IORegistryEntryGetParentIterator(entry.0, c"IOService".as_ptr(), &mut iterator)
        } != 0
            || iterator == 0
        {
            return Ok(None);
        }
        let iterator = Object(iterator);
        let mut parents = 0usize;
        loop {
            // SAFETY: live iterator; returned object reference transfers ownership.
            let parent = unsafe { IOIteratorNext(iterator.0) };
            if parent == 0 {
                break;
            }
            let parent = Object(parent);
            parents += 1;
            if parents > 64 {
                return Ok(None);
            }
            pending.push_back((parent, depth + 1));
        }
        // Every ancestry branch must resolve to a verified physical device.
        // Unknown roots, disk images, RAM disks and synthesized-only graphs
        // cannot prove independence merely because another volume name differs.
        if parents == 0 {
            return Ok(None);
        }
    }
    let after = mount(file)?;
    let current = match_media(&bsd);
    if devices.is_empty()
        || filesystem::identity(file)? != identity
        || name(&after.f_mntfromname) != Some(device)
        || after.f_type != before.f_type
        || name(&after.f_fstypename) != Some(kind)
        || current.as_ref().and_then(|m| identifier(m.0)) != Some(media_id)
    {
        return Ok(None);
    }
    Ok(Some(devices))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_protocol_rejects_virtual_file_ram_and_unknown_locations() {
        for (protocol, location) in [
            ("Virtual Interface", "File"),
            ("Virtual Interface", "RAM"),
            ("USB", "File"),
            ("PCI-Express", "Virtual"),
            ("Apple Fabric", "External"),
            ("SCSI Parallel Interface", "Internal"),
            ("", "Internal"),
        ] {
            assert!(!physical_protocol(protocol, location));
        }
        for (protocol, location) in [
            ("USB", "External"),
            ("SATA", "Internal"),
            ("PCI-Express", "Internal"),
            ("Apple Fabric", "Internal"),
        ] {
            assert!(physical_protocol(protocol, location));
        }
    }
    #[test]
    fn held_files_on_one_native_volume_cannot_be_different_physical_domains() {
        let owned = tempfile::Builder::new()
            .prefix("stillharbour-failure-domain-")
            .tempdir()
            .unwrap();
        let root = owned.path().canonicalize().unwrap();
        assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap()));
        std::fs::write(root.join("owner"), uuid::Uuid::new_v4().to_string()).unwrap();
        let a = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join("one"))
            .unwrap();
        let b = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join("two"))
            .unwrap();
        let first = local(&a).unwrap();
        let second = local(&b).unwrap();
        assert_eq!(first, second);
        if let (Some(first), Some(second)) = (first, second) {
            assert!(!first.is_disjoint(&second));
        }
    }
}

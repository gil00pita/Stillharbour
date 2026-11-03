//! Darwin descriptor metadata. Policy v1 proves NO extended ACL/xattrs/resource
//! fork and preserves ownership, POSIX permissions, birth and modification time.
//! Policy v2 additionally captures/verifies bounded opaque OS provenance bytes;
//! it never ignores attributes, strips provenance or trusts setter success.
use super::*;
use std::{
    ffi::c_void,
    mem::{MaybeUninit, size_of},
    os::fd::AsRawFd,
};

// Apple Libc sys/acl.h: ACL_TYPE_EXTENDED=0x100, ACL_FIRST_ENTRY=0.
unsafe extern "C" {
    fn acl_get_fd_np(fd: libc::c_int, kind: libc::c_int) -> *mut c_void;
    fn acl_get_entry(acl: *mut c_void, entry: libc::c_int, value: *mut *mut c_void) -> libc::c_int;
    fn acl_valid(acl: *mut c_void) -> libc::c_int;
    fn acl_get_flagset_np(acl: *mut c_void, value: *mut *mut c_void) -> libc::c_int;
    fn acl_get_flag_np(flags: *mut c_void, flag: libc::c_uint) -> libc::c_int;
    fn acl_free(value: *mut c_void) -> libc::c_int;
}
struct Acl(*mut c_void);
impl Drop for Acl {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: this opaque ACL was allocated by acl_get_fd_np.
            unsafe {
                acl_free(self.0);
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
fn stat(file: &File) -> Result<libc::stat> {
    let mut value = MaybeUninit::<libc::stat>::uninit();
    // SAFETY: borrowed fd and writable, correctly sized libc stat buffer.
    if unsafe { libc::fstat(file.as_raw_fd(), value.as_mut_ptr()) } != 0 {
        return Err(error());
    }
    // SAFETY: successful fstat initialized the whole structure.
    Ok(unsafe { value.assume_init() })
}
fn provenance(file: &File) -> Result<Option<Vec<u8>>> {
    const NAME: &std::ffi::CStr = c"com.apple.provenance";
    // SAFETY: size-zero descriptor query never writes through the null buffer.
    let size = unsafe { libc::flistxattr(file.as_raw_fd(), std::ptr::null_mut(), 0, 0) };
    if size < 0 {
        return Err(error());
    }
    if size == 0 {
        return Ok(None);
    }
    // Admit exactly one fixed name, never a broader allowlist or omitted stream.
    if size as usize != NAME.to_bytes_with_nul().len() {
        return Err(extra());
    }
    let mut names = vec![0u8; size as usize];
    // SAFETY: live borrowed descriptor and bounded writable name buffer.
    let read =
        unsafe { libc::flistxattr(file.as_raw_fd(), names.as_mut_ptr().cast(), names.len(), 0) };
    if read != size {
        return Err(changed());
    }
    if names != NAME.to_bytes_with_nul() {
        return Err(extra());
    }
    // SAFETY: fixed native C name and size-zero descriptor query.
    let length = unsafe {
        libc::fgetxattr(
            file.as_raw_fd(),
            NAME.as_ptr(),
            std::ptr::null_mut(),
            0,
            0,
            0,
        )
    };
    if length < 0 {
        return Err(error());
    }
    if length as usize > MAX_PROVENANCE {
        return Err(unsupported(
            "Native provenance exceeds its 4096 byte preservation bound.",
        ));
    }
    let mut value = vec![0u8; length as usize];
    // SAFETY: bounded live value buffer; only an existing descriptor is read.
    let read = unsafe {
        libc::fgetxattr(
            file.as_raw_fd(),
            NAME.as_ptr(),
            value.as_mut_ptr().cast(),
            value.len(),
            0,
            0,
        )
    };
    if read != length {
        return Err(changed());
    }
    Ok(Some(value))
}
fn extra() -> AppError {
    unsupported(
        "macOS extended attributes or a resource fork outside the versioned provenance policy are present; strict replacement is unavailable.",
    )
}
fn no_acl(file: &File) -> Result<()> {
    // SAFETY: borrowed fd; Darwin returns an allocated opaque ACL or null/error.
    let acl = Acl(unsafe { acl_get_fd_np(file.as_raw_fd(), 0x100) });
    if acl.0.is_null() {
        if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
            return Ok(());
        }
        return Err(error());
    }
    // SAFETY: live system ACL. Inspect all flag bits; unknown/nonzero flags are
    // unsupported rather than silently normalised into an empty ACL.
    if unsafe { acl_valid(acl.0) } != 0 {
        return Err(error());
    }
    let mut flags = std::ptr::null_mut();
    // SAFETY: valid system ACL and writable pointer to its borrowed flagset.
    if unsafe { acl_get_flagset_np(acl.0, &mut flags) } != 0 || flags.is_null() {
        return Err(error());
    }
    // SAFETY: flagset remains alive with the ACL; all bits are queried read-only.
    if unsafe { acl_get_flag_np(flags, u32::MAX) } != 0 {
        return Err(unsupported(
            "Native ACL flags need another full-fidelity adapter.",
        ));
    }
    let mut entry = std::ptr::null_mut();
    // SAFETY: live system ACL and writable opaque entry pointer, read only.
    match unsafe { acl_get_entry(acl.0, 0, &mut entry) } {
        0 => Err(unsupported(
            "macOS extended ACL entries need another fidelity adapter; replacement is unavailable.",
        )),
        -1 if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINVAL) => Ok(()),
        _ => Err(error()),
    }
}
pub(super) fn private_directory(file: &File) -> Result<(u32, u32, u32)> {
    let value = stat(file)?;
    // SAFETY: read-only native current user identity.
    if !file.metadata()?.is_dir()
        || value.st_uid != unsafe { libc::geteuid() }
        || value.st_mode & 0o022 != 0
    {
        return Err(unsupported(
            "Private output requires a user-owned directory with no group or other write permissions.",
        ));
    }
    no_acl(file)?;
    Ok((
        value.st_uid,
        value.st_gid,
        u32::from(value.st_mode) & 0o7777,
    ))
}
pub(super) fn private_file(file: &File) -> Result<()> {
    let value = stat(file)?;
    // SAFETY: read-only native identity. Owner-only mode was installed atomically
    // at creation; extended inherited permissions may never be cleared later.
    if !file.metadata()?.is_file()
        || value.st_uid != unsafe { libc::geteuid() }
        || value.st_mode & 0o077 != 0
        || value.st_nlink != 1
    {
        return Err(unsupported(
            "New private output did not establish owner-only native permissions.",
        ));
    }
    no_acl(file)
}
pub(super) fn inspect(file: &File) -> Result<NativeMetadata> {
    let provenance = provenance(file)?;
    no_acl(file)?;
    let value = stat(file)?;
    if value.st_flags != 0 {
        return Err(unsupported(
            "macOS file flags require another restore/publication adapter; replacement is unavailable.",
        ));
    }
    Ok(NativeMetadata::Macos {
        uid: value.st_uid,
        gid: value.st_gid,
        mode: u32::from(value.st_mode) & 0o7777,
        flags: value.st_flags,
        created_seconds: value.st_birthtime,
        created_nanoseconds: value.st_birthtime_nsec,
        modified_seconds: value.st_mtime,
        modified_nanoseconds: value.st_mtime_nsec,
        provenance,
    })
}
pub(super) fn restore(file: &File, metadata: &NativeMetadata) -> Result<()> {
    let NativeMetadata::Macos {
        uid,
        gid,
        mode,
        flags,
        created_seconds,
        created_nanoseconds,
        modified_seconds,
        modified_nanoseconds,
        provenance: expected_provenance,
    } = metadata
    else {
        return Err(unsupported("Metadata belongs to another native OS."));
    };
    let actual_provenance = provenance(file)?;
    no_acl(file)?;
    if expected_provenance.is_none() && actual_provenance.is_some() {
        return Err(unsupported(
            "The v1 no-extra-stream policy cannot restore into a target with native provenance.",
        ));
    }
    if let Some(value) = expected_provenance {
        if value.len() > MAX_PROVENANCE {
            return Err(invalid("Invalid native provenance bound."));
        }
        if actual_provenance.as_ref() != Some(value) {
            // SAFETY: fixed C name and captured bounded bytes, NEW owned output
            // only. No original descriptor or pathname reaches this operation.
            if unsafe {
                libc::fsetxattr(
                    file.as_raw_fd(),
                    c"com.apple.provenance".as_ptr(),
                    value.as_ptr().cast(),
                    value.len(),
                    0,
                    0,
                )
            } != 0
            {
                return Err(AppError::new(
                    ErrorCode::Io,
                    "Native provenance restoration was refused; the original is unchanged.",
                )
                .caused_by(std::io::Error::last_os_error()));
            }
        }
        // Protected OS attributes can acknowledge writes without installing
        // the requested value. Only independently observed bytes count.
        if provenance(file)?.as_ref() != Some(value) {
            return Err(changed());
        }
    }
    if *flags != 0 {
        return Err(unsupported("Nonzero native file flags are unsupported."));
    }
    let before = stat(file)?;
    // SAFETY: borrowed handle to an asserted NEW owned output. No pathname is
    // used. Skip ownership syscalls when already equal, avoiding extra rights.
    if (before.st_uid != *uid || before.st_gid != *gid)
        && unsafe { libc::fchown(file.as_raw_fd(), *uid, *gid) } != 0
    {
        return Err(error());
    }
    // SAFETY: bounded POSIX mode on this newly created owned file only.
    if unsafe { libc::fchmod(file.as_raw_fd(), *mode as libc::mode_t) } != 0 {
        return Err(error());
    }
    let mut attributes = libc::attrlist {
        bitmapcount: libc::ATTR_BIT_MAP_COUNT,
        reserved: 0,
        commonattr: libc::ATTR_CMN_CRTIME,
        volattr: 0,
        dirattr: 0,
        fileattr: 0,
        forkattr: 0,
    };
    let mut created = libc::timespec {
        tv_sec: *created_seconds,
        tv_nsec: *created_nanoseconds,
    };
    // SAFETY: exact Darwin attrlist and packed single timespec for birth time.
    if unsafe {
        libc::fsetattrlist(
            file.as_raw_fd(),
            (&mut attributes as *mut libc::attrlist).cast(),
            (&mut created as *mut libc::timespec).cast(),
            size_of::<libc::timespec>(),
            0,
        )
    } != 0
    {
        return Err(AppError::new(
            ErrorCode::Io,
            "Native creation-time restoration failed; the original is unchanged.",
        )
        .caused_by(std::io::Error::last_os_error()));
    }
    let times = [
        libc::timespec {
            tv_sec: before.st_atime,
            tv_nsec: before.st_atime_nsec,
        },
        libc::timespec {
            tv_sec: *modified_seconds,
            tv_nsec: *modified_nanoseconds,
        },
    ];
    // SAFETY: exact two-timespec buffer, access time retained and mtime restored.
    if unsafe { libc::futimens(file.as_raw_fd(), times.as_ptr()) } != 0 {
        return Err(error());
    }
    Ok(())
}

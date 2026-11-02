//! Darwin descriptor metadata. Policy v1 proves NO extended ACL/xattrs/resource
//! fork and preserves ownership, POSIX permissions, birth and modification time.
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
fn no_extra(file: &File) -> Result<()> {
    // SAFETY: size-zero query never writes through the null buffer.
    let size = unsafe { libc::flistxattr(file.as_raw_fd(), std::ptr::null_mut(), 0, 0) };
    if size < 0 {
        return Err(error());
    }
    if size != 0 {
        return Err(unsupported(
            "macOS extended attributes or a resource fork are present; this no-extra-stream policy cannot authorize replacement.",
        ));
    }
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
pub(super) fn inspect(file: &File) -> Result<NativeMetadata> {
    no_extra(file)?;
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
    } = metadata
    else {
        return Err(unsupported("Metadata belongs to another native OS."));
    };
    no_extra(file)?;
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
        return Err(error());
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

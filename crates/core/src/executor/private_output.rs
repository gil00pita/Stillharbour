//! Private owner-only outputs: permissions are installed atomically at create.
use super::*;

pub(super) fn create(parent: &File, leaf: &str) -> Result<File> {
    #[cfg(target_os = "macos")]
    {
        // An inherited allow ACL can override mode 0600. Refuse such a parent
        // before create; clearing it later cannot revoke a reader opened in the
        // exposure window. Verify the new object before returning it to the
        // only caller that can write secret bytes.
        let private = crate::backup::native_metadata::private_directory(parent)?;
        let file = super::native::create_file(parent, leaf)?;
        private.revalidate(parent)?;
        crate::backup::native_metadata::verify_private_file(&file)?;
        Ok(file)
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, HANDLE, LocalFree},
            Security::{
                Authorization::{
                    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
                    SDDL_REVISION_1,
                },
                GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser,
            },
            System::Threading::{GetCurrentProcess, OpenProcessToken},
        };
        struct Token(HANDLE);
        impl Drop for Token {
            fn drop(&mut self) {
                // SAFETY: this guard owns the successful OpenProcessToken handle.
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
        struct Allocation(*mut std::ffi::c_void);
        impl Drop for Allocation {
            fn drop(&mut self) {
                // SAFETY: conversion APIs return LocalAlloc allocations.
                unsafe {
                    LocalFree(self.0);
                }
            }
        }
        let fail = || {
            AppError::new(
                ErrorCode::Io,
                "Private output permissions could not be established; no secret was written. Retain any empty output for review.",
            )
        };
        let mut token = std::ptr::null_mut();
        // SAFETY: current process pseudo-handle and valid output address.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return Err(fail());
        }
        let token = Token(token);
        let mut length = 0;
        // SAFETY: the initial size query writes only the required length.
        unsafe {
            GetTokenInformation(token.0, TokenUser, std::ptr::null_mut(), 0, &mut length);
        }
        if !(std::mem::size_of::<TOKEN_USER>()..=65536).contains(&(length as usize)) {
            return Err(fail());
        }
        // usize storage gives TOKEN_USER sufficient native pointer alignment.
        let mut storage = vec![0usize; (length as usize).div_ceil(std::mem::size_of::<usize>())];
        // SAFETY: aligned live output storage has at least length bytes.
        if unsafe {
            GetTokenInformation(
                token.0,
                TokenUser,
                storage.as_mut_ptr().cast(),
                length,
                &mut length,
            )
        } == 0
        {
            return Err(fail());
        }
        // SAFETY: the successful TokenUser query populated TOKEN_USER.
        let user = unsafe { &*storage.as_ptr().cast::<TOKEN_USER>() };
        let mut sid = std::ptr::null_mut();
        // SAFETY: the query's SID remains in live storage and sid is an output pointer.
        if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut sid) } == 0 {
            return Err(fail());
        }
        let sid_guard = Allocation(sid.cast());
        let mut units = Vec::new();
        for index in 0..256 {
            // SAFETY: Windows guarantees a terminated SID string (max SID string
            // length is below this bound); stop on the first terminator.
            let unit = unsafe { *sid.add(index) };
            if unit == 0 {
                break;
            }
            units.push(unit);
        }
        if units.len() == 256 {
            return Err(fail());
        }
        let user_sid = String::from_utf16(&units).map_err(|_| fail())?;
        drop(sid_guard);
        let descriptor: Vec<u16> = format!("D:P(A;;FA;;;SY)(A;;FA;;;{user_sid})")
            .encode_utf16()
            .chain([0])
            .collect();
        let mut security = std::ptr::null_mut();
        // SAFETY: fixed grammar and OS-generated SID; valid output address.
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                descriptor.as_ptr(),
                SDDL_REVISION_1,
                &mut security,
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(fail());
        }
        let security = Allocation(security);
        let file = super::native::open_with_security(parent, leaf, false, true, security.0)?;
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Security::{
            ACL_SIZE_INFORMATION, AclSizeInformation,
            Authorization::{GetSecurityInfo, SE_FILE_OBJECT},
            DACL_SECURITY_INFORMATION, GetAclInformation, GetSecurityDescriptorControl,
            GetSecurityDescriptorDacl, SE_DACL_PROTECTED,
        };
        let mut actual = std::ptr::null_mut();
        // SAFETY: live newly-created file and valid descriptor output pointer.
        if unsafe {
            GetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut actual,
            )
        } != 0
        {
            return Err(fail());
        }
        let actual = Allocation(actual);
        let mut control = 0;
        let mut revision = 0;
        // SAFETY: a successful GetSecurityInfo returned this live descriptor.
        if unsafe { GetSecurityDescriptorControl(actual.0, &mut control, &mut revision) } == 0
            || control & SE_DACL_PROTECTED == 0
        {
            return Err(fail());
        }
        fn acl_bytes(descriptor: *mut std::ffi::c_void) -> Result<Vec<u8>> {
            let mut present = 0;
            let mut defaulted = 0;
            let mut acl = std::ptr::null_mut();
            let denied = || {
                AppError::new(
                    ErrorCode::Io,
                    "The output filesystem did not enforce private key permissions. No secret was written.",
                )
            };
            // SAFETY: caller owns the descriptor; validate the ACL before reading
            // its bounded SDK-reported bytes. This contains permissions, no key.
            unsafe {
                if GetSecurityDescriptorDacl(descriptor, &mut present, &mut acl, &mut defaulted)
                    == 0
                    || present == 0
                    || acl.is_null()
                {
                    return Err(denied());
                }
                let mut information = ACL_SIZE_INFORMATION::default();
                if GetAclInformation(
                    acl,
                    (&mut information as *mut ACL_SIZE_INFORMATION).cast(),
                    std::mem::size_of_val(&information) as u32,
                    AclSizeInformation,
                ) == 0
                    || information.AceCount != 2
                    || !(8..=65536).contains(&information.AclBytesInUse)
                {
                    return Err(denied());
                }
                Ok(
                    std::slice::from_raw_parts(
                        acl.cast::<u8>(),
                        information.AclBytesInUse as usize,
                    )
                    .to_vec(),
                )
            }
        }
        if acl_bytes(actual.0)? != acl_bytes(security.0)? {
            return Err(fail());
        }
        Ok(file)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (parent, leaf);
        Err(AppError::new(
            ErrorCode::Unsupported,
            "Private recovery-key outputs have not been validated on this OS.",
        ))
    }
}

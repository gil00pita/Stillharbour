//! Test-only permission loss on an asserted generated ordinary file. A held
//! restoration handle and original descriptor survive the access denial.
use super::*;

pub(super) struct DeniedSource {
    run: PathBuf,
    run_chain: DirectoryChain,
    path: PathBuf,
    parent: DirectoryChain,
    file: File,
    identity: FileIdentity,
    original: MetadataManifest,
    saved: platform::Saved,
    armed: bool,
    retain: Arc<AtomicBool>,
}
impl DeniedSource {
    pub(super) fn new(run: &Path, path: &Path, retain: Arc<AtomicBool>) -> Self {
        let run = run.canonicalize().unwrap();
        let path = path.canonicalize().unwrap();
        assert!(path.starts_with(&run) && path != run);
        let run_chain = DirectoryChain::open(&run).unwrap();
        assert!(
            !std::fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        let parent = DirectoryChain::open(path.parent().unwrap()).unwrap();
        let file = platform::open(&path).unwrap();
        let identity = filesystem::identity(&file).unwrap();
        assert_eq!(identity.links, 1);
        let original = MetadataManifest::inspect(&file).unwrap();
        let saved = platform::save(&file).unwrap();
        Self {
            run,
            run_chain,
            path,
            parent,
            file,
            identity,
            original,
            saved,
            armed: false,
            retain,
        }
    }
    fn validate(&self) -> Result<()> {
        if !self.path.starts_with(&self.run) || filesystem::identity(&self.file)? != self.identity {
            return Err(conflict(
                "Generated source permission fixture changed native identity.",
            ));
        }
        let parent = DirectoryChain::open(self.path.parent().unwrap())?;
        let same = |a: &FileIdentity, b: &FileIdentity| a.volume == b.volume && a.file == b.file;
        if parent.identities().len() != self.parent.identities().len()
            || !parent
                .identities()
                .iter()
                .zip(self.parent.identities())
                .all(|(a, b)| same(a, b))
            || !parent
                .identities()
                .iter()
                .any(|id| same(id, self.run_chain.identity()))
            || !same(
                DirectoryChain::open(&self.run)?.identity(),
                self.run_chain.identity(),
            )
        {
            return Err(conflict(
                "Generated source permission fixture changed containment.",
            ));
        }
        platform::member(
            parent.leaf(),
            self.path.file_name().unwrap(),
            &self.identity,
        )?;
        Ok(())
    }
    pub(super) fn deny(&mut self) -> Result<()> {
        self.validate()?;
        self.armed = true;
        platform::deny(&self.file)
    }
    pub(super) fn restore(&mut self) -> Result<()> {
        let restored = (|| {
            self.validate()?;
            platform::restore(&self.file, &self.saved)?;
            self.original.compare(&self.file)
        })();
        if restored.is_ok() {
            self.armed = false;
        } else {
            self.retain.store(true, Ordering::Release);
        }
        restored
    }
}
impl Drop for DeniedSource {
    fn drop(&mut self) {
        if self.armed && self.restore().is_err() {
            self.retain.store(true, Ordering::Release);
            eprintln!(
                "Generated source permissions could not be restored; the fixture remains retained."
            );
        }
    }
}
#[cfg(windows)]
mod platform {
    use super::*;
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    use windows_sys::{
        Wdk::Storage::FileSystem::NtSetSecurityObject,
        Win32::{
            Foundation::LocalFree,
            Security::{Authorization::*, *},
            Storage::FileSystem::*,
        },
    };
    pub(super) struct Saved(Vec<u32>);
    pub(super) fn member(
        parent: &File,
        name: &std::ffi::OsStr,
        expected: &FileIdentity,
    ) -> Result<()> {
        let entry = filesystem::windows::destination_entry(parent, name)?
            .ok_or_else(|| conflict("Generated permission source disappeared."))?;
        if entry.name != name
            || entry.file_id != expected.file
            || filesystem::identity(parent)?.volume != expected.volume
            || entry.attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0
        {
            return Err(conflict(
                "Generated permission source changed native membership.",
            ));
        }
        Ok(())
    }
    fn failed() -> AppError {
        AppError::new(ErrorCode::Io, "Generated native permission fixture failed.")
            .caused_by(std::io::Error::last_os_error())
    }
    pub(super) fn open(path: &Path) -> Result<File> {
        // WRITE_DAC is acquired BEFORE denial. Deny write-data/delete sharing so
        // this exact ordinary file cannot be replaced while the test owns it.
        Ok(std::fs::OpenOptions::new()
            .access_mode(FILE_GENERIC_READ | WRITE_DAC)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)?)
    }
    pub(super) fn save(file: &File) -> Result<Saved> {
        let mut raw = std::ptr::null_mut();
        // SAFETY: live generated file and system-allocated descriptor output.
        let status = unsafe {
            GetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut raw,
            )
        };
        if status != 0 || raw.is_null() {
            return Err(failed());
        }
        // SAFETY: successful GetSecurityInfo allocated a valid descriptor.
        let length = unsafe { GetSecurityDescriptorLength(raw) } as usize;
        if !(20..=256 * 1024).contains(&length) {
            unsafe {
                LocalFree(raw);
            };
            return Err(failed());
        }
        let mut aligned = vec![0u32; length.div_ceil(4)];
        // SAFETY: bounded live system descriptor and adequately sized aligned copy.
        unsafe {
            std::ptr::copy_nonoverlapping(
                raw.cast::<u8>(),
                aligned.as_mut_ptr().cast::<u8>(),
                length,
            );
            LocalFree(raw);
        }
        Ok(Saved(aligned))
    }
    pub(super) fn deny(file: &File) -> Result<()> {
        let mut acl = std::mem::MaybeUninit::<ACL>::zeroed();
        // SAFETY: exact ACL storage; an initialized empty DACL denies fresh
        // payload reads. Only this constructor-owned generated FILE is changed.
        if unsafe {
            InitializeAcl(
                acl.as_mut_ptr(),
                std::mem::size_of::<ACL>() as u32,
                ACL_REVISION,
            )
        } == 0
        {
            return Err(failed());
        }
        if unsafe {
            SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                acl.as_ptr(),
                std::ptr::null(),
            )
        } != 0
        {
            return Err(failed());
        }
        Ok(())
    }
    pub(super) fn restore(file: &File, saved: &Saved) -> Result<()> {
        let mut descriptor = saved.0.clone();
        // SAFETY: aligned descriptor was copied from GetSecurityInfo, never IPC.
        let bytes = unsafe {
            std::slice::from_raw_parts_mut(
                descriptor.as_mut_ptr().cast::<u8>(),
                descriptor.len() * 4,
            )
        };
        let control = u16::from_le_bytes([bytes[2], bytes[3]]);
        let offset = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize;
        if offset < 20 || offset + 8 > bytes.len() {
            return Err(failed());
        }
        let protection = if control & SE_DACL_PROTECTED != 0 {
            PROTECTED_DACL_SECURITY_INFORMATION
        } else {
            UNPROTECTED_DACL_SECURITY_INFORMATION
        };
        // SAFETY: bounded system-derived DACL; restore protection and exact ACEs
        // on the same held generated file, without a tree propagation API.
        if unsafe {
            SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | protection,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                bytes.as_ptr().add(offset).cast(),
                std::ptr::null(),
            )
        } != 0
        {
            return Err(failed());
        }
        if control & SE_DACL_AUTO_INHERITED != 0 {
            bytes[2..4].copy_from_slice(&(control | SE_DACL_AUTO_INHERIT_REQ).to_le_bytes());
        }
        if unsafe {
            NtSetSecurityObject(
                file.as_raw_handle(),
                DACL_SECURITY_INFORMATION,
                descriptor.as_mut_ptr().cast(),
            )
        } != 0
        {
            return Err(failed());
        }
        Ok(())
    }
}
#[cfg(not(windows))]
mod platform {
    use super::*;
    use std::os::{fd::AsRawFd, unix::fs::MetadataExt};
    pub(super) struct Saved(u32);
    pub(super) fn member(
        parent: &File,
        name: &std::ffi::OsStr,
        expected: &FileIdentity,
    ) -> Result<()> {
        use std::os::unix::ffi::OsStrExt;
        let name = std::ffi::CString::new(name.as_bytes())
            .map_err(|_| invalid("Invalid generated source component."))?;
        let stat = rustix::fs::statat(parent, &name, rustix::fs::AtFlags::SYMLINK_NOFOLLOW)
            .map_err(std::io::Error::from)?;
        #[cfg(target_os = "macos")]
        let volume = stat.st_dev as u64;
        #[cfg(not(target_os = "macos"))]
        let volume = stat.st_dev;
        if volume != expected.volume
            || stat.st_ino != expected.file
            || stat.st_nlink != 1
            || rustix::fs::FileType::from_raw_mode(stat.st_mode)
                != rustix::fs::FileType::RegularFile
        {
            return Err(conflict(
                "Generated permission source changed native membership.",
            ));
        }
        Ok(())
    }
    pub(super) fn open(path: &Path) -> Result<File> {
        filesystem::open_readonly(path)
    }
    pub(super) fn save(file: &File) -> Result<Saved> {
        Ok(Saved(file.metadata()?.mode() & 0o7777))
    }
    fn mode(file: &File, mode: u32) -> Result<()> {
        // SAFETY: exact held generated file; no path or privilege changes.
        if unsafe { libc::fchmod(file.as_raw_fd(), mode as libc::mode_t) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    }
    pub(super) fn deny(file: &File) -> Result<()> {
        mode(file, 0)
    }
    pub(super) fn restore(file: &File, saved: &Saved) -> Result<()> {
        mode(file, saved.0)
    }
}

#[cfg(all(test, any(windows, target_os = "macos")))]
mod tests {
    use super::*;
    struct OwnedRun {
        run: tempfile::TempDir,
        retain: Arc<AtomicBool>,
    }
    impl Drop for OwnedRun {
        fn drop(&mut self) {
            if std::thread::panicking() || self.retain.load(Ordering::Acquire) {
                self.run.disable_cleanup(true);
            }
        }
    }
    #[test]
    fn generated_permission_fixture_keeps_native_containment_after_owned_sibling_creation() {
        let owned = OwnedRun {
            run: tempfile::tempdir().unwrap(),
            retain: Arc::new(AtomicBool::new(false)),
        };
        let run = owned.run.path().canonicalize().unwrap();
        let source = run.join("generated-source");
        std::fs::create_dir(&source).unwrap();
        let path = source.join("generated-file.bin");
        std::fs::write(&path, b"retained generated source bytes").unwrap();
        let mut denial = DeniedSource::new(&run, &path, owned.retain.clone());
        std::fs::create_dir(run.join("owned-staging-sibling")).unwrap();
        std::fs::create_dir(source.join("owned-source-sibling")).unwrap();
        denial.validate().unwrap();
        denial.deny().unwrap();
        assert!(filesystem::open_readonly(&path).is_err());
        denial.restore().unwrap();
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"retained generated source bytes"
        );
        assert!(!owned.retain.load(Ordering::Acquire));
    }
}

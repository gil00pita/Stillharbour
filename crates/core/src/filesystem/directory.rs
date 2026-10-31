//! Ordinary directory chains opened component by component. No writes.
#[cfg(not(windows))]
use super::open_readonly;
use super::{FileIdentity, identity};
use crate::domain::{AppError, ErrorCode, Result};
use std::{
    fs::File,
    path::{Component, Path, PathBuf},
};

pub(crate) struct DirectoryChain {
    handles: Vec<File>,
    identities: Vec<FileIdentity>,
}
impl DirectoryChain {
    pub(crate) fn open(path: &Path) -> Result<Self> {
        Self::open_inner(path, false)
    }
    #[cfg(windows)]
    pub(crate) fn open_for_owned_publication(path: &Path) -> Result<Self> {
        Self::open_inner(path, true)
    }
    fn open_inner(path: &Path, _owned_publication: bool) -> Result<Self> {
        if !path.is_absolute() || path.components().count() > 96 {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Choose an absolute folder with at most 96 components.",
            ));
        }
        let mut prefix = PathBuf::new();
        let mut chain = Self {
            handles: Vec::new(),
            identities: Vec::new(),
        };
        let _count = path.components().count();
        for (_index, component) in path.components().enumerate() {
            let handle = match component {
                Component::Prefix(_) if chain.handles.is_empty() => {
                    prefix.push(component);
                    continue;
                }
                Component::RootDir if chain.handles.is_empty() => {
                    prefix.push(component);
                    #[cfg(windows)]
                    let file = super::windows::open_root(&prefix)?;
                    #[cfg(not(windows))]
                    let file = open_readonly(&prefix)?;
                    file
                }
                Component::Normal(name) if !chain.handles.is_empty() => {
                    #[cfg(windows)]
                    let file = if _owned_publication && _index + 1 == _count {
                        super::windows::owned_directory_child(chain.leaf(), name)?
                    } else {
                        super::windows::child(chain.leaf(), name, true)?
                    };
                    #[cfg(unix)]
                    let file = {
                        use rustix::fs::{Mode, OFlags, openat};
                        use std::os::unix::ffi::OsStrExt;
                        let name = std::ffi::CString::new(name.as_bytes()).map_err(|_| {
                            AppError::new(
                                ErrorCode::InvalidInput,
                                "Invalid native folder component.",
                            )
                        })?;
                        File::from(
                            openat(
                                chain.leaf(),
                                &name,
                                OFlags::RDONLY
                                    | OFlags::DIRECTORY
                                    | OFlags::NOFOLLOW
                                    | OFlags::NONBLOCK
                                    | OFlags::CLOEXEC,
                                Mode::empty(),
                            )
                            .map_err(std::io::Error::from)?,
                        )
                    };
                    file
                }
                _ => {
                    return Err(AppError::new(
                        ErrorCode::InvalidInput,
                        "Folder paths must not contain relative traversal.",
                    ));
                }
            };
            if !handle.metadata()?.is_dir() {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "Choose an ordinary directory.",
                ));
            }
            #[cfg(target_os = "macos")]
            {
                use std::os::macos::fs::MetadataExt;
                if handle.metadata()?.st_flags() & 0x40000000 != 0 {
                    return Err(AppError::new(
                        ErrorCode::Unsupported,
                        "Cloud folder placeholders are not downloaded.",
                    ));
                }
            }
            chain.identities.push(identity(&handle)?);
            chain.handles.push(handle);
        }
        if chain.handles.is_empty() {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Choose an ordinary directory.",
            ));
        }
        Ok(chain)
    }
    pub(crate) fn leaf(&self) -> &File {
        self.handles.last().expect("nonempty directory chain")
    }
    pub(crate) fn identity(&self) -> &FileIdentity {
        self.identities.last().expect("nonempty directory chain")
    }
    pub(crate) fn overlaps(&self, other: &Self) -> bool {
        let same = |a: &FileIdentity, b: &FileIdentity| a.volume == b.volume && a.file == b.file;
        self.identities.iter().any(|id| same(id, other.identity()))
            || other.identities.iter().any(|id| same(id, self.identity()))
    }
}

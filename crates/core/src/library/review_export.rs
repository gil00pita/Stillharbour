//! Explicit metadata adapter for newly generated plan-review files. This never
//! copies, renames, replaces or deletes a photo, sidecar or user-created file.
use super::{Library, native, store};
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem::{self, DirectoryChain, FileIdentity},
    operations::{PlanExportFormat, PlanExportSaved},
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

const MAX_EXPORT: usize = 16 * 1024 * 1024;

fn conflict(message: &str) -> AppError {
    AppError::new(ErrorCode::Conflict, message)
}

fn same(a: &FileIdentity, b: &FileIdentity) -> bool {
    a.volume == b.volume && a.file == b.file
}

fn output_chain(path: &Path) -> Result<DirectoryChain> {
    if path.components().any(|part| {
        part.as_os_str()
            .to_string_lossy()
            .to_ascii_lowercase()
            .ends_with(".photoslibrary")
    }) {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Save review files outside Apple Photos library packages.",
        ));
    }
    #[cfg(windows)]
    {
        DirectoryChain::open_for_owned_publication(path)
    }
    #[cfg(not(windows))]
    {
        DirectoryChain::open(path)
    }
}

impl Library {
    pub fn organization_plan_save_export(
        &self,
        id: String,
        format: PlanExportFormat,
        path: PathBuf,
        expected: (FileIdentity, String),
    ) -> Result<PlanExportSaved> {
        let _admission = self.destination_gate.try_lock().map_err(|_| {
            AppError::new(
                ErrorCode::Busy,
                "Another destination operation is running. Try again after it finishes.",
            )
        })?;
        let export = self.organization_plan_export(id.clone(), format)?;
        if path.components().any(|part| {
            part.as_os_str()
                .to_string_lossy()
                .to_ascii_lowercase()
                .ends_with(".photoslibrary")
        }) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Save review files outside Apple Photos library packages.",
            ));
        }
        if export.text.len() > MAX_EXPORT {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "The review export exceeds 16 MiB.",
            ));
        }
        let selected = native::open_root(&path, Some(&expected.0))?;
        if native::volume_token(&selected)? != expected.1 {
            return Err(conflict("The selected export volume changed."));
        }
        let path = path.canonicalize()?;
        // The selected object was checked above. Independently reacquire its
        // chain and compare identity. Only the output leaf admits directory
        // writers; ancestors still refuse delete/reparse replacement handles.
        drop(selected);
        let directory = output_chain(&path)?;
        if !same(directory.identity(), &expected.0)
            || native::volume_token(directory.leaf())? != expected.1
        {
            return Err(conflict("The selected export folder changed."));
        }
        let app_data = DirectoryChain::open(&self.options.data_directory.canonicalize()?)?;
        if directory.overlaps(&app_data) {
            return Err(conflict(
                "Save review files outside application data and cache folders.",
            ));
        }
        let roots = store::roots(&self.catalog)?;
        if roots.len() > 32 {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "Too many library roots for export overlap review.",
            ));
        }
        let mut held = Vec::new();
        for root in roots {
            let chain = DirectoryChain::open(&root.path.canonicalize()?)?;
            if !same(chain.identity(), &root.identity)
                || native::volume_token(chain.leaf())? != root.volume
            {
                return Err(conflict(
                    "An indexed root changed identity. Reconnect or relink it before saving review files.",
                ));
            }
            if directory.overlaps(&chain) {
                return Err(conflict(
                    "Save review files outside every indexed library, without ancestor overlap.",
                ));
            }
            held.push(chain);
        }
        let token = uuid::Uuid::new_v4().to_string();
        let stage = format!(".stillharbour-review-{token}.partial");
        let digest = blake3::hash(export.text.as_bytes()).to_hex().to_string();
        // The immutable review text is regenerable from the retained catalog.
        // Journal intent before output creation; ambiguous outcomes never cause
        // automatic replacement, stage deletion or replay of source mutations.
        let evidence = serde_json::json!({"version":1,"exportId":token,"directoryNative":native::encode_path(&path),
            "directoryVolume":expected.1,"directoryDevice":expected.0.volume,"directoryFile":expected.0.file,
            "stage":stage,"filename":export.filename,"digest":digest,"bytes":export.text.len().to_string()});
        export_event(self, &id, "review_export_intent", evidence.clone())?;
        let result = save_new(
            &path,
            &directory,
            &stage,
            &export.filename,
            export.text.as_bytes(),
        );
        match result {
            Ok(()) => {
                let current = DirectoryChain::open(&path)?;
                if !same(current.identity(), directory.identity())
                    || native::volume_token(current.leaf())? != expected.1
                {
                    return Err(conflict(
                        "The export folder changed. Review the recorded export before retrying.",
                    ));
                }
                export_event(self, &id, "review_export_completed", evidence)?;
                Ok(PlanExportSaved {
                    path: path.join(export.filename).to_string_lossy().into_owned(),
                    bytes: export.text.len().to_string(),
                    digest,
                })
            }
            Err(error) => {
                let _ = export_event(
                    self,
                    &id,
                    "review_export_needs_attention",
                    serde_json::json!({"exportId":token,"code":error.code}),
                );
                Err(AppError::new(error.code.clone(),
                    "The review file could not be saved. Existing files were retained; any partial export is retained for review. Choose a different folder if the filename already exists.").caused_by(error))
            }
        }
    }
}

fn export_event(
    library: &Library,
    id: &str,
    phase: &str,
    evidence: serde_json::Value,
) -> Result<()> {
    let id = id.to_owned();
    let phase = phase.to_owned();
    library.catalog.call(move |db| {
        db.execute(
            "INSERT INTO operation_journal(plan_id,phase,evidence,created_at) VALUES(?1,?2,?3,?4)",
            rusqlite::params![id, phase, evidence.to_string(), store::now()],
        )?;
        Ok(())
    })
}

fn save_new(
    path: &Path,
    directory: &DirectoryChain,
    stage: &str,
    filename: &str,
    bytes: &[u8],
) -> Result<()> {
    // Only fixed backend-generated UUID names enter this adapter.
    if !stage.starts_with(".stillharbour-review-")
        || !stage.ends_with(".partial")
        || !filename.starts_with("stillharbour-plan-")
        || !(filename.ends_with(".json") || filename.ends_with(".csv"))
        || [stage, filename]
            .iter()
            .any(|name| name.contains(['/', '\\', ':', '\0']) || name.len() > 255)
        || bytes.len() > MAX_EXPORT
    {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid review export output.",
        ));
    }
    #[cfg(windows)]
    let mut file = {
        use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};
        use windows_sys::Win32::Storage::FileSystem::*;
        OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(
                FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_OPEN_NO_RECALL | FILE_FLAG_WRITE_THROUGH,
            )
            .open(path.join(stage))?
    };
    #[cfg(unix)]
    let mut file = {
        use rustix::fs::{Mode, OFlags, openat};
        File::from(
            openat(
                directory.leaf(),
                stage,
                OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            )
            .map_err(std::io::Error::from)?,
        )
    };
    let expected = filesystem::identity(&file)?;
    if expected.links != 1 || !same_volume(directory.identity(), &expected) {
        return Err(conflict(
            "The owned review stage has an unexpected identity.",
        ));
    }
    file.write_all(bytes)?;
    file.sync_all()?;
    verify(&mut file, bytes)?;
    #[cfg(windows)]
    {
        drop(file);
        // Reuse only the app-produced output publication adapter; this extends
        // its former snapshot-only scope, never generic original operations.
        let mut published = filesystem::catalog_snapshot::publish(
            &path.join(stage),
            &path.join(filename),
            &expected,
        )?;
        verify(&mut published, bytes)?;
    }
    #[cfg(target_os = "macos")]
    {
        use std::{ffi::CString, os::fd::AsRawFd};
        let from = CString::new(stage).map_err(|_| conflict("Invalid review stage name."))?;
        let to = CString::new(filename).map_err(|_| conflict("Invalid review filename."))?;
        // SAFETY: live held directory descriptor and bounded owned C strings;
        // RENAME_EXCL forbids replacing any existing destination object.
        if unsafe {
            libc::renameatx_np(
                directory.leaf().as_raw_fd(),
                from.as_ptr(),
                directory.leaf().as_raw_fd(),
                to.as_ptr(),
                libc::RENAME_EXCL,
            )
        } != 0
        {
            let error = std::io::Error::last_os_error();
            return Err(AppError::new(
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    ErrorCode::Conflict
                } else {
                    ErrorCode::Io
                },
                "Review-file publication failed; existing outputs are retained.",
            )
            .caused_by(error));
        }
        directory.leaf().sync_all()?;
        let mut published = native::child_file(directory.leaf(), &to, false)?;
        if !same(&filesystem::identity(&published)?, &expected) {
            return Err(conflict("The published review identity changed."));
        }
        verify(&mut published, bytes)?;
        let _ = path;
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (path, filename);
        return Err(AppError::new(
            ErrorCode::Unsupported,
            "Review export publication needs native platform validation.",
        ));
    }
    Ok(())
}

fn same_volume(a: &FileIdentity, b: &FileIdentity) -> bool {
    a.volume == b.volume
}

fn verify(file: &mut File, expected: &[u8]) -> Result<()> {
    file.seek(SeekFrom::Start(0))?;
    if file.metadata()?.len() != expected.len() as u64 {
        return Err(conflict("Review-file byte verification failed."));
    }
    let mut buffer = [0u8; 65536];
    for chunk in expected.chunks(buffer.len()) {
        file.read_exact(&mut buffer[..chunk.len()])?;
        if &buffer[..chunk.len()] != chunk {
            return Err(conflict("Review-file byte verification failed."));
        }
    }
    if file.read(&mut buffer[..1])? != 0 {
        return Err(conflict("The review file changed during verification."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn review_export_ipc_never_accepts_renderer_paths_names_bytes_or_overwrite_flags() {
        let valid = serde_json::json!({"kind":"organization_plan_save_export","selection_id":"native-session", "plan_id":uuid::Uuid::new_v4().to_string(), "format":"json"});
        assert!(
            serde_json::from_value::<crate::library::model::LibraryRequest>(valid.clone()).is_ok()
        );
        for field in [
            "path",
            "filename",
            "text",
            "bytes",
            "overwrite",
            "force",
            "approved",
        ] {
            let mut hostile = valid.clone();
            hostile[field] = serde_json::json!("unapproved");
            assert!(
                serde_json::from_value::<crate::library::model::LibraryRequest>(hostile).is_err()
            );
        }
    }
    #[test]
    fn review_export_publishes_exact_bytes_and_never_replaces_existing_files() {
        let fixture = tempfile::tempdir().unwrap();
        let path = fixture.path().canonicalize().unwrap();
        std::fs::write(path.join("destination-only"), b"retain").unwrap();
        let directory = output_chain(&path).unwrap();
        let filename = format!("stillharbour-plan-{}.json", uuid::Uuid::new_v4());
        let stage = format!(".stillharbour-review-{}.partial", uuid::Uuid::new_v4());
        save_new(&path, &directory, &stage, &filename, b"{\"review\":true}").unwrap();
        assert_eq!(
            std::fs::read(path.join(&filename)).unwrap(),
            b"{\"review\":true}"
        );
        assert!(!path.join(&stage).exists());
        let next = format!(".stillharbour-review-{}.partial", uuid::Uuid::new_v4());
        assert_eq!(
            save_new(&path, &directory, &next, &filename, b"replacement")
                .unwrap_err()
                .code,
            ErrorCode::Conflict
        );
        assert_eq!(
            std::fs::read(path.join(&filename)).unwrap(),
            b"{\"review\":true}"
        );
        assert_eq!(std::fs::read(path.join(next)).unwrap(), b"replacement");
        assert_eq!(
            std::fs::read(path.join("destination-only")).unwrap(),
            b"retain"
        );
    }
}

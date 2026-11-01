//! Instrumented setup cleanup only. No cache/original file is removed and no
//! renderer command accepts these arguments. The desktop owns the TempDir/ID.
use super::{Library, native, original_jobs};
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem::DirectoryChain,
    jobs::operation_locks::{Access, Registry, Scope},
};
use std::path::Path;

pub fn remove_empty_registration(library: &Library, id: String, owned_path: &Path) -> Result<()> {
    crate::operations::validate_id(&id)?;
    let path = owned_path.canonicalize()?;
    if !path
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.starts_with("stillharbour-managed-destination-test-"))
    {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Only the constructor-owned managed destination can be unregistered.",
        ));
    }
    let directory = DirectoryChain::open(&path)?;
    let identity = directory.identity().clone();
    let volume = native::volume_token(directory.leaf())?;
    let _lease = Registry::shared().try_acquire(vec![Scope::new(directory, Access::Write)])?;
    let encoded = native::encode_path(&path);
    library.catalog.call(move |db| {
        let tx=db.transaction()?;
        let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM roots WHERE id=?1 AND native_path=?2 AND device=?3 AND inode=?4 AND volume=?5 AND label='Generated managed copies destination' AND mode='managed' AND is_base=0 AND availability='online')",rusqlite::params![id,encoded,identity.volume.to_string(),identity.file.to_string(),volume],|row|row.get(0))?;
        if !valid {return Err(AppError::new(ErrorCode::Conflict,"Generated destination registration changed; retain it for review."));}
        original_jobs::require_reconciled_root_db(&tx,&id)?;
        let nonempty:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM assets WHERE root_id=?1) OR EXISTS(SELECT 1 FROM scans WHERE root_id=?1 AND state IN ('queued','running','cancel_requested'))",[&id],|row|row.get(0))?;
        if nonempty {return Err(AppError::new(ErrorCode::Busy,"Wait for the empty generated destination's jobs to stop; no assets or cache entries may be removed."));}
        // With zero assets, no asset/selection rows are removed. Leave the
        // cache index and files untouched, including any unowned orphan entry.
        // This transaction cannot invoke the unsupported cache adapter.
        tx.execute("DELETE FROM scan_issues WHERE job_id IN (SELECT id FROM scans WHERE root_id=?)",[&id])?;
        tx.execute("DELETE FROM scans WHERE root_id=?",[&id])?;
        tx.execute("DELETE FROM directories WHERE root_id=?",[&id])?;
        tx.execute("DELETE FROM roots WHERE id=?",[&id])?;
        tx.execute("UPDATE library_state SET revision=revision+1 WHERE singleton=1",[])?;
        tx.commit()?; Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        catalog::Catalog,
        filesystem,
        library::{
            LibraryOptions, WorkerPaths,
            model::{RootMode, ScanFilter},
            store,
        },
    };
    fn register(library: &Library, path: &Path, label: &str) -> String {
        let path = path.canonicalize().unwrap();
        let file = native::open_root(&path, None).unwrap();
        store::add_root(
            &library.catalog,
            path,
            filesystem::identity(&file).unwrap(),
            native::volume_token(&file).unwrap(),
            label.into(),
            RootMode::Managed,
            ScanFilter::default(),
        )
        .unwrap()
    }
    #[test]
    fn generated_metadata_cleanup_refuses_foreign_bindings_and_active_work_without_touching_files()
    {
        let run = tempfile::tempdir().unwrap();
        let data = run.path().join("data");
        std::fs::create_dir(&data).unwrap();
        let library = Library::new(
            std::sync::Arc::new(Catalog::open(data.join("Fixture.stillharbour")).unwrap()),
            LibraryOptions {
                data_directory: data.clone(),
                workers: WorkerPaths {
                    executable: data.join("must-not-start"),
                    exiftool: None,
                    working_directory: data.join("worker"),
                },
            },
        )
        .unwrap();
        let source = run.path().join("source");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("source.txt"), b"original retained").unwrap();
        let base = register(&library, &source, "Generated source");
        let owned = tempfile::Builder::new()
            .prefix("stillharbour-managed-destination-test-")
            .tempdir()
            .unwrap();
        std::fs::write(
            owned.path().join("destination-only.txt"),
            b"neighbor retained",
        )
        .unwrap();
        let id = register(
            &library,
            owned.path(),
            "Generated managed copies destination",
        );
        assert!(remove_empty_registration(&library, base.clone(), owned.path()).is_err());
        assert!(remove_empty_registration(&library, id.clone(), &source).is_err());
        let job = store::begin_job(&library.catalog, id.clone(), ScanFilter::default()).unwrap();
        assert!(remove_empty_registration(&library, id.clone(), owned.path()).is_err());
        assert!(library.root_exists(id.clone()).unwrap());
        store::finish_job(
            &library.catalog,
            job.id,
            "completed".into(),
            "Generated empty observation".into(),
        )
        .unwrap();
        // begin_job can retain an initial preparation record; stop it as setup
        // metadata, without launching a decoder or deleting any cache object.
        store::stop_initial_preparation(
            &library.catalog,
            id.clone(),
            "completed".into(),
            "No generated media".into(),
        )
        .unwrap();
        remove_empty_registration(&library, id.clone(), owned.path()).unwrap();
        assert!(!library.root_exists(id).unwrap());
        assert!(library.root_exists(base).unwrap());
        assert_eq!(
            std::fs::read(source.join("source.txt")).unwrap(),
            b"original retained"
        );
        assert_eq!(
            std::fs::read(owned.path().join("destination-only.txt")).unwrap(),
            b"neighbor retained"
        );
    }
}

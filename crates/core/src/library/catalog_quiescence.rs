//! Native catalog activation barrier. The shell closes RPC admission first.
use super::*;
use crate::{
    filesystem::DirectoryChain,
    jobs::{
        JobPermit,
        operation_locks::{Access, Lease, Registry, Scope},
    },
};

pub struct CatalogRestoreLease<'a> {
    _cache: std::sync::RwLockWriteGuard<'a, ()>,
    _scan: JobPermit,
    _thumbnails: JobPermit,
    _original: JobPermit,
    _scopes: Lease,
}
fn stopped() -> AppError {
    AppError::new(
        ErrorCode::Busy,
        "Catalog recovery is waiting for native work to stop. Existing catalogs and files remain retained; close and reopen Stillharbour before trying again.",
    )
}
impl Library {
    /// Permanently closes ordinary library admission for this session, drains
    /// owned workers, then freezes actor writes. A failure does not reopen it.
    pub fn quiesce_for_catalog_restore(&self) -> Result<CatalogRestoreLease<'_>> {
        if self.maintenance_pending.swap(true, Ordering::AcqRel) {
            return Err(stopped());
        }
        self.cancel_all();
        let roots = store::roots(&self.catalog)?;
        if roots.len() > 32 {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "Catalog activation currently supports at most 32 roots; existing catalogs remain unchanged.",
            ));
        }
        let until = Instant::now() + Duration::from_secs(30);
        loop {
            #[cfg(any(test, feature = "copy-workflow"))]
            let mut copy_busy = false;
            #[cfg(not(any(test, feature = "copy-workflow")))]
            let copy_busy = false;
            #[cfg(any(test, feature = "copy-workflow"))]
            for root in &roots {
                copy_busy |= self.copy_workflow.cancel_root(&root.view.id)?;
            }
            {
                let mut queue = self.thumbnails.state.lock().map_err(|_| lock_error())?;
                queue.cancel.store(true, Ordering::Release);
                queue.pending.clear();
            }
            let monitor_done = self
                .monitor
                .lock()
                .map_err(|_| lock_error())?
                .as_ref()
                .is_none_or(|m| m.done.load(Ordering::Acquire));
            let idle = !copy_busy
                && monitor_done
                && self.scan_idle()
                && self.backup_shutdown_ready()?
                && self.inspections.idle()?
                && self.comparisons.idle()?
                && self
                    .thumbnail_jobs
                    .lock()
                    .map_err(|_| lock_error())?
                    .is_empty()
                && self
                    .thumbnails
                    .state
                    .lock()
                    .map_err(|_| lock_error())?
                    .inflight
                    .is_empty();
            if idle
                && let (Ok(scan), Ok(thumbnails), Ok(original), Ok(cache)) = (
                    self.scan_gate.try_start(),
                    self.thumbnail_job_gate.try_start(),
                    self.original_gate.try_start(),
                    self.cache_gate.try_write(),
                )
            {
                let mut scopes = vec![Scope::new(
                    DirectoryChain::open(&self.options.data_directory)?,
                    Access::Write,
                )];
                for root in &roots {
                    let held = native::open_root(&root.path, Some(&root.identity))?;
                    if native::volume_token(&held)? != root.volume {
                        return Err(stopped());
                    }
                    scopes.push(Scope::new(DirectoryChain::open(&root.path)?, Access::Write));
                }
                match Registry::shared().try_acquire(scopes) {
                    Ok(scopes) => {
                        self.catalog.freeze_writes_for_restore()?;
                        return Ok(CatalogRestoreLease {
                            _cache: cache,
                            _scan: scan,
                            _thumbnails: thumbnails,
                            _original: original,
                            _scopes: scopes,
                        });
                    }
                    Err(error) if error.code == ErrorCode::Busy => {}
                    Err(error) => return Err(error),
                }
            }
            if Instant::now() >= until {
                return Err(stopped());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_restore_quiescence_waits_for_reader_and_monitor_and_retains_last_edit() {
        let owned = tempfile::tempdir().unwrap();
        let data = owned.path().canonicalize().unwrap();
        let catalog = Arc::new(Catalog::open(data.join("Generated.stillharbour")).unwrap());
        let library = Library::new(
            catalog.clone(),
            LibraryOptions {
                data_directory: data.clone(),
                workers: workers::WorkerPaths {
                    executable: data.join("missing-worker"),
                    exiftool: None,
                    working_directory: data.join("worker"),
                },
            },
        )
        .unwrap();
        library.start_monitoring().unwrap();
        let original_reader = library.original_gate.try_start().unwrap();
        std::thread::scope(|scope| {
            let owner = library.clone();
            let work = scope.spawn(move || {
                let _lease = owner.quiesce_for_catalog_restore().unwrap();
                assert!(
                    owner
                        .monitor
                        .lock()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .done
                        .load(Ordering::Acquire)
                );
            });
            let until = Instant::now() + Duration::from_secs(5);
            while !library.maintenance_pending.load(Ordering::Acquire) {
                assert!(Instant::now() < until);
                std::thread::yield_now();
            }
            // An active owned reader must drain before the actor freeze. The
            // last already-admitted edit is retained before the barrier.
            catalog.call(|db| {
                let frozen: bool = db.query_row("PRAGMA query_only", [], |r| r.get(0))?;
                assert!(!frozen);
                db.execute("INSERT INTO job_events(job_id,state) VALUES('generated-last-edit','retained')", [])?;
                Ok(())
            }).unwrap();
            drop(original_reader);
            work.join().unwrap();
        });
        assert!(
            catalog
                .call(|db| {
                    db.execute("DELETE FROM job_events", [])?;
                    Ok(())
                })
                .is_err()
        );
        let saved: String = catalog
            .call(|db| {
                Ok(db.query_row(
                    "SELECT state FROM job_events WHERE job_id='generated-last-edit'",
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(saved, "retained");
        assert!(library.ensure_work_admission().is_err());
    }
}

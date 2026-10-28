//! One bounded database actor owns all catalog writes and the lifetime lock.
use crate::domain::{AppError, ErrorCode, Result};
use crate::library::read_cache::ReadCache;
use fs2::FileExt;
use rusqlite::{
    Connection, OpenFlags,
    backup::{Backup, StepResult},
};
use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
    sync::mpsc::{self, SyncSender},
    thread,
    time::Duration,
};

pub const SCHEMA_VERSION: u32 = 2;
const APPLICATION_ID: i32 = 0x53544842; // STHB

/// Read linked-build provenance without opening a user catalog or starting Tauri.
pub fn sqlite_diagnostics() -> Result<serde_json::Value> {
    let db = Connection::open_in_memory()?;
    let options = db
        .prepare("PRAGMA compile_options")?
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(serde_json::json!({
        "version": db.query_row("SELECT sqlite_version()", [], |row| row.get::<_, String>(0))?,
        "sourceId": db.query_row("SELECT sqlite_source_id()", [], |row| row.get::<_, String>(0))?,
        "compileOptions": options,
        "userCatalogOpened": false
    }))
}

#[derive(Debug, Clone)]
pub struct CatalogSummary {
    pub id: String,
    pub schema: u32,
    pub sqlite_version: String,
}

type CatalogWork = Box<dyn FnOnce(&mut Connection, &mut ReadCache) + Send>;

enum Request {
    Summary(SyncSender<Result<CatalogSummary>>),
    Snapshot(PathBuf, SyncSender<Result<()>>),
    Run(CatalogWork),
    Stop,
}

pub struct Catalog {
    sender: SyncSender<Request>,
    worker: Option<thread::JoinHandle<()>>,
    #[cfg(feature = "fixture-probes")]
    performance_volume: Option<u64>,
}

impl Catalog {
    pub fn open(path: PathBuf) -> Result<Self> {
        #[cfg(feature = "fixture-probes")]
        let performance_volume = crate::performance::path_volume(&path);
        let (sender, receiver) = mpsc::sync_channel(16);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("catalog".into())
            .spawn(move || {
                let opened = open_database(&path);
                match opened {
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                    }
                    Ok((mut connection, _lock)) => {
                        let mut cache = ReadCache::default();
                        if ready_tx.send(Ok(())).is_err() {
                            return;
                        }
                        while let Ok(request) = receiver.recv() {
                            match request {
                                Request::Summary(reply) => {
                                    let _ = reply.send(summary(&connection));
                                }
                                Request::Snapshot(path, reply) => {
                                    let _ = reply.send(snapshot(&connection, &path));
                                }
                                Request::Run(work) => work(&mut connection, &mut cache),
                                Request::Stop => break,
                            }
                        }
                    }
                }
            })?;
        ready_rx.recv().map_err(|_| stopped())??;
        Ok(Self {
            sender,
            worker: Some(worker),
            #[cfg(feature = "fixture-probes")]
            performance_volume,
        })
    }

    pub fn summary(&self) -> Result<CatalogSummary> {
        let (reply, response) = mpsc::sync_channel(1);
        self.sender
            .try_send(Request::Summary(reply))
            .map_err(|_| busy())?;
        response.recv().map_err(|_| stopped())?
    }

    pub(crate) fn call<T: Send + 'static>(
        &self,
        work: impl FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        self.cached(move |db, _cache| work(db))
    }

    pub(crate) fn cached<T: Send + 'static>(
        &self,
        work: impl FnOnce(&mut Connection, &mut ReadCache) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        use crate::performance::{self, Endpoint, Metric, Span};
        let _span = Span::new(Metric::CatalogCall, Endpoint::Catalog);
        let queued = performance::clock();
        #[cfg(feature = "fixture-probes")]
        let volume = performance::volume_alias(self.performance_volume);
        let (reply, response) = mpsc::sync_channel(1);
        self.sender
            .try_send(Request::Run(Box::new(move |connection, cache| {
                performance::since(Metric::CatalogQueueWait, Endpoint::Catalog, queued, 0);
                let _work = Span::new(Metric::CatalogWork, Endpoint::Catalog);
                #[cfg(feature = "fixture-probes")]
                let before = performance::catalog_counters(connection);
                let result = work(connection, cache);
                #[cfg(feature = "fixture-probes")]
                performance::catalog_io(connection, before, volume);
                let _ = reply.send(result);
            })))
            .map_err(|_| busy())?;
        response.recv().map_err(|_| stopped())?
    }

    /// Backend-owned destination only; M0 does not expose catalog export through IPC.
    pub fn snapshot(&self, destination: PathBuf) -> Result<()> {
        let (reply, response) = mpsc::sync_channel(1);
        self.sender
            .try_send(Request::Snapshot(destination, reply))
            .map_err(|_| busy())?;
        response.recv().map_err(|_| stopped())?
    }
}

impl Drop for Catalog {
    fn drop(&mut self) {
        let _ = self.sender.send(Request::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn busy() -> AppError {
    AppError::new(ErrorCode::Busy, "The catalog queue is full or unavailable.")
}
fn stopped() -> AppError {
    AppError::new(ErrorCode::Catalog, "The catalog worker stopped.")
}

fn open_database(path: &Path) -> Result<(Connection, std::fs::File)> {
    let _span = crate::performance::Span::new(
        crate::performance::Metric::CatalogOpen,
        crate::performance::Endpoint::Catalog,
    );
    if rusqlite::version_number() < 3_051_003 {
        return Err(AppError::new(
            ErrorCode::Unsupported,
            "SQLite must include the WAL-reset fix (3.51.3 or newer).",
        ));
    }
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path.with_extension("lock"))?;
    lock.try_lock_exclusive().map_err(|error| {
        AppError::new(ErrorCode::CatalogInUse, "This catalog is already open.").caused_by(error)
    })?;
    let mut connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.busy_timeout(Duration::from_secs(2))?;
    connection.execute_batch(
        "PRAGMA trusted_schema=OFF; PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;",
    )?;
    let version: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(AppError::new(
            ErrorCode::CatalogSchemaNewer,
            "This catalog requires a newer application.",
        ));
    }
    let app_id: i32 = connection.query_row("PRAGMA application_id", [], |row| row.get(0))?;
    if version == 0 {
        let objects: u32 = connection.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
            [],
            |row| row.get(0),
        )?;
        if app_id != 0 || objects != 0 {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Unrecognised catalog; no migration was attempted.",
            ));
        }
        let tx = connection.transaction()?;
        tx.execute_batch("CREATE TABLE catalog_identity (singleton INTEGER PRIMARY KEY CHECK(singleton=1), uuid TEXT NOT NULL UNIQUE); CREATE TABLE job_events (sequence INTEGER PRIMARY KEY, job_id TEXT NOT NULL, state TEXT NOT NULL);")?;
        tx.execute(
            "INSERT INTO catalog_identity VALUES (1, ?1)",
            [uuid::Uuid::new_v4().to_string()],
        )?;
        tx.pragma_update(None, "application_id", APPLICATION_ID)?;
        tx.pragma_update(None, "user_version", 1)?;
        tx.commit()?;
    } else if app_id != APPLICATION_ID {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "The file is not a Stillharbour catalog.",
        ));
    }
    let current: u32 = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current < SCHEMA_VERSION {
        // Retain an independently openable pre-migration catalog before changing
        // an existing schema. New empty catalogs have no prior user records.
        if version > 0 {
            let parent = path.parent().ok_or_else(|| {
                AppError::new(ErrorCode::InvalidInput, "Catalog has no parent directory.")
            })?;
            let retained = parent.join(format!(
                "pre-migration-v{current}-{}.stillharbour",
                uuid::Uuid::new_v4()
            ));
            snapshot(&connection, &retained)?;
        }
        crate::library::store::migrate(&mut connection)?;
    }
    connection.pragma_update(None, "journal_mode", "WAL")?;
    validate(&connection)?;
    Ok((connection, lock))
}

fn summary(connection: &Connection) -> Result<CatalogSummary> {
    Ok(CatalogSummary {
        id: connection.query_row(
            "SELECT uuid FROM catalog_identity WHERE singleton=1",
            [],
            |row| row.get(0),
        )?,
        schema: connection.query_row("PRAGMA user_version", [], |row| row.get(0))?,
        sqlite_version: rusqlite::version().into(),
    })
}

fn validate(connection: &Connection) -> Result<()> {
    let integrity: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    let foreign_key_error = connection.prepare("PRAGMA foreign_key_check")?.exists([])?;
    if integrity != "ok" || foreign_key_error {
        return Err(AppError::new(
            ErrorCode::Catalog,
            "Catalog validation failed.",
        ));
    }
    Ok(())
}

fn snapshot(source: &Connection, destination: &Path) -> Result<()> {
    let parent = destination.parent().ok_or_else(|| {
        AppError::new(ErrorCode::InvalidInput, "Snapshot requires a local parent.")
    })?;
    let staged = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut target = Connection::open(staged.path())?;
        target.execute_batch("PRAGMA trusted_schema=OFF; PRAGMA synchronous=FULL;")?;
        {
            let backup = Backup::new(source, &mut target)?;
            let mut done = false;
            for _ in 0..200 {
                if backup.step(128)? == StepResult::Done {
                    done = true;
                    break;
                }
                thread::sleep(Duration::from_millis(5));
            }
            if !done {
                return Err(AppError::new(
                    ErrorCode::Busy,
                    "Catalog snapshot timed out; existing files are preserved.",
                ));
            }
        }
        target.execute_batch("PRAGMA journal_mode=DELETE;")?;
        validate(&target)?;
        if summary(&target)?.id != summary(source)?.id {
            return Err(AppError::new(
                ErrorCode::Catalog,
                "Snapshot identity validation failed.",
            ));
        }
        target.close().map_err(|(_, error)| AppError::from(error))?;
    }
    staged.as_file().sync_all()?;
    // persist_noclobber is collision-safe but not a universal durability primitive.
    // Native no-replace + parent flush is used where verified; no overwrite fallback.
    crate::filesystem::publish_noreplace(staged.path(), destination)?;
    crate::filesystem::sync_directory(parent)?;
    let independently_opened =
        Connection::open_with_flags(destination, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    validate(&independently_opened)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn build_diagnostics_use_no_user_catalog() {
        let info = sqlite_diagnostics().unwrap();
        assert_eq!(info["version"], rusqlite::version());
        assert_eq!(info["userCatalogOpened"], false);
        assert!(info["sourceId"].as_str().unwrap().len() > 64);
        assert!(info["compileOptions"].as_array().unwrap().len() > 10);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn wal_snapshot_is_independently_openable_and_never_replaces() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("working.stillharbour");
        let catalog = Catalog::open(path.clone()).unwrap();
        assert!(
            matches!(Catalog::open(path.clone()), Err(error) if error.code == ErrorCode::CatalogInUse)
        );
        let id = catalog.summary().unwrap().id;
        let output = temp.path().join("Family.stillharbour");
        catalog.snapshot(output.clone()).unwrap();
        let before = std::fs::read(&output).unwrap();
        assert_eq!(
            catalog.snapshot(output.clone()).unwrap_err().code,
            ErrorCode::Conflict
        );
        assert_eq!(std::fs::read(&output).unwrap(), before);
        assert!(!temp.path().join("Family.stillharbour-wal").exists());
        drop(catalog);
        let snapshot = Catalog::open(output).unwrap();
        assert_eq!(snapshot.summary().unwrap().id, id);
        assert_eq!(Catalog::open(path).unwrap().summary().unwrap().id, id);
    }

    #[test]
    fn future_schema_and_foreign_databases_are_not_migrated() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("foreign");
        let db = Connection::open(&path).unwrap();
        db.execute_batch("CREATE TABLE important(value TEXT); INSERT INTO important VALUES ('preserve'); PRAGMA user_version=999;").unwrap();
        drop(db);
        assert!(
            matches!(Catalog::open(path.clone()), Err(error) if error.code == ErrorCode::CatalogSchemaNewer)
        );
        let db = Connection::open(&path).unwrap();
        assert_eq!(
            db.query_row("SELECT value FROM important", [], |row| row
                .get::<_, String>(0))
                .unwrap(),
            "preserve"
        );
        db.pragma_update(None, "user_version", 0).unwrap();
        drop(db);
        assert!(matches!(Catalog::open(path), Err(error) if error.code == ErrorCode::InvalidInput));
    }
}

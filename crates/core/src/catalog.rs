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

pub const SCHEMA_VERSION: u32 = 3;
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

// Fields drop in declaration order: close SQLite before releasing the lifetime
// lock, including early returns and unwinding on the actor thread.
struct CatalogDatabase {
    connection: Connection,
    _lock: CatalogLock,
}
struct CatalogLock(std::fs::File);
impl Drop for CatalogLock {
    fn drop(&mut self) {
        // Closing our fd alone can leave flock alive in a concurrently forked
        // non-catalog child until exec closes its inherited CLOEXEC descriptor.
        // An explicit unlock ends only this owner's lock; active catalogs retain
        // their guard and still reject a second writer.
        let _ = FileExt::unlock(&self.0);
    }
}

pub struct Catalog {
    #[cfg(feature = "fixture-probes")]
    probe_calls: std::sync::atomic::AtomicU64,
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
                    Ok(mut database) => {
                        let connection = &mut database.connection;
                        let mut cache = ReadCache::default();
                        if ready_tx.send(Ok(())).is_err() {
                            return;
                        }
                        while let Ok(request) = receiver.recv() {
                            match request {
                                Request::Summary(reply) => {
                                    let _ = reply.send(summary(connection));
                                }
                                Request::Snapshot(path, reply) => {
                                    let _ = reply.send(snapshot(connection, &path));
                                }
                                Request::Run(work) => work(connection, &mut cache),
                                Request::Stop => break,
                            }
                        }
                    }
                }
            })?;
        ready_rx.recv().map_err(|_| stopped())??;
        Ok(Self {
            #[cfg(feature = "fixture-probes")]
            probe_calls: std::sync::atomic::AtomicU64::new(0),
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
        #[cfg(feature = "fixture-probes")]
        self.probe_calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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

    /// Diagnostic-only bounded counter; absent from production and IPC.
    #[cfg(feature = "fixture-probes")]
    pub fn probe_call_count(&self) -> u64 {
        self.probe_calls.load(std::sync::atomic::Ordering::Relaxed)
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

fn open_database(path: &Path) -> Result<CatalogDatabase> {
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
    let lock = CatalogLock(lock);
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
            #[cfg(test)]
            migration_test_exit("after_snapshot");
        }
        crate::library::store::migrate(&mut connection)?;
        #[cfg(test)]
        migration_test_exit("after_commit");
    }
    connection.pragma_update(None, "journal_mode", "WAL")?;
    validate(&connection)?;
    Ok(CatalogDatabase {
        connection,
        _lock: lock,
    })
}

#[cfg(test)]
pub(crate) fn migration_test_exit(stage: &str) {
    if std::env::var("STILLHARBOUR_M2_MIGRATION_CRASH").as_deref() == Ok(stage) {
        std::process::exit(73);
    }
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
    #[cfg(windows)]
    let _published = {
        let expected = crate::filesystem::identity(staged.as_file())?;
        // Disarm automatic path cleanup before native publication. On failure
        // retain our stage rather than unlinking a path that could be replaced.
        let staged = staged.into_temp_path().keep().map_err(|error| {
            AppError::new(
                ErrorCode::Io,
                "Could not retain the owned catalog snapshot stage.",
            )
            .caused_by(error)
        })?;
        crate::filesystem::catalog_snapshot::publish(&staged, destination, &expected)?
    };
    #[cfg(not(windows))]
    {
        // persist_noclobber is collision-safe but not a universal durability primitive.
        // Native no-replace + parent flush is used where verified; no overwrite fallback.
        crate::filesystem::publish_noreplace(staged.path(), destination)?;
        crate::filesystem::sync_directory(parent)?;
    }
    let independently_opened =
        Connection::open_with_flags(destination, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    validate(&independently_opened)?;
    if summary(&independently_opened)?.id != summary(source)?.id {
        return Err(AppError::new(
            ErrorCode::Catalog,
            "Published catalog snapshot identity validation failed.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(unix)]
    fn closing_catalog_releases_lock_while_an_unrelated_child_is_pre_exec() {
        use std::io::{Read, Write};
        use std::os::{
            fd::AsRawFd,
            unix::{net::UnixStream, process::CommandExt},
        };
        use std::process::{Command, Stdio};
        let owned = tempfile::tempdir().unwrap();
        let path = owned.path().join("fixture.stillharbour");
        let catalog = Catalog::open(path.clone()).unwrap();
        let identity = catalog.summary().unwrap().id;
        let (mut ready_reader, ready_writer) = UnixStream::pair().unwrap();
        let (mut release_writer, release_reader) = UnixStream::pair().unwrap();
        ready_reader
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        release_reader
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let (ready_fd, release_fd) = (ready_writer.as_raw_fd(), release_reader.as_raw_fd());
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .arg("--list")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // SAFETY: the child closure only performs async-signal-safe read/write
        // syscalls before exec. The parent retains both socket owners until join,
        // and the receive timeout bounds the child if the parent test fails.
        unsafe {
            command.pre_exec(move || {
                let mut byte = 1u8;
                if libc::write(ready_fd, std::ptr::from_ref(&byte).cast(), 1) != 1 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::read(release_fd, std::ptr::from_mut(&mut byte).cast(), 1) != 1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = std::thread::spawn(move || command.spawn().unwrap().wait().unwrap());
        ready_reader.read_exact(&mut [0]).unwrap();
        drop(catalog);
        // A non-catalog child can transiently inherit CLOEXEC descriptors until
        // exec; it must not extend the application-owned catalog lock lifetime.
        let reopened = Catalog::open(path);
        release_writer.write_all(&[1]).unwrap();
        assert!(child.join().unwrap().success());
        let reopened = reopened.expect("A closed catalog retained its lock in the pre-exec child");
        assert_eq!(reopened.summary().unwrap().id, identity);
    }

    #[test]
    fn build_diagnostics_use_no_user_catalog() {
        let info = sqlite_diagnostics().unwrap();
        assert_eq!(info["version"], rusqlite::version());
        assert_eq!(info["userCatalogOpened"], false);
        assert!(info["sourceId"].as_str().unwrap().len() > 64);
        assert!(info["compileOptions"].as_array().unwrap().len() > 10);
    }

    #[test]
    #[cfg(any(target_os = "macos", windows))]
    fn wal_snapshot_is_independently_openable_and_never_replaces() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("working.stillharbour");
        let catalog = Catalog::open(path.clone()).unwrap();
        assert!(
            matches!(Catalog::open(path.clone()), Err(error) if error.code == ErrorCode::CatalogInUse)
        );
        let id = catalog.summary().unwrap().id;
        catalog
            .call(|db| {
                db.execute(
                    "INSERT INTO job_events VALUES (1,'fixture','retained event')",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        let output = temp.path().join("Family.stillharbour");
        catalog.snapshot(output.clone()).unwrap();
        let before = std::fs::read(&output).unwrap();
        assert_eq!(
            catalog.snapshot(output.clone()).unwrap_err().code,
            ErrorCode::Conflict
        );
        assert_eq!(std::fs::read(&output).unwrap(), before);
        assert!(!temp.path().join("Family.stillharbour-wal").exists());
        let independent =
            Connection::open_with_flags(&output, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        assert_eq!(
            independent
                .query_row("SELECT state FROM job_events WHERE sequence=1", [], |r| {
                    r.get::<_, String>(0)
                })
                .unwrap(),
            "retained event"
        );
        drop(independent);
        drop(catalog);
        let snapshot = Catalog::open(output).unwrap();
        assert_eq!(snapshot.summary().unwrap().id, id);
        assert_eq!(Catalog::open(path).unwrap().summary().unwrap().id, id);
    }

    fn legacy_fixture(path: &Path) -> String {
        let catalog = Catalog::open(path.to_owned()).unwrap();
        let id = catalog.summary().unwrap().id;
        catalog
            .call(|db| {
                db.execute(
                    "INSERT INTO job_events VALUES(1,'fixture','retained user data')",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        drop(catalog);
        let db = Connection::open(path).unwrap();
        db.execute_batch("DROP TABLE operation_journal; DROP TABLE organization_plans; DROP TABLE organization_templates; PRAGMA user_version=2;").unwrap();
        id
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn schema_three_migration_retains_v2_and_rolls_back_failure() {
        let owned = tempfile::tempdir().unwrap();
        let path = owned.path().join("legacy.stillharbour");
        let id = legacy_fixture(&path);
        let current = Catalog::open(path.clone()).unwrap();
        assert_eq!(current.summary().unwrap().id, id);
        assert_eq!(current.summary().unwrap().schema, 3);
        assert_eq!(
            current
                .call(|db| Ok(db.query_row(
                    "SELECT state FROM job_events WHERE sequence=1",
                    [],
                    |r| r.get::<_, String>(0)
                )?))
                .unwrap(),
            "retained user data"
        );
        let retained = std::fs::read_dir(owned.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("pre-migration-v2-")
            })
            .unwrap();
        let backup =
            Connection::open_with_flags(retained, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        assert_eq!(summary(&backup).unwrap().id, id);
        assert_eq!(summary(&backup).unwrap().schema, 2);
        assert_eq!(
            backup
                .query_row("SELECT state FROM job_events WHERE sequence=1", [], |r| {
                    r.get::<_, String>(0)
                })
                .unwrap(),
            "retained user data"
        );
        drop(backup);
        drop(current);
        let bad = owned.path().join("collision.stillharbour");
        let id = legacy_fixture(&bad);
        {
            let db = Connection::open(&bad).unwrap();
            db.execute_batch("CREATE TABLE organization_plans(private_value TEXT); INSERT INTO organization_plans VALUES('preserved collision');").unwrap();
        }
        assert!(Catalog::open(bad.clone()).is_err());
        let db = Connection::open(&bad).unwrap();
        assert_eq!(summary(&db).unwrap().schema, 2);
        assert_eq!(summary(&db).unwrap().id, id);
        assert_eq!(
            db.query_row("SELECT private_value FROM organization_plans", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
            "preserved collision"
        );
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name='organization_templates'",
                [],
                |r| r.get::<_, u32>(0)
            )
            .unwrap(),
            0
        );
    }
    #[test]
    fn migration_process_fixture_child() {
        let Ok(path) = std::env::var("STILLHARBOUR_M2_MIGRATION_FILE") else {
            return;
        };
        let path = PathBuf::from(path);
        let parent = path.parent().unwrap();
        assert!(
            parent
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("stillharbour-m2-migration-")
        );
        let token = std::env::var("STILLHARBOUR_M2_MIGRATION_TOKEN").unwrap();
        assert!(uuid::Uuid::parse_str(&token).is_ok());
        assert_eq!(
            std::fs::read_to_string(parent.join("owned-fixture-token")).unwrap(),
            token
        );
        let _catalog = Catalog::open(path).unwrap();
        panic!("The fixture crash boundary was not reached");
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn process_exit_during_migration_retains_a_consistent_catalog_and_backup() {
        for stage in ["after_snapshot", "before_commit", "after_commit"] {
            let owned = tempfile::Builder::new()
                .prefix("stillharbour-m2-migration-")
                .tempdir()
                .unwrap();
            let token = uuid::Uuid::new_v4().to_string();
            std::fs::write(owned.path().join("owned-fixture-token"), &token).unwrap();
            let path = owned.path().join("catalog.stillharbour");
            let id = legacy_fixture(&path);
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "catalog::tests::migration_process_fixture_child",
                    "--nocapture",
                ])
                .env("STILLHARBOUR_M2_MIGRATION_FILE", &path)
                .env("STILLHARBOUR_M2_MIGRATION_TOKEN", &token)
                .env("STILLHARBOUR_M2_MIGRATION_CRASH", stage)
                .spawn()
                .unwrap();
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            let status = loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                if std::time::Instant::now() >= deadline {
                    child.kill().unwrap();
                    let _ = child.wait();
                    panic!("Owned migration child timed out");
                }
                std::thread::sleep(Duration::from_millis(10));
            };
            assert_eq!(status.code(), Some(73));
            // SQLite performs its own hot-journal recovery; never replay an app
            // mutation or run scripts recovered from the catalog.
            let db = Connection::open(&path).unwrap();
            validate(&db).unwrap();
            assert_eq!(summary(&db).unwrap().id, id);
            assert_eq!(
                summary(&db).unwrap().schema,
                if stage == "after_commit" { 3 } else { 2 }
            );
            assert_eq!(
                db.query_row("SELECT state FROM job_events WHERE sequence=1", [], |r| {
                    r.get::<_, String>(0)
                })
                .unwrap(),
                "retained user data"
            );
            let retained = std::fs::read_dir(owned.path())
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .find(|p| {
                    p.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("pre-migration-v2-")
                })
                .unwrap();
            let backup =
                Connection::open_with_flags(retained, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            validate(&backup).unwrap();
            assert_eq!(summary(&backup).unwrap().schema, 2);
            assert_eq!(summary(&backup).unwrap().id, id);
        }
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

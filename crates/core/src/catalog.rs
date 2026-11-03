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

pub const SCHEMA_VERSION: u32 = 17;
const APPLICATION_ID: i32 = 0x53544842; // STHB
const INITIAL_SCHEMA: &str = "CREATE TABLE catalog_identity (singleton INTEGER PRIMARY KEY CHECK(singleton=1), uuid TEXT NOT NULL UNIQUE); CREATE TABLE job_events (sequence INTEGER PRIMARY KEY, job_id TEXT NOT NULL, state TEXT NOT NULL);";

/// Build the trusted comparison schema only from compiled application SQL.
/// Imported schema strings never become an input to migration execution.
pub fn current_schema_fingerprint() -> Result<Vec<crate::catalog_restore::SchemaObject>> {
    compiled_schema_fingerprint(SCHEMA_VERSION)
}

fn compiled_schema_fingerprint(version: u32) -> Result<Vec<crate::catalog_restore::SchemaObject>> {
    let mut connection = Connection::open_in_memory()?;
    connection.execute_batch(INITIAL_SCHEMA)?;
    connection.execute(
        "INSERT INTO catalog_identity VALUES(1,?1)",
        [uuid::Uuid::new_v4().to_string()],
    )?;
    connection.pragma_update(None, "application_id", APPLICATION_ID)?;
    connection.pragma_update(None, "user_version", 1)?;
    crate::library::store::migrate_through(&mut connection, version)?;
    crate::catalog_restore::schema_objects(&connection)
}

pub(crate) fn initialize_portable(db: &mut Connection, id: &str) -> Result<()> {
    db.execute_batch(INITIAL_SCHEMA)?;
    db.execute("INSERT INTO catalog_identity VALUES(1,?)", [id])?;
    db.pragma_update(None, "application_id", APPLICATION_ID)?;
    db.pragma_update(None, "user_version", 1)?;
    crate::library::store::migrate_through(db, SCHEMA_VERSION)?;
    Ok(())
}

#[cfg(feature = "fixture-probes")]
pub fn inspect_portable_export_fixture(path: &Path) -> Result<serde_json::Value> {
    // macOS temporary folders may use the /var ancestor alias. Resolve only
    // the owned parent; keep SQLite's no-follow check on the output leaf.
    let parent = path
        .parent()
        .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "Missing generated export parent."))?
        .canonicalize()?;
    let leaf = path
        .file_name()
        .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "Missing generated export name."))?;
    let path = parent.join(leaf);
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    db.execute_batch("PRAGMA trusted_schema=OFF;")?;
    if crate::catalog_restore::schema_objects(&db)? != current_schema_fingerprint()? {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Unexpected generated export schema.",
        ));
    }
    let schema = summary(&db)?;
    let roots:i64=db.query_row("SELECT count(*) FROM roots WHERE availability<>'needs_relink' OR volume<>'portable-unbound' OR device<>'0' OR inode<>'0' OR length(native_path)<>0",[],|r|r.get(0))?;
    let profiles:i64=db.query_row("SELECT (SELECT count(*) FROM backup_snapshot_profiles)+(SELECT count(*) FROM backup_snapshot_schedules)+(SELECT count(*) FROM catalog_metadata_plans)+(SELECT count(*) FROM operation_journal)",[],|r|r.get(0))?;
    let integrity: String = db.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if roots != 0 || profiles != 0 || integrity != "ok" {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Generated portable copy retained unexpected authority or invalid data.",
        ));
    }
    Ok(
        serde_json::json!({"schema":schema.schema,"catalogId":schema.id,"sqliteVersion":schema.sqlite_version,"unboundRoots":true,"nativeAuthorities":roots,"credentialScheduleApprovalRows":profiles,"integrity":integrity,"media":db.query_row("SELECT count(*) FROM assets WHERE is_media=1",[],|r|r.get::<_,i64>(0))?}),
    )
}

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
    FreezeForRestore(SyncSender<Result<()>>),
    Run(CatalogWork),
    Stop,
}

// Fields drop in declaration order: close SQLite before releasing the lifetime
// lock, including early returns and unwinding on the actor thread.
struct CatalogDatabase {
    connection: Connection,
    _lock: CatalogLock,
    _parents: Option<crate::filesystem::DirectoryChain>,
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
    native_path: PathBuf,
    #[cfg(feature = "fixture-probes")]
    probe_calls: std::sync::atomic::AtomicU64,
    sender: SyncSender<Request>,
    worker: Option<thread::JoinHandle<()>>,
    #[cfg(feature = "fixture-probes")]
    performance_volume: Option<u64>,
    // Retained until the actor connection/lock close, including background jobs
    // that retain Catalog independently of the native application's state.
    native_owner: std::sync::Mutex<Option<std::sync::Arc<dyn Send + Sync>>>,
}

struct ExistingCatalog {
    identity: crate::filesystem::FileIdentity,
    volume: String,
    allow_initialization: bool,
}
impl ExistingCatalog {
    fn validate(&self, path: &Path) -> Result<()> {
        let metadata = std::fs::symlink_metadata(path)?;
        let file = std::fs::File::open(path)?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || crate::filesystem::identity(&file)? != self.identity
            || crate::library::native::volume_token(&file)? != self.volume
        {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "The selected catalog changed; no empty replacement was created.",
            ));
        }
        Ok(())
    }
}

impl Catalog {
    pub fn open(path: PathBuf) -> Result<Self> {
        Self::open_inner(path, None)
    }

    /// The caller retains the native activation witness and parent chains.
    /// Unlike first-use creation this path never permits SQLite CREATE.
    pub fn open_bound_existing(
        path: PathBuf,
        identity: crate::filesystem::FileIdentity,
        volume: String,
    ) -> Result<Self> {
        Self::open_inner(
            path,
            Some(ExistingCatalog {
                identity,
                volume,
                allow_initialization: false,
            }),
        )
    }

    /// Only a freshly OS-authorized first-use seed may initialize schema zero.
    /// The native activation selection owns this authority, never renderer data.
    pub fn open_bound_seed(
        path: PathBuf,
        identity: crate::filesystem::FileIdentity,
        volume: String,
    ) -> Result<Self> {
        Self::open_inner(
            path,
            Some(ExistingCatalog {
                identity,
                volume,
                allow_initialization: true,
            }),
        )
    }

    fn open_inner(path: PathBuf, expected: Option<ExistingCatalog>) -> Result<Self> {
        #[cfg(feature = "fixture-probes")]
        let performance_volume = crate::performance::path_volume(&path);
        let (sender, receiver) = mpsc::sync_channel(16);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("catalog".into())
            .spawn(move || {
                let opened = open_database(&path, expected.as_ref());
                match opened {
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                    }
                    Ok(mut database) => {
                        let connection = &mut database.connection;
                        let mut cache = ReadCache::default();
                        let native_path = match std::fs::canonicalize(&path) {
                            Ok(path) => path,
                            Err(error) => {
                                let _ = ready_tx.send(Err(error.into()));
                                return;
                            }
                        };
                        if ready_tx.send(Ok(native_path)).is_err() {
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
                                Request::FreezeForRestore(reply) => {
                                    let frozen = (|| {
                                        connection.pragma_update(None, "query_only", true)?;
                                        let value: bool = connection.query_row(
                                            "PRAGMA query_only", [], |row| row.get(0),
                                        )?;
                                        if !value {
                                            return Err(AppError::new(ErrorCode::Catalog,
                                                "Catalog restore could not stop catalog writes; close and reopen Stillharbour."));
                                        }
                                        Ok(())
                                    })();
                                    let _ = reply.send(frozen);
                                }
                                Request::Run(work) => work(connection, &mut cache),
                                Request::Stop => break,
                            }
                        }
                    }
                }
            })?;
        let native_path = ready_rx.recv().map_err(|_| stopped())??;
        Ok(Self {
            native_path,
            #[cfg(feature = "fixture-probes")]
            probe_calls: std::sync::atomic::AtomicU64::new(0),
            sender,
            worker: Some(worker),
            #[cfg(feature = "fixture-probes")]
            performance_volume,
            native_owner: std::sync::Mutex::new(None),
        })
    }

    /// Native lifetime ownership only, never renderer or imported authority.
    pub fn retain_native_owner(&self, owner: std::sync::Arc<dyn Send + Sync>) -> Result<()> {
        let mut current = self.native_owner.lock().map_err(|_| stopped())?;
        if current.is_some() {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "Native catalog lifetime ownership is already configured.",
            ));
        }
        *current = Some(owner);
        Ok(())
    }

    /// Fixed generated-fixture catalog edits. No original-file operation or SQL
    /// string is accepted, and production builds omit this test support.
    #[cfg(feature = "fixture-probes")]
    pub fn fixture_catalog_recovery_edit(&self, root_id: &str, second: bool) -> Result<()> {
        uuid::Uuid::parse_str(root_id).map_err(|_| {
            AppError::new(ErrorCode::InvalidInput, "Invalid generated root identity.")
        })?;
        let root_id = root_id.to_owned();
        self.call(move |db| {
            let tx = db.transaction()?;
            let label = if second {
                "Generated second newer catalog edit"
            } else {
                "Generated first newer catalog edit"
            };
            if tx.execute(
                "UPDATE roots SET label=?1 WHERE id=?2 AND label='Generated M4 application scope'",
                rusqlite::params![label, root_id],
            )? != 1
            {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "This is not the unchanged generated catalog fixture root.",
                ));
            }
            tx.execute(
                "UPDATE library_state SET revision=revision+1 WHERE singleton=1",
                [],
            )?;
            tx.commit()?;
            Ok(())
        })
    }

    #[cfg(feature = "fixture-probes")]
    pub fn fixture_catalog_recovery_label(&self, root_id: &str) -> Result<String> {
        uuid::Uuid::parse_str(root_id).map_err(|_| {
            AppError::new(ErrorCode::InvalidInput, "Invalid generated root identity.")
        })?;
        let root_id = root_id.to_owned();
        self.call(move |db| Ok(db.query_row("SELECT CASE WHEN length(CAST(label AS BLOB))<=128 THEN label END FROM roots WHERE id=?", [root_id], |row| row.get(0))?))
    }

    #[cfg(feature = "fixture-probes")]
    pub fn fixture_catalog_recovery_root_binding(
        &self,
        root_id: &str,
    ) -> Result<crate::catalog_restore::CatalogRootMapping> {
        uuid::Uuid::parse_str(root_id).map_err(|_| {
            AppError::new(ErrorCode::InvalidInput, "Invalid generated root identity.")
        })?;
        let root_id = root_id.to_owned();
        self.call(move |db| {
            let (path, device, inode, volume, platform): (Vec<u8>,String,String,String,String) = db.query_row(
                "SELECT CASE WHEN length(native_path)<=32768 THEN native_path END,CASE WHEN length(device)<=20 THEN device END,CASE WHEN length(inode)<=20 THEN inode END,CASE WHEN length(volume)<=4096 THEN volume END,CASE WHEN length(platform)<=32 THEN platform END FROM roots WHERE id=?",
                [&root_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)),
            )?;
            let invalid = || AppError::new(ErrorCode::InvalidInput, "Generated native root binding is invalid.");
            if platform != std::env::consts::OS { return Err(invalid()); }
            crate::library::native::decode_path(&path)?;
            Ok(crate::catalog_restore::CatalogRootMapping {
                root_id, path,
                identity: crate::filesystem::FileIdentity {
                    volume: device.parse().map_err(|_| invalid())?,
                    file: inode.parse().map_err(|_| invalid())?,
                    // Root rows persist native device/inode, not mutable nlink.
                    links: 1,
                },
                volume,
            })
        })
    }

    pub fn summary(&self) -> Result<CatalogSummary> {
        let (reply, response) = mpsc::sync_channel(1);
        self.sender
            .try_send(Request::Summary(reply))
            .map_err(|_| busy())?;
        response.recv().map_err(|_| stopped())?
    }

    /// Backend scope validation only. Never use this display-independent native
    /// namespace as authority to copy a live SQLite database or its companions.
    pub(crate) fn native_path(&self) -> &Path {
        &self.native_path
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

    /// Irreversible for this actor lifetime. The native shell first closes RPC
    /// admission and drains workers; the queued barrier then refuses later SQL
    /// writes while keeping consistent snapshots and bounded reads available.
    /// Failure recovery requires reopening the retained original catalog.
    pub fn freeze_writes_for_restore(&self) -> Result<()> {
        let (reply, response) = mpsc::sync_channel(1);
        self.sender
            .try_send(Request::FreezeForRestore(reply))
            .map_err(|_| busy())?;
        response.recv().map_err(|_| stopped())?
    }

    pub fn schema_fingerprint(&self) -> Result<Vec<crate::catalog_restore::SchemaObject>> {
        self.call(|connection| crate::catalog_restore::schema_objects(connection))
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

fn open_database(path: &Path, expected: Option<&ExistingCatalog>) -> Result<CatalogDatabase> {
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
    // Resolve inherited OS parent aliases (notably macOS /var -> /private/var)
    // before SQLite NOFOLLOW, without resolving away a forbidden leaf symlink.
    let bound_path = if let Some(expected) = expected {
        expected.validate(path)?;
        let parent = path.parent().ok_or_else(|| {
            AppError::new(ErrorCode::InvalidInput, "Catalog requires a local parent.")
        })?;
        let leaf = path.file_name().ok_or_else(|| {
            AppError::new(ErrorCode::InvalidInput, "Catalog requires a file name.")
        })?;
        Some(std::fs::canonicalize(parent)?.join(leaf))
    } else {
        None
    };
    let path = bound_path.as_deref().unwrap_or(path);
    let parents = if expected.is_some() {
        let parent = path.parent().expect("validated catalog parent");
        // This writable catalog creates retained snapshots in its parent.
        // Share writes only at that leaf; deny-delete ancestor guards remain.
        #[cfg(windows)]
        let chain = crate::filesystem::DirectoryChain::open_for_owned_publication(parent)?;
        #[cfg(not(windows))]
        let chain = crate::filesystem::DirectoryChain::open(parent)?;
        Some(chain)
    } else {
        None
    };
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
    if let Some(expected) = expected {
        expected.validate(path)?;
    }
    let mut flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    if expected.is_none() {
        flags |= OpenFlags::SQLITE_OPEN_CREATE;
    } else {
        flags |= OpenFlags::SQLITE_OPEN_NOFOLLOW;
    }
    let mut connection = Connection::open_with_flags(path, flags)?;
    if let Some(expected) = expected {
        expected.validate(path)?;
    }
    if let Some(parents) = &parents {
        let current = crate::filesystem::DirectoryChain::open(
            path.parent().expect("validated catalog parent"),
        )?;
        if current.identities().len() != parents.identities().len()
            || current
                .identities()
                .iter()
                .zip(parents.identities())
                .any(|(current, held)| current.volume != held.volume || current.file != held.file)
        {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "The selected catalog parent changed during opening.",
            ));
        }
    }
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
    if expected.is_some()
        && version != 0
        && crate::catalog_restore::schema_objects(&connection)?
            != compiled_schema_fingerprint(version)?
    {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "The selected catalog schema differs from the compiled schema; no imported schema was executed.",
        ));
    }
    if version == 0 {
        if expected.is_some_and(|binding| !binding.allow_initialization) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "The selected existing catalog has no schema. No empty catalog was initialized.",
            ));
        }
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
        tx.execute_batch(INITIAL_SCHEMA)?;
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
        _parents: parents,
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
    #[cfg(test)]
    let partial_copy =
        crate::library::snapshots::process_failures::catalog_copy_requested(source, destination);
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
            let pages = 128;
            #[cfg(test)]
            let pages = if partial_copy { 1 } else { pages };
            for _ in 0..200 {
                let step = backup.step(pages)?;
                #[cfg(test)]
                if partial_copy && step == StepResult::More {
                    crate::library::snapshots::process_failures::catalog_copy_step(
                        backup.progress().remaining,
                    );
                }
                if step == StepResult::Done {
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
    fn catalog_restore_bound_open_rejects_schema_before_reading_redefined_identity() {
        let owned = tempfile::tempdir().unwrap();
        let path = owned.path().join("Generated.stillharbour");
        drop(Catalog::open(path.clone()).unwrap());
        let db = Connection::open(&path).unwrap();
        db.execute_batch("DROP TABLE catalog_identity; CREATE VIEW catalog_identity AS SELECT 1 AS singleton,randomblob(1048576) AS uuid;").unwrap();
        db.close().unwrap();
        let before = std::fs::read(&path).unwrap();
        let file = std::fs::File::open(&path).unwrap();
        let identity = crate::filesystem::identity(&file).unwrap();
        let volume = crate::library::native::volume_token(&file).unwrap();
        drop(file);
        let result = Catalog::open_bound_existing(path.clone(), identity, volume);
        assert!(matches!(result, Err(error) if error.code == ErrorCode::InvalidInput));
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
    #[test]
    fn catalog_restore_native_owner_survives_every_catalog_reference() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        struct Owner(Arc<AtomicBool>);
        impl Drop for Owner {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }
        let owned = tempfile::tempdir().unwrap();
        let catalog = Arc::new(Catalog::open(owned.path().join("Generated.stillharbour")).unwrap());
        let dropped = Arc::new(AtomicBool::new(false));
        catalog
            .retain_native_owner(Arc::new(Owner(dropped.clone())))
            .unwrap();
        assert!(catalog.retain_native_owner(Arc::new(())).is_err());
        let background_reference = catalog.clone();
        drop(catalog);
        assert!(!dropped.load(Ordering::Acquire));
        assert!(background_reference.summary().is_ok());
        drop(background_reference);
        assert!(dropped.load(Ordering::Acquire));
    }
    #[test]
    fn catalog_restore_compiled_schema_matches_owned_catalog_and_detects_extra_objects() {
        let owned = tempfile::tempdir().unwrap();
        let catalog = Catalog::open(owned.path().join("Generated.stillharbour")).unwrap();
        let expected = current_schema_fingerprint().unwrap();
        assert!(!expected.is_empty());
        assert_eq!(catalog.schema_fingerprint().unwrap(), expected);
        catalog
            .call(|db| {
                db.execute_batch("CREATE TABLE generated_extra(value TEXT)")?;
                Ok(())
            })
            .unwrap();
        assert_ne!(catalog.schema_fingerprint().unwrap(), expected);
    }
    #[test]
    fn catalog_restore_freeze_retains_edits_allows_backup_and_requires_reopen_for_writes() {
        let owned = tempfile::tempdir().unwrap();
        let path = owned.path().join("Generated.stillharbour");
        let retained = owned.path().join("pre-restore.stillharbour");
        let catalog = Catalog::open(path.clone()).unwrap();
        let id = catalog.summary().unwrap().id;
        catalog.call(|db| {
            db.execute("INSERT INTO job_events(job_id,state) VALUES('generated-local-edit','retained')", [])?;
            Ok(())
        }).unwrap();
        catalog.freeze_writes_for_restore().unwrap();
        catalog.freeze_writes_for_restore().unwrap();
        assert!(
            catalog
                .call(|db| {
                    db.execute("DELETE FROM job_events", [])?;
                    Ok(())
                })
                .is_err()
        );
        assert_eq!(catalog.summary().unwrap().id, id);
        catalog.snapshot(retained.clone()).unwrap();
        let backup =
            Connection::open_with_flags(&retained, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let saved: String = backup
            .query_row(
                "SELECT state FROM job_events WHERE job_id='generated-local-edit'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(saved, "retained");
        drop(backup);
        drop(catalog);
        let catalog = Catalog::open(path).unwrap();
        catalog.call(|db| {
            db.execute("INSERT INTO job_events(job_id,state) VALUES('generated-after-reopen','writable')", [])?;
            Ok(())
        }).unwrap();
    }
    #[test]
    fn catalog_restore_bound_existing_never_creates_missing_or_adopts_another_object() {
        let owned = tempfile::tempdir().unwrap();
        let path = owned.path().join("Generated.stillharbour");
        let catalog = Catalog::open(path.clone()).unwrap();
        let id = catalog.summary().unwrap().id;
        drop(catalog);
        let file = std::fs::File::open(&path).unwrap();
        let identity = crate::filesystem::identity(&file).unwrap();
        let volume = crate::library::native::volume_token(&file).unwrap();
        drop(file);
        let catalog =
            Catalog::open_bound_existing(path.clone(), identity.clone(), volume.clone()).unwrap();
        assert_eq!(catalog.summary().unwrap().id, id);
        drop(catalog);
        let mut wrong = identity.clone();
        wrong.file ^= 1;
        assert!(Catalog::open_bound_existing(path.clone(), wrong, volume.clone()).is_err());
        let missing = owned.path().join("Absent.stillharbour");
        assert!(Catalog::open_bound_existing(missing.clone(), identity, volume).is_err());
        assert!(!missing.exists());
        std::fs::write(&path, []).unwrap();
        let file = std::fs::File::open(&path).unwrap();
        let identity = crate::filesystem::identity(&file).unwrap();
        let volume = crate::library::native::volume_token(&file).unwrap();
        drop(file);
        assert!(Catalog::open_bound_existing(path.clone(), identity, volume).is_err());
        assert_eq!(std::fs::metadata(path).unwrap().len(), 0);
    }
    #[test]
    #[cfg(unix)]
    fn catalog_restore_bound_open_resolves_parent_alias_but_rejects_leaf_symlink() {
        let owned = tempfile::tempdir().unwrap();
        let real = owned.path().join("real");
        std::fs::create_dir(&real).unwrap();
        let alias = owned.path().join("parent-alias");
        std::os::unix::fs::symlink(&real, &alias).unwrap();
        let path = alias.join("Generated.stillharbour");
        let catalog = Catalog::open(path.clone()).unwrap();
        let catalog_id = catalog.summary().unwrap().id;
        drop(catalog);
        let file = std::fs::File::open(&path).unwrap();
        let identity = crate::filesystem::identity(&file).unwrap();
        let volume = crate::library::native::volume_token(&file).unwrap();
        drop(file);
        let catalog = Catalog::open_bound_existing(path, identity.clone(), volume.clone()).unwrap();
        assert_eq!(catalog.summary().unwrap().id, catalog_id);
        drop(catalog);
        let leaf_alias = real.join("Leaf.stillharbour");
        std::os::unix::fs::symlink(real.join("Generated.stillharbour"), &leaf_alias).unwrap();
        assert!(Catalog::open_bound_existing(leaf_alias, identity, volume).is_err());
    }
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

    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn schema_ten_migration_retains_schema_nine_before_adding_progress() {
        let owned = tempfile::tempdir().unwrap();
        let path = owned.path().join("progress.stillharbour");
        let catalog = Catalog::open(path.clone()).unwrap();
        let id = catalog.summary().unwrap().id;
        drop(catalog);
        {
            let db = Connection::open(&path).unwrap();
            strip_m4_workflow_schema(&db).unwrap();
            db.execute_batch("DROP TABLE backup_comparison_history; DROP TABLE backup_sync_profiles; ALTER TABLE scans DROP COLUMN total_files; ALTER TABLE scans DROP COLUMN processed_files; PRAGMA user_version=9;").unwrap();
        }
        let catalog = Catalog::open(path).unwrap();
        assert_eq!(catalog.summary().unwrap().id, id);
        assert_eq!(catalog.summary().unwrap().schema, SCHEMA_VERSION);
        let retained = std::fs::read_dir(owned.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("pre-migration-v9-")
            })
            .unwrap();
        let backup =
            Connection::open_with_flags(retained, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        assert_eq!(summary(&backup).unwrap().schema, 9);
        assert_eq!(summary(&backup).unwrap().id, id);
        assert_eq!(
            backup
                .query_row(
                    "SELECT count(*) FROM pragma_table_info('scans') WHERE name='total_files'",
                    [],
                    |r| r.get::<_, u32>(0)
                )
                .unwrap(),
            0
        );
    }

    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn partial_scan_progress_schema_is_retained_without_migration() {
        let owned = tempfile::tempdir().unwrap();
        let path = owned.path().join("partial.stillharbour");
        let catalog = Catalog::open(path.clone()).unwrap();
        let id = catalog.summary().unwrap().id;
        drop(catalog);
        {
            let db = Connection::open(&path).unwrap();
            strip_m4_workflow_schema(&db).unwrap();
            db.execute_batch(
                "DROP TABLE backup_comparison_history; DROP TABLE backup_sync_profiles; ALTER TABLE scans DROP COLUMN processed_files; PRAGMA user_version=9;",
            )
            .unwrap();
        }
        assert!(Catalog::open(path.clone()).is_err());
        let db = Connection::open(path).unwrap();
        assert_eq!(summary(&db).unwrap().schema, 9);
        assert_eq!(summary(&db).unwrap().id, id);
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM pragma_table_info('scans') WHERE name='total_files'",
                [],
                |r| r.get::<_, u32>(0)
            )
            .unwrap(),
            1
        );
    }
    fn strip_original_job_schema(db: &Connection) -> rusqlite::Result<()> {
        strip_m4_workflow_schema(db).unwrap();
        db.execute_batch("DROP TABLE backup_comparison_history; DROP TABLE backup_sync_profiles; ALTER TABLE scans DROP COLUMN total_files; ALTER TABLE scans DROP COLUMN processed_files;")?;
        crate::library::inspection_store::strip_schema(db)?;
        db.execute_batch("DROP TABLE import_destination_preference; DROP TABLE original_operation_events; DROP TABLE original_operation_jobs; DROP TABLE original_operation_plans;")
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
        strip_original_job_schema(&db).unwrap();
        db.execute_batch("DROP INDEX operation_copy_effect_group;DROP INDEX operation_copy_effect_candidate;DROP INDEX operation_copy_effect_lookup;DROP TABLE operation_jobs; DROP TABLE operation_journal; DROP TABLE organization_plans; DROP TABLE organization_templates; PRAGMA user_version=2;").unwrap();
        id
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn planning_migration_retains_v2_and_rolls_back_failure() {
        let owned = tempfile::tempdir().unwrap();
        let path = owned.path().join("legacy.stillharbour");
        let id = legacy_fixture(&path);
        let current = Catalog::open(path.clone()).unwrap();
        assert_eq!(current.summary().unwrap().id, id);
        assert_eq!(current.summary().unwrap().schema, SCHEMA_VERSION);
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
    #[cfg(any(windows, target_os = "macos"))]
    fn schema_four_indices_retain_v3_history_and_roll_back_both_indices_on_collision() {
        let run = tempfile::tempdir().unwrap();
        for collision in [false, true] {
            let path = run.path().join(if collision {
                "collision.stillharbour"
            } else {
                "indexed.stillharbour"
            });
            let id = legacy_three_fixture(&path);
            if collision {
                let db = Connection::open(&path).unwrap();
                db.execute_batch("CREATE TABLE operation_copy_stream(private_value TEXT);INSERT INTO operation_copy_stream VALUES('retained collision');").unwrap();
                assert!(Catalog::open(path.clone()).is_err());
                assert_eq!(summary(&db).unwrap().schema, 3);
                assert_eq!(
                    db.query_row(
                        "SELECT private_value FROM operation_copy_stream",
                        [],
                        |row| row.get::<_, String>(0)
                    )
                    .unwrap(),
                    "retained collision"
                );
                assert_eq!(
                    db.query_row(
                        "SELECT count(*) FROM sqlite_schema WHERE name='operation_journal_scope'",
                        [],
                        |row| row.get::<_, u32>(0)
                    )
                    .unwrap(),
                    0
                );
            } else {
                let catalog = Catalog::open(path.clone()).unwrap();
                assert_eq!(catalog.summary().unwrap().schema, SCHEMA_VERSION);
                catalog.call(|db| {let count:u32=db.query_row("SELECT count(*) FROM sqlite_schema WHERE type='index' AND name IN ('operation_journal_scope','operation_copy_stream')",[],|row|row.get(0))?;assert_eq!(count,2);Ok(())}).unwrap();
            }
            let snapshot = std::fs::read_dir(run.path())
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| {
                    path.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("pre-migration-v3-")
                })
                .find(|path| {
                    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                        .unwrap();
                    summary(&db).unwrap().id == id
                })
                .unwrap();
            let db =
                Connection::open_with_flags(snapshot, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            validate(&db).unwrap();
            assert_eq!(summary(&db).unwrap().schema, 3);
            assert_eq!(
                db.query_row("SELECT state FROM job_events WHERE sequence=1", [], |row| {
                    row.get::<_, String>(0)
                })
                .unwrap(),
                "retained user data"
            );
        }
    }
    fn legacy_three_fixture(path: &Path) -> String {
        let catalog = Catalog::open(path.to_owned()).unwrap();
        let id = catalog.summary().unwrap().id;
        catalog.call(|db| {
            strip_original_job_schema(db)?;
            db.execute_batch("INSERT INTO job_events VALUES(1,'fixture','retained user data');DROP INDEX operation_copy_effect_group;DROP INDEX operation_copy_effect_candidate;DROP INDEX operation_copy_effect_lookup;DROP INDEX operation_journal_scope;DROP INDEX operation_copy_stream;DROP TABLE operation_jobs;PRAGMA user_version=3;")?;Ok(())
        }).unwrap();
        drop(catalog);
        id
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn scan_branch_v4_migrates_without_losing_progress_or_identity() {
        let owned = tempfile::tempdir().unwrap();
        let path = owned.path().join("scan-branch.stillharbour");
        let id = legacy_three_fixture(&path);
        {
            let db = Connection::open(&path).unwrap();
            db.execute_batch("ALTER TABLE scans ADD COLUMN total_files INTEGER; ALTER TABLE scans ADD COLUMN processed_files INTEGER; PRAGMA user_version=4;").unwrap();
            db.execute("INSERT INTO roots(id,label,native_path,display_path,mode,is_base,platform,volume,device,inode,availability,filter) VALUES('fixture-root','Generated',X'','Generated','referenced',1,'fixture','fixture','0','0','unknown',?1)", [serde_json::to_string(&crate::library::model::ScanFilter::default()).unwrap()]).unwrap();
            db.execute_batch("INSERT INTO scans(id,root_id,state,message,started_at,total_files,processed_files) VALUES('retained-progress','fixture-root','interrupted','Retained fixture','2026-11-01',12,5);").unwrap();
        }
        let catalog = Catalog::open(path).unwrap();
        assert_eq!(catalog.summary().unwrap().id, id);
        assert_eq!(catalog.summary().unwrap().schema, SCHEMA_VERSION);
        catalog.call(|db| {
            let indexes:u32 = db.query_row("SELECT count(*) FROM sqlite_schema WHERE type='index' AND name IN ('operation_journal_scope','operation_copy_stream')", [], |row| row.get(0))?;
            assert_eq!(indexes, 2);
            let columns:u32 = db.query_row("SELECT count(*) FROM pragma_table_info('scans') WHERE name IN ('total_files','processed_files')", [], |row| row.get(0))?;
            assert_eq!(columns, 2);
            assert_eq!(db.query_row("SELECT total_files,processed_files FROM scans WHERE id='retained-progress'", [], |row| Ok((row.get::<_,u32>(0)?,row.get::<_,u32>(1)?)))?, (12,5));
            assert_eq!(db.query_row("SELECT state FROM job_events WHERE sequence=1", [], |row|row.get::<_,String>(0))?, "retained user data");
            Ok(())
        }).unwrap();
        let snapshot = std::fs::read_dir(owned.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("pre-migration-v4-")
            })
            .unwrap();
        let backup =
            Connection::open_with_flags(snapshot, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        assert_eq!(summary(&backup).unwrap().schema, 4);
        assert_eq!(summary(&backup).unwrap().id, id);
    }
    fn legacy_four_fixture(path: &Path) -> String {
        let catalog = Catalog::open(path.to_owned()).unwrap();
        let id = catalog.summary().unwrap().id;
        catalog.call(|db|{strip_original_job_schema(db)?;db.execute_batch("INSERT INTO job_events VALUES(1,'fixture','retained user data');DROP INDEX operation_copy_effect_group;DROP INDEX operation_copy_effect_candidate;DROP INDEX operation_copy_effect_lookup;DROP TABLE operation_jobs;PRAGMA user_version=4;")?;Ok(())}).unwrap();
        drop(catalog);
        id
    }
    fn legacy_five_fixture(path: &Path) -> String {
        let catalog = Catalog::open(path.to_owned()).unwrap();
        let id = catalog.summary().unwrap().id;
        catalog.call(|db|{strip_original_job_schema(db)?;db.execute_batch("INSERT INTO job_events VALUES(1,'fixture','retained user data');DROP INDEX operation_copy_effect_group;DROP INDEX operation_copy_effect_candidate;DROP INDEX operation_copy_effect_lookup;PRAGMA user_version=5;")?;Ok(())}).unwrap();
        drop(catalog);
        id
    }
    fn legacy_six_fixture(path: &Path) -> String {
        let catalog = Catalog::open(path.to_owned()).unwrap();
        let id = catalog.summary().unwrap().id;
        catalog.call(|db| {
            strip_original_job_schema(db)?;
            db.execute_batch("INSERT INTO job_events VALUES(1,'fixture','retained user data'); PRAGMA user_version=6;")?;
            let filter = serde_json::to_string(&crate::library::model::ScanFilter::default()).unwrap();
            db.execute("INSERT INTO roots(id,label,native_path,display_path,mode,is_base,platform,volume,device,inode,availability,filter) VALUES('327d8c01-938e-4c13-8514-91f848e2eabc','Offline metadata fixture',X'00','Generated offline observation','referenced',1,'fixture','retained-volume','1','2','offline',?)", [filter])?;
            Ok(())
        }).unwrap();
        drop(catalog);
        id
    }
    fn legacy_eight_fixture(path: &Path) -> String {
        let catalog = Catalog::open(path.to_owned()).unwrap();
        let id = catalog.summary().unwrap().id;
        catalog.call(|db| {
            strip_m4_workflow_schema(db).unwrap();
            db.execute_batch("DROP TABLE backup_comparison_history; DROP TABLE backup_sync_profiles; ALTER TABLE scans DROP COLUMN total_files; ALTER TABLE scans DROP COLUMN processed_files;")?;
            crate::library::inspection_store::strip_schema(db)?;
            db.execute_batch("INSERT INTO job_events VALUES(1,'fixture','retained user data'); PRAGMA user_version=8;")?;
            Ok(())
        }).unwrap();
        id
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn inspection_migration_retains_v8_snapshot_and_rolls_back_partial_schema_on_collision() {
        for collision in [false, true] {
            let owned = tempfile::tempdir().unwrap();
            let path = owned.path().join("Fixture.stillharbour");
            let id = legacy_eight_fixture(&path);
            if collision {
                Connection::open(&path).unwrap().execute_batch("CREATE TABLE inspection_folder_children(value TEXT); INSERT INTO inspection_folder_children VALUES('retained neighbor');").unwrap();
            }
            let result = Catalog::open(path.clone());
            if collision {
                assert!(result.is_err());
                let db = Connection::open(&path).unwrap();
                assert_eq!(summary(&db).unwrap().schema, 8);
                assert_eq!(summary(&db).unwrap().id, id);
                assert_eq!(
                    db.query_row("SELECT value FROM inspection_folder_children", [], |row| {
                        row.get::<_, String>(0)
                    })
                    .unwrap(),
                    "retained neighbor"
                );
                assert_eq!(db.query_row("SELECT count(*) FROM sqlite_schema WHERE type='table' AND name='inspection_jobs'",[],|row|row.get::<_,i64>(0)).unwrap(),0);
            } else {
                let catalog = result.unwrap();
                assert_eq!(catalog.summary().unwrap().schema, SCHEMA_VERSION);
                assert_eq!(catalog.summary().unwrap().id, id);
            }
            let snapshot = std::fs::read_dir(owned.path())
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .find(|path| {
                    path.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("pre-migration-v8-")
                })
                .unwrap();
            let backup =
                Connection::open_with_flags(snapshot, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            assert_eq!(summary(&backup).unwrap().schema, 8);
            assert_eq!(summary(&backup).unwrap().id, id);
            assert_eq!(
                backup
                    .query_row("SELECT state FROM job_events WHERE sequence=1", [], |row| {
                        row.get::<_, String>(0)
                    })
                    .unwrap(),
                "retained user data"
            );
        }
    }
    fn legacy_seven_fixture(path: &Path) -> String {
        let catalog = Catalog::open(path.to_owned()).unwrap();
        let id = catalog.summary().unwrap().id;
        catalog.call(|db|{
            strip_m4_workflow_schema(db).unwrap();
            db.execute_batch("DROP TABLE backup_comparison_history; DROP TABLE backup_sync_profiles; ALTER TABLE scans DROP COLUMN total_files; ALTER TABLE scans DROP COLUMN processed_files;")?;
            crate::library::inspection_store::strip_schema(db)?;
            db.execute_batch("DROP TABLE import_destination_preference; INSERT INTO job_events VALUES(1,'fixture','retained user data'); PRAGMA user_version=7;")?;
            db.execute("INSERT INTO roots(id,label,native_path,display_path,mode,is_base,platform,volume,device,inode,availability,filter) VALUES('327d8c01-938e-4c13-8514-91f848e2eabc','Offline metadata fixture',X'00','Generated offline observation','referenced',1,'fixture','retained-volume','1','2','offline',?)",[serde_json::to_string(&crate::library::model::ScanFilter::default()).unwrap()])?;
            Ok(())
        }).unwrap();
        id
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn import_destination_migration_retains_v7_identity_and_snapshot_and_rolls_back_collision() {
        for collision in [false, true] {
            let owned = tempfile::tempdir().unwrap();
            let path = owned.path().join("Fixture.stillharbour");
            let id = legacy_seven_fixture(&path);
            if collision {
                let db = Connection::open(&path).unwrap();
                db.execute_batch("CREATE TABLE import_destination_preference(value TEXT); INSERT INTO import_destination_preference VALUES('preserve collision');").unwrap();
                assert!(Catalog::open(path.clone()).is_err());
                assert_eq!(summary(&db).unwrap().schema, 7);
                assert_eq!(
                    db.query_row(
                        "SELECT value FROM import_destination_preference",
                        [],
                        |row| row.get::<_, String>(0)
                    )
                    .unwrap(),
                    "preserve collision"
                );
            } else {
                let catalog = Catalog::open(path.clone()).unwrap();
                assert_eq!(catalog.summary().unwrap().id, id);
                assert_eq!(catalog.summary().unwrap().schema, SCHEMA_VERSION);
                catalog.call(|db|{
                    assert_eq!(db.query_row("SELECT root_id FROM import_destination_preference WHERE singleton=1",[],|row|row.get::<_,Option<String>>(0))?,None);
                    assert_eq!(db.query_row("SELECT count(*) FROM original_operation_jobs",[],|row|row.get::<_,u32>(0))?,0);
                    assert_eq!(db.query_row("SELECT native_path FROM roots",[],|row|row.get::<_,Vec<u8>>(0))?,vec![0]);
                    Ok(())
                }).unwrap();
            }
            let snapshot = std::fs::read_dir(owned.path())
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .find(|path| {
                    path.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("pre-migration-v7-")
                })
                .unwrap();
            let retained =
                Connection::open_with_flags(snapshot, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            assert_eq!(summary(&retained).unwrap().id, id);
            assert_eq!(summary(&retained).unwrap().schema, 7);
            assert_eq!(
                retained
                    .query_row("SELECT id FROM roots", [], |row| row.get::<_, String>(0))
                    .unwrap(),
                "327d8c01-938e-4c13-8514-91f848e2eabc"
            );
            assert_eq!(
                retained
                    .query_row("SELECT state FROM job_events WHERE sequence=1", [], |row| {
                        row.get::<_, String>(0)
                    })
                    .unwrap(),
                "retained user data"
            );
            assert_eq!(
                retained
                    .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                    .unwrap(),
                "ok"
            );
        }
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn organization_job_migration_retains_v6_identity_roots_and_history_and_rolls_back_collision() {
        let owned = tempfile::tempdir().unwrap();
        for collision in [false, true] {
            let path = owned.path().join(if collision {
                "collision.stillharbour"
            } else {
                "migrated.stillharbour"
            });
            let id = legacy_six_fixture(&path);
            if collision {
                let db = Connection::open(&path).unwrap();
                db.execute_batch("CREATE TABLE original_operation_jobs_recent(value TEXT); INSERT INTO original_operation_jobs_recent VALUES('preserve collision');").unwrap();
                assert!(Catalog::open(path.clone()).is_err());
                assert_eq!(summary(&db).unwrap().schema, 6);
                assert_eq!(db.query_row("SELECT count(*) FROM sqlite_schema WHERE name IN ('original_operation_plans','original_operation_jobs','original_operation_events')", [], |row| row.get::<_,i64>(0)).unwrap(), 0);
                assert_eq!(
                    db.query_row(
                        "SELECT value FROM original_operation_jobs_recent",
                        [],
                        |row| row.get::<_, String>(0)
                    )
                    .unwrap(),
                    "preserve collision"
                );
            } else {
                let migrated = Catalog::open(path.clone()).unwrap();
                assert_eq!(migrated.summary().unwrap().id, id);
                assert_eq!(migrated.summary().unwrap().schema, SCHEMA_VERSION);
                migrated
                    .call(|db| {
                        assert_eq!(
                            db.query_row(
                                "SELECT count(*) FROM original_operation_jobs",
                                [],
                                |row| row.get::<_, i64>(0)
                            )?,
                            0
                        );
                        assert_eq!(
                            db.query_row("SELECT id FROM roots", [], |row| row
                                .get::<_, String>(0))?,
                            "327d8c01-938e-4c13-8514-91f848e2eabc"
                        );
                        assert_eq!(
                            db.query_row("SELECT native_path FROM roots", [], |row| row
                                .get::<_, Vec<u8>>(0))?,
                            vec![0]
                        );
                        assert_eq!(
                            db.query_row(
                                "SELECT state FROM job_events WHERE sequence=1",
                                [],
                                |row| row.get::<_, String>(0)
                            )?,
                            "retained user data"
                        );
                        Ok(())
                    })
                    .unwrap();
            }
            let snapshot = std::fs::read_dir(owned.path())
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| {
                    path.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("pre-migration-v6-")
                })
                .find(|path| {
                    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                        .unwrap();
                    summary(&db).unwrap().id == id
                })
                .unwrap();
            let retained =
                Connection::open_with_flags(snapshot, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            assert_eq!(summary(&retained).unwrap().schema, 6);
            assert_eq!(summary(&retained).unwrap().id, id);
            assert_eq!(
                retained
                    .query_row("SELECT state FROM job_events WHERE sequence=1", [], |row| {
                        row.get::<_, String>(0)
                    })
                    .unwrap(),
                "retained user data"
            );
            assert_eq!(
                retained
                    .query_row("SELECT id FROM roots", [], |row| row.get::<_, String>(0))
                    .unwrap(),
                "327d8c01-938e-4c13-8514-91f848e2eabc"
            );
            assert_eq!(
                retained
                    .query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                    .unwrap(),
                "ok"
            );
        }
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn operation_job_migration_retains_v4_and_rolls_back_table_on_index_collision() {
        let run = tempfile::tempdir().unwrap();
        for collision in [false, true] {
            let path = run.path().join(if collision {
                "collision.stillharbour"
            } else {
                "jobs.stillharbour"
            });
            let id = legacy_four_fixture(&path);
            if collision {
                let db = Connection::open(&path).unwrap();
                db.execute_batch("CREATE TABLE operation_jobs_recent(private_value TEXT);INSERT INTO operation_jobs_recent VALUES('retain collision');").unwrap();
                drop(db);
                assert!(Catalog::open(path.clone()).is_err());
                let db = Connection::open(&path).unwrap();
                assert_eq!(summary(&db).unwrap().schema, 4);
                assert_eq!(
                    db.query_row(
                        "SELECT count(*) FROM sqlite_schema WHERE name='operation_jobs'",
                        [],
                        |row| row.get::<_, u32>(0)
                    )
                    .unwrap(),
                    0
                );
                assert_eq!(
                    db.query_row(
                        "SELECT private_value FROM operation_jobs_recent",
                        [],
                        |row| row.get::<_, String>(0)
                    )
                    .unwrap(),
                    "retain collision"
                );
            } else {
                let catalog = Catalog::open(path.clone()).unwrap();
                assert_eq!(catalog.summary().unwrap().schema, SCHEMA_VERSION);
                assert_eq!(catalog.summary().unwrap().id, id);
            }
            let retained = std::fs::read_dir(run.path())
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| {
                    path.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("pre-migration-v4-")
                })
                .find(|path| {
                    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                        .unwrap();
                    summary(&db).unwrap().id == id
                })
                .unwrap();
            let db =
                Connection::open_with_flags(retained, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            assert_eq!(summary(&db).unwrap().schema, 4);
            assert_eq!(summary(&db).unwrap().id, id);
            assert_eq!(
                db.query_row("SELECT state FROM job_events WHERE sequence=1", [], |row| {
                    row.get::<_, String>(0)
                })
                .unwrap(),
                "retained user data"
            );
        }
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn effect_indexes_retain_v5_snapshot_and_roll_back_on_collision() {
        let run = tempfile::tempdir().unwrap();
        for collision in [false, true] {
            let path = run.path().join(if collision {
                "collision.stillharbour"
            } else {
                "effects.stillharbour"
            });
            let id = legacy_five_fixture(&path);
            if collision {
                let db = Connection::open(&path).unwrap();
                db.execute_batch("CREATE TABLE operation_copy_effect_candidate(private_value TEXT);INSERT INTO operation_copy_effect_candidate VALUES('keep');").unwrap();
                drop(db);
                assert!(Catalog::open(path.clone()).is_err());
                let db = Connection::open(&path).unwrap();
                assert_eq!(summary(&db).unwrap().schema, 5);
                assert_eq!(db.query_row("SELECT count(*) FROM sqlite_schema WHERE name='operation_copy_effect_group'",[],|r|r.get::<_,u32>(0)).unwrap(),0);
                assert_eq!(
                    db.query_row(
                        "SELECT private_value FROM operation_copy_effect_candidate",
                        [],
                        |r| r.get::<_, String>(0)
                    )
                    .unwrap(),
                    "keep"
                );
            } else {
                let catalog = Catalog::open(path.clone()).unwrap();
                assert_eq!(catalog.summary().unwrap().id, id);
                assert_eq!(catalog.summary().unwrap().schema, SCHEMA_VERSION);
            }
            let retained = std::fs::read_dir(run.path())
                .unwrap()
                .map(|e| e.unwrap().path())
                .filter(|p| {
                    p.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("pre-migration-v5-")
                })
                .find(|p| {
                    let db =
                        Connection::open_with_flags(p, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
                    summary(&db).unwrap().id == id
                })
                .unwrap();
            let db =
                Connection::open_with_flags(retained, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            assert_eq!(summary(&db).unwrap().schema, 5);
            assert_eq!(summary(&db).unwrap().id, id);
            assert_eq!(
                db.query_row("SELECT state FROM job_events WHERE sequence=1", [], |r| {
                    r.get::<_, String>(0)
                })
                .unwrap(),
                "retained user data"
            );
        }
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
        let _catalog = if std::env::var("STILLHARBOUR_M2_MIGRATION_BOUND").as_deref() == Ok("1") {
            let held = crate::filesystem::open_readonly(&path).unwrap();
            let identity = crate::filesystem::identity(&held).unwrap();
            let volume = crate::library::native::volume_token(&held).unwrap();
            drop(held);
            Catalog::open_bound_existing(path, identity, volume).unwrap()
        } else {
            Catalog::open(path).unwrap()
        };
        panic!("The fixture crash boundary was not reached");
    }
    fn legacy_profile_fixture(path: &Path, version: u32) -> String {
        assert!(matches!(version, 9..=16));
        let catalog = Catalog::open(path.to_owned()).unwrap();
        let id = catalog.summary().unwrap().id;
        catalog.call(move |db| {
            if version >= 13 {
                strip_geographic_schema(db)?;
                db.pragma_update(None,"user_version",13)?;
                crate::library::store::migrate_through(db,version)?;
                db.execute_batch("INSERT INTO job_events VALUES(1,'fixture','retained user data');")?;
                return Ok(());
            }
            strip_m4_workflow_schema(db).unwrap();
            db.execute_batch("INSERT INTO job_events VALUES(1,'fixture','retained user data');")?;
            if version<12 {db.execute_batch("DROP TABLE backup_comparison_history;")?;}
            if version<11 { db.execute_batch("DROP TABLE backup_sync_profiles;")?; }
            if version == 9 { db.execute_batch("ALTER TABLE scans DROP COLUMN total_files; ALTER TABLE scans DROP COLUMN processed_files;")?; }
            db.pragma_update(None, "user_version", version)?;
            Ok(())
        }).unwrap();
        id
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn profile_migration_retains_v10_snapshot_and_rolls_back_name_collision() {
        for collision in [false, true] {
            let owned = tempfile::tempdir().unwrap();
            let path = owned.path().join("Profiles.stillharbour");
            let id = legacy_profile_fixture(&path, 10);
            if collision {
                Connection::open(&path).unwrap().execute_batch("CREATE TABLE retained_profile_neighbor(value TEXT); INSERT INTO retained_profile_neighbor VALUES('retain'); CREATE TRIGGER backup_sync_profiles_immutable_update BEFORE UPDATE ON retained_profile_neighbor BEGIN SELECT 1; END;").unwrap();
            }
            let result = Catalog::open(path.clone());
            if collision {
                assert!(result.is_err());
                let db = Connection::open(&path).unwrap();
                assert_eq!(summary(&db).unwrap().schema, 10);
                assert_eq!(
                    db.query_row(
                        "SELECT count(*) FROM sqlite_schema WHERE name='backup_sync_profiles'",
                        [],
                        |r| r.get::<_, u32>(0)
                    )
                    .unwrap(),
                    0
                );
                assert_eq!(
                    db.query_row("SELECT value FROM retained_profile_neighbor", [], |r| {
                        r.get::<_, String>(0)
                    })
                    .unwrap(),
                    "retain"
                );
            } else {
                let catalog = result.unwrap();
                assert_eq!(catalog.summary().unwrap().schema, SCHEMA_VERSION);
                assert_eq!(catalog.summary().unwrap().id, id);
            }
            let retained = std::fs::read_dir(owned.path())
                .unwrap()
                .map(|e| e.unwrap().path())
                .find(|p| {
                    p.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("pre-migration-v10-")
                })
                .unwrap();
            let db =
                Connection::open_with_flags(retained, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            assert_eq!(summary(&db).unwrap().schema, 10);
            assert_eq!(summary(&db).unwrap().id, id);
            assert_eq!(
                db.query_row("SELECT state FROM job_events WHERE sequence=1", [], |r| {
                    r.get::<_, String>(0)
                })
                .unwrap(),
                "retained user data"
            );
        }
    }
    #[test]
    #[cfg(any(windows, target_os = "macos"))]
    fn process_exit_during_migration_retains_a_consistent_catalog_and_backup() {
        for (version, stage) in [2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
            .into_iter()
            .flat_map(|version| {
                ["after_snapshot", "before_commit", "after_commit"].map(|stage| (version, stage))
            })
        {
            let owned = tempfile::Builder::new()
                .prefix("stillharbour-m2-migration-")
                .tempdir()
                .unwrap();
            let token = uuid::Uuid::new_v4().to_string();
            std::fs::write(owned.path().join("owned-fixture-token"), &token).unwrap();
            let path = owned.path().join("catalog.stillharbour");
            let id = if version == 2 {
                legacy_fixture(&path)
            } else if version == 3 {
                legacy_three_fixture(&path)
            } else if version == 4 {
                legacy_four_fixture(&path)
            } else if version == 5 {
                legacy_five_fixture(&path)
            } else if version == 6 {
                legacy_six_fixture(&path)
            } else if version == 7 {
                legacy_seven_fixture(&path)
            } else if version == 8 {
                legacy_eight_fixture(&path)
            } else {
                legacy_profile_fixture(&path, version)
            };
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "catalog::tests::migration_process_fixture_child",
                    "--nocapture",
                ])
                .env("STILLHARBOUR_M2_MIGRATION_FILE", &path)
                .env("STILLHARBOUR_M2_MIGRATION_TOKEN", &token)
                .env(
                    "STILLHARBOUR_M2_MIGRATION_BOUND",
                    if version >= 13 { "1" } else { "0" },
                )
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
                if stage == "after_commit" {
                    SCHEMA_VERSION
                } else {
                    version
                }
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
                        .starts_with(&format!("pre-migration-v{version}-"))
                })
                .unwrap();
            let backup =
                Connection::open_with_flags(retained, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            validate(&backup).unwrap();
            assert_eq!(summary(&backup).unwrap().schema, version);
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

#[cfg(test)]
mod comparison_history_migration_tests {
    use super::*;
    #[test]
    fn schema_twelve_history_migration_retains_eleven_and_rolls_back_collision() {
        for collision in [false, true] {
            let owned = tempfile::tempdir().unwrap();
            let path = owned.path().join("History.stillharbour");
            let current = Catalog::open(path.clone()).unwrap();
            let id = current.summary().unwrap().id;
            current
                .call(|db| {
                    strip_m4_workflow_schema(db).unwrap();
                    db.execute_batch(
                        "DROP TABLE backup_comparison_history; PRAGMA user_version=11;",
                    )?;
                    Ok(())
                })
                .unwrap();
            drop(current);
            if collision {
                let db = rusqlite::Connection::open(&path).unwrap();
                db.execute_batch("CREATE TABLE retained_neighbor(value TEXT); INSERT INTO retained_neighbor VALUES('preserve'); CREATE TRIGGER backup_comparison_history_update BEFORE UPDATE ON retained_neighbor BEGIN SELECT 1; END;").unwrap();
            }
            let result = Catalog::open(path.clone());
            if collision {
                assert!(result.is_err());
                let db = rusqlite::Connection::open(&path).unwrap();
                assert_eq!(
                    db.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
                        .unwrap(),
                    11
                );
                assert_eq!(
                    db.query_row(
                        "SELECT count(*) FROM sqlite_schema WHERE name='backup_comparison_history'",
                        [],
                        |r| r.get::<_, u32>(0)
                    )
                    .unwrap(),
                    0
                );
            } else {
                let opened = result.unwrap();
                assert_eq!(opened.summary().unwrap().schema, SCHEMA_VERSION);
                assert_eq!(opened.summary().unwrap().id, id);
            }
            let retained = std::fs::read_dir(owned.path())
                .unwrap()
                .map(|e| e.unwrap().path())
                .find(|p| {
                    p.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("pre-migration-v11-")
                })
                .unwrap();
            let db = rusqlite::Connection::open_with_flags(
                retained,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            assert_eq!(
                db.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))
                    .unwrap(),
                11
            );
            assert_eq!(
                db.query_row("SELECT uuid FROM catalog_identity", [], |r| r
                    .get::<_, String>(0))
                    .unwrap(),
                id
            );
        }
    }
}

#[cfg(test)]
fn strip_m4_workflow_schema(db: &Connection) -> rusqlite::Result<()> {
    strip_geographic_schema(db)?;
    db.execute_batch("DROP TABLE backup_sync_events; DROP TABLE backup_sync_plans; DROP TABLE backup_sync_jobs; DROP TABLE backup_snapshot_schedules; DROP TABLE backup_snapshot_events; DROP TABLE backup_snapshot_records; DROP TABLE backup_snapshot_jobs; DROP TABLE backup_snapshot_profiles; ")
}

#[cfg(test)]
fn strip_geographic_schema(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch("DROP TRIGGER catalog_tags_changed; DROP TABLE catalog_tag_context; DROP TABLE catalog_tag_overrides; DROP TABLE catalog_tag_history; DROP TABLE catalog_tag_category_history; DROP TABLE catalog_metadata_plans; DROP TABLE catalog_export_journal;")?;
    db.execute_batch("DROP TABLE asset_notes; DROP TABLE asset_note_history; DROP TABLE catalog_collection_members; DROP TABLE catalog_collections; DROP TABLE catalog_tag_categories;")?;
    db.execute_batch("DROP TRIGGER assets_location_insert; DROP TRIGGER assets_location_update; DROP TRIGGER assets_location_delete; DROP VIEW asset_location_observations; DROP TABLE asset_locations; DROP TABLE asset_spatial;")
}

#[cfg(test)]
mod geographic_migration_tests {
    use super::*;
    #[test]
    fn bound_startup_migrates_only_exact_compiled_historical_layouts() {
        for (legacy, tampered) in [
            (13, false),
            (14, false),
            (14, true),
            (15, false),
            (15, true),
            (16, false),
            (16, true),
        ] {
            let owned = tempfile::tempdir().unwrap();
            let path = owned.path().join("Bound.stillharbour");
            let catalog = Catalog::open(path.clone()).unwrap();
            let id = catalog.summary().unwrap().id;
            catalog.call(move|db| {
                strip_geographic_schema(db)?;
                db.pragma_update(None,"user_version",13)?;
                crate::library::store::migrate_through(db,legacy)?;
                db.execute("INSERT INTO job_events(job_id,state) VALUES('generated','retained before upgrade')",[])?;
                assert_eq!(crate::catalog_restore::schema_objects(db)?,compiled_schema_fingerprint(legacy)?);
                if tampered {db.execute_batch("CREATE VIEW unreviewed_schema AS SELECT randomblob(1000000000);")?;}
                Ok(())
            }).unwrap();
            drop(catalog);
            let held = crate::filesystem::open_readonly(&path).unwrap();
            let identity = crate::filesystem::identity(&held).unwrap();
            let volume = crate::library::native::volume_token(&held).unwrap();
            drop(held);
            let result = Catalog::open_bound_existing(path.clone(), identity, volume);
            if tampered {
                assert!(matches!(result,Err(error) if error.code==ErrorCode::InvalidInput));
                let db = Connection::open(&path).unwrap();
                assert_eq!(summary(&db).unwrap().schema, legacy);
            } else {
                let catalog = result.unwrap();
                assert_eq!(catalog.summary().unwrap().schema, SCHEMA_VERSION);
                assert_eq!(catalog.summary().unwrap().id, id);
                let retained = std::fs::read_dir(owned.path())
                    .unwrap()
                    .map(|e| e.unwrap().path())
                    .find(|p| {
                        p.file_name()
                            .unwrap()
                            .to_string_lossy()
                            .starts_with(&format!("pre-migration-v{legacy}-"))
                    })
                    .unwrap();
                let db = Connection::open_with_flags(retained, OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .unwrap();
                assert_eq!(summary(&db).unwrap().schema, legacy);
                assert_eq!(summary(&db).unwrap().id, id);
                assert_eq!(
                    db.query_row(
                        "SELECT state FROM job_events WHERE job_id='generated'",
                        [],
                        |r| r.get::<_, String>(0)
                    )
                    .unwrap(),
                    "retained before upgrade"
                );
            }
        }
    }
    #[test]
    fn schema_fourteen_retains_thirteen_and_backfills_without_rewriting_observations() {
        for collision in [false, true] {
            let owned = tempfile::tempdir().unwrap();
            let path = owned.path().join("Map.stillharbour");
            let catalog = Catalog::open(path.clone()).unwrap();
            let id = catalog.summary().unwrap().id;
            catalog.call(|db|{
                strip_geographic_schema(db)?;
                db.execute_batch("INSERT INTO job_events(job_id,state) VALUES('fixture','retained thirteen'); PRAGMA user_version=13;")?;
                db.execute("INSERT INTO roots(id,label,native_path,display_path,mode,is_base,platform,volume,device,inode,availability,filter) VALUES('root','Retained root',x'01','Generated','referenced',1,'fixture','fixture','0','0','offline','{}')",[])?;
                db.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan,raw_metadata,favorite,rating,tags) VALUES('photo','root',x'01','Zero.jpg',x'','','Zero.jpg','Zero','jpg','fixture',1,'retained-revision','2026-11-01','2026-11-01','capture','ready','ready','present',1,'fixture','[{\"GPSLatitude\":0,\"GPSLongitude\":0}]',1,5,'[\"retained\"]')",[])?;
                Ok(())
            }).unwrap();
            drop(catalog);
            if collision {
                Connection::open(&path).unwrap().execute_batch("CREATE TABLE asset_locations(retained TEXT); INSERT INTO asset_locations VALUES('retain collision');").unwrap();
            }
            let result = Catalog::open(path.clone());
            if collision {
                assert!(result.is_err());
                let db = Connection::open(&path).unwrap();
                assert_eq!(summary(&db).unwrap().schema, 13);
                assert_eq!(
                    db.query_row("SELECT retained FROM asset_locations", [], |r| r
                        .get::<_, String>(0))
                        .unwrap(),
                    "retain collision"
                );
            } else {
                let opened = result.unwrap();
                assert_eq!(opened.summary().unwrap().schema, SCHEMA_VERSION);
                assert_eq!(opened.summary().unwrap().id, id);
                opened
                    .call(|db| {
                        assert_eq!(
                            db.query_row("SELECT count(*) FROM asset_spatial", [], |r| r
                                .get::<_, u32>(0))?,
                            1
                        );
                        let retained: (String, bool, u8, String) = db.query_row(
                            "SELECT revision,favorite,rating,tags FROM assets WHERE id='photo'",
                            [],
                            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                        )?;
                        assert_eq!(
                            retained,
                            ("retained-revision".into(), true, 5, "[\"retained\"]".into())
                        );
                        assert_eq!(
                            db.query_row("SELECT latitude FROM asset_locations", [], |r| r
                                .get::<_, f64>(0))?,
                            0.0
                        );
                        Ok(())
                    })
                    .unwrap();
                assert_eq!(
                    opened.schema_fingerprint().unwrap(),
                    current_schema_fingerprint().unwrap()
                );
            }
            let retained = std::fs::read_dir(owned.path())
                .unwrap()
                .map(|e| e.unwrap().path())
                .find(|p| {
                    p.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("pre-migration-v13-")
                })
                .unwrap();
            let db =
                Connection::open_with_flags(retained, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            assert_eq!(summary(&db).unwrap().schema, 13);
            assert_eq!(summary(&db).unwrap().id, id);
            assert_eq!(
                db.query_row(
                    "SELECT raw_metadata FROM assets WHERE id='photo'",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                "[{\"GPSLatitude\":0,\"GPSLongitude\":0}]"
            );
            assert_eq!(
                db.query_row(
                    "SELECT state FROM job_events WHERE job_id='fixture'",
                    [],
                    |r| r.get::<_, String>(0)
                )
                .unwrap(),
                "retained thirteen"
            );
        }
    }
}

#[cfg(test)]
mod backup_workflow_migration_tests {
    use super::*;
    #[test]
    fn schema_thirteen_preserves_twelve_identity_data_snapshot_and_rolls_back_collision() {
        for collision in [false, true] {
            let owned = tempfile::tempdir().unwrap();
            let path = owned.path().join("Migration.stillharbour");
            let catalog = Catalog::open(path.clone()).unwrap();
            let expected = catalog.summary().unwrap().id;
            catalog.call(|db| {
                strip_m4_workflow_schema(db)?;
                db.execute_batch("INSERT INTO job_events VALUES(1,'fixture','retained twelve');PRAGMA user_version=12;")?;
                Ok(())
            }).unwrap();
            drop(catalog);
            if collision {
                let db = Connection::open(&path).unwrap();
                db.execute_batch("CREATE TABLE backup_sync_jobs(retained TEXT);INSERT INTO backup_sync_jobs VALUES('retain collision');").unwrap();
            }
            let result = Catalog::open(path.clone());
            if collision {
                assert!(result.is_err());
                let db = Connection::open(&path).unwrap();
                assert_eq!(summary(&db).unwrap().schema, 12);
                assert_eq!(
                    db.query_row("SELECT retained FROM backup_sync_jobs", [], |r| r
                        .get::<_, String>(0))
                        .unwrap(),
                    "retain collision"
                );
                assert_eq!(
                    db.query_row(
                        "SELECT count(*) FROM sqlite_schema WHERE name='backup_snapshot_profiles'",
                        [],
                        |r| r.get::<_, u32>(0)
                    )
                    .unwrap(),
                    0
                );
            } else {
                let catalog = result.unwrap();
                assert_eq!(catalog.summary().unwrap().schema, SCHEMA_VERSION);
                assert_eq!(catalog.summary().unwrap().id, expected);
                assert_eq!(
                    catalog
                        .call(|db| Ok(db.query_row(
                            "SELECT state FROM job_events WHERE sequence=1",
                            [],
                            |r| r.get::<_, String>(0)
                        )?))
                        .unwrap(),
                    "retained twelve"
                );
            }
            let retained = std::fs::read_dir(owned.path())
                .unwrap()
                .map(|e| e.unwrap().path())
                .find(|p| {
                    p.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("pre-migration-v12-")
                })
                .unwrap();
            let db =
                Connection::open_with_flags(retained, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            assert_eq!(summary(&db).unwrap().schema, 12);
            assert_eq!(summary(&db).unwrap().id, expected);
            assert_eq!(
                db.query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0))
                    .unwrap(),
                "ok"
            );
        }
    }
}

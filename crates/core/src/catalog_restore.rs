//! Native-only catalog activation descriptors and defensive read-only validation.
//! Imported schema text is compared as data and is never executed.
use crate::{
    domain::{AppError, ErrorCode, Result},
    filesystem::{self, FileIdentity},
    library::native,
};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod activation;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SchemaObject {
    pub kind: String,
    pub name: String,
    pub table: String,
    pub sql: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentCatalogBinding {
    pub path: Vec<u8>,
    pub identity: FileIdentity,
    pub volume: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogFileBinding {
    pub path: Vec<u8>,
    pub identity: FileIdentity,
    pub volume: String,
    pub digest: String,
    pub bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogRootMapping {
    pub root_id: String,
    pub path: Vec<u8>,
    pub identity: FileIdentity,
    pub volume: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedCatalogRestore {
    pub version: u32,
    pub transaction_id: String,
    pub catalog_id: String,
    pub source_restore_job_id: String,
    pub source_snapshot_id: String,
    pub source_catalog_revision: String,
    pub candidate_catalog_revision: String,
    pub current_catalog_revision: String,
    pub schema: u32,
    pub current: CurrentCatalogBinding,
    pub pre_restore: CatalogFileBinding,
    pub candidate: CatalogFileBinding,
    pub roots: Vec<CatalogRootMapping>,
}
fn rejected() -> AppError {
    AppError::new(
        ErrorCode::InvalidInput,
        "The catalog is outside the reviewed current-schema activation contract; no imported schema was executed.",
    )
}
/// Bounds are enforced in SQLite before values cross into Rust allocations.
pub fn schema_objects(db: &Connection) -> Result<Vec<SchemaObject>> {
    let mut statement = db.prepare("SELECT CASE WHEN length(CAST(type AS BLOB))<=32 THEN type END,CASE WHEN length(CAST(name AS BLOB))<=512 THEN name END,CASE WHEN length(CAST(tbl_name AS BLOB))<=512 THEN tbl_name END,CASE WHEN sql IS NULL OR length(CAST(sql AS BLOB))<=262144 THEN sql ELSE NULL END,sql IS NOT NULL AND length(CAST(sql AS BLOB))>262144 FROM sqlite_schema ORDER BY type,name,tbl_name,sql LIMIT 257")?;
    let mut total = 0usize;
    let mut objects = Vec::new();
    for row in statement.query_map([], |r| {
        Ok((
            r.get::<_, Option<String>>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<String>>(3)?,
            r.get::<_, bool>(4)?,
        ))
    })? {
        let (kind, name, table, sql, oversized) = row?;
        let (Some(kind), Some(name), Some(table)) = (kind, name, table) else {
            return Err(rejected());
        };
        total = total
            .checked_add(
                kind.len() + name.len() + table.len() + sql.as_ref().map_or(0, String::len),
            )
            .ok_or_else(rejected)?;
        if oversized || total > 1024 * 1024 || objects.len() >= 256 {
            return Err(rejected());
        }
        objects.push(SchemaObject {
            kind,
            name,
            table,
            sql,
        });
    }
    Ok(objects)
}
pub struct ValidatedCatalog {
    pub revision: String,
    pub roots: Vec<CatalogRootMapping>,
}
/// Only fixed read-only SQL runs before exact schema equality is established.
pub fn validate_catalog(
    path: &Path,
    catalog_id: &str,
    expected: &[SchemaObject],
) -> Result<ValidatedCatalog> {
    let held = filesystem::open_readonly(path)?;
    let identity = filesystem::identity(&held)?;
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    db.execute_batch("PRAGMA trusted_schema=OFF; PRAGMA query_only=ON;")?;
    let app: i32 = db.query_row("PRAGMA application_id", [], |r| r.get(0))?;
    let version: u32 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if app != 0x53544842
        || version != crate::catalog::SCHEMA_VERSION
        || schema_objects(&db)? != expected
    {
        return Err(rejected());
    }
    let actual: String = db.query_row(
        "SELECT uuid FROM catalog_identity WHERE singleton=1",
        [],
        |r| r.get(0),
    )?;
    let check: String = db.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if actual != catalog_id || check != "ok" || db.prepare("PRAGMA foreign_key_check")?.exists([])?
    {
        return Err(rejected());
    }
    let revision = db.query_row(
        "SELECT CAST(revision AS TEXT) FROM library_state WHERE singleton=1",
        [],
        |r| r.get(0),
    )?;
    let mut roots = Vec::new();
    let mut statement=db.prepare("SELECT CASE WHEN length(id)=36 THEN id END,CASE WHEN length(native_path)<=32768 THEN native_path END,CASE WHEN length(device)<=32 THEN device END,CASE WHEN length(inode)<=32 THEN inode END,CASE WHEN length(volume)<=4096 THEN volume END,CASE WHEN length(platform)<=32 THEN platform END FROM roots ORDER BY id LIMIT 33")?;
    for row in statement.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Vec<u8>>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
        ))
    })? {
        let (root_id, path, device, inode, volume, platform) = row?;
        if platform != std::env::consts::OS {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "Catalog activation currently requires same-platform native root mappings.",
            ));
        }
        uuid::Uuid::parse_str(&root_id).map_err(|_| rejected())?;
        native::decode_path(&path)?;
        if roots.len() >= 32 {
            return Err(rejected());
        }
        roots.push(CatalogRootMapping {
            root_id,
            path,
            identity: FileIdentity {
                volume: device.parse().map_err(|_| rejected())?,
                file: inode.parse().map_err(|_| rejected())?,
                links: 1,
            },
            volume,
        });
    }
    if roots.is_empty() || filesystem::identity(&filesystem::open_readonly(path)?)? != identity {
        return Err(rejected());
    }
    Ok(ValidatedCatalog { revision, roots })
}
impl CatalogFileBinding {
    pub fn native_path(&self) -> Result<PathBuf> {
        native::decode_path(&self.path)
    }
}

pub fn capture_file(path: &Path) -> Result<CatalogFileBinding> {
    let parent = path.parent().ok_or_else(rejected)?;
    let leaf = path.file_name().ok_or_else(rejected)?;
    let chain = filesystem::DirectoryChain::open(parent)?;
    let parent_id = filesystem::identity(chain.leaf())?;
    let parent_volume = native::volume_token(chain.leaf())?;
    let mut file = native::open_asset(parent, &parent_id, &parent_volume, Path::new(leaf))?;
    let identity = filesystem::identity(&file)?;
    let bytes = file.metadata()?.len();
    if identity.links != 1 || bytes > 512 * 1024 * 1024 {
        return Err(rejected());
    }
    let volume = native::volume_token(&file)?;
    let digest = crate::executor::snapshot_bridge::digest(
        &mut file,
        &std::sync::atomic::AtomicBool::new(false),
    )?;
    let current_parent = filesystem::DirectoryChain::open(parent)?;
    if current_parent.identity().volume != chain.identity().volume
        || current_parent.identity().file != chain.identity().file
    {
        return Err(rejected());
    }
    Ok(CatalogFileBinding {
        path: native::encode_path(path),
        identity,
        volume,
        digest,
        bytes,
    })
}
pub fn validate_frozen(binding: &CatalogFileBinding) -> Result<()> {
    let current = capture_file(&binding.native_path()?)?;
    if current.path != binding.path
        || current.identity != binding.identity
        || current.volume != binding.volume
        || current.digest != binding.digest
        || current.bytes != binding.bytes
    {
        return Err(rejected());
    }
    Ok(())
}

/// Identity only: sharing permits the owned SQLite actor's existing write
/// handle. This never treats live DB bytes as a consistent backup.
fn open_current_file(path: &Path) -> Result<std::fs::File> {
    let _parent = filesystem::DirectoryChain::open(path.parent().ok_or_else(rejected)?)?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
        use windows_sys::Win32::Storage::FileSystem::*;
        options
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_OPEN_NO_RECALL);
        let file = options.open(path)?;
        if !file.metadata()?.is_file()
            || file.metadata()?.file_attributes()
                & (FILE_ATTRIBUTE_REPARSE_POINT
                    | FILE_ATTRIBUTE_OFFLINE
                    | FILE_ATTRIBUTE_RECALL_ON_OPEN
                    | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
                != 0
        {
            return Err(rejected());
        }
        Ok(file)
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        let file = options.open(path)?;
        if !file.metadata()?.is_file() {
            return Err(rejected());
        }
        Ok(file)
    }
}
pub fn capture_current(path: &Path) -> Result<CurrentCatalogBinding> {
    let file = open_current_file(path)?;
    Ok(CurrentCatalogBinding {
        path: native::encode_path(path),
        identity: filesystem::identity(&file)?,
        volume: native::volume_token(&file)?,
    })
}
pub fn validate_current_revision(
    binding: &CurrentCatalogBinding,
    catalog_id: &str,
    expected_schema: &[SchemaObject],
    expected_revision: &str,
) -> Result<()> {
    let path = native::decode_path(&binding.path)?;
    let witness = open_current_file(&path)?;
    if filesystem::identity(&witness)? != binding.identity
        || native::volume_token(&witness)? != binding.volume
    {
        return Err(rejected());
    }
    let db = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    db.execute_batch("PRAGMA trusted_schema=OFF; PRAGMA query_only=ON;")?;
    let schema: u32 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let app: i32 = db.query_row("PRAGMA application_id", [], |r| r.get(0))?;
    if schema != crate::catalog::SCHEMA_VERSION
        || app != 0x53544842
        || schema_objects(&db)? != expected_schema
    {
        return Err(rejected());
    }
    let actual: String = db.query_row(
        "SELECT uuid FROM catalog_identity WHERE singleton=1",
        [],
        |r| r.get(0),
    )?;
    let revision: String = db.query_row(
        "SELECT CAST(revision AS TEXT) FROM library_state WHERE singleton=1",
        [],
        |r| r.get(0),
    )?;
    let after = capture_current(&path)?;
    if actual != catalog_id
        || revision != expected_revision
        || after.identity != binding.identity
        || after.volume != binding.volume
    {
        return Err(rejected());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imported_schema_text_is_compared_as_data_before_catalog_queries() {
        let run = tempfile::tempdir().unwrap();
        let root = run.path().canonicalize().unwrap();
        let catalog = crate::catalog::Catalog::open(root.join("owned.stillharbour")).unwrap();
        let schema = catalog.schema_fingerprint().unwrap();
        assert_eq!(
            schema,
            crate::catalog::current_schema_fingerprint().unwrap()
        );
        let source = root.join("untrusted.stillharbour");
        catalog.snapshot(source.clone()).unwrap();
        let db = Connection::open(&source).unwrap();
        db.execute_batch("CREATE TRIGGER untrusted_activation AFTER UPDATE ON catalog_identity BEGIN DELETE FROM catalog_identity; END; CREATE VIEW untrusted_view AS SELECT * FROM catalog_identity;").unwrap();
        db.close().unwrap();
        let before = capture_file(&source).unwrap();
        let failure = validate_catalog(&source, &catalog.summary().unwrap().id, &schema)
            .err()
            .unwrap();
        assert_eq!(failure.code, ErrorCode::InvalidInput);
        validate_frozen(&before).unwrap();
        let db = Connection::open_with_flags(&source, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM catalog_identity", [], |r| r
                .get::<_, u32>(0))
                .unwrap(),
            1
        );
        assert_eq!(schema_objects(&db).unwrap().len(), schema.len() + 2);
    }
    #[test]
    fn schema_and_native_file_bounds_reject_unknown_or_changed_inputs() {
        let db = Connection::open_in_memory().unwrap();
        for n in 0..257 {
            db.execute_batch(&format!("CREATE TABLE t{n}(value TEXT)"))
                .unwrap();
        }
        assert!(schema_objects(&db).is_err());
        let run = tempfile::tempdir().unwrap();
        let path = run
            .path()
            .canonicalize()
            .unwrap()
            .join("generated-only.bin");
        std::fs::write(&path, b"generated frozen bytes").unwrap();
        let binding = capture_file(&path).unwrap();
        std::fs::write(&path, b"changed generated bytes").unwrap();
        assert!(validate_frozen(&binding).is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"changed generated bytes");
    }
}

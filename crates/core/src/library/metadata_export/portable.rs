use super::*;
use rusqlite::{
    Connection, OpenFlags,
    backup::{Backup, StepResult},
    types::Value,
};
use std::time::{Duration, Instant};

// Only organization and read-only evidence enter the portable copy. Credential,
// approval, timing, native destination, operation and active-job tables stay empty.
const TABLES: &[&str] = &[
    "organization_templates",
    "roots",
    "assets",
    "directories",
    "asset_notes",
    "asset_note_history",
    "catalog_collections",
    "catalog_collection_members",
    "catalog_tag_categories",
    "catalog_tag_overrides",
    "catalog_tag_history",
    "catalog_tag_category_history",
    "inspection_jobs",
    "inspection_job_roots",
    "inspection_items",
    "inspection_groups",
    "inspection_group_members",
    "inspection_folders",
    "inspection_keeper_reviews",
    "inspection_summaries",
    "inspection_issues",
    "inspection_fingerprints",
    "integrity_baselines",
];
pub(super) fn build(source: &Connection, folder: &Path, destination: &Path) -> Result<()> {
    let pages: i64 = source.query_row("PRAGMA page_count", [], |r| r.get(0))?;
    let page_size: i64 = source.query_row("PRAGMA page_size", [], |r| r.get(0))?;
    if pages
        .checked_mul(page_size)
        .is_none_or(|bytes| bytes > 512 * 1024 * 1024)
    {
        return Err(invalid(
            "The catalog exceeds the 512 MiB portable export bound.",
        ));
    }
    let snapshot_path = folder.join("private-consistent-source.sqlite");
    let mut snapshot = Connection::open(&snapshot_path)?;
    snapshot.execute_batch("PRAGMA synchronous=FULL;PRAGMA trusted_schema=OFF;")?;
    {
        let backup = Backup::new(source, &mut snapshot)?;
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            match backup.step(128)? {
                StepResult::Done => break,
                StepResult::Busy | StepResult::Locked => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                _ => {}
            }
            if Instant::now() > deadline {
                return Err(invalid(
                    "The consistent catalog export exceeded its 30 second snapshot bound.",
                ));
            }
        }
    }
    snapshot.execute_batch("PRAGMA journal_mode=DELETE;")?;
    snapshot.close().map_err(|(_, e)| AppError::from(e))?;
    let snapshot = Connection::open_with_flags(&snapshot_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    if crate::catalog_restore::schema_objects(&snapshot)?
        != crate::catalog::current_schema_fingerprint()?
    {
        return Err(invalid(
            "Catalog export requires the exact compiled schema.",
        ));
    }
    let id: String = snapshot.query_row(
        "SELECT uuid FROM catalog_identity WHERE singleton=1",
        [],
        |r| r.get(0),
    )?;
    // Allocate only a new output in the backend-owned private review folder.
    // Retain write access for FlushFileBuffers on Windows, and deny delete
    // sharing there while SQLite opens its separate connection.
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::*;
        options
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_OPEN_NO_RECALL);
    }
    let file = options.open(destination)?;
    #[cfg(windows)]
    let file = filesystem::windows::checked(file)?;
    let identity = filesystem::identity(&file)?;
    if !file.metadata()?.is_file() || identity.links != 1 {
        return Err(conflict());
    }
    let mut output = Connection::open_with_flags(
        destination,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    crate::catalog::initialize_portable(&mut output, &id)?;
    output.execute_batch("PRAGMA foreign_keys=OFF;PRAGMA trusted_schema=OFF;PRAGMA synchronous=FULL;PRAGMA journal_mode=DELETE;")?;
    let tx = output.transaction()?;
    for table in TABLES {
        let mut stmt = snapshot.prepare(&format!("SELECT * FROM {table}"))?;
        let columns = stmt.column_count();
        let mut rows = stmt.query([])?;
        let sql = format!(
            "INSERT INTO {table} VALUES({})",
            vec!["?"; columns].join(",")
        );
        let mut insert = tx.prepare(&sql)?;
        while let Some(row) = rows.next()? {
            let values = (0..columns)
                .map(|i| row.get::<_, Value>(i))
                .collect::<rusqlite::Result<Vec<_>>>()?;
            insert.execute(rusqlite::params_from_iter(values))?;
        }
    }
    tx.execute("UPDATE roots SET native_path=x'',volume='portable-unbound',device='0',inode='0',availability='needs_relink',file_count=(SELECT count(*) FROM assets a WHERE a.root_id=roots.id AND is_media=1 AND presence NOT IN('missing','excluded'))",[])?;
    // Error diagnostics are transient and may contain helper/private endpoint data.
    // Export only the documented metadata allowlist; source catalog retains raw data.
    tx.execute("UPDATE assets SET native_file='',seen_scan='',preview_state='unavailable',preview_error=NULL,metadata_error=NULL",[])?;
    {
        const FIELDS: &[&str] = &[
            "Keywords",
            "Subject",
            "GPSLatitude",
            "GPSLongitude",
            "GPSLatitudeRef",
            "GPSLongitudeRef",
            "DateTimeOriginal",
            "CreateDate",
            "OffsetTimeOriginal",
            "Orientation",
            "ImageWidth",
            "ImageHeight",
            "Make",
            "Model",
            "LensModel",
            "ExifVersion",
        ];
        let mut statement=snapshot.prepare("SELECT id,CASE WHEN length(CAST(raw_metadata AS BLOB))<=1048576 THEN raw_metadata END FROM assets")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let raw: Option<String> = row.get(1)?;
            let safe = raw
                .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
                .and_then(|v| {
                    v.as_array()
                        .and_then(|a| a.first())
                        .and_then(|o| o.as_object())
                        .cloned()
                })
                .map(|mut o| {
                    o.retain(|key, _| FIELDS.contains(&key.as_str()));
                    serde_json::json!([o]).to_string()
                });
            tx.execute(
                "UPDATE assets SET raw_metadata=? WHERE id=?",
                params![safe, id],
            )?;
        }
    }
    // Read-only summaries retain stable history references without copying
    // executable journals, approvals, helper endpoints or credential bindings.
    for table in [
        "organization_plans",
        "operation_journal",
        "backup_sync_jobs",
        "backup_snapshot_jobs",
        "backup_snapshot_records",
    ] {
        let columns = snapshot
            .prepare(&format!("PRAGMA table_info({table})"))?
            .query_map([], |r| r.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let names = columns
            .into_iter()
            .filter(|name| {
                [
                    "id",
                    "plan_id",
                    "profile_id",
                    "state",
                    "created_at",
                    "updated_at",
                    "finished_at",
                    "started_at",
                    "digest",
                    "snapshot_id",
                    "root_id",
                    "phase",
                ]
                .contains(&name.as_str())
            })
            .collect::<Vec<_>>();
        if names.is_empty() {
            continue;
        }
        let mut statement = snapshot.prepare(&format!(
            "SELECT {} FROM {table} LIMIT 50001",
            names.join(",")
        ))?;
        let mut rows = statement.query([])?;
        let mut count = 0;
        while let Some(row) = rows.next()? {
            count += 1;
            if count > 50000 {
                return Err(invalid(
                    "Portable history exceeds 50,000 rows for one history family.",
                ));
            }
            let mut record = serde_json::Map::new();
            for (i, name) in names.iter().enumerate() {
                let value: Option<String> = row.get(i)?;
                if let Some(value) = value {
                    if value.len() > 1024 {
                        return Err(invalid("Portable history reference exceeds its bound."));
                    }
                    record.insert(name.clone(), value.into());
                }
            }
            tx.execute(
                "INSERT INTO job_events(job_id,state) VALUES(?,?)",
                params![
                    format!("portable-history:{table}"),
                    serde_json::Value::Object(record).to_string()
                ],
            )?;
        }
    }
    tx.execute("UPDATE inspection_jobs SET state='interrupted' WHERE state IN('queued','running','cancel_requested')",[])?;
    tx.execute("UPDATE library_state SET revision=(SELECT CAST(revision AS INTEGER) FROM library_state WHERE singleton=1)+1",[])?;
    tx.commit()?;
    if output.prepare("PRAGMA foreign_key_check")?.exists([])? {
        return Err(invalid("Portable catalog foreign-key validation failed."));
    }
    let integrity: String = output.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if integrity != "ok" {
        return Err(invalid("Portable catalog integrity validation failed."));
    }
    output.execute_batch("PRAGMA foreign_keys=ON;")?;
    output.close().map_err(|(_, e)| AppError::from(e))?;
    options.write(false).create_new(false);
    let current = options.open(destination)?;
    #[cfg(windows)]
    let current = filesystem::windows::checked(current)?;
    if !equal(&identity, &filesystem::identity(&current)?)
        || filesystem::identity(&file)?.links != 1
    {
        return Err(conflict());
    }
    drop(current);
    if file.metadata()?.len() > 512 * 1024 * 1024 {
        return Err(invalid(
            "Portable catalog exceeds the 512 MiB export bound.",
        ));
    }
    file.sync_all()?;
    let reopened = Connection::open_with_flags(destination, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    if crate::catalog_restore::schema_objects(&reopened)?
        != crate::catalog::current_schema_fingerprint()?
        || reopened.query_row("SELECT uuid FROM catalog_identity", [], |r| {
            r.get::<_, String>(0)
        })? != id
    {
        return Err(invalid("Independent portable catalog validation failed."));
    }
    Ok(())
}

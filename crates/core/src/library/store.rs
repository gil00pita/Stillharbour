//! All library SQL executes on the existing bounded catalog actor.
use super::{
    filters::{decimal, invalid, validate_date},
    model::*,
    native,
    read_cache::{ReadCache, Summary},
};
use crate::performance::{Endpoint, Metric, Span};
use crate::{
    catalog::Catalog,
    domain::{AppError, ErrorCode, Result},
    filesystem::FileIdentity,
};
use rusqlite::{Connection, OptionalExtension, Row, params, params_from_iter, types::Value};
use std::path::{Path, PathBuf};

// List only the display projection: never fetch native path blobs, raw metadata,
// content digests or scan bookkeeping for a page/neighbor lookup.
pub(super) const ASSET_COLUMNS: &str = "id,root_id,name,relative_path,folder,extension,detected_format,bytes,date,date_source,raw_date,offset,modified_at,born_at,added_at,revision,presence,width,height,camera,metadata_state,metadata_error,preview_state,preview_error,favorite,rating,tags";

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
pub(crate) fn migrate(db: &mut Connection) -> Result<()> {
    let tx = db.transaction()?;
    tx.execute_batch(r#"
    CREATE TABLE roots (
      id TEXT PRIMARY KEY, label TEXT NOT NULL, native_path BLOB NOT NULL, display_path TEXT NOT NULL,
      mode TEXT NOT NULL CHECK(mode IN ('referenced','managed')), is_base INTEGER NOT NULL,
      platform TEXT NOT NULL, volume TEXT NOT NULL, device TEXT NOT NULL, inode TEXT NOT NULL,
      availability TEXT NOT NULL, filter TEXT NOT NULL, last_scan TEXT, file_count INTEGER NOT NULL DEFAULT 0
    );
    CREATE UNIQUE INDEX one_base_root ON roots(is_base) WHERE is_base=1;
    CREATE TABLE assets (
      sequence INTEGER PRIMARY KEY, id TEXT NOT NULL UNIQUE, root_id TEXT NOT NULL REFERENCES roots(id),
      relative_native BLOB NOT NULL, relative_path TEXT NOT NULL, folder_native BLOB NOT NULL,
      folder TEXT NOT NULL, name TEXT NOT NULL, stem TEXT NOT NULL, extension TEXT NOT NULL,
      native_file TEXT NOT NULL, bytes INTEGER NOT NULL CHECK(bytes>=0), revision TEXT NOT NULL,
      modified_at TEXT, born_at TEXT, added_at TEXT NOT NULL, date TEXT NOT NULL, date_source TEXT NOT NULL,
      raw_date TEXT, offset TEXT, detected_format TEXT, width INTEGER, height INTEGER, camera TEXT,
      raw_metadata TEXT, content_digest TEXT, metadata_state TEXT NOT NULL, metadata_error TEXT,
      preview_state TEXT NOT NULL, preview_error TEXT, presence TEXT NOT NULL,
      is_media INTEGER NOT NULL, seen_scan TEXT NOT NULL,
      favorite INTEGER NOT NULL DEFAULT 0 CHECK(favorite IN (0,1)), rating INTEGER NOT NULL DEFAULT 0 CHECK(rating BETWEEN 0 AND 5),
      tags TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(tags)), UNIQUE(root_id,relative_native)
    );
    CREATE TRIGGER media_count_insert AFTER INSERT ON assets WHEN NEW.is_media=1 AND NEW.presence NOT IN ('excluded','missing') BEGIN
      UPDATE roots SET file_count=file_count+1 WHERE id=NEW.root_id;
    END;
    CREATE TRIGGER media_count_update AFTER UPDATE OF presence,is_media,root_id ON assets BEGIN
      UPDATE roots SET file_count=file_count-(OLD.is_media=1 AND OLD.presence NOT IN ('excluded','missing')) WHERE id=OLD.root_id;
      UPDATE roots SET file_count=file_count+(NEW.is_media=1 AND NEW.presence NOT IN ('excluded','missing')) WHERE id=NEW.root_id;
    END;
    CREATE TRIGGER media_count_delete AFTER DELETE ON assets WHEN OLD.is_media=1 AND OLD.presence NOT IN ('excluded','missing') BEGIN
      UPDATE roots SET file_count=file_count-1 WHERE id=OLD.root_id;
    END;
    CREATE INDEX assets_timeline ON assets(date DESC,id DESC) WHERE is_media=1;
    CREATE INDEX assets_root_timeline ON assets(root_id,date DESC,id DESC) WHERE is_media=1;
    CREATE INDEX assets_visible_timeline ON assets(date DESC,id DESC) WHERE is_media=1 AND presence NOT IN ('excluded','missing');
    CREATE INDEX assets_added ON assets(added_at DESC,id DESC) WHERE is_media=1;
    CREATE INDEX assets_modified ON assets(coalesce(modified_at,'') DESC,id DESC) WHERE is_media=1;
    CREATE INDEX assets_name ON assets(name,id);
    CREATE INDEX assets_size ON assets(bytes DESC,id DESC);
    CREATE INDEX assets_directory ON assets(root_id,folder_native);
    CREATE TABLE directories(root_id TEXT NOT NULL REFERENCES roots(id), native_path BLOB NOT NULL,parent_native BLOB,
      seen_scan TEXT NOT NULL,state TEXT NOT NULL,PRIMARY KEY(root_id,native_path));
    CREATE TABLE scans(id TEXT PRIMARY KEY,root_id TEXT NOT NULL REFERENCES roots(id),state TEXT NOT NULL,
      discovered INTEGER NOT NULL DEFAULT 0,indexed INTEGER NOT NULL DEFAULT 0,skipped INTEGER NOT NULL DEFAULT 0,
      errors INTEGER NOT NULL DEFAULT 0,message TEXT NOT NULL,started_at TEXT NOT NULL);
    CREATE INDEX scans_root ON scans(root_id,started_at DESC);
    CREATE TABLE scan_issues(sequence INTEGER PRIMARY KEY,job_id TEXT NOT NULL REFERENCES scans(id),
      relative_path TEXT NOT NULL,code TEXT NOT NULL,message TEXT NOT NULL);
    CREATE TABLE library_state(singleton INTEGER PRIMARY KEY CHECK(singleton=1),revision INTEGER NOT NULL);
    INSERT INTO library_state VALUES(1,0);
    CREATE TABLE selections(id TEXT PRIMARY KEY,created_at TEXT NOT NULL,revision TEXT NOT NULL);
    CREATE TABLE selection_members(selection_id TEXT NOT NULL REFERENCES selections(id) ON DELETE CASCADE,
      asset_id TEXT NOT NULL REFERENCES assets(id),PRIMARY KEY(selection_id,asset_id));
    CREATE TABLE thumbnail_cache(key TEXT PRIMARY KEY,asset_id TEXT NOT NULL REFERENCES assets(id),revision TEXT NOT NULL,
      edge INTEGER NOT NULL,bytes INTEGER NOT NULL,last_used TEXT NOT NULL);
    PRAGMA user_version=2;
    "#)?;
    tx.commit()?;
    Ok(())
}

#[derive(Clone)]
pub struct RootRecord {
    pub view: LibraryRoot,
    pub path: PathBuf,
    pub identity: FileIdentity,
    pub volume: String,
}

fn root_row(row: &Row<'_>) -> rusqlite::Result<RootRecord> {
    let raw: Vec<u8> = row.get("native_path")?;
    let filter: String = row.get("filter")?;
    let mode: String = row.get("mode")?;
    let path = native::decode_path(&raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(e))
    })?;
    Ok(RootRecord {
        view: LibraryRoot {
            id: row.get("id")?,
            label: row.get("label")?,
            display_path: row.get("display_path")?,
            mode: if mode == "managed" {
                RootMode::Managed
            } else {
                RootMode::Referenced
            },
            is_base: row.get("is_base")?,
            availability: row.get("availability")?,
            filter: serde_json::from_str(&filter).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?,
            last_scan: row.get("last_scan")?,
            file_count: row.get("file_count")?,
        },
        path,
        identity: FileIdentity {
            volume: row
                .get::<_, String>("device")?
                .parse()
                .map_err(|_| rusqlite::Error::InvalidQuery)?,
            file: row
                .get::<_, String>("inode")?
                .parse()
                .map_err(|_| rusqlite::Error::InvalidQuery)?,
            links: 1,
        },
        volume: row.get("volume")?,
    })
}
const ROOT_SQL: &str = "SELECT r.* FROM roots r";
pub fn roots(catalog: &Catalog) -> Result<Vec<RootRecord>> {
    catalog.call(|db| {
        Ok(db
            .prepare(&format!("{ROOT_SQL} ORDER BY is_base DESC,label,id"))?
            .query_map([], root_row)?
            .collect::<rusqlite::Result<_>>()?)
    })
}
pub fn root(catalog: &Catalog, id: String) -> Result<RootRecord> {
    catalog.call(move |db| Ok(db.query_row(&format!("{ROOT_SQL} WHERE r.id=?"), [id], root_row)?))
}
pub fn add_root(
    catalog: &Catalog,
    path: PathBuf,
    identity: FileIdentity,
    volume: String,
    label: String,
    mode: RootMode,
    filter: ScanFilter,
) -> Result<String> {
    catalog.call(move |db| {
        let id=uuid::Uuid::new_v4().to_string();
        let base:bool=db.query_row("SELECT count(*)=0 FROM roots",[],|r|r.get(0))?;
        db.execute("INSERT INTO roots(id,label,native_path,display_path,mode,is_base,platform,volume,device,inode,availability,filter) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'online',?11)",params![id,label,native::encode_path(&path),path.to_string_lossy(),mode_name(mode),base,std::env::consts::OS,volume,identity.volume.to_string(),identity.file.to_string(),serde_json::to_string(&filter).map_err(|e|invalid("Invalid scan filter.").caused_by(e))?])?;
        bump(db)?;Ok(id)
    })
}
fn mode_name(mode: RootMode) -> &'static str {
    if mode == RootMode::Managed {
        "managed"
    } else {
        "referenced"
    }
}
pub fn set_availability(catalog: &Catalog, id: String, state: String) -> Result<()> {
    catalog.call(move |db| {
        db.execute(
            "UPDATE roots SET availability=? WHERE id=? AND availability<>?",
            params![state, id, state],
        )?;
        Ok(())
    })
}
pub fn update_mode(catalog: &Catalog, id: String, mode: RootMode) -> Result<()> {
    catalog.call(move |db| {
        if db.execute(
            "UPDATE roots SET mode=? WHERE id=?",
            params![mode_name(mode), id],
        )? != 1
        {
            return Err(invalid("Unknown library root."));
        }
        Ok(())
    })
}
pub fn relink(
    catalog: &Catalog,
    id: String,
    path: PathBuf,
    identity: FileIdentity,
    volume: String,
) -> Result<()> {
    catalog.call(move|db|{
    db.execute("UPDATE roots SET native_path=?,display_path=?,device=?,inode=?,volume=?,availability='online' WHERE id=?",params![native::encode_path(&path),path.to_string_lossy(),identity.volume.to_string(),identity.file.to_string(),volume,id])?;bump(db)
})
}

fn job_row(row: &Row<'_>) -> rusqlite::Result<ScanJob> {
    Ok(ScanJob {
        id: row.get("id")?,
        root_id: row.get("root_id")?,
        state: row.get("state")?,
        discovered: row.get("discovered")?,
        indexed: row.get("indexed")?,
        skipped: row.get("skipped")?,
        errors: row.get("errors")?,
        message: row.get("message")?,
        started_at: row.get("started_at")?,
    })
}
pub fn jobs(catalog: &Catalog) -> Result<Vec<ScanJob>> {
    catalog.call(|db| {
        Ok(db
            .prepare("SELECT * FROM scans ORDER BY started_at DESC LIMIT 32")?
            .query_map([], job_row)?
            .collect::<rusqlite::Result<_>>()?)
    })
}
pub fn job(catalog: &Catalog, id: String) -> Result<ScanJob> {
    catalog.call(move |db| Ok(db.query_row("SELECT * FROM scans WHERE id=?", [id], job_row)?))
}
pub fn begin_job(catalog: &Catalog, root_id: String, filter: ScanFilter) -> Result<ScanJob> {
    catalog.call(move|db|{
    let id=uuid::Uuid::new_v4().to_string();let started=now();
    let tx=db.transaction()?;
    tx.execute("UPDATE roots SET filter=?,availability='scanning' WHERE id=?",params![serde_json::to_string(&filter).map_err(|e|invalid("Invalid filter.").caused_by(e))?,root_id])?;
    tx.execute("UPDATE assets SET preview_state='pending',preview_error=NULL WHERE root_id=? AND preview_state='unavailable'",[&root_id])?;
    tx.execute("INSERT INTO scans(id,root_id,state,message,started_at) VALUES(?,?,'running','Discovering files',?)",params![id,root_id,started])?;
    tx.commit()?;
    Ok(ScanJob{id,root_id,state:"running".into(),discovered:0,indexed:0,skipped:0,errors:0,message:"Discovering files".into(),started_at:started})
})
}
pub fn recover_jobs(catalog: &Catalog) -> Result<Vec<String>> {
    catalog.call(|db|{
    let roots=db.prepare("SELECT DISTINCT root_id FROM scans WHERE state IN ('running','cancel_requested')")?.query_map([],|r|r.get(0))?.collect::<rusqlite::Result<Vec<String>>>()?;
    db.execute("UPDATE scans SET state='interrupted',message='Interrupted; inventory retained. Rescan to reconcile.' WHERE state IN ('running','cancel_requested')",[])?;
    db.execute("UPDATE roots SET availability='unknown' WHERE availability='scanning'",[])?;Ok(roots)
})
}
pub fn progress(
    catalog: &Catalog,
    id: String,
    discovered: u32,
    indexed: u32,
    skipped: u32,
    errors: u32,
    message: String,
) -> Result<()> {
    catalog.call(move |db| {
        db.execute(
            "UPDATE scans SET discovered=?,indexed=?,skipped=?,errors=?,message=? WHERE id=?",
            params![discovered, indexed, skipped, errors, message, id],
        )?;
        Ok(())
    })
}
pub fn cancel_requested(catalog: &Catalog, id: String) -> Result<()> {
    catalog.call(move|db|{db.execute("UPDATE scans SET state='cancel_requested',message='Stopping at a safe read boundary' WHERE id=? AND state='running'",[id])?;Ok(())})
}
pub fn issue(
    catalog: &Catalog,
    id: String,
    relative: String,
    code: String,
    message: String,
) -> Result<()> {
    catalog.call(move|db|{db.execute("INSERT INTO scan_issues(job_id,relative_path,code,message) SELECT ?1,?2,?3,?4 WHERE (SELECT count(*) FROM scan_issues WHERE job_id=?1)<200",params![id,relative,code,message])?;Ok(())})
}
pub fn job_issues(catalog: &Catalog, id: String) -> Result<Vec<String>> {
    catalog.call(move|db|Ok(db.prepare("SELECT relative_path||': '||message FROM scan_issues WHERE job_id=? ORDER BY sequence LIMIT 200")?.query_map([id],|r|r.get(0))?.collect::<rusqlite::Result<_>>()?))
}

#[derive(Clone)]
pub struct Discovery {
    pub relative: PathBuf,
    pub revision: String,
    pub native_file: String,
    pub links: u64,
    pub bytes: i64,
    pub modified: Option<String>,
    pub born: Option<String>,
    pub detected: Option<String>,
    pub media: bool,
    pub placeholder: bool,
}
pub fn discover_batch(
    catalog: &Catalog,
    root_id: String,
    job_id: String,
    entries: Vec<Discovery>,
) -> Result<Vec<(String, String)>> {
    catalog.call(move|db|{
    let tx=db.transaction()?;let mut ids=Vec::with_capacity(entries.len());
    for item in entries {
        let relative=native::encode_path(&item.relative);let name=item.relative.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let folder=item.relative.parent().unwrap_or_else(||Path::new(""));let date=item.modified.clone().or(item.born.clone()).unwrap_or_default();
        let source=if item.modified.is_some(){"filesystem_modified"}else if item.born.is_some(){"filesystem_birth"}else{"undated"};
        let proposed=uuid::Uuid::new_v4().to_string();
        if item.links==1 && item.born.is_some() && !tx.prepare("SELECT 1 FROM assets WHERE root_id=? AND relative_native=?")?.exists(params![root_id,relative])? {
            // A unique one-link object with matching birth evidence can retain
            // catalog identity after an external rename. Ambiguous hard links
            // and missing birth evidence never merge independent locations.
            let candidates=tx.prepare("SELECT id FROM assets WHERE root_id=? AND native_file=? AND born_at=? AND seen_scan<>? LIMIT 2")?.query_map(params![root_id,item.native_file,item.born,job_id],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
            if candidates.len()==1 {
                tx.execute("UPDATE assets SET relative_native=?,relative_path=?,folder_native=?,folder=?,name=?,stem=?,extension=? WHERE id=?",params![relative,item.relative.to_string_lossy(),native::encode_path(folder),folder.to_string_lossy(),name,item.relative.file_stem().unwrap_or_default().to_string_lossy(),item.relative.extension().unwrap_or_default().to_string_lossy().to_ascii_lowercase(),candidates[0]])?;
            }
        }
        tx.execute(r#"INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,modified_at,born_at,added_at,date,date_source,detected_format,metadata_state,preview_state,presence,is_media,seen_scan)
        VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,'present',?21,?22)
        ON CONFLICT(root_id,relative_native) DO UPDATE SET
          name=excluded.name,relative_path=excluded.relative_path,folder=excluded.folder,bytes=excluded.bytes,native_file=excluded.native_file,
          metadata_state=CASE WHEN assets.revision=excluded.revision THEN assets.metadata_state ELSE excluded.metadata_state END,
          metadata_error=CASE WHEN assets.revision=excluded.revision THEN assets.metadata_error END,
          preview_state=CASE WHEN assets.revision=excluded.revision THEN assets.preview_state ELSE excluded.preview_state END,
          preview_error=CASE WHEN assets.revision=excluded.revision THEN assets.preview_error END,
          date=CASE WHEN assets.revision=excluded.revision THEN assets.date ELSE excluded.date END,
          date_source=CASE WHEN assets.revision=excluded.revision THEN assets.date_source ELSE excluded.date_source END,
          raw_date=CASE WHEN assets.revision=excluded.revision THEN assets.raw_date END,
          offset=CASE WHEN assets.revision=excluded.revision THEN assets.offset END,
          raw_metadata=CASE WHEN assets.revision=excluded.revision THEN assets.raw_metadata END,
          content_digest=CASE WHEN assets.revision=excluded.revision THEN assets.content_digest END,
          width=CASE WHEN assets.revision=excluded.revision THEN assets.width END,height=CASE WHEN assets.revision=excluded.revision THEN assets.height END,
          camera=CASE WHEN assets.revision=excluded.revision THEN assets.camera END,
          detected_format=excluded.detected_format,modified_at=excluded.modified_at,born_at=excluded.born_at,
          revision=excluded.revision,presence='present',is_media=excluded.is_media,seen_scan=excluded.seen_scan"#,
          params![proposed,root_id,relative,item.relative.to_string_lossy(),native::encode_path(folder),folder.to_string_lossy(),name,item.relative.file_stem().unwrap_or_default().to_string_lossy(),item.relative.extension().unwrap_or_default().to_string_lossy().to_ascii_lowercase(),item.native_file,item.bytes,item.revision,item.modified,item.born,now(),date,source,item.detected,if item.placeholder{"placeholder"}else{"pending"},if item.placeholder{"placeholder"}else{"pending"},item.media,job_id])?;
        let id:String=tx.query_row("SELECT id FROM assets WHERE root_id=? AND relative_native=?",params![root_id,relative],|r|r.get(0))?;
        ids.push((id,item.revision));
    }
    tx.execute("UPDATE library_state SET revision=revision+1 WHERE singleton=1",[])?;tx.commit()?;Ok(ids)
})
}
#[cfg(unix)]
pub fn directory(
    catalog: &Catalog,
    root_id: String,
    path: PathBuf,
    job_id: String,
    state: String,
) -> Result<()> {
    catalog.call(move|db|{
    let parent=path.parent().map(native::encode_path);
    db.execute("INSERT INTO directories(root_id,native_path,parent_native,seen_scan,state) VALUES(?,?,?,?,?) ON CONFLICT(root_id,native_path) DO UPDATE SET seen_scan=excluded.seen_scan,state=excluded.state",params![root_id,native::encode_path(&path),parent,job_id,state])?;Ok(())
})
}
pub fn exclude_scope(
    catalog: &Catalog,
    root_id: String,
    path: PathBuf,
    job_id: String,
) -> Result<()> {
    catalog.call(move|db|{
    let raw=native::encode_path(&path);let mut prefix=raw.clone();prefix.push(b'/');
    db.execute("UPDATE assets SET presence='excluded',seen_scan=? WHERE root_id=? AND (relative_native=? OR substr(relative_native,1,?)=?)",params![job_id,root_id,raw,prefix.len() as i64,prefix])?;Ok(())
})
}
#[cfg(unix)]
pub fn uncertain(catalog: &Catalog, root_id: String, path: PathBuf, job_id: String) -> Result<()> {
    catalog.call(move|db|{
    db.execute("UPDATE assets SET presence='inaccessible',seen_scan=? WHERE root_id=? AND relative_native=?",params![job_id,root_id,native::encode_path(&path)])?;Ok(())
})
}
pub fn finish_job(catalog: &Catalog, id: String, state: String, message: String) -> Result<()> {
    catalog.call(move|db|{
    let tx=db.transaction()?;let root_id:String=tx.query_row("SELECT root_id FROM scans WHERE id=?",[&id],|r|r.get(0))?;
    if state=="completed"||state=="partial" {
        // Only a completed enumeration of an individual directory establishes
        // absence. Failed, excluded, offline and canceled scopes retain history.
        tx.execute("UPDATE assets SET presence='missing' WHERE root_id=?1 AND seen_scan<>?2 AND folder_native IN (SELECT native_path FROM directories WHERE root_id=?1 AND seen_scan=?2 AND state='complete')",params![root_id,id])?;
        tx.execute("WITH RECURSIVE absent(path) AS (SELECT d.native_path FROM directories d JOIN directories p ON d.root_id=p.root_id AND d.parent_native=p.native_path WHERE d.root_id=?1 AND d.seen_scan<>?2 AND p.seen_scan=?2 AND p.state='complete' UNION SELECT d.native_path FROM directories d JOIN absent a ON d.parent_native=a.path WHERE d.root_id=?1) UPDATE assets SET presence='missing' WHERE root_id=?1 AND seen_scan<>?2 AND folder_native IN (SELECT path FROM absent)",params![root_id,id])?;
        tx.execute("UPDATE roots SET last_scan=? WHERE id=?",params![now(),root_id])?;
    }
    tx.execute("UPDATE scans SET state=?,message=? WHERE id=?",params![state,message,id])?;
    tx.execute("UPDATE roots SET availability=CASE WHEN availability='scanning' THEN 'online' ELSE availability END WHERE id=?",[root_id])?;
    tx.execute("UPDATE library_state SET revision=revision+1 WHERE singleton=1",[])?;tx.commit()?;Ok(())
})
}
fn bump(db: &Connection) -> Result<()> {
    db.execute(
        "UPDATE library_state SET revision=revision+1 WHERE singleton=1",
        [],
    )?;
    Ok(())
}
pub fn revision(catalog: &Catalog) -> Result<String> {
    catalog.call(|db| {
        Ok(db.query_row(
            "SELECT CAST(revision AS TEXT) FROM library_state WHERE singleton=1",
            [],
            |r| r.get(0),
        )?)
    })
}

#[derive(Default, Clone)]
pub struct Metadata {
    pub date: Option<String>,
    pub source: String,
    pub raw_date: Option<String>,
    pub offset: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub camera: Option<String>,
    pub format: Option<String>,
    pub raw: String,
    pub digest: Option<String>,
}
pub fn metadata(
    catalog: &Catalog,
    id: String,
    revision: String,
    value: Option<Metadata>,
    error: Option<String>,
    excluded: bool,
) -> Result<()> {
    catalog.call(move|db|{
    if let Some(m)=value {
        db.execute("UPDATE assets SET date=coalesce(?1,date),date_source=CASE WHEN ?1 IS NULL THEN date_source ELSE ?2 END,raw_date=?3,offset=?4,width=?5,height=?6,camera=?7,detected_format=coalesce(?8,detected_format),raw_metadata=?9,metadata_state='ready',metadata_error=?10,presence=CASE WHEN ?11 THEN 'excluded' ELSE presence END,content_digest=?14 WHERE id=?12 AND revision=?13",params![m.date,m.source,m.raw_date,m.offset,m.width,m.height,m.camera,m.format,m.raw,error,excluded,id,revision,m.digest])?;
    } else {db.execute("UPDATE assets SET metadata_state='unavailable',metadata_error=?,presence=CASE WHEN ? THEN 'excluded' ELSE presence END WHERE id=? AND revision=?",params![error,excluded,id,revision])?;}
    bump(db)
})
}

pub(super) fn asset_row(row: &Row<'_>) -> rusqlite::Result<AssetSummary> {
    let tags: String = row.get("tags")?;
    let (preview_state, preview_error) =
        current_preview_state(row.get("preview_state")?, row.get("preview_error")?);
    Ok(AssetSummary {
        id: row.get("id")?,
        root_id: row.get("root_id")?,
        name: row.get("name")?,
        relative_path: row.get("relative_path")?,
        folder: row.get("folder")?,
        extension: row.get("extension")?,
        detected_format: row.get("detected_format")?,
        bytes: row.get::<_, i64>("bytes")?.to_string(),
        date: row.get("date")?,
        date_source: row.get("date_source")?,
        raw_date: row.get("raw_date")?,
        offset: row.get("offset")?,
        modified_at: row.get("modified_at")?,
        born_at: row.get("born_at")?,
        added_at: row.get("added_at")?,
        revision: row.get("revision")?,
        presence: row.get("presence")?,
        width: row.get("width")?,
        height: row.get("height")?,
        camera: row.get("camera")?,
        metadata_state: row.get("metadata_state")?,
        metadata_error: row.get("metadata_error")?,
        preview_state,
        preview_error,
        favorite: row.get("favorite")?,
        rating: row.get("rating")?,
        tags: serde_json::from_str(&tags).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
    })
}
pub fn asset(catalog: &Catalog, id: String) -> Result<AssetSummary> {
    catalog.call(move |db| Ok(db.query_row("SELECT * FROM assets WHERE id=?", [id], asset_row)?))
}
pub fn asset_native(catalog: &Catalog, id: String) -> Result<(String, PathBuf, String)> {
    catalog.call(move |db| {
        let (root, raw, revision): (String, Vec<u8>, String) = db.query_row(
            "SELECT root_id,relative_native,revision FROM assets WHERE id=?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        Ok((root, native::decode_path(&raw)?, revision))
    })
}
pub fn detail(catalog: &Catalog, id: String) -> Result<AssetDetail> {
    let item = asset(catalog, id.clone())?;
    let root = root(catalog, item.root_id.clone())?.view;
    catalog.call(move|db|{
        let (raw,stem,folder):(Option<String>,String,Vec<u8>)=db.query_row("SELECT raw_metadata,stem,folder_native FROM assets WHERE id=?",[&id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        let companions=db.prepare("SELECT name FROM assets WHERE root_id=? AND folder_native=? AND stem=? AND id<>? AND presence='present' ORDER BY name LIMIT 100")?.query_map(params![item.root_id,folder,stem,id],|r|r.get(0))?.collect::<rusqlite::Result<_>>()?;
        Ok(AssetDetail{asset:item,root,raw_metadata:raw,companions})
    })
}

pub(super) fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
pub(crate) fn query_where(query: &AssetQuery) -> Result<(String, Vec<Value>)> {
    if query.name.len() > 256
        || query.folder.len() > 2048
        || query.tag.len() > 128
        || query.extension.len() > 16
    {
        return Err(invalid("Query field exceeds its limit."));
    }
    validate_date(&query.from_date)?;
    validate_date(&query.through_date)?;
    if !query.from_date.is_empty()
        && !query.through_date.is_empty()
        && query.from_date > query.through_date
    {
        return Err(invalid("Start date exceeds end date."));
    }
    let mut sql = String::from("is_media=1");
    let mut values = Vec::new();
    if !query.include_missing {
        sql.push_str(" AND presence NOT IN ('excluded','missing')");
    } else {
        sql.push_str(" AND presence<>'excluded'");
    }
    if let Some(root) = &query.root_id {
        sql.push_str(" AND root_id=?");
        values.push(root.clone().into());
    }
    if !query.name.is_empty() {
        sql.push_str(" AND name LIKE ? ESCAPE '\\'");
        values.push(format!("%{}%", escape_like(&query.name)).into());
    }
    if let Some(id) = &query.directory_id {
        if query.root_id.is_none() {
            return Err(invalid("An exact directory query requires a library."));
        }
        sql.push_str(" AND folder_native=?");
        values.push(super::directory::decode_id(id)?.into());
    }
    if !query.folder.is_empty() {
        sql.push_str(" AND (folder=? OR folder LIKE ? ESCAPE '\\')");
        values.push(query.folder.clone().into());
        values.push(format!("{}/%", escape_like(query.folder.trim_end_matches('/'))).into());
    }
    if !query.from_date.is_empty() {
        sql.push_str(" AND date>=?");
        values.push(query.from_date.clone().into());
    }
    if !query.through_date.is_empty() {
        sql.push_str(" AND date<?");
        values.push(format!("{}T99", query.through_date).into());
    }
    let min = decimal(&query.min_bytes)?;
    let max = decimal(&query.max_bytes)?;
    if min.zip(max).is_some_and(|(a, b)| a > b) {
        return Err(invalid("Minimum size exceeds maximum size."));
    }
    for (value, clause) in [(min, " AND bytes>=?"), (max, " AND bytes<=?")] {
        if let Some(value) = value {
            sql.push_str(clause);
            values.push(
                i64::try_from(value)
                    .map_err(|_| invalid("Size exceeds catalog limits."))?
                    .into(),
            );
        }
    }
    if !query.extension.is_empty() {
        sql.push_str(" AND extension=?");
        values.push(query.extension.to_ascii_lowercase().into());
    }
    if query.favorites_only {
        sql.push_str(" AND favorite=1");
    }
    if !query.tag.is_empty() {
        sql.push_str(" AND EXISTS(SELECT 1 FROM json_each(assets.tags) WHERE value=?)");
        values.push(query.tag.clone().into());
    }
    if !matches!(
        query.sort.as_str(),
        "capture" | "name" | "size" | "modified" | "added"
    ) {
        return Err(invalid("Unknown sort order."));
    }
    Ok((sql, values))
}
#[derive(Serialize, Deserialize)]
struct Cursor {
    query: String,
    key: String,
    id: String,
}
use serde::{Deserialize, Serialize};
pub fn query(
    catalog: &Catalog,
    query: AssetQuery,
    cursor: Option<String>,
    limit: u32,
) -> Result<AssetPage> {
    let _span = Span::new(Metric::PageQuery, Endpoint::Catalog);
    if limit == 0 || limit > 240 {
        return Err(invalid("Page size must be between 1 and 240."));
    }
    let (where_sql, mut values) = query_where(&query)?;
    let hash = query_hash(&query)?;
    let (column, direction, comparison) = sort_order(&query.sort);
    let mut page_where = where_sql.clone();
    if let Some(cursor) = cursor {
        if cursor.len() > 8192 {
            return Err(invalid("Cursor is oversized."));
        }
        let c: Cursor =
            serde_json::from_str(&cursor).map_err(|_| invalid("Invalid page cursor."))?;
        if c.query != hash || c.key.len() > 4096 || c.id.len() > 64 {
            return Err(invalid("Cursor belongs to another query."));
        }
        let key = if query.sort == "size" {
            Value::Integer(c.key.parse().map_err(|_| invalid("Invalid size cursor."))?)
        } else {
            Value::Text(c.key)
        };
        page_where.push_str(&format!(" AND ({column},id){comparison}(?,?)"));
        values.extend([key, c.id.into()]);
    }
    let base_values = query_where(&query)?.1;
    let source = ordered_source(&query);
    catalog.cached(move|db,cache|{
        let total = cached_count(db,cache,&query,&where_sql,&base_values,&hash)?;
        let sql=format!("SELECT {ASSET_COLUMNS} FROM {source} WHERE {page_where} ORDER BY {column} {direction},id {direction} LIMIT {}",limit+1);
        let mut assets=db.prepare(&sql)?.query_map(params_from_iter(values),asset_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
        let more=assets.len()>limit as usize;assets.truncate(limit as usize);
        let next=if more {assets.last().map(|last|serde_json::to_string(&Cursor{query:hash,key:asset_sort_key(last, &query.sort),id:last.id.clone()})).transpose().map_err(|e|invalid("Could not encode cursor.").caused_by(e))?}else{None};
        let revision:String=db.query_row("SELECT CAST(revision AS TEXT) FROM library_state WHERE singleton=1",[],|r|r.get(0))?;
        Ok(AssetPage{assets,next_cursor:next,total,revision})
    })
}
fn query_hash(query: &AssetQuery) -> Result<String> {
    Ok(blake3::hash(
        &serde_json::to_vec(query).map_err(|e| invalid("Invalid query.").caused_by(e))?,
    )
    .to_hex()
    .to_string())
}

fn inventory_query(query: &AssetQuery) -> bool {
    query.directory_id.is_none()
        && !query.favorites_only
        && [
            &query.folder,
            &query.name,
            &query.from_date,
            &query.through_date,
            &query.min_bytes,
            &query.max_bytes,
            &query.extension,
            &query.tag,
        ]
        .iter()
        .all(|field| field.is_empty())
}

// ANALYZE can make SQLite prefer a root-index skip scan plus a temporary sort
// for an unfiltered global LIMIT. Pin only this verified, simple browsing case;
// selective folder/tag/name filters keep the planner's narrower alternatives.
pub(super) fn ordered_source(query: &AssetQuery) -> &'static str {
    if !inventory_query(query) {
        return "assets";
    }
    match (
        query.sort.as_str(),
        query.root_id.is_some(),
        query.include_missing,
    ) {
        ("capture", true, _) => "assets INDEXED BY assets_root_timeline",
        ("capture", false, false) => "assets INDEXED BY assets_visible_timeline",
        ("capture", false, true) => "assets INDEXED BY assets_timeline",
        ("added", _, _) => "assets INDEXED BY assets_added",
        ("modified", _, _) => "assets INDEXED BY assets_modified",
        ("name", _, _) => "assets INDEXED BY assets_name",
        ("size", _, _) => "assets INDEXED BY assets_size",
        _ => "assets",
    }
}

fn cached_count(
    db: &Connection,
    cache: &mut ReadCache,
    query: &AssetQuery,
    where_sql: &str,
    values: &[Value],
    hash: &str,
) -> Result<u32> {
    let key = format!("count:{hash}");
    if let Some(Summary::Count(count)) = cache.get(db, &key)? {
        return Ok(count);
    }
    let count = if inventory_query(query) && !query.include_missing {
        if let Some(root) = &query.root_id {
            db.query_row(
                "SELECT coalesce(sum(file_count),0) FROM roots WHERE id=?",
                [root],
                |r| r.get(0),
            )?
        } else {
            db.query_row("SELECT coalesce(sum(file_count),0) FROM roots", [], |r| {
                r.get(0)
            })?
        }
    } else {
        db.query_row(
            &format!("SELECT count(*) FROM assets WHERE {where_sql}"),
            params_from_iter(values),
            |r| r.get(0),
        )?
    };
    cache.insert(key, Summary::Count(count));
    Ok(count)
}
fn asset_sort_key(asset: &AssetSummary, sort: &str) -> String {
    match sort {
        "name" => asset.name.clone(),
        "size" => asset.bytes.clone(),
        "added" => asset.added_at.clone(),
        "modified" => asset.modified_at.clone().unwrap_or_default(),
        _ => asset.date.clone(),
    }
}
/// Read a bounded window around a stable catalog identity, including when it no
/// longer matches the filter. The anchor supplies only its native sort key;
/// every returned row must still match the validated query.
pub fn seek_asset(
    catalog: &Catalog,
    query: AssetQuery,
    id: String,
    position: AssetSeekPosition,
    limit: u32,
) -> Result<AssetPage> {
    if uuid::Uuid::parse_str(&id).is_err() || !(1..=240).contains(&limit) {
        return Err(invalid("Invalid photo identity or page size."));
    }
    let (where_sql, mut values) = query_where(&query)?;
    let hash = query_hash(&query)?;
    let source = ordered_source(&query);
    let (column, direction, comparison) = sort_order(&query.sort);
    let before = position == AssetSeekPosition::Before;
    let cmp = if before {
        if comparison == "<" { ">" } else { "<" }
    } else if comparison == "<" {
        "<="
    } else {
        ">="
    };
    let order = if before {
        if direction == "DESC" { "ASC" } else { "DESC" }
    } else {
        direction
    };
    catalog.cached(move |db,cache| {
        let total = cached_count(db,cache,&query,&where_sql,&values,&hash)?;
        let key: Value = db.query_row(&format!("SELECT {column} FROM assets WHERE id=?"), [&id], |r| r.get(0)).optional()?.ok_or_else(|| invalid("This photo is no longer in the catalog."))?;
        values.extend([key,id.into()]);
        let mut assets = db.prepare(&format!("SELECT {ASSET_COLUMNS} FROM {source} WHERE {where_sql} AND ({column},id){cmp}(?,?) ORDER BY {column} {order},id {order} LIMIT {}",limit+1))?.query_map(params_from_iter(values),asset_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
        let more = assets.len() > limit as usize;
        assets.truncate(limit as usize);
        if before { assets.reverse(); }
        let next_cursor = if more || before { assets.last().map(|asset| serde_json::to_string(&Cursor { query: hash, key:asset_sort_key(asset,&query.sort),id:asset.id.clone() })).transpose().map_err(|e| invalid("Could not encode cursor.").caused_by(e))? } else { None };
        let revision = db.query_row("SELECT CAST(revision AS TEXT) FROM library_state WHERE singleton=1",[],|r|r.get(0))?;
        Ok(AssetPage { assets, total, next_cursor, revision })
    })
}

fn sort_order(sort: &str) -> (&'static str, &'static str, &'static str) {
    match sort {
        "name" => ("name", "ASC", ">"),
        "size" => ("bytes", "DESC", "<"),
        "modified" => ("coalesce(modified_at,'')", "DESC", "<"),
        "added" => ("added_at", "DESC", "<"),
        _ => ("date", "DESC", "<"),
    }
}

pub fn neighbors(catalog: &Catalog, query: AssetQuery, id: String) -> Result<AssetNeighbors> {
    if uuid::Uuid::parse_str(&id).is_err() {
        return Err(invalid("Invalid photo identity."));
    }
    let (where_sql, values) = query_where(&query)?;
    let (column, direction, comparison) = sort_order(&query.sort);
    let source = ordered_source(&query);
    catalog.call(move |db| {
        let mut anchor_values = values.clone(); anchor_values.push(id.clone().into());
        let key: Value = db.query_row(&format!("SELECT {column} FROM assets WHERE {where_sql} AND id=?"), params_from_iter(anchor_values), |row| row.get(0))
            .optional()?.ok_or_else(|| invalid("This photo is no longer in the filtered collection."))?;
        let find = |cmp: &str, order: &str| -> Result<Option<AssetSummary>> {
            let mut parameters = values.clone(); parameters.extend([key.clone(), id.clone().into()]);
            Ok(db.query_row(&format!("SELECT {ASSET_COLUMNS} FROM {source} WHERE {where_sql} AND ({column},id){cmp}(?,?) ORDER BY {column} {order},id {order} LIMIT 1"), params_from_iter(parameters), asset_row).optional()?)
        };
        Ok(AssetNeighbors {
            next: find(comparison, direction)?,
            previous: find(if comparison == "<" { ">" } else { "<" }, if direction == "DESC" { "ASC" } else { "DESC" })?,
        })
    })
}

pub fn buckets(catalog: &Catalog, query: AssetQuery) -> Result<Vec<DateBucket>> {
    let _span = Span::new(Metric::DateBuckets, Endpoint::Catalog);
    let (where_sql, values) = query_where(&query)?;
    let date_column = if query.sort == "added" {
        "added_at"
    } else {
        "date"
    };
    let key = format!("buckets:{}", query_hash(&query)?);
    catalog.cached(move |db,cache| {
        if let Some(Summary::Buckets(buckets)) = cache.get(db, &key)? { return Ok(buckets); }

        let rows = db.prepare(&format!("SELECT CASE WHEN length({date_column})<10 THEN 'Undated' ELSE substr({date_column},1,7) END AS period,count(*) FROM assets WHERE {where_sql} GROUP BY period ORDER BY (period='Undated'),period DESC"))?
            .query_map(params_from_iter(values), |r| Ok((r.get::<_,String>(0)?,r.get::<_,u32>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut buckets: Vec<DateBucket> = Vec::new();
        for (period, count) in rows {
            let year = if period == "Undated" { period.clone() } else { period.get(..4).ok_or_else(|| invalid("Invalid catalog date bucket."))?.to_owned() };
            let month = MonthBucket { period, count };
            if let Some(bucket) = buckets.last_mut().filter(|bucket| bucket.year == year) {
                bucket.count = bucket.count.checked_add(count).ok_or_else(|| invalid("Date count exceeds the supported catalog size."))?;
                bucket.months.push(month);
            } else {
                buckets.push(DateBucket { year, count, months: vec![month] });
            }
        }
        cache.insert(key,Summary::Buckets(buckets.clone()));
        Ok(buckets)
    })
}
pub fn seek(catalog: &Catalog, query: AssetQuery, year: String, limit: u32) -> Result<AssetPage> {
    if query.sort != "capture"
        || (year != "Undated" && (year.len() != 4 || !year.bytes().all(|b| b.is_ascii_digit())))
    {
        return Err(invalid(
            "Year navigation requires capture-date sort and a valid year.",
        ));
    }
    seek_date(catalog, query, year, limit)
}

pub fn seek_date(
    catalog: &Catalog,
    query: AssetQuery,
    period: String,
    limit: u32,
) -> Result<AssetPage> {
    let _span = Span::new(Metric::DateSeek, Endpoint::Catalog);
    let valid_year = period.len() == 4 && period.bytes().all(|b| b.is_ascii_digit());
    let valid_month = period.len() == 7
        && period.is_ascii()
        && period.as_bytes()[4] == b'-'
        && period[..4].bytes().all(|b| b.is_ascii_digit())
        && period[5..].bytes().all(|b| b.is_ascii_digit())
        && period[5..]
            .parse::<u32>()
            .is_ok_and(|m| (1..=12).contains(&m));
    if !matches!(query.sort.as_str(), "capture" | "added")
        || !(valid_year || valid_month || period == "Undated")
    {
        return Err(invalid(
            "Date navigation requires a chronological sort and YYYY, YYYY-MM or Undated.",
        ));
    }
    query_where(&query)?;
    let hash = blake3::hash(
        &serde_json::to_vec(&query).map_err(|e| invalid("Invalid query.").caused_by(e))?,
    )
    .to_hex()
    .to_string();
    let key = if period == "Undated" {
        "!".into()
    } else if valid_month {
        format!("{period}-31T99")
    } else {
        format!("{period}-12-31T99")
    };
    let cursor = serde_json::to_string(&Cursor {
        query: hash,
        key,
        id: "~".into(),
    })
    .map_err(|e| invalid("Invalid year cursor.").caused_by(e))?;
    self::query(catalog, query, Some(cursor), limit)
}
pub fn freeze(catalog: &Catalog, query: AssetQuery) -> Result<SelectionToken> {
    let _span = Span::new(Metric::SelectionFreeze, Endpoint::Catalog);
    let (where_sql, values) = query_where(&query)?;
    catalog.call(move |db| {
        let tx = db.transaction()?;
        tx.execute(
            "DELETE FROM selections WHERE julianday(created_at) < julianday('now','-1 hour')",
            [],
        )?;
        let active: u32 = tx.query_row("SELECT count(*) FROM selections", [], |r| r.get(0))?;
        if active >= 32 {
            return Err(AppError::new(
                ErrorCode::Busy,
                "Too many active query selections. Reopen the library to release them.",
            ));
        }
        let id = uuid::Uuid::new_v4().to_string();
        let revision: String = tx.query_row(
            "SELECT CAST(revision AS TEXT) FROM library_state WHERE singleton=1",
            [],
            |r| r.get(0),
        )?;
        tx.execute(
            "INSERT INTO selections VALUES(?,?,?)",
            params![id, now(), revision],
        )?;
        let mut bindings = vec![Value::Text(id.clone())];
        bindings.extend(values);
        let count = tx.execute(
            &format!("INSERT INTO selection_members SELECT ?,id FROM assets WHERE {where_sql}"),
            params_from_iter(bindings),
        )?;
        tx.commit()?;
        Ok(SelectionToken {
            id,
            count: u32::try_from(count).map_err(|_| invalid("Selection is too large."))?,
            revision,
        })
    })
}
fn selection_scope(db: &Connection, selection: AssetSelection) -> Result<(String, Vec<Value>)> {
    match selection {
        AssetSelection::Ids { ids } => {
            if ids.is_empty() || ids.len() > 500 || ids.iter().any(|id| id.len() > 64) {
                return Err(invalid("Select between one and 500 explicit assets."));
            }
            Ok((
                format!("id IN ({})", vec!["?"; ids.len()].join(",")),
                ids.into_iter().map(Value::Text).collect(),
            ))
        }
        AssetSelection::Token { id } => {
            if !db
                .prepare("SELECT 1 FROM selections WHERE id=?")?
                .exists([&id])?
            {
                return Err(invalid("Selection expired. Select the query again."));
            }
            Ok((
                "id IN (SELECT asset_id FROM selection_members WHERE selection_id=?)".into(),
                vec![id.into()],
            ))
        }
    }
}

pub fn add_tags(catalog: &Catalog, selection: AssetSelection, tags: Vec<String>) -> Result<u32> {
    if tags.is_empty()
        || tags.len() > 64
        || tags.iter().any(|tag| {
            tag.trim().is_empty() || tag.len() > 128 || tag.chars().any(char::is_control)
        })
    {
        return Err(invalid(
            "Add between one and 64 nonempty tags of up to 128 bytes each.",
        ));
    }
    let tags: Vec<_> = tags.into_iter().map(|tag| tag.trim().to_owned()).collect();
    let json = serde_json::to_string(&tags).map_err(|e| invalid("Invalid tags.").caused_by(e))?;
    catalog.call(move |db| {
        let tx = db.transaction()?;
        let (scope, values) = selection_scope(&tx, selection)?;
        let mut parameters = vec![Value::Text(json)]; parameters.extend(values.clone());
        let count = tx.execute(&format!("UPDATE assets SET tags=(SELECT json_group_array(value) FROM (SELECT value FROM json_each(assets.tags) UNION SELECT value FROM json_each(?))) WHERE {scope}"), params_from_iter(parameters))?;
        if tx.prepare(&format!("SELECT 1 FROM assets WHERE {scope} AND json_array_length(tags)>64 LIMIT 1"))?.exists(params_from_iter(values))? {
            return Err(invalid("Adding these tags would exceed the 64-tag limit. No tags were changed."));
        }
        let count = u32::try_from(count).map_err(|_| invalid("Selection is too large."))?;
        bump(&tx)?; tx.commit()?;
        Ok(count)
    })
}

pub fn annotate(
    catalog: &Catalog,
    selection: AssetSelection,
    patch: AnnotationPatch,
) -> Result<u32> {
    if patch.rating.is_some_and(|r| r > 5) {
        return Err(invalid("Ratings range from zero to five."));
    }
    let tags = patch
        .tags
        .map(|tags| -> Result<String> {
            if tags.len() > 64
                || tags.iter().any(|t| {
                    t.trim().is_empty() || t.len() > 128 || t.chars().any(char::is_control)
                })
            {
                return Err(invalid("Use at most 64 nonempty tags of 128 bytes each."));
            }
            let mut tags: Vec<_> = tags.into_iter().map(|t| t.trim().to_owned()).collect();
            tags.sort();
            tags.dedup();
            serde_json::to_string(&tags).map_err(|e| invalid("Invalid tags.").caused_by(e))
        })
        .transpose()?;
    catalog.call(move|db|{
        let tx=db.transaction()?;
        let (scope, values) = selection_scope(&tx, selection)?;
        let mut params=vec![patch.favorite.map(|b|Value::Integer(i64::from(b))).unwrap_or(Value::Null),patch.rating.map(|r|Value::Integer(i64::from(r))).unwrap_or(Value::Null),tags.map(Value::Text).unwrap_or(Value::Null)];params.extend(values);
        let count=tx.execute(&format!("UPDATE assets SET favorite=coalesce(?,favorite),rating=coalesce(?,rating),tags=coalesce(?,tags) WHERE {scope}"),params_from_iter(params))?;
        tx.execute("UPDATE library_state SET revision=revision+1 WHERE singleton=1",[])?;tx.commit()?;Ok(count as u32)
    })
}

#[derive(Serialize, Deserialize)]
struct PreviewFailure {
    policy: String,
    message: String,
}

pub(crate) fn encode_preview_error(error: Option<String>) -> Result<Option<String>> {
    error
        .map(|message| {
            serde_json::to_string(&PreviewFailure {
                policy: super::THUMBNAIL_POLICY.into(),
                message,
            })
            .map_err(|e| invalid("Could not encode preview state.").caused_by(e))
        })
        .transpose()
}
fn current_preview_state(state: String, error: Option<String>) -> (String, Option<String>) {
    if state != "unavailable" {
        return (state, error);
    }
    if let Some(value) = error
        .as_deref()
        .and_then(|text| serde_json::from_str::<PreviewFailure>(text).ok())
        && value.policy == super::THUMBNAIL_POLICY
    {
        return (state, Some(value.message));
    }
    // Derived failures from another processing policy must not suppress the
    // first request to a newly capable decoder. Never rewrite rows on open.
    ("pending".into(), None)
}

pub fn preview_state(
    catalog: &Catalog,
    id: String,
    revision: String,
    state: String,
    error: Option<String>,
) -> Result<()> {
    let error = encode_preview_error(error)?;
    catalog.call(move |db| {
        db.execute(
            "UPDATE assets SET preview_state=?,preview_error=? WHERE id=? AND revision=?",
            params![state, error, id, revision],
        )?;
        Ok(())
    })
}
pub fn cache_touch(catalog: &Catalog, key: String) -> Result<bool> {
    catalog.call(move |db| {
        Ok(db.execute(
            "UPDATE thumbnail_cache SET last_used=? WHERE key=?",
            params![now(), key],
        )? > 0)
    })
}
pub fn cache_add(
    catalog: &Catalog,
    key: String,
    id: String,
    revision: String,
    edge: u32,
    bytes: i64,
) -> Result<Vec<String>> {
    catalog.call(move|db|{
    db.execute("INSERT INTO thumbnail_cache VALUES(?,?,?,?,?,?) ON CONFLICT(key) DO UPDATE SET last_used=excluded.last_used",params![key,id,revision,edge,bytes,now()])?;
    let mut total:i64=db.query_row("SELECT coalesce(sum(bytes),0) FROM thumbnail_cache",[],|r|r.get(0))?;
    let candidates=db.prepare("SELECT key,bytes FROM thumbnail_cache ORDER BY last_used,key")?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let mut removed=Vec::new();
    for (key,size) in candidates {if total<=256*1024*1024{break}db.execute("DELETE FROM thumbnail_cache WHERE key=?",[&key])?;total-=size;removed.push(key);}
    Ok(removed)
})
}
pub fn sample_assets(catalog: &Catalog, root_id: String) -> Result<Vec<(PathBuf, String)>> {
    catalog.call(move|db|{
    let raw=db.prepare("SELECT relative_native,content_digest FROM assets WHERE root_id=? AND presence='present' AND content_digest IS NOT NULL ORDER BY sequence LIMIT 8")?.query_map([root_id],|r|Ok((r.get::<_,Vec<u8>>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
    raw.into_iter().map(|(p,r)|Ok((native::decode_path(&p)?,r))).collect()
})
}

// Cache index remains the durable, retryable ownership list until all file
// removals succeed. These operations never resolve or enumerate an original.
pub fn root_exists(catalog: &Catalog, id: String) -> Result<bool> {
    catalog.call(move |db| {
        Ok(
            db.query_row("SELECT EXISTS(SELECT 1 FROM roots WHERE id=?)", [id], |r| {
                r.get(0)
            })?,
        )
    })
}
pub fn maintenance_preview(
    catalog: &Catalog,
    target: MaintenanceTarget,
) -> Result<(MaintenancePreview, String)> {
    catalog.call(move |db| {
        let root_id = match &target { MaintenanceTarget::Library {root_id} => {
            if uuid::Uuid::parse_str(root_id).is_err() { return Err(invalid("Invalid library identity.")); } Some(root_id.clone())
        }, MaintenanceTarget::PreviewCache => None };
        let (root_label,root_display_path)=if let Some(id)=&root_id {db.query_row("SELECT label,display_path FROM roots WHERE id=?",[id],|r|Ok((Some(r.get::<_,String>(0)?),Some(r.get::<_,String>(1)?))))?} else {(None,None)};
        let asset_count = if let Some(id) = &root_id { db.query_row("SELECT count(*) FROM assets WHERE root_id=?", [id], |r|r.get(0))? } else {0};
        let (cached_previews, cache_bytes) = db.query_row("SELECT count(*),CAST(coalesce(sum(c.bytes),0) AS TEXT) FROM thumbnail_cache c JOIN assets a ON a.id=c.asset_id WHERE (?1 IS NULL OR a.root_id=?1)", [&root_id], |r| Ok((r.get(0)?,r.get(1)?)))?;
        let revision = db.query_row("SELECT CAST(revision AS TEXT) FROM library_state WHERE singleton=1",[],|r|r.get(0))?;
        Ok((MaintenancePreview {id:uuid::Uuid::new_v4().to_string(),target,root_label,root_display_path,asset_count,cached_previews,cache_bytes},revision))
    })
}
pub fn cache_keys(catalog: &Catalog, root: Option<String>, after: String) -> Result<Vec<String>> {
    catalog.call(move|db| Ok(db.prepare("SELECT c.key FROM thumbnail_cache c JOIN assets a ON a.id=c.asset_id WHERE (?1 IS NULL OR a.root_id=?1) AND c.key>?2 ORDER BY c.key LIMIT 256")?.query_map(params![root,after],|r|r.get(0))?.collect::<rusqlite::Result<_>>()?))
}
pub fn finish_maintenance(
    catalog: &Catalog,
    root: Option<String>,
    revision: String,
) -> Result<MaintenanceResult> {
    catalog.call(move |db| {
        let tx=db.transaction()?;
        let current: String=tx.query_row("SELECT CAST(revision AS TEXT) FROM library_state WHERE singleton=1",[],|r|r.get(0))?;
        if current!=revision { return Err(AppError::new(ErrorCode::Conflict,"The catalog changed. Review this action again. Already cleared previews can regenerate.")); }
        let cleared_entries = if let Some(id)=&root {
            if !tx.query_row("SELECT EXISTS(SELECT 1 FROM roots WHERE id=?)",[id],|r|r.get::<_,bool>(0))? {return Err(invalid("Unknown library root."));}
            tx.execute("DELETE FROM selections WHERE id IN (SELECT m.selection_id FROM selection_members m JOIN assets a ON a.id=m.asset_id WHERE a.root_id=?)",[id])?;
            let count=tx.execute("DELETE FROM thumbnail_cache WHERE asset_id IN (SELECT id FROM assets WHERE root_id=?)",[id])?;
            tx.execute("DELETE FROM scan_issues WHERE job_id IN (SELECT id FROM scans WHERE root_id=?)",[id])?;
            tx.execute("DELETE FROM scans WHERE root_id=?",[id])?;
            tx.execute("DELETE FROM directories WHERE root_id=?",[id])?;
            tx.execute("DELETE FROM assets WHERE root_id=?",[id])?;
            tx.execute("DELETE FROM roots WHERE id=?",[id])?;
            tx.execute("UPDATE roots SET is_base=1 WHERE id=(SELECT id FROM roots ORDER BY label,id LIMIT 1) AND NOT EXISTS(SELECT 1 FROM roots WHERE is_base=1)",[])?;
            count
        } else {
            let count=tx.execute("DELETE FROM thumbnail_cache",[])?;
            tx.execute("UPDATE assets SET preview_state='pending',preview_error=NULL WHERE preview_state='ready'",[])?;
            count
        };
        bump(&tx)?;tx.commit()?;
        Ok(MaintenanceResult {removed_root_id:root,cleared_entries:cleared_entries as u32})
    })
}

#[cfg(test)]
mod preview_policy_tests {
    use super::*;
    #[test]
    fn legacy_failures_retry_but_current_failures_are_suppressed_and_envelope_stays_private() {
        assert_eq!(
            current_preview_state("unavailable".into(), Some("Legacy HEIC unsupported".into())),
            ("pending".into(), None)
        );
        let message = Some("This file cannot be previewed.".to_owned());
        let encoded = encode_preview_error(message.clone()).unwrap();
        assert_eq!(
            current_preview_state("unavailable".into(), encoded),
            ("unavailable".into(), message)
        );
        let other = serde_json::to_string(&PreviewFailure {
            policy: "old-policy".into(),
            message: "legacy".into(),
        })
        .unwrap();
        assert_eq!(
            current_preview_state("unavailable".into(), Some(other)),
            ("pending".into(), None)
        );
        assert_eq!(
            current_preview_state("ready".into(), None),
            ("ready".into(), None)
        );
    }
}

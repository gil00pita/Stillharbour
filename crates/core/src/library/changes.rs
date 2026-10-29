//! Durable catalog observations, distinct from filesystem events or recovery evidence.
use super::{filters::invalid, model::*, native, store};
use crate::{catalog::Catalog, domain::Result};
use rusqlite::{Connection, params};
use std::path::Path;

pub(super) fn record(db: &Connection, change: LibraryChange) -> Result<()> {
    let journal = format!("library-change:{}", change.root_id);
    let payload =
        serde_json::to_string(&change).map_err(|_| invalid("Invalid change observation."))?;
    db.execute(
        "INSERT INTO job_events(job_id,state) VALUES(?,?)",
        params![journal, payload],
    )?;
    Ok(())
}

pub(super) fn file_change(
    db: &Connection,
    root: &str,
    scan: &str,
    id: &str,
    previous: Option<(String, String, String, Vec<u8>)>,
    path: &Path,
    revision: &str,
) -> Result<()> {
    let (label, baseline): (String, bool) = db.query_row(
        "SELECT label,last_scan IS NOT NULL FROM roots WHERE id=?",
        [root],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if !baseline {
        return Ok(());
    }
    let raw = native::encode_path(path);
    let locator = super::directory::encode_id(&raw);
    let path = path.to_string_lossy().into_owned();
    let kind = match &previous {
        None => LibraryChangeKind::Added,
        Some((_, _, _, old)) if old != &raw => LibraryChangeKind::Moved,
        Some((_, _, presence, _)) if presence == "missing" => LibraryChangeKind::Reappeared,
        Some((_, old, _, _)) if old != revision => LibraryChangeKind::Updated,
        _ => return Ok(()),
    };
    record(
        db,
        LibraryChange {
            version: 1,
            sequence: String::new(),
            root_id: root.into(),
            root_label: label,
            asset_id: Some(id.into()),
            entry_kind: "file".into(),
            kind,
            previous_native: previous.as_ref().map(|p| super::directory::encode_id(&p.3)),
            native_path: Some(locator),
            previous_path: previous.as_ref().map(|p| p.0.clone()),
            path: Some(path),
            previous_revision: previous.map(|p| p.1),
            revision: Some(revision.into()),
            observed_at: store::now(),
            scan_id: scan.into(),
        },
    )
}

pub(super) fn directory_change(
    db: &Connection,
    root: &str,
    scan: &str,
    path: &Path,
    kind: LibraryChangeKind,
) -> Result<()> {
    if path.as_os_str().is_empty() {
        return Ok(());
    }
    let (label, baseline): (String, bool) = db.query_row(
        "SELECT label,last_scan IS NOT NULL FROM roots WHERE id=?",
        [root],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if !baseline {
        return Ok(());
    }
    let removed = kind == LibraryChangeKind::Removed;
    let display = path.to_string_lossy().into_owned();
    record(
        db,
        LibraryChange {
            version: 1,
            sequence: String::new(),
            root_id: root.into(),
            root_label: label,
            asset_id: None,
            entry_kind: "folder".into(),
            kind,
            previous_native: removed
                .then(|| super::directory::encode_id(&native::encode_path(path))),
            native_path: (!removed)
                .then(|| super::directory::encode_id(&native::encode_path(path))),
            previous_path: removed.then_some(display.clone()),
            path: (!removed).then_some(display),
            previous_revision: None,
            revision: None,
            observed_at: store::now(),
            scan_id: scan.into(),
        },
    )
}

pub(super) fn missing_files(
    db: &Connection,
    root: &str,
    scan: &str,
    predicate: &str,
) -> Result<()> {
    // INSERT SELECT journals all differences without an unbounded Rust array.
    db.execute(&format!("INSERT INTO job_events(job_id,state) SELECT 'library-change:'||a.root_id,json_object('version',1,'sequence','','rootId',a.root_id,'rootLabel',r.label,'assetId',a.id,'entryKind','file','kind','removed','previousNative','d'||lower(hex(a.relative_native)),'nativePath',NULL,'previousPath',a.relative_path,'path',NULL,'previousRevision',a.revision,'revision',NULL,'observedAt',?3,'scanId',?2) FROM assets a JOIN roots r ON r.id=a.root_id WHERE a.root_id=?1 AND a.seen_scan<>?2 AND a.presence NOT IN ('missing','excluded') AND r.last_scan IS NOT NULL AND {predicate}"),params![root,scan,store::now()])?;
    Ok(())
}

pub(super) fn missing_directories(
    db: &Connection,
    root: &str,
    scan: &str,
    absent: &str,
) -> Result<()> {
    let mut statement=db.prepare(&format!("{absent} SELECT native_path FROM directories WHERE root_id=?1 AND state NOT IN ('missing','excluded','blocked') AND seen_scan<>?2 AND native_path IN (SELECT path FROM absent)"))?;
    let mut rows = statement.query(params![root, scan])?;
    while let Some(row) = rows.next()? {
        let path = native::decode_path(&row.get::<_, Vec<u8>>(0)?)?;
        directory_change(db, root, scan, &path, LibraryChangeKind::Removed)?;
    }
    Ok(())
}

pub(super) fn page(
    catalog: &Catalog,
    before: Option<String>,
    limit: u32,
) -> Result<LibraryChangePage> {
    if !(1..=100).contains(&limit) {
        return Err(invalid("Read at most 100 library changes."));
    }
    let before = before
        .map(|s| {
            s.parse::<i64>()
                .ok()
                .filter(|n| *n > 0)
                .ok_or_else(|| invalid("Invalid change-history cursor."))
        })
        .transpose()?
        .unwrap_or(i64::MAX);
    catalog.call(move|db| {
        let mut values=db.prepare("SELECT sequence,CASE WHEN length(CAST(state AS BLOB))<=262144 THEN state END FROM job_events WHERE sequence<? AND job_id LIKE 'library-change:%' ORDER BY sequence DESC LIMIT ?")?
            .query_map(params![before,limit+1], |r|Ok((r.get::<_,i64>(0)?,r.get::<_,Option<String>>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let more=values.len()>limit as usize; values.truncate(limit as usize);
        let mut changes=Vec::with_capacity(values.len());
        for (sequence,payload) in values {
            let mut change:LibraryChange=serde_json::from_str(&payload.ok_or_else(||invalid("The library change record exceeds its size limit."))?).map_err(|_|invalid("The library change record could not be read."))?;
            if change.version!=1 {return Err(invalid("This change history requires a newer application."));}
            change.sequence=sequence.to_string();changes.push(change);
        }
        let next_before=more.then(|| changes.last().map(|c|c.sequence.clone())).flatten();
        Ok(LibraryChangePage {changes,next_before})
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_oversized_or_future_history_fails_explicitly_without_affecting_legacy_events() {
        let owned = tempfile::tempdir().unwrap();
        let catalog = Catalog::open(owned.path().join("history.stillharbour")).unwrap();
        catalog
            .call(|db| {
                db.execute(
                    "INSERT INTO job_events(job_id,state) VALUES('legacy','recorded')",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        assert!(page(&catalog, None, 25).unwrap().changes.is_empty());
        catalog
            .call(|db| {
                db.execute(
                    "INSERT INTO job_events(job_id,state) VALUES('library-change:test',?)",
                    ["x".repeat(262145)],
                )?;
                Ok(())
            })
            .unwrap();
        assert!(
            page(&catalog, None, 25)
                .unwrap_err()
                .message
                .contains("size limit")
        );
        catalog
            .call(|db| {
                db.execute(
                    "UPDATE job_events SET state='invalid json' WHERE job_id='library-change:test'",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        assert!(page(&catalog, None, 25).is_err());
        catalog.call(|db| {
            let payload=serde_json::json!({"version":2,"sequence":"","rootId":"root","rootLabel":"Library","assetId":null,"entryKind":"file","kind":"added","previousNative":null,"nativePath":null,"previousPath":null,"path":"photo.jpg","previousRevision":null,"revision":null,"observedAt":"2026-10-29T12:00:00Z","scanId":"scan"});
            db.execute("UPDATE job_events SET state=? WHERE job_id='library-change:test'",[payload.to_string()])?;Ok(())
        }).unwrap();
        assert!(
            page(&catalog, None, 25)
                .unwrap_err()
                .message
                .contains("newer application")
        );
    }
}

#[cfg(all(test, unix))]
mod native_byte_tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;
    #[test]
    fn same_display_native_names_keep_distinct_move_locators_in_catalog_only_fixture() {
        // APFS rejects arbitrary non-UTF8 names. Exercise lossless catalog
        // representation without claiming native filename/volume support.
        let owned = tempfile::tempdir().unwrap();
        let catalog = Catalog::open(owned.path().join("native-paths.stillharbour")).unwrap();
        catalog.call(|db| {
            db.execute("INSERT INTO roots(id,label,native_path,display_path,mode,is_base,platform,volume,device,inode,availability,filter,last_scan) VALUES('root','Generated catalog',x'','Generated','referenced',1,'unix','v','1','2','online',?,'baseline')",[serde_json::to_string(&ScanFilter::default()).unwrap()])?;
            let old=b"native-\xff.jpg".to_vec();let path=std::path::PathBuf::from(std::ffi::OsString::from_vec(b"native-\xfe.jpg".to_vec()));
            file_change(db,"root","scan","asset",Some((String::from_utf8_lossy(&old).into_owned(),"old".into(),"present".into(),old)),&path,"new")
        }).unwrap();
        let result = page(&catalog, None, 25).unwrap();
        let change = &result.changes[0];
        assert_eq!(change.kind, LibraryChangeKind::Moved);
        assert_eq!(change.previous_path, change.path);
        assert_ne!(change.previous_native, change.native_path);
    }
}

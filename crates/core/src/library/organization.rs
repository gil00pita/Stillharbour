//! Catalog-only snapshot adapter; never opens original files or destinations.
use super::{Library, filters::invalid, native};
use crate::{
    domain::Result,
    organization::{self, OrganizationPreview, OrganizationTemplate, PREVIEW_LIMIT, PreviewSource},
};
use rusqlite::{OptionalExtension, params_from_iter, types::Value};
use std::collections::{BTreeMap, BTreeSet};

struct SnapshotEntry {
    source: PreviewSource,
    stem: String,
    folder: Vec<u8>,
    counted_self: bool,
}

impl Library {
    pub fn organization_preview(
        &self,
        asset_ids: Vec<String>,
        template: OrganizationTemplate,
    ) -> Result<OrganizationPreview> {
        validate_ids(&asset_ids)?;
        let (revision, sources) = self.catalog.call(move |db| snapshot(db, asset_ids))?;
        // Rendering is pure and happens outside the single database actor.
        organization::preview(revision, template, sources)
    }
}

fn validate_ids(ids: &[String]) -> Result<()> {
    if ids.is_empty()
        || ids.len() > PREVIEW_LIMIT
        || ids
            .iter()
            .any(|id| id.len() != 36 || uuid::Uuid::parse_str(id).is_err())
        || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
    {
        return Err(invalid(
            "Choose 1–240 distinct catalog photo IDs. Query-wide previews are not available yet.",
        ));
    }
    Ok(())
}

fn snapshot(
    db: &mut rusqlite::Connection,
    ids: Vec<String>,
) -> Result<(String, Vec<PreviewSource>)> {
    let tx = db.transaction()?;
    let revision: i64 = tx.query_row(
        "SELECT revision FROM library_state WHERE singleton=1",
        [],
        |r| r.get(0),
    )?;
    let sources = {
        let mut statement = tx.prepare(
            "SELECT a.root_id,a.relative_native,a.relative_path,a.revision,a.bytes,a.date,a.date_source,
             a.presence,r.availability,r.volume,r.device,r.inode,a.native_file,a.stem,a.folder_native
             FROM assets a JOIN roots r ON r.id=a.root_id WHERE a.id=? AND a.is_media=1"
        )?;
        let mut sources = Vec::with_capacity(ids.len());
        for id in ids {
            let source = statement.query_row([&id], |row| {
                let native_path: Vec<u8> = row.get(1)?;
                let root_id: String = row.get(0)?;
                let stem: String = row.get(13)?;
                let folder: Vec<u8> = row.get(14)?;
                let presence: String = row.get(7)?;
                let date_source: String = row.get(6)?;
                let volume: String = row.get(9)?;
                let device: String = row.get(10)?;
                let inode: String = row.get(11)?;
                let native_file: String = row.get(12)?;
                let relative_path = native::decode_path(&native_path).ok()
                    .and_then(|path| path.to_str().map(str::to_owned));
                let source = PreviewSource {
                    id: id.clone(), root_id,
                    root_identity: serde_json::to_string(&(volume, device, inode, native_file)).expect("strings serialize"),
                    native_path, relative_path, display_path: row.get(2)?, revision: row.get(3)?,
                    bytes: u64::try_from(row.get::<_, i64>(4)?).map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Integer, Box::new(e)))?,
                    capture: matches!(date_source.as_str(), "embedded_original" | "embedded_created").then(|| row.get(5)).transpose()?,
                    available: presence == "present" && row.get::<_, String>(8)? == "online",
                    companion_candidates: 0,
                };
                Ok(SnapshotEntry { source, stem, folder, counted_self: !matches!(presence.as_str(), "excluded" | "missing") })
            }).optional()?.ok_or_else(|| invalid("A selected photo is no longer in the catalog. Refresh and select it again."))?;
            sources.push(source);
        }
        count_companions(&tx, &mut sources)?;
        sources.into_iter().map(|entry| entry.source).collect()
    };
    tx.commit()?;
    Ok((revision.to_string(), sources))
}

fn count_companions(db: &rusqlite::Connection, entries: &mut [SnapshotEntry]) -> Result<()> {
    // Scan each selected directory once, not once per photo. Both the requested
    // stem set and returned groups are bounded by the 240-photo selection.
    let mut folders: BTreeMap<(String, Vec<u8>), Vec<usize>> = BTreeMap::new();
    for (index, entry) in entries.iter().enumerate() {
        folders
            .entry((entry.source.root_id.clone(), entry.folder.clone()))
            .or_default()
            .push(index);
    }
    for ((root, folder), indices) in folders {
        let stems: BTreeSet<_> = indices
            .iter()
            .map(|&index| entries[index].stem.clone())
            .collect();
        let placeholders = std::iter::repeat_n("?", stems.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT stem,count(*) FROM assets WHERE root_id=? AND folder_native=? AND stem IN ({placeholders}) AND presence NOT IN ('excluded','missing') GROUP BY stem"
        );
        let mut values = vec![Value::Text(root), Value::Blob(folder)];
        values.extend(stems.into_iter().map(Value::Text));
        let counts = db
            .prepare(&sql)?
            .query_map(params_from_iter(values), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
            })?
            .collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
        for index in indices {
            let entry = &mut entries[index];
            entry.source.companion_candidates = counts
                .get(&entry.stem)
                .copied()
                .unwrap_or(0)
                .checked_sub(u32::from(entry.counted_self))
                .ok_or_else(|| {
                    invalid("Companion observations are inconsistent. Refresh the catalog.")
                })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::MissingDatePolicy;
    use rusqlite::params;

    #[test]
    fn rejects_paths_duplicates_and_unbounded_selections() {
        let id = uuid::Uuid::new_v4().to_string();
        assert!(validate_ids(std::slice::from_ref(&id)).is_ok());
        for ids in [
            vec![],
            vec!["/private/library/photo.jpg".into()],
            vec![id.clone(), id.clone()],
            vec![id; PREVIEW_LIMIT + 1],
        ] {
            assert!(validate_ids(&ids).is_err());
        }
    }

    #[test]
    fn ipc_does_not_accept_destination_paths_or_execution_authority() {
        use super::super::model::LibraryRequest;
        let request = serde_json::json!({"kind":"organization_preview", "asset_ids":[uuid::Uuid::new_v4().to_string()], "template":{"pattern":"{original_stem}.{ext}", "event":"", "missingDate":"undated"}});
        assert!(serde_json::from_value::<LibraryRequest>(request.clone()).is_ok());
        let mut extra = request.clone();
        extra["destination"] = serde_json::json!("/private/library");
        assert!(serde_json::from_value::<LibraryRequest>(extra).is_err());
        let mut extra = request;
        extra["template"]["approved"] = serde_json::json!(true);
        assert!(serde_json::from_value::<LibraryRequest>(extra).is_err());
        for kind in [
            "organization_execute",
            "organization_approve",
            "import",
            "quarantine",
        ] {
            assert!(
                serde_json::from_value::<LibraryRequest>(serde_json::json!({"kind":kind})).is_err()
            );
        }
    }

    #[test]
    fn snapshot_is_catalog_only_preserves_native_names_and_distinguishes_date_provenance() {
        let mut db = rusqlite::Connection::open_in_memory().unwrap();
        super::super::store::migrate(&mut db).unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        db.execute("INSERT INTO roots(id,label,native_path,display_path,mode,is_base,platform,volume,device,inode,availability,filter) VALUES('root','Fixture',x'01','Unavailable fixture','referenced',1,'fixture','volume','device','inode','online','{}')", []).unwrap();
        db.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan) VALUES(?1,'root',?2,'photo.JPG',x'','','photo.JPG','photo','jpg','identity',42,'revision','2026-10-30','2024-02-29T23:45:12','filesystem_modified','ready','pending','present',1,'fixture')", params![id, native::encode_path(std::path::Path::new("photo.JPG"))]).unwrap();
        db.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan) VALUES('companion','root',x'02','photo.xmp',x'','','photo.xmp','photo','xmp','companion',5,'revision','2026-10-30','','undated','ready','pending','present',0,'fixture')", []).unwrap();
        let template = OrganizationTemplate {
            pattern: "{capture:%Y}/{original_stem}.{ext}".into(),
            event: String::new(),
            missing_date: MissingDatePolicy::Undated,
        };
        let (revision, sources) = snapshot(&mut db, vec![id.clone()]).unwrap();
        let first = organization::preview(revision, template.clone(), sources).unwrap();
        assert_eq!(first.rows[0].companion_candidates, 1);
        assert_eq!(
            first.rows[0].target_relative_path.as_deref(),
            Some("Undated/photo.JPG")
        );
        db.execute("UPDATE assets SET date_source='embedded_original'", [])
            .unwrap();
        let (revision, sources) = snapshot(&mut db, vec![id.clone()]).unwrap();
        let second = organization::preview(revision, template.clone(), sources).unwrap();
        assert_eq!(
            second.rows[0].target_relative_path.as_deref(),
            Some("2024/photo.JPG")
        );
        assert_ne!(first.fingerprint, second.fingerprint);
        db.execute("UPDATE roots SET availability='offline'", [])
            .unwrap();
        db.execute(
            "UPDATE assets SET presence='missing' WHERE id='companion'",
            [],
        )
        .unwrap();
        let (revision, sources) = snapshot(&mut db, vec![id]).unwrap();
        let offline = organization::preview(revision, template, sources).unwrap();
        assert_eq!(offline.blocked, 1);
        assert_eq!(offline.rows[0].companion_candidates, 0);
        assert_eq!(
            db.query_row("SELECT count(*) FROM scans", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            0
        );
        assert!(snapshot(&mut db, vec![uuid::Uuid::new_v4().to_string()]).is_err());
    }
}

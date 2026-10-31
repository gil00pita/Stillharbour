//! Catalog-only snapshot adapter; never opens original files or destinations.
use super::{Library, filters::invalid, native};
use crate::{
    domain::Result,
    organization::{
        self, COMPANION_REVIEW_LIMIT, OrganizationCompanion, OrganizationCompanionReview,
        OrganizationPreview, OrganizationTemplate, PREVIEW_LIMIT, PreviewSource,
    },
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
    pub fn organization_companions(&self, asset_id: String) -> Result<OrganizationCompanionReview> {
        validate_ids(std::slice::from_ref(&asset_id))?;
        self.catalog.call(move |db| companion_review(db, asset_id))
    }

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

fn companion_review(
    db: &mut rusqlite::Connection,
    asset_id: String,
) -> Result<OrganizationCompanionReview> {
    // This is an indexed observation, including excluded/missing entries. Never
    // enumerate originals or confirm a bundle from names alone.
    let tx = db.transaction()?;
    let (root, folder, stem, name, revision, native_path, identity): (
        String,
        Vec<u8>,
        String,
        String,
        String,
        Vec<u8>,
        String,
    ) = tx
        .query_row(
            "SELECT a.root_id,a.folder_native,a.stem,a.name,a.revision,a.relative_native,
         json_array(a.native_file,a.presence,r.volume,r.device,r.inode,r.availability)
         FROM assets a JOIN roots r ON r.id=a.root_id WHERE a.id=? AND a.is_media=1",
            [&asset_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| {
            invalid("The selected photo is no longer in the catalog. Refresh and select it again.")
        })?;
    let catalog_revision: i64 = tx.query_row(
        "SELECT revision FROM library_state WHERE singleton=1",
        [],
        |r| r.get(0),
    )?;
    let mut candidates = Vec::new();
    let mut native_candidates = Vec::new();
    let mut truncated = false;
    {
        let mut statement = tx.prepare(
            "SELECT a.id,a.relative_path,a.revision,a.bytes,a.presence,r.availability,
             a.is_media,a.stem,a.relative_native,a.native_file,r.volume,r.device,r.inode
             FROM assets a JOIN roots r ON r.id=a.root_id
             WHERE a.root_id=?1 AND a.folder_native=?2 AND a.id<>?3
             AND (a.stem=?4 OR (a.stem=?5 AND a.extension IN ('xmp','aae')))
             ORDER BY a.relative_native,a.id LIMIT ?6",
        )?;
        let mut rows = statement.query(rusqlite::params![
            root,
            folder,
            asset_id,
            stem,
            name,
            (COMPANION_REVIEW_LIMIT + 1) as i64,
        ])?;
        while let Some(row) = rows.next()? {
            if candidates.len() == COMPANION_REVIEW_LIMIT {
                truncated = true;
                break;
            }
            let bytes: i64 = row.get(3)?;
            let bytes = u64::try_from(bytes)
                .map_err(|_| invalid("A companion has an invalid catalog byte count."))?;
            candidates.push(OrganizationCompanion {
                asset_id: row.get(0)?,
                source_path: row.get(1)?,
                source_revision: row.get(2)?,
                bytes: bytes.to_string(),
                presence: row.get(4)?,
                root_availability: row.get(5)?,
                is_media: row.get(6)?,
                filename_sidecar: row.get::<_, String>(7)? != stem,
            });
            native_candidates.push((
                row.get::<_, Vec<u8>>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, String>(12)?,
            ));
        }
    }
    let evidence = serde_json::to_vec(&(
        "organization-companions-v1",
        &asset_id,
        catalog_revision,
        root,
        folder,
        stem,
        name,
        revision,
        native_path,
        identity,
        &candidates,
        native_candidates,
        truncated,
    ))
    .map_err(|e| invalid("Could not fingerprint companion observations.").caused_by(e))?;
    let fingerprint = blake3::hash(&evidence).to_hex().to_string();
    tx.commit()?;
    Ok(OrganizationCompanionReview {
        asset_id,
        catalog_revision: catalog_revision.to_string(),
        fingerprint,
        candidates,
        truncated,
        inventory_complete: false,
        execution_available: false,
    })
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

pub(super) fn snapshot(
    db: &mut rusqlite::Connection,
    ids: Vec<String>,
) -> Result<(String, Vec<PreviewSource>)> {
    snapshot_inner(db, ids, true)
}

pub(super) fn source_snapshot(
    db: &mut rusqlite::Connection,
    ids: Vec<String>,
) -> Result<(String, Vec<PreviewSource>)> {
    snapshot_inner(db, ids, false)
}

fn snapshot_inner(
    db: &mut rusqlite::Connection,
    ids: Vec<String>,
    count_candidates: bool,
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
        if count_candidates {
            count_companions(&tx, &mut sources)?;
        }
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

    fn companion_fixture() -> (rusqlite::Connection, String) {
        let mut db = rusqlite::Connection::open_in_memory().unwrap();
        super::super::store::migrate(&mut db).unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        db.execute("INSERT INTO roots(id,label,native_path,display_path,mode,is_base,platform,volume,device,inode,availability,filter) VALUES('root','Fixture',x'01','Unavailable fixture','referenced',1,'fixture','volume','device','inode','online','{}')", []).unwrap();
        db.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan) VALUES(?1,'root',?2,'photo.JPG',x'','','photo.JPG','photo','jpg','identity',42,'revision','2026-10-30','','undated','ready','pending','present',1,'fixture')", params![id, native::encode_path(std::path::Path::new("photo.JPG"))]).unwrap();
        (db, id)
    }

    fn add_candidate(
        db: &rusqlite::Connection,
        source: &str,
        id: &str,
        name: &str,
        stem: &str,
        extension: &str,
    ) {
        db.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan)
            SELECT ?2,root_id,?3,?4,folder_native,folder,?4,?5,?6,?2,5,revision,added_at,date,date_source,metadata_state,preview_state,'present',0,seen_scan FROM assets WHERE id=?1",
            params![source, id, native::encode_path(std::path::Path::new(name)), name, stem, extension]).unwrap();
    }

    #[test]
    fn companion_review_includes_unavailable_candidates_without_scanning_or_confirming_bundles() {
        let (mut db, id) = companion_fixture();
        add_candidate(&db, &id, "xmp", "photo.xmp", "photo", "xmp");
        add_candidate(&db, &id, "aae", "photo.JPG.aae", "photo.JPG", "aae");
        add_candidate(&db, &id, "other", "different.xmp", "different", "xmp");
        add_candidate(&db, &id, "nested", "nested/photo.xmp", "photo", "xmp");
        db.execute(
            "UPDATE assets SET folder_native=x'01' WHERE id='nested'",
            [],
        )
        .unwrap();
        db.execute("UPDATE assets SET presence='excluded' WHERE id='xmp'", [])
            .unwrap();
        db.execute("UPDATE assets SET presence='missing' WHERE id='aae'", [])
            .unwrap();
        db.execute("UPDATE roots SET availability='offline'", [])
            .unwrap();
        let review = companion_review(&mut db, id.clone()).unwrap();
        assert_eq!(review.candidates.len(), 2);
        assert!(!review.inventory_complete);
        assert!(!review.execution_available);
        assert!(!review.truncated);
        assert!(
            review
                .candidates
                .iter()
                .all(|c| c.root_availability == "offline")
        );
        let aae = review
            .candidates
            .iter()
            .find(|c| c.asset_id == "aae")
            .unwrap();
        assert!(aae.filename_sidecar);
        assert_eq!(aae.presence, "missing");
        let xmp = review
            .candidates
            .iter()
            .find(|c| c.asset_id == "xmp")
            .unwrap();
        assert!(!xmp.filename_sidecar);
        assert_eq!(xmp.presence, "excluded");
        db.execute("UPDATE assets SET native_file='changed' WHERE id='xmp'", [])
            .unwrap();
        let changed = companion_review(&mut db, id.clone()).unwrap();
        assert_ne!(review.fingerprint, changed.fingerprint);
        assert_eq!(
            db.query_row("SELECT count(*) FROM scans", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            0
        );
        assert!(companion_review(&mut db, "not-in-catalog".into()).is_err());
        assert!(companion_review(&mut db, "xmp".into()).is_err());
        assert_eq!(
            db.query_row("SELECT count(*) FROM assets", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            5
        );
    }

    #[test]
    fn companion_review_is_bounded_deterministic_and_binds_source_identity() {
        let (mut db, id) = companion_fixture();
        let empty = companion_review(&mut db, id.clone()).unwrap();
        assert!(empty.candidates.is_empty());
        assert!(!empty.inventory_complete);
        db.execute("UPDATE roots SET inode='changed'", []).unwrap();
        assert_ne!(
            empty.fingerprint,
            companion_review(&mut db, id.clone()).unwrap().fingerprint
        );
        for index in 0..COMPANION_REVIEW_LIMIT {
            add_candidate(
                &db,
                &id,
                &format!("id-{index:03}"),
                &format!("photo.{index:03}"),
                "photo",
                "unknown",
            );
        }
        let full = companion_review(&mut db, id.clone()).unwrap();
        assert_eq!(full.candidates.len(), COMPANION_REVIEW_LIMIT);
        assert!(!full.truncated);
        add_candidate(&db, &id, "last", "photo.zzz", "photo", "unknown");
        let truncated = companion_review(&mut db, id.clone()).unwrap();
        assert!(truncated.truncated);
        assert_eq!(truncated.candidates.len(), COMPANION_REVIEW_LIMIT);
        assert_ne!(full.fingerprint, truncated.fingerprint);
        assert_eq!(
            truncated.fingerprint,
            companion_review(&mut db, id).unwrap().fingerprint
        );
    }

    #[test]
    fn companion_ipc_accepts_only_catalog_ids_without_extra_authority() {
        use super::super::model::LibraryRequest;
        let request = serde_json::json!({"kind":"organization_companions", "asset_id":uuid::Uuid::new_v4().to_string()});
        assert!(serde_json::from_value::<LibraryRequest>(request.clone()).is_ok());
        for key in ["path", "destination", "confirmed", "approved"] {
            let mut extra = request.clone();
            extra[key] = serde_json::json!(true);
            assert!(serde_json::from_value::<LibraryRequest>(extra).is_err());
        }
    }

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

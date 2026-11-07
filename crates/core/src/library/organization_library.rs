//! Whole-catalog, read-only organization observations. SQLite TEMP tables keep
//! membership and the global output namespace off the renderer's heap. Rendering
//! and IPC are bounded; no original or destination is opened by this adapter.
use super::{Library, filters::invalid, organization::template_snapshot};
use crate::{
    domain::Result,
    organization::{
        self, LibraryOrganizationPreview, OrganizationIssue, OrganizationRow, OrganizationTemplate,
        PREVIEW_LIMIT,
    },
};
use rusqlite::{Connection, OptionalExtension, params};

struct Session {
    id: String,
    revision: String,
    template: OrganizationTemplate,
    processed: u32,
    total: u32,
}

impl Library {
    pub fn organization_library_start(
        &self,
        template: OrganizationTemplate,
    ) -> Result<LibraryOrganizationPreview> {
        organization::validate_template(&template)?;
        self.catalog.call(move |db| start(db, template))
    }

    pub fn organization_library_advance(
        &self,
        preview_id: String,
    ) -> Result<LibraryOrganizationPreview> {
        let (session, sources) = self.catalog.call(move |db| load_page(db, &preview_id))?;
        let rows = organization::render_rows(
            &session.template,
            &sources,
            session.processed.saturating_add(1),
        )?;
        self.catalog.call(move |db| commit_page(db, session, rows))
    }

    pub fn organization_library_page(
        &self,
        preview_id: String,
        offset: u32,
    ) -> Result<LibraryOrganizationPreview> {
        self.catalog.call(move |db| {
            let session = session(db, &preview_id)?;
            if session.processed != session.total {
                return Err(invalid("Wait for the whole-library preview to finish."));
            }
            report(db, &session, offset)
        })
    }

    pub fn organization_library_discard(&self, preview_id: String) -> Result<()> {
        self.catalog.call(move |db| {
            initialize(db)?;
            validate_handle(&preview_id)?;
            // Late cleanup from a superseded page cannot delete a new preview.
            if db.execute("DELETE FROM structure_session WHERE id=?", [&preview_id])? > 0 {
                db.execute("DELETE FROM structure_members", [])?;
                db.execute("DELETE FROM structure_rows", [])?;
            }
            Ok(())
        })
    }
}

fn load_page(
    db: &mut Connection,
    preview_id: &str,
) -> Result<(Session, Vec<organization::PreviewSource>)> {
    let session = session(db, preview_id)?;
    if session.processed == session.total {
        return Err(invalid("This whole-library preview is already complete."));
    }
    let ids = db
        .prepare(
            "SELECT asset_id FROM structure_members WHERE sequence>? ORDER BY sequence LIMIT ?",
        )?
        .query_map(params![session.processed, PREVIEW_LIMIT as u32], |r| {
            r.get::<_, String>(0)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let (revision, sources) = template_snapshot(db, ids, &session.template)?;
    if revision != session.revision {
        return Err(changed());
    }
    Ok((session, sources))
}

fn initialize(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS structure_session(id TEXT PRIMARY KEY,revision TEXT NOT NULL,template TEXT NOT NULL,processed INTEGER NOT NULL,total INTEGER NOT NULL);
         CREATE TEMP TABLE IF NOT EXISTS structure_members(sequence INTEGER PRIMARY KEY,asset_id TEXT NOT NULL UNIQUE);
         CREATE TEMP TABLE IF NOT EXISTS structure_rows(sequence INTEGER PRIMARY KEY,target TEXT,folded TEXT,data TEXT NOT NULL,blocked INTEGER NOT NULL,collision INTEGER NOT NULL DEFAULT 0,case_collision INTEGER NOT NULL DEFAULT 0);
         CREATE INDEX IF NOT EXISTS structure_target ON structure_rows(target);
         CREATE INDEX IF NOT EXISTS structure_folded ON structure_rows(folded);"
    )?;
    Ok(())
}

fn revision(db: &Connection) -> Result<String> {
    Ok(db
        .query_row(
            "SELECT revision FROM library_state WHERE singleton=1",
            [],
            |r| r.get::<_, i64>(0),
        )?
        .to_string())
}
fn changed() -> crate::domain::AppError {
    invalid("The catalog changed during this preview. Build a fresh whole-library preview.")
}
fn validate_handle(id: &str) -> Result<()> {
    if id.len() != 36 || uuid::Uuid::parse_str(id).is_err() {
        return Err(invalid("Invalid whole-library preview ID."));
    }
    Ok(())
}
fn session(db: &Connection, id: &str) -> Result<Session> {
    initialize(db)?;
    validate_handle(id)?;
    let raw = db
        .query_row(
            "SELECT revision,template,processed,total FROM structure_session WHERE id=?",
            [id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, u32>(2)?,
                    r.get::<_, u32>(3)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| invalid("This whole-library preview expired. Build a fresh preview."))?;
    if revision(db)? != raw.0 {
        return Err(changed());
    }
    Ok(Session {
        id: id.into(),
        revision: raw.0,
        template: serde_json::from_str(&raw.1)
            .map_err(|e| invalid("Invalid preview rules.").caused_by(e))?,
        processed: raw.2,
        total: raw.3,
    })
}

fn start(
    db: &mut Connection,
    template: OrganizationTemplate,
) -> Result<LibraryOrganizationPreview> {
    organization::validate_template(&template)?;
    initialize(db)?;
    let tx = db.transaction()?;
    let revision = revision(&tx)?;
    tx.execute("DELETE FROM structure_session", [])?;
    tx.execute("DELETE FROM structure_members", [])?;
    tx.execute("DELETE FROM structure_rows", [])?;
    // No gallery query, album, selection, availability or pagination filter.
    tx.execute("INSERT INTO structure_members(sequence,asset_id) SELECT row_number() OVER (ORDER BY root_id,relative_native,id),id FROM assets WHERE is_media=1", [])?;
    let total = tx.query_row("SELECT count(*) FROM structure_members", [], |r| {
        r.get::<_, u32>(0)
    })?;
    let session = Session {
        id: uuid::Uuid::new_v4().to_string(),
        revision,
        template,
        processed: 0,
        total,
    };
    tx.execute(
        "INSERT INTO structure_session VALUES(?,?,?,0,?)",
        params![
            session.id,
            session.revision,
            serde_json::to_string(&session.template)
                .map_err(|e| invalid("Could not encode preview rules.").caused_by(e))?,
            total
        ],
    )?;
    tx.commit()?;
    report(db, &session, 0)
}

fn commit_page(
    db: &mut Connection,
    expected: Session,
    rows: Vec<OrganizationRow>,
) -> Result<LibraryOrganizationPreview> {
    let tx = db.transaction()?;
    let current = session(&tx, &expected.id)?;
    if current.processed != expected.processed {
        return Err(invalid(
            "This preview page was already processed. Refresh the preview.",
        ));
    }
    for row in &rows {
        insert_row(&tx, row)?;
    }
    let processed = expected
        .processed
        .checked_add(rows.len() as u32)
        .ok_or_else(|| invalid("Preview sequence overflow."))?;
    if processed > current.total {
        return Err(invalid("Invalid preview page size."));
    }
    tx.execute(
        "UPDATE structure_session SET processed=? WHERE id=?",
        params![processed, current.id],
    )?;
    tx.commit()?;
    report(
        db,
        &Session {
            processed,
            ..current
        },
        0,
    )
}

fn insert_row(db: &Connection, row: &OrganizationRow) -> Result<()> {
    let mut collision = false;
    let mut case_collision = false;
    if let Some(target) = &row.target_relative_path {
        // Equal outputs and existing descendants, found through the target index.
        collision |= db.execute(
            "UPDATE structure_rows SET collision=1 WHERE target=?1 OR (target>=?2 AND target<?3)",
            params![target, format!("{target}/"), format!("{target}0")],
        )? > 0;
        // Existing ancestors: at most the portable path component limit.
        for (index, _) in target.match_indices('/') {
            collision |= db.execute(
                "UPDATE structure_rows SET collision=1 WHERE target=?",
                [&target[..index]],
            )? > 0;
        }
        case_collision = db.execute(
            "UPDATE structure_rows SET case_collision=1 WHERE folded=? AND target<>?",
            params![target.to_ascii_lowercase(), target],
        )? > 0;
    }
    db.execute("INSERT INTO structure_rows(sequence,target,folded,data,blocked,collision,case_collision) VALUES(?,?,?,?,?,?,?)", params![row.sequence, row.target_relative_path, row.target_relative_path.as_ref().map(|s| s.to_ascii_lowercase()), serde_json::to_string(row).map_err(|e| invalid("Could not encode preview row.").caused_by(e))?, !row.issues.is_empty(), collision, case_collision])?;
    Ok(())
}

fn report(db: &Connection, session: &Session, offset: u32) -> Result<LibraryOrganizationPreview> {
    let complete = session.processed == session.total;
    if offset > session.total || (!complete && offset != 0) {
        return Err(invalid("Invalid whole-library preview offset."));
    }
    let mut rows = Vec::new();
    let mut blocked = 0;
    if complete {
        blocked = db.query_row("SELECT count(*) FROM structure_rows WHERE blocked=1 OR collision=1 OR case_collision=1", [], |r| r.get::<_, u32>(0))?;
        let mut statement = db.prepare("SELECT data,collision,case_collision FROM structure_rows WHERE sequence>? ORDER BY sequence LIMIT ?")?;
        let mut records = statement.query(params![offset, PREVIEW_LIMIT as u32])?;
        while let Some(record) = records.next()? {
            let data: String = record.get(0)?;
            let mut row: OrganizationRow = serde_json::from_str(&data)
                .map_err(|e| invalid("Invalid preview row.").caused_by(e))?;
            if record.get::<_, bool>(1)? {
                row.issues.push(OrganizationIssue::Collision);
            }
            if record.get::<_, bool>(2)? {
                row.issues.push(OrganizationIssue::PossibleCaseCollision);
            }
            rows.push(row);
        }
    }
    Ok(LibraryOrganizationPreview {
        preview_id: session.id.clone(),
        catalog_revision: session.revision.clone(),
        total: session.total,
        processed: session.processed,
        proposed: if complete { session.total - blocked } else { 0 },
        blocked,
        complete,
        offset,
        rows,
        execution_available: false,
    })
}

#[cfg(test)]
mod tests {
    use super::super::native;
    use super::*;
    use crate::organization::MissingDatePolicy;
    use std::path::Path;

    fn fixture(names: &[String]) -> Connection {
        let mut db = Connection::open_in_memory().unwrap();
        super::super::store::migrate(&mut db).unwrap();
        db.execute("INSERT INTO roots(id,label,native_path,display_path,mode,is_base,platform,volume,device,inode,availability,filter) VALUES('root','Generated',x'01','Unavailable fixture','referenced',1,'fixture','volume','device','inode','online','{}')", []).unwrap();
        for name in names {
            let path = Path::new(name);
            db.execute("INSERT INTO assets(id,root_id,relative_native,relative_path,folder_native,folder,name,stem,extension,native_file,bytes,revision,added_at,date,date_source,metadata_state,preview_state,presence,is_media,seen_scan) VALUES(?1,'root',?2,?3,?4,'',?5,?6,'jpg','fixture',42,'revision','2026-11-07','2024-02-29T23:45:12','embedded_original','ready','pending','present',1,'fixture')", params![uuid::Uuid::new_v4().to_string(), native::encode_path(path), name, native::encode_path(path.parent().unwrap()), path.file_name().unwrap().to_str().unwrap(), path.file_stem().unwrap().to_str().unwrap()]).unwrap();
        }
        db
    }
    fn template(pattern: &str) -> OrganizationTemplate {
        OrganizationTemplate {
            pattern: pattern.into(),
            event: String::new(),
            missing_date: MissingDatePolicy::Undated,
            filename_stem_case: None,
        }
    }
    fn advance(db: &mut Connection, id: &str) -> LibraryOrganizationPreview {
        let (session, sources) = load_page(db, id).unwrap();
        let rows =
            organization::render_rows(&session.template, &sources, session.processed + 1).unwrap();
        commit_page(db, session, rows).unwrap()
    }
    fn finish(
        db: &mut Connection,
        mut preview: LibraryOrganizationPreview,
    ) -> LibraryOrganizationPreview {
        while !preview.complete {
            preview = advance(db, &preview.preview_id);
        }
        preview
    }
    #[test]
    fn covers_entire_catalog_with_global_sequence_and_bounded_pages() {
        let names = (0..501)
            .map(|n| format!("{n:04}/photo.JPG"))
            .collect::<Vec<_>>();
        let mut db = fixture(&names);
        // Sidecar observations do not become independent media outputs.
        db.execute(
            "UPDATE assets SET is_media=0 WHERE relative_path='0001/photo.JPG'",
            [],
        )
        .unwrap();
        let first = start(&mut db, template("{sequence:04}_{original_stem}.{ext}")).unwrap();
        assert_eq!(first.total, 500);
        assert!(first.rows.is_empty());
        let first = advance(&mut db, &first.preview_id);
        assert_eq!(first.processed, 240);
        assert!(!first.complete);
        assert!(
            first.rows.is_empty(),
            "Partial namespaces must not appear complete"
        );
        let final_page = finish(&mut db, first);
        assert_eq!(final_page.processed, 500);
        assert_eq!(final_page.proposed, 500);
        assert_eq!(final_page.rows.len(), PREVIEW_LIMIT);
        assert!(!final_page.execution_available);
        let session = session(&db, &final_page.preview_id).unwrap();
        let last = report(&db, &session, 480).unwrap();
        assert_eq!(last.rows.len(), 20);
        assert_eq!(last.rows[19].sequence, 500);
        assert_eq!(
            last.rows[19].target_relative_path.as_deref(),
            Some("0500_photo.JPG")
        );
        assert!(report(&db, &session, 501).is_err());
        assert!(load_page(&mut db, &session.id).is_err());
        assert_eq!(
            db.query_row("SELECT count(*) FROM assets", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            501
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM scans", [], |r| r.get::<_, u32>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn finds_cross_batch_collisions_and_updates_earlier_pages() {
        let mut names = (1..240)
            .map(|n| format!("{n:04}/unique{n}.JPG"))
            .collect::<Vec<_>>();
        names.extend(["0000/photo.JPG".into(), "9999/photo.JPG".into()]);
        let mut db = fixture(&names);
        let first = start(&mut db, template("{original_stem}.{ext}")).unwrap();
        let done = finish(&mut db, first);
        assert_eq!(done.total, 241);
        assert_eq!(done.blocked, 2);
        assert!(done.rows[0].issues.contains(&OrganizationIssue::Collision));
        let session = session(&db, &done.preview_id).unwrap();
        let last = report(&db, &session, 240).unwrap();
        assert!(last.rows[0].issues.contains(&OrganizationIssue::Collision));
    }
    #[test]
    fn global_namespace_blocks_ancestors_and_advises_ascii_case_without_dropping_other_issues() {
        let mut db = fixture(&[]);
        let preview = start(&mut db, template("{original_stem}.{ext}")).unwrap();
        for (i, target) in ["Folder.JPG", "Folder.JPG/child.JPG", "folder.JPG"]
            .iter()
            .enumerate()
        {
            let row = OrganizationRow {
                asset_id: format!("asset{i}"),
                root_id: "root".into(),
                source_path: String::new(),
                source_revision: "rev".into(),
                source_components: None,
                target_relative_path: Some((*target).into()),
                sequence: i as u32 + 1,
                bytes: "42".into(),
                issues: vec![OrganizationIssue::SourceUnavailable],
                used_undated: false,
                companion_candidates: 0,
            };
            insert_row(&db, &row).unwrap();
        }
        db.execute("UPDATE structure_session SET total=3,processed=3", [])
            .unwrap();
        let session = session(&db, &preview.preview_id).unwrap();
        let done = report(&db, &session, 0).unwrap();
        assert_eq!(done.blocked, 3);
        assert!(done.rows[0].issues.contains(&OrganizationIssue::Collision));
        assert!(done.rows[1].issues.contains(&OrganizationIssue::Collision));
        assert!(
            done.rows[0]
                .issues
                .contains(&OrganizationIssue::PossibleCaseCollision)
        );
        assert!(
            done.rows[2]
                .issues
                .contains(&OrganizationIssue::PossibleCaseCollision)
        );
        assert!(
            done.rows
                .iter()
                .all(|row| row.issues.contains(&OrganizationIssue::SourceUnavailable))
        );
    }
    #[test]
    fn unavailable_and_excluded_sources_remain_blocked_in_whole_library_scope() {
        let names = ["missing.JPG", "excluded.JPG", "present.JPG"].map(String::from);
        let mut db = fixture(&names);
        db.execute(
            "UPDATE assets SET presence='missing' WHERE relative_path='missing.JPG'",
            [],
        )
        .unwrap();
        db.execute(
            "UPDATE assets SET presence='excluded' WHERE relative_path='excluded.JPG'",
            [],
        )
        .unwrap();
        let preview = start(&mut db, template("{original_stem}.{ext}")).unwrap();
        let done = finish(&mut db, preview);
        assert_eq!(done.total, 3);
        assert_eq!(done.blocked, 2);
        db.execute("UPDATE roots SET availability='offline'", [])
            .unwrap();
        let preview = start(&mut db, template("{original_stem}.{ext}")).unwrap();
        let done = finish(&mut db, preview);
        assert_eq!(done.blocked, 3);
    }
    #[test]
    fn catalog_changes_superseded_handles_and_racing_pages_fail_closed() {
        let mut db = fixture(&["photo.JPG".into()]);
        let first = start(&mut db, template("{original_stem}.{ext}")).unwrap();
        let second = start(&mut db, template("{original_stem}.{ext}")).unwrap();
        assert!(load_page(&mut db, &first.preview_id).is_err());
        let (expected, sources) = load_page(&mut db, &second.preview_id).unwrap();
        let rows = organization::render_rows(&expected.template, &sources, 1).unwrap();
        advance(&mut db, &second.preview_id);
        assert!(commit_page(&mut db, expected, rows).is_err());
        db.execute("UPDATE library_state SET revision=revision+1", [])
            .unwrap();
        assert!(session(&db, &second.preview_id).is_err());
        let fresh = start(&mut db, template("{original_stem}.{ext}")).unwrap();
        let (expected, sources) = load_page(&mut db, &fresh.preview_id).unwrap();
        let rows = organization::render_rows(&expected.template, &sources, 1).unwrap();
        db.execute("UPDATE library_state SET revision=revision+1", [])
            .unwrap();
        assert!(commit_page(&mut db, expected, rows).is_err());
        assert!(session(&db, "/path/to/a/catalog").is_err());
    }
    #[test]
    fn empty_catalog_and_invalid_rules_are_explicit_and_ipc_has_no_mutation_authority() {
        let mut db = fixture(&[]);
        assert!(start(&mut db, template("../{original_stem}.{ext}")).is_err());
        let preview = start(&mut db, template("{original_stem}.{ext}")).unwrap();
        assert!(preview.complete);
        assert_eq!(preview.total, 0);
        assert!(preview.rows.is_empty());
        let request = serde_json::json!({"kind":"organization_library_start","template":template("{original_stem}.{ext}")});
        assert!(
            serde_json::from_value::<super::super::model::LibraryRequest>(request.clone()).is_ok()
        );
        for key in ["asset_ids", "query", "destination", "approved", "execute"] {
            let mut extra = request.clone();
            extra[key] = serde_json::json!(true);
            assert!(serde_json::from_value::<super::super::model::LibraryRequest>(extra).is_err());
        }
    }
}

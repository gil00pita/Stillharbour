use super::*;
use serde::{Deserialize, Serialize};

pub(crate) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch(r#"
    CREATE TABLE catalog_tag_overrides(asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,label TEXT NOT NULL,removed INTEGER NOT NULL CHECK(removed IN(0,1)),PRIMARY KEY(asset_id,label));
    CREATE TABLE catalog_tag_context(singleton INTEGER PRIMARY KEY CHECK(singleton=1),operation_id TEXT NOT NULL);
    CREATE TABLE catalog_tag_history(sequence INTEGER PRIMARY KEY,operation_id TEXT NOT NULL,asset_id TEXT NOT NULL,before_tags TEXT NOT NULL CHECK(json_valid(before_tags)),after_tags TEXT NOT NULL CHECK(json_valid(after_tags)),created_at TEXT NOT NULL);
    CREATE INDEX tag_history_operation ON catalog_tag_history(operation_id,sequence);
    CREATE TRIGGER tag_history_update BEFORE UPDATE ON catalog_tag_history BEGIN SELECT RAISE(ABORT,'Retain tag history'); END;
    CREATE TRIGGER tag_history_delete BEFORE DELETE ON catalog_tag_history BEGIN SELECT RAISE(ABORT,'Retain tag history'); END;
    CREATE TRIGGER catalog_tags_changed AFTER UPDATE OF tags ON assets WHEN OLD.tags<>NEW.tags BEGIN
      INSERT INTO catalog_tag_history(operation_id,asset_id,before_tags,after_tags,created_at) VALUES(coalesce((SELECT operation_id FROM catalog_tag_context WHERE singleton=1),lower(hex(randomblob(16)))),NEW.id,OLD.tags,NEW.tags,strftime('%Y-%m-%dT%H:%M:%fZ','now'));
      INSERT INTO catalog_tag_overrides SELECT NEW.id,value,1 FROM json_each(OLD.tags) WHERE value NOT IN(SELECT value FROM json_each(NEW.tags)) ON CONFLICT(asset_id,label) DO UPDATE SET removed=1;
      INSERT INTO catalog_tag_overrides SELECT NEW.id,value,0 FROM json_each(NEW.tags) WHERE true ON CONFLICT(asset_id,label) DO UPDATE SET removed=0;
    END;
    CREATE TABLE catalog_metadata_plans(id TEXT PRIMARY KEY,kind TEXT NOT NULL,revision TEXT NOT NULL,digest TEXT NOT NULL,payload TEXT NOT NULL CHECK(json_valid(payload)),created_at TEXT NOT NULL,state TEXT NOT NULL CHECK(state IN('review','applied')));
    CREATE TRIGGER metadata_plan_content BEFORE UPDATE OF id,kind,revision,digest,payload,created_at ON catalog_metadata_plans BEGIN SELECT RAISE(ABORT,'Retain reviewed metadata plan'); END;
    CREATE TABLE catalog_tag_category_history(operation_id TEXT PRIMARY KEY,before_json TEXT NOT NULL,after_json TEXT NOT NULL);
    CREATE TABLE catalog_export_journal(sequence INTEGER PRIMARY KEY,export_id TEXT NOT NULL,phase TEXT NOT NULL,evidence TEXT NOT NULL,created_at TEXT NOT NULL);
    CREATE TRIGGER export_journal_update BEFORE UPDATE ON catalog_export_journal BEGIN SELECT RAISE(ABORT,'Retain export history'); END;
    CREATE TRIGGER export_journal_delete BEFORE DELETE ON catalog_export_journal BEGIN SELECT RAISE(ABORT,'Retain export history'); END;
    "#)?;
    Ok(())
}
fn revision(db: &Connection) -> Result<String> {
    Ok(db.query_row(
        "SELECT CAST(revision AS TEXT) FROM library_state WHERE singleton=1",
        [],
        |r| r.get(0),
    )?)
}
fn require_revision(db: &Connection, expected: &str) -> Result<()> {
    if revision(db)? != expected {
        return Err(conflict());
    }
    Ok(())
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Change {
    id: String,
    before: Vec<String>,
    after: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Edit {
    from: String,
    to: Option<String>,
    changes: Vec<Change>,
    before_categories: Vec<TagCategory>,
    after_categories: Vec<TagCategory>,
}
fn encoded<T: Serialize>(value: &T) -> Result<String> {
    let s =
        serde_json::to_string(value).map_err(|e| invalid("Invalid metadata plan.").caused_by(e))?;
    if s.len() > 16 * 1024 * 1024 {
        return Err(invalid(
            "The metadata review exceeds 16 MiB; use a smaller scope.",
        ));
    }
    Ok(s)
}
pub(super) fn preview(
    db: &mut Connection,
    from: String,
    to: Option<String>,
) -> Result<TagEditReview> {
    let from = label(&from)?;
    let to = to.map(|s| label(&s)).transpose()?;
    if to.as_ref() == Some(&from) {
        return Err(invalid("Choose a different exact tag label."));
    }
    let tx = db.transaction()?;
    let revision = revision(&tx)?;
    let before_categories = categories(&tx)?;
    let mut after_categories = before_categories.clone();
    if let Some(source) = before_categories.iter().find(|c| c.label == from) {
        let target = to
            .as_ref()
            .and_then(|name| before_categories.iter().find(|c| &c.label == name));
        match (&to, target) {
            (Some(name), None) => {
                let node = after_categories
                    .iter_mut()
                    .find(|c| c.id == source.id)
                    .unwrap();
                node.label = name.clone();
                node.version = next(node.version)?;
            }
            (_, target) => {
                for node in &mut after_categories {
                    if node.parent_id.as_ref() == Some(&source.id) {
                        node.parent_id = target
                            .map(|n| n.id.clone())
                            .or_else(|| source.parent_id.clone());
                        node.version = next(node.version)?;
                    }
                }
                after_categories.retain(|n| n.id != source.id);
            }
        }
    }
    validate_categories(&after_categories)?;
    let changes = {
        let mut stmt=tx.prepare("SELECT id,CASE WHEN length(CAST(tags AS BLOB))<=65536 THEN tags END FROM assets WHERE is_media=1 AND EXISTS(SELECT 1 FROM json_each(tags) WHERE value=?) ORDER BY id LIMIT 50001")?;
        let rows = stmt.query_map([&from], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        let mut changes = Vec::new();
        for row in rows {
            let (id, raw) = row?;
            let before: Vec<String> =
                serde_json::from_str(&raw).map_err(|_| invalid("Invalid retained tags."))?;
            if before.len() > 64 {
                return Err(invalid("Invalid retained tag count."));
            }
            let mut after = before
                .iter()
                .filter(|v| *v != &from)
                .cloned()
                .collect::<Vec<_>>();
            if let Some(name) = &to {
                after.push(name.clone());
            }
            after.sort();
            after.dedup();
            changes.push(Change { id, before, after });
            if changes.len() > 50000 {
                return Err(invalid(
                    "A tag review supports up to 50,000 matching media.",
                ));
            }
        }
        changes
    };
    if changes.is_empty() && before_categories == after_categories {
        return Err(invalid("This exact tag label has no catalog associations."));
    }
    let merges = if let Some(name) = &to {
        tx.prepare("SELECT 1 FROM assets WHERE EXISTS(SELECT 1 FROM json_each(tags) WHERE value=?) LIMIT 1")?.exists([name])?||before_categories.iter().any(|v|&v.label==name)
    } else {
        false
    };
    let id = uuid::Uuid::new_v4().to_string();
    let payload = encoded(&Edit {
        from: from.clone(),
        to: to.clone(),
        changes,
        before_categories,
        after_categories,
    })?;
    let digest = blake3::hash(payload.as_bytes()).to_hex().to_string();
    let count: Edit =
        serde_json::from_str(&payload).map_err(|_| invalid("Invalid metadata review."))?;
    tx.execute(
        "INSERT INTO catalog_metadata_plans VALUES(?,'tags',?,?,?,?,'review')",
        params![id, revision, digest, payload, store::now()],
    )?;
    tx.commit()?;
    Ok(TagEditReview {
        id,
        digest,
        revision,
        from,
        to,
        affected_count: count.changes.len() as u32,
        merges,
    })
}
fn same_categories(current: &[TagCategory], expected: &[TagCategory]) -> bool {
    // SQL displays by label, while a rename can change that order. History
    // compares stable identities and values rather than presentation order.
    let mut current = current.iter().collect::<Vec<_>>();
    let mut expected = expected.iter().collect::<Vec<_>>();
    current.sort_by(|a, b| a.id.cmp(&b.id));
    expected.sort_by(|a, b| a.id.cmp(&b.id));
    current == expected
}
fn write_categories(db: &Connection, before: &[TagCategory], after: &[TagCategory]) -> Result<()> {
    // Detach first to preserve foreign keys while moving/removing category nodes.
    db.execute("UPDATE catalog_tag_categories SET parent_id=NULL", [])?;
    for value in before {
        if !after.iter().any(|v| v.id == value.id) {
            db.execute("DELETE FROM catalog_tag_categories WHERE id=?", [&value.id])?;
        }
    }
    for value in after {
        db.execute("INSERT INTO catalog_tag_categories(id,version,label,parent_id) VALUES(?,?,?,NULL) ON CONFLICT(id) DO UPDATE SET version=excluded.version,label=excluded.label",params![value.id,value.version,value.label])?;
    }
    for value in after {
        db.execute(
            "UPDATE catalog_tag_categories SET parent_id=? WHERE id=?",
            params![value.parent_id, value.id],
        )?;
    }
    Ok(())
}
pub(super) fn apply(db: &mut Connection, id: String, digest: String) -> Result<TagHistoryEntry> {
    identity(&id)?;
    let tx = db.transaction()?;
    let (revision,raw):(String,String)=tx.query_row("SELECT revision,payload FROM catalog_metadata_plans WHERE length(CAST(payload AS BLOB))<=16777216 AND id=? AND digest=? AND kind='tags' AND state='review' AND julianday(created_at)>=julianday('now','-1 hour')",params![id,digest],|r|Ok((r.get(0)?,r.get(1)?))).optional()?.ok_or_else(||invalid("The reviewed tag change expired or was already applied."))?;
    require_revision(&tx, &revision)?;
    if blake3::hash(raw.as_bytes()).to_hex().as_str() != digest {
        return Err(conflict());
    }
    let edit: Edit =
        serde_json::from_str(&raw).map_err(|_| invalid("Invalid retained metadata review."))?;
    if !same_categories(&categories(&tx)?, &edit.before_categories) {
        return Err(conflict());
    }
    tx.execute("INSERT INTO catalog_tag_context VALUES(1,?)", [&id])?;
    for c in &edit.changes {
        let current: String =
            tx.query_row("SELECT tags FROM assets WHERE id=?", [&c.id], |r| r.get(0))?;
        if serde_json::from_str::<Vec<String>>(&current).ok().as_ref() != Some(&c.before) {
            return Err(conflict());
        }
        tx.execute(
            "UPDATE assets SET tags=? WHERE id=?",
            params![encoded(&c.after)?, c.id],
        )?;
    }
    write_categories(&tx, &edit.before_categories, &edit.after_categories)?;
    tx.execute(
        "INSERT INTO catalog_tag_category_history VALUES(?,?,?)",
        params![
            id,
            encoded(&edit.before_categories)?,
            encoded(&edit.after_categories)?
        ],
    )?;
    tx.execute("DELETE FROM catalog_tag_context", [])?;
    tx.execute(
        "UPDATE catalog_metadata_plans SET state='applied' WHERE id=?",
        [&id],
    )?;
    bump(&tx)?;
    let entry = TagHistoryEntry {
        id,
        affected_count: edit.changes.len() as u32,
        created_at: store::now(),
    };
    tx.commit()?;
    Ok(entry)
}
pub(super) fn history(db: &Connection) -> Result<Vec<TagHistoryEntry>> {
    Ok(db.prepare("WITH operations AS (SELECT operation_id FROM catalog_tag_history UNION SELECT operation_id FROM catalog_tag_category_history) SELECT o.operation_id,count(DISTINCT h.asset_id),coalesce(max(h.created_at),p.created_at,'') AS happened FROM operations o LEFT JOIN catalog_tag_history h ON h.operation_id=o.operation_id LEFT JOIN catalog_metadata_plans p ON p.id=o.operation_id GROUP BY o.operation_id ORDER BY happened DESC,max(h.sequence) DESC,p.rowid DESC LIMIT 20")?.query_map([],|r|Ok(TagHistoryEntry{id:r.get(0)?,affected_count:r.get(1)?,created_at:r.get(2)?}))?.collect::<rusqlite::Result<_>>()?)
}
pub(super) fn undo(db: &mut Connection, id: String, expected: String) -> Result<TagHistoryEntry> {
    if id.len() > 64 || id.is_empty() {
        return Err(invalid("Invalid tag history identity."));
    }
    let tx = db.transaction()?;
    require_revision(&tx, &expected)?;
    let changes = {
        let mut statement = tx.prepare("SELECT asset_id,CASE WHEN length(CAST(before_tags AS BLOB))<=65536 THEN before_tags END,CASE WHEN length(CAST(after_tags AS BLOB))<=65536 THEN after_tags END FROM catalog_tag_history WHERE operation_id=? ORDER BY sequence DESC LIMIT 50001")?;
        let rows = statement.query_map([&id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        let mut changes = Vec::new();
        let mut bytes = 0;
        for row in rows {
            let row = row?;
            bytes += row.0.len() + row.1.len() + row.2.len();
            if bytes > 16 * 1024 * 1024 {
                return Err(invalid("Tag undo exceeds its 16 MiB history bound."));
            }
            changes.push(row);
        }
        changes
    };
    if changes.len() > 50000 {
        return Err(invalid(
            "This tag history is unavailable or exceeds the undo bound.",
        ));
    }
    let op = uuid::Uuid::new_v4().to_string();
    tx.execute("INSERT INTO catalog_tag_context VALUES(1,?)", [&op])?;
    for (asset, before, after) in &changes {
        let current: String =
            tx.query_row("SELECT tags FROM assets WHERE id=?", [asset], |r| r.get(0))?;
        if &current != after {
            return Err(conflict());
        }
        tx.execute(
            "UPDATE assets SET tags=? WHERE id=?",
            params![before, asset],
        )?;
    }
    let cats: Option<(String, String)> = tx
        .query_row(
            "SELECT before_json,after_json FROM catalog_tag_category_history WHERE length(CAST(before_json AS BLOB))<=1048576 AND length(CAST(after_json AS BLOB))<=1048576 AND operation_id=?",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if changes.is_empty() && cats.is_none() {
        return Err(invalid("This tag history is unavailable."));
    }
    if let Some((before, after)) = cats {
        let before: Vec<TagCategory> =
            serde_json::from_str(&before).map_err(|_| invalid("Invalid category history."))?;
        let after: Vec<TagCategory> =
            serde_json::from_str(&after).map_err(|_| invalid("Invalid category history."))?;
        validate_categories(&before)?;
        validate_categories(&after)?;
        if !same_categories(&categories(&tx)?, &after) {
            return Err(conflict());
        }
        let mut restored = before;
        for node in &mut restored {
            node.version = next(
                node.version.max(
                    after
                        .iter()
                        .find(|n| n.id == node.id)
                        .map_or(0, |n| n.version),
                ),
            )?;
        }
        write_categories(&tx, &after, &restored)?;
        tx.execute(
            "INSERT INTO catalog_tag_category_history VALUES(?,?,?)",
            params![op, encoded(&after)?, encoded(&restored)?],
        )?;
    }
    tx.execute("DELETE FROM catalog_tag_context", [])?;
    // Category-only undo has no asset trigger. Retain its timestamp and identity
    // in an already-applied audit record so it remains visible and reversible.
    let audit = encoded(&serde_json::json!({"undo_of": id}))?;
    let digest = blake3::hash(audit.as_bytes()).to_hex().to_string();
    tx.execute(
        "INSERT INTO catalog_metadata_plans VALUES(?,'tag_undo',?,?,?,?,'applied')",
        params![op, expected, digest, audit, store::now()],
    )?;
    bump(&tx)?;
    tx.commit()?;
    Ok(TagHistoryEntry {
        id: op,
        affected_count: changes.len() as u32,
        created_at: store::now(),
    })
}
pub(super) fn import_keywords(
    db: &mut Connection,
    id: String,
    expected: String,
    restore_removed: bool,
) -> Result<AssetNotes> {
    let tx = db.transaction()?;
    require_revision(&tx, &expected)?;
    let observed = notes(&tx, &id)?;
    if !observed.keywords_available || observed.observed_keywords.is_empty() {
        return Err(invalid(
            "No retained source keywords are available to import.",
        ));
    }
    if observed.keywords_truncated {
        return Err(invalid(
            "Keyword observations exceed the import bounds. No tags were changed.",
        ));
    }
    let raw: String = tx.query_row("SELECT tags FROM assets WHERE id=?", [&id], |r| r.get(0))?;
    let mut tags: Vec<String> =
        serde_json::from_str(&raw).map_err(|_| invalid("Invalid catalog tags."))?;
    for value in &observed.observed_keywords {
        let value = label(value)?;
        let removed = tx
            .prepare(
                "SELECT 1 FROM catalog_tag_overrides WHERE asset_id=? AND label=? AND removed=1",
            )?
            .exists(params![id, value])?;
        if restore_removed || !removed {
            tags.push(value);
        }
    }
    tags.sort();
    tags.dedup();
    if tags.len() > 64 {
        return Err(invalid(
            "Import would exceed 64 manual tags. No tags were changed.",
        ));
    }
    tx.execute(
        "UPDATE assets SET tags=? WHERE id=?",
        params![encoded(&tags)?, id],
    )?;
    bump(&tx)?;
    tx.commit()?;
    Ok(observed)
}

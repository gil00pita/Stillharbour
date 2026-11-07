//! Catalog-only organization. Nothing here opens media, sidecars or helpers.
mod albums;
pub mod model;
mod tags;
use super::{
    filters::invalid,
    model::{AssetQuery, AssetSelection},
    store,
};
use crate::{
    catalog::Catalog,
    domain::{AppError, ErrorCode, Result},
};
pub(super) use albums::import_scanned_keywords;
use model::*;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter, types::Value};
use std::collections::{BTreeSet, HashMap, HashSet};
pub(crate) use tags::migrate as migrate_completion;

const MAX_COLLECTIONS: usize = 200;
const MAX_CATEGORIES: usize = 512;

pub(super) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch(r#"
        CREATE TABLE asset_notes (
          asset_id TEXT PRIMARY KEY REFERENCES assets(id) ON DELETE CASCADE,
          version INTEGER NOT NULL CHECK(version>0),
          caption TEXT NOT NULL CHECK(length(CAST(caption AS BLOB))<=4096),
          rejected INTEGER NOT NULL CHECK(rejected IN (0,1))
        );
        CREATE TABLE asset_note_history (
          asset_id TEXT NOT NULL,version INTEGER NOT NULL CHECK(version>0),
          caption TEXT NOT NULL CHECK(length(CAST(caption AS BLOB))<=4096),
          rejected INTEGER NOT NULL CHECK(rejected IN (0,1)),recorded_at TEXT NOT NULL,
          PRIMARY KEY(asset_id,version)
        );
        CREATE TRIGGER asset_note_history_no_update BEFORE UPDATE ON asset_note_history BEGIN SELECT RAISE(ABORT,'Retain annotation history'); END;
        CREATE TRIGGER asset_note_history_no_delete BEFORE DELETE ON asset_note_history BEGIN SELECT RAISE(ABORT,'Retain annotation history'); END;
        CREATE TABLE catalog_collections (
          id TEXT PRIMARY KEY,version INTEGER NOT NULL CHECK(version>0),
          kind TEXT NOT NULL CHECK(kind IN ('album','search')),
          name TEXT NOT NULL CHECK(length(CAST(name AS BLOB)) BETWEEN 1 AND 128),
          query_json TEXT,archived INTEGER NOT NULL DEFAULT 0 CHECK(archived IN (0,1)),
          CHECK((kind='album' AND query_json IS NULL) OR (kind='search' AND json_valid(query_json))),
          UNIQUE(kind,name)
        );
        CREATE TABLE catalog_collection_members (
          collection_id TEXT NOT NULL REFERENCES catalog_collections(id) ON DELETE CASCADE,
          asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
          PRIMARY KEY(collection_id,asset_id)
        );
        CREATE INDEX collection_members_asset ON catalog_collection_members(asset_id,collection_id);
        CREATE TABLE catalog_tag_categories (
          id TEXT PRIMARY KEY,version INTEGER NOT NULL CHECK(version>0),
          label TEXT NOT NULL UNIQUE CHECK(length(CAST(label AS BLOB)) BETWEEN 1 AND 128),
          parent_id TEXT REFERENCES catalog_tag_categories(id),CHECK(parent_id IS NULL OR parent_id<>id)
        );
        CREATE INDEX tag_category_parent ON catalog_tag_categories(parent_id);
    "#)?;
    Ok(())
}
fn identity(id: &str) -> Result<()> {
    uuid::Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| invalid("Invalid catalog identity."))
}
fn label(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        return Err(invalid(
            "Use a nonempty label of at most 128 bytes without control characters.",
        ));
    }
    Ok(value.into())
}
fn conflict() -> AppError {
    AppError::new(
        ErrorCode::Conflict,
        "Catalog metadata changed. Reload it before saving; your draft has not replaced the newer version.",
    )
}
fn next(version: u32) -> Result<u32> {
    version
        .checked_add(1)
        .filter(|n| *n <= i32::MAX as u32)
        .ok_or_else(|| invalid("Metadata version exceeds its limit."))
}
fn bump(db: &Connection) -> Result<()> {
    db.execute(
        "UPDATE library_state SET revision=revision+1 WHERE singleton=1",
        [],
    )?;
    Ok(())
}

pub fn request(catalog: &Catalog, request: CatalogMetadataRequest) -> Result<CatalogMetadataReply> {
    use CatalogMetadataRequest::*;
    catalog.call(move |db| match request {
        Albums { search, offset } => Ok(CatalogMetadataReply::Albums(albums::list(db, search, offset)?)),
        CreateAlbum { name, asset_ids } => Ok(CatalogMetadataReply::Collection(albums::create(db, name, asset_ids)?)),
        ExportHistory {} => {
            let rows=db.prepare("SELECT e.export_id,e.phase,(SELECT count(*) FROM catalog_export_journal f WHERE f.export_id=e.export_id AND f.phase='file_verified'),e.created_at FROM catalog_export_journal e WHERE e.sequence=(SELECT max(last.sequence) FROM catalog_export_journal last WHERE last.export_id=e.export_id) ORDER BY e.sequence DESC LIMIT 20")?.query_map([],|r|Ok(super::metadata_export::model::MetadataExportRecord{id:r.get(0)?,last_phase:r.get(1)?,files_verified:r.get(2)?,updated_at:r.get(3)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(CatalogMetadataReply::ExportHistory(rows))
        },
        TagEditPreview { from, to } => Ok(CatalogMetadataReply::TagEditReview(tags::preview(
            db, from, to,
        )?)),
        TagEditApply { id, digest } => Ok(CatalogMetadataReply::TagHistoryEntry(tags::apply(
            db, id, digest,
        )?)),
        TagHistory {} => Ok(CatalogMetadataReply::TagHistory(tags::history(db)?)),
        TagUndo {
            id,
            expected_revision,
        } => Ok(CatalogMetadataReply::TagHistoryEntry(tags::undo(
            db,
            id,
            expected_revision,
        )?)),
        ImportKeywords {
            asset_id,
            expected_revision,
            restore_removed,
        } => Ok(CatalogMetadataReply::Notes(tags::import_keywords(
            db,
            asset_id,
            expected_revision,
            restore_removed,
        )?)),
        Notes { asset_id } => Ok(CatalogMetadataReply::Notes(notes(db, &asset_id)?)),
        SaveNotes {
            asset_id,
            expected_version,
            caption,
            rejected,
        } => Ok(CatalogMetadataReply::Notes(save_notes(
            db,
            &asset_id,
            expected_version,
            caption,
            rejected,
        )?)),
        Collections {} => Ok(CatalogMetadataReply::Collections(collections(db)?)),
        CreateCollection {
            collection_kind,
            name,
            query,
        } => Ok(CatalogMetadataReply::Collection(create_collection(
            db,
            collection_kind,
            name,
            query,
        )?)),
        UpdateCollection {
            id,
            expected_version,
            name,
            query,
        } => Ok(CatalogMetadataReply::Collection(update_collection(
            db,
            &id,
            expected_version,
            name,
            query,
        )?)),
        ArchiveCollection {
            id,
            expected_version,
            archived,
        } => {
            let tx = db.transaction()?;
            let current = collection(&tx, &id)?;
            if current.version != expected_version {
                return Err(conflict());
            }
            tx.execute(
                "UPDATE catalog_collections SET archived=?,version=? WHERE id=?",
                params![archived, next(expected_version)?, id],
            )?;
            bump(&tx)?;
            let value = collection(&tx, &id)?;
            tx.commit()?;
            Ok(CatalogMetadataReply::Collection(value))
        }
        CollectionMembers {
            id,
            expected_version,
            expected_count,
            selection,
            remove,
        } => Ok(CatalogMetadataReply::Membership(members(
            db,
            &id,
            expected_version,
            expected_count,
            selection,
            remove,
        )?)),
        TagCategories {} => Ok(CatalogMetadataReply::TagCategories(categories(db)?)),
        SaveTagCategory {
            id,
            expected_version,
            label,
            parent_id,
        } => Ok(CatalogMetadataReply::TagCategory(save_category(
            db,
            id,
            expected_version,
            label,
            parent_id,
        )?)),
    })
}

fn notes(db: &Connection, id: &str) -> Result<AssetNotes> {
    identity(id)?;
    let raw:Option<String>=db.query_row("SELECT CASE WHEN length(CAST(raw_metadata AS BLOB))<=1048576 THEN raw_metadata END FROM assets WHERE id=? AND is_media=1",[id],|r|r.get(0)).optional()?.ok_or_else(||invalid("This media item is no longer in the catalog."))?;
    let saved: Option<(u32, String, i64)> = db
        .query_row(
            "SELECT version,CASE WHEN length(CAST(caption AS BLOB))<=4096 THEN caption END,rejected FROM asset_notes WHERE asset_id=?",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let (version, caption, rejected) = saved.unwrap_or((0, String::new(), 0));
    if caption.len() > 4096 || !matches!(rejected, 0 | 1) {
        return Err(invalid("Invalid stored catalog notes."));
    }
    let mut values = BTreeSet::new();
    let mut truncated = false;
    let mut available = false;
    if let Some(raw) = raw.and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        && let Some(value) = raw.as_array().and_then(|v| v.first())
    {
        for key in ["Keywords", "Subject"] {
            if let Some(field) = value.get(key) {
                available = true;
                let items = if let Some(items) = field.as_array() {
                    items.iter().collect::<Vec<_>>()
                } else {
                    vec![field]
                };
                for item in items {
                    if let Some(text) = item.as_str() {
                        if text.len() <= 128 && values.len() < 64 {
                            values.insert(text.to_owned());
                        } else {
                            truncated = true;
                        }
                    } else {
                        truncated = true;
                    }
                }
            }
        }
    }
    Ok(AssetNotes {
        asset_id: id.into(),
        version,
        caption,
        rejected: rejected != 0,
        observed_keywords: values.into_iter().collect(),
        keywords_available: available,
        keywords_truncated: truncated,
    })
}
fn save_notes(
    db: &mut Connection,
    id: &str,
    expected: u32,
    caption: String,
    rejected: bool,
) -> Result<AssetNotes> {
    if caption.len() > 4096
        || caption
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(invalid(
            "Captions may contain up to 4096 bytes of text, tabs and line breaks.",
        ));
    }
    let tx = db.transaction()?;
    let current = notes(&tx, id)?;
    if current.version != expected {
        return Err(conflict());
    }
    let version = next(expected)?;
    tx.execute("INSERT INTO asset_notes VALUES(?1,?2,?3,?4) ON CONFLICT(asset_id) DO UPDATE SET version=excluded.version,caption=excluded.caption,rejected=excluded.rejected",params![id,version,caption,rejected])?;
    tx.execute(
        "INSERT INTO asset_note_history VALUES(?,?,?,?,?)",
        params![id, version, caption, rejected, store::now()],
    )?;
    bump(&tx)?;
    let result = notes(&tx, id)?;
    tx.commit()?;
    Ok(result)
}

fn validated_query(kind: CollectionKind, query: Option<AssetQuery>) -> Result<Option<String>> {
    match (kind, query) {
        (CollectionKind::Album, None) => Ok(None),
        (CollectionKind::Search, Some(mut query)) => {
            query.defer_pending_previews = None;
            store::query_where(&query)?;
            let json = serde_json::to_string(&query)
                .map_err(|e| invalid("Invalid saved search.").caused_by(e))?;
            if json.len() > 8192 {
                return Err(invalid("Saved search exceeds its limit."));
            }
            Ok(Some(json))
        }
        _ => Err(invalid(
            "Albums contain memberships; saved searches require a validated filter.",
        )),
    }
}
fn collection(db: &Connection, id: &str) -> Result<CatalogCollection> {
    identity(id)?;
    let (version,kind,name,raw,archived,count):(u32,String,String,Option<String>,i64,u32)=db.query_row("SELECT version,kind,CASE WHEN length(CAST(name AS BLOB))<=128 THEN name END,CASE WHEN query_json IS NULL OR length(CAST(query_json AS BLOB))<=8192 THEN query_json END,archived,(SELECT count(*) FROM catalog_collection_members m WHERE m.collection_id=c.id) FROM catalog_collections c WHERE id=?",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?.ok_or_else(||invalid("This catalog collection no longer exists."))?;
    let kind = match kind.as_str() {
        "album" => CollectionKind::Album,
        "search" => CollectionKind::Search,
        _ => return Err(invalid("Invalid stored collection kind.")),
    };
    if name.len() > 128
        || !matches!(archived, 0 | 1)
        || raw.as_ref().is_some_and(|v| v.len() > 8192)
    {
        return Err(invalid("Invalid stored collection."));
    }
    let query = raw
        .map(|v| {
            serde_json::from_str::<AssetQuery>(&v)
                .map_err(|_| invalid("Unsupported saved search schema."))
        })
        .transpose()?;
    validated_query(kind, query.clone())?;
    Ok(CatalogCollection {
        id: id.into(),
        version,
        kind,
        name,
        query,
        archived: archived != 0,
        member_count: count,
    })
}
fn collections(db: &Connection) -> Result<Vec<CatalogCollection>> {
    let ids = db
        .prepare("SELECT CASE WHEN length(CAST(id AS BLOB))=36 THEN id END FROM catalog_collections ORDER BY kind,name,id LIMIT 201")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if ids.len() > MAX_COLLECTIONS {
        return Err(invalid("Catalog collection limit exceeded."));
    }
    ids.into_iter().map(|id| collection(db, &id)).collect()
}
fn create_collection(
    db: &mut Connection,
    kind: CollectionKind,
    name: String,
    query: Option<AssetQuery>,
) -> Result<CatalogCollection> {
    let tx = db.transaction()?;
    let result = create_collection_in(&tx, kind, name, query)?;
    tx.commit()?;
    Ok(result)
}
fn create_collection_in(
    tx: &Connection,
    kind: CollectionKind,
    name: String,
    query: Option<AssetQuery>,
) -> Result<CatalogCollection> {
    let name = label(&name)?;
    let raw = validated_query(kind, query)?;
    albums::check_name(tx, kind, &name, None)?;

    if tx.query_row("SELECT count(*) FROM catalog_collections", [], |r| {
        r.get::<_, i64>(0)
    })? >= MAX_COLLECTIONS as i64
    {
        return Err(invalid(
            "This catalog supports 200 collections including archived entries.",
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO catalog_collections(id,version,kind,name,query_json) VALUES(?,1,?,?,?)",
        params![id, kind.text(), name, raw],
    )?;
    bump(tx)?;
    collection(tx, &id)
}
fn update_collection(
    db: &mut Connection,
    id: &str,
    expected: u32,
    name: String,
    query: Option<AssetQuery>,
) -> Result<CatalogCollection> {
    let name = label(&name)?;
    let tx = db.transaction()?;
    let current = collection(&tx, id)?;
    albums::check_name(&tx, current.kind, &name, Some(id))?;
    if current.version != expected {
        return Err(conflict());
    }
    if tx
        .prepare("SELECT 1 FROM catalog_collections WHERE kind=? AND name=? AND id<>?")?
        .exists(params![current.kind.text(), name, id])?
    {
        return Err(AppError::new(
            ErrorCode::Conflict,
            "A collection with this exact name already exists, including archived entries.",
        ));
    }
    let raw = validated_query(current.kind, query)?;
    tx.execute(
        "UPDATE catalog_collections SET name=?,query_json=?,version=? WHERE id=?",
        params![name, raw, next(expected)?, id],
    )?;
    bump(&tx)?;
    let result = collection(&tx, id)?;
    tx.commit()?;
    Ok(result)
}
fn members(
    db: &mut Connection,
    id: &str,
    expected: u32,
    expected_count: u32,
    selection: AssetSelection,
    remove: bool,
) -> Result<CollectionMembership> {
    if expected_count == 0 || expected_count > 50000 {
        return Err(invalid(
            "Select between one and 50,000 catalog media items.",
        ));
    }
    let tx = db.transaction()?;
    let current = collection(&tx, id)?;
    if current.version != expected {
        return Err(conflict());
    }
    if current.kind != CollectionKind::Album || current.archived {
        return Err(invalid("Choose an active virtual album."));
    }
    let explicit_count = match &selection {
        AssetSelection::Ids { ids } => {
            for id in ids {
                identity(id)?;
            }
            if ids.iter().collect::<HashSet<_>>().len() != ids.len() {
                return Err(invalid("Selection contains repeated identities."));
            }
            Some(ids.len())
        }
        AssetSelection::Token { id } => {
            identity(id)?;
            let fresh=tx.prepare("SELECT 1 FROM selections WHERE id=? AND julianday(created_at)>=julianday('now','-1 hour')")?.exists([id])?;
            if !fresh {
                return Err(AppError::new(
                    ErrorCode::Conflict,
                    "The frozen selection expired. Select the matching results again before editing album membership.",
                ));
            }
            None
        }
    };
    let (scope, values) = store::selection_scope(&tx, selection)?;
    let count: i64 = tx.query_row(
        &format!("SELECT count(*) FROM assets WHERE is_media=1 AND {scope}"),
        params_from_iter(values.clone()),
        |r| r.get(0),
    )?;
    if count == 0
        || count > 50000
        || i64::from(expected_count) != count
        || explicit_count.is_some_and(|n| n as i64 != count)
    {
        return Err(invalid(
            "Select up to 50,000 existing catalog media items; no partial selection was applied.",
        ));
    }
    let mut bound = vec![Value::Text(id.into())];
    bound.extend(values);
    let sql = if remove {
        format!(
            "DELETE FROM catalog_collection_members WHERE collection_id=? AND asset_id IN(SELECT id FROM assets WHERE is_media=1 AND {scope})"
        )
    } else {
        format!(
            "INSERT OR IGNORE INTO catalog_collection_members SELECT ?,id FROM assets WHERE is_media=1 AND {scope}"
        )
    };
    let changed = tx.execute(&sql, params_from_iter(bound))?;
    if changed != 0 {
        tx.execute(
            "UPDATE catalog_collections SET version=? WHERE id=?",
            params![next(expected)?, id],
        )?;
        bump(&tx)?;
    }
    let result = CollectionMembership {
        collection: collection(&tx, id)?,
        changed: u32::try_from(changed)
            .map_err(|_| invalid("Membership count exceeds its limit."))?,
    };
    tx.commit()?;
    Ok(result)
}

pub(super) fn categories(db: &Connection) -> Result<Vec<TagCategory>> {
    let values=db.prepare("SELECT CASE WHEN length(CAST(id AS BLOB))=36 THEN id END,version,CASE WHEN length(CAST(label AS BLOB))<=128 THEN label END,CASE WHEN parent_id IS NULL THEN NULL WHEN length(CAST(parent_id AS BLOB))=36 THEN parent_id ELSE '' END FROM catalog_tag_categories ORDER BY label,id LIMIT 513")?.query_map([],|r|Ok(TagCategory{id:r.get(0)?,version:r.get(1)?,label:r.get(2)?,parent_id:r.get(3)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
    validate_categories(&values)?;
    Ok(values)
}
fn validate_categories(values: &[TagCategory]) -> Result<()> {
    if values.len() > MAX_CATEGORIES {
        return Err(invalid("This catalog supports 512 tag categories."));
    }
    let parents = values
        .iter()
        .map(|node| (node.id.as_str(), node.parent_id.as_deref()))
        .collect::<HashMap<_, _>>();
    for node in values {
        if node.version == 0 {
            return Err(invalid("Invalid stored tag category version."));
        }
        identity(&node.id)?;
        label(&node.label)?;
        let mut cursor = Some(node.id.as_str());
        let mut seen = HashSet::new();
        while let Some(id) = cursor {
            if seen.len() >= 16 || !seen.insert(id) {
                return Err(invalid(
                    "Tag categories must be acyclic and at most sixteen levels deep.",
                ));
            }
            cursor = *parents
                .get(id)
                .ok_or_else(|| invalid("A tag category parent is unavailable."))?;
        }
    }
    Ok(())
}
fn save_category(
    db: &mut Connection,
    id: Option<String>,
    expected: Option<u32>,
    name: String,
    parent: Option<String>,
) -> Result<TagCategory> {
    let label = label(&name)?;
    if let Some(parent) = &parent {
        identity(parent)?;
    }
    let tx = db.transaction()?;
    let mut all = categories(&tx)?;
    let value = if let Some(id) = id {
        identity(&id)?;
        let previous = all
            .iter()
            .find(|v| v.id == id)
            .ok_or_else(|| invalid("This tag category no longer exists."))?;
        if Some(previous.version) != expected {
            return Err(conflict());
        }
        if previous.label != label {
            return Err(invalid(
                "Category arrangement preserves tag labels. Renaming or merging needs a separate reviewed action.",
            ));
        }
        let value = TagCategory {
            id,
            version: next(previous.version)?,
            label,
            parent_id: parent,
        };
        all.retain(|v| v.id != value.id);
        value
    } else {
        if expected.is_some() {
            return Err(invalid("New categories have no prior version."));
        }
        if all.iter().any(|node| node.label == label) {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "This exact tag label already has a category. Choose it to edit its parent.",
            ));
        }
        TagCategory {
            id: uuid::Uuid::new_v4().to_string(),
            version: 1,
            label,
            parent_id: parent,
        }
    };
    all.push(value.clone());
    validate_categories(&all)?;
    tx.execute("INSERT INTO catalog_tag_categories VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET version=excluded.version,parent_id=excluded.parent_id",params![value.id,value.version,value.label,value.parent_id])?;
    bump(&tx)?;
    tx.commit()?;
    Ok(value)
}

pub(super) fn query_filter(
    query: &AssetQuery,
    sql: &mut String,
    values: &mut Vec<Value>,
) -> Result<()> {
    if let Some(id) = &query.album_id {
        identity(id)?;
        sql.push_str(" AND id IN(SELECT m.asset_id FROM catalog_collection_members m JOIN catalog_collections c ON c.id=m.collection_id WHERE c.id=? AND c.kind='album' AND c.archived=0)");
        values.push(id.clone().into());
    }
    if let Some(rejected) = query.rejected {
        sql.push_str(
            " AND coalesce((SELECT rejected FROM asset_notes n WHERE n.asset_id=assets.id),0)=?",
        );
        values.push(Value::Integer(i64::from(rejected)));
    }
    if let Some(min) = query.min_rating {
        if min > 5 {
            return Err(invalid("Ratings range from zero to five."));
        }
        sql.push_str(" AND rating>=?");
        values.push(Value::Integer(min.into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod album_tests;

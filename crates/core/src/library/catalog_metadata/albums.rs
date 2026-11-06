//! Album projections and catalog-only atomic creation; no source file access.
use super::*;

pub(super) fn check_name(
    db: &Connection,
    kind: CollectionKind,
    name: &str,
    except: Option<&str>,
) -> Result<()> {
    let folded = name.trim().to_lowercase();
    let names = db
        .prepare("SELECT id,name FROM catalog_collections WHERE kind=?")?
        .query_map([kind.text()], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let duplicate = names
        .iter()
        .any(|(id, other)| Some(id.as_str()) != except && other.trim().to_lowercase() == folded);
    let tagged = kind == CollectionKind::Album && db.prepare("SELECT DISTINCT value FROM assets,json_each(assets.tags) WHERE is_media=1 AND json_each.type='text' LIMIT 10001")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?.iter().any(|v| v.trim().to_lowercase() == folded);
    if duplicate || tagged {
        return Err(AppError::new(
            ErrorCode::Conflict,
            "An album or collection with this name already exists. Choose another name (including archived albums and tag names).",
        ));
    }
    Ok(())
}

pub(super) fn create(
    db: &mut Connection,
    name: String,
    ids: Vec<String>,
) -> Result<CatalogCollection> {
    if ids.len() > 500 || ids.iter().collect::<HashSet<_>>().len() != ids.len() {
        return Err(invalid("Choose up to 500 distinct photos per album save."));
    }
    let tx = db.transaction()?;
    let album = create_collection_in(&tx, CollectionKind::Album, name, None)?;
    for id in ids {
        identity(&id)?;
        let changed = tx.execute("INSERT INTO catalog_collection_members SELECT ?,id FROM assets WHERE id=? AND is_media=1", params![album.id, id])?;
        if changed != 1 {
            return Err(invalid(
                "A selected photo is no longer available in the catalog. No album was saved.",
            ));
        }
    }
    let result = collection(&tx, &album.id)?;
    tx.commit()?;
    Ok(result)
}

pub(super) fn list(db: &Connection, search: String, offset: u32) -> Result<AlbumPage> {
    if search.len() > 256 || offset > 100000 {
        return Err(invalid("Album search exceeds its limit."));
    }
    let mut all = collections(db)?
        .into_iter()
        .filter(|c| c.kind == CollectionKind::Album && !c.archived)
        .map(|c| {
            (
                c.id.clone(),
                c.name,
                false,
                c.version,
                AssetQuery {
                    album_id: Some(c.id),
                    include_missing: true,
                    ..AssetQuery::default()
                },
            )
        })
        .collect::<Vec<_>>();
    let tags = db.prepare("SELECT DISTINCT value FROM assets,json_each(assets.tags) WHERE is_media=1 AND presence<>'excluded' AND json_each.type='text' ORDER BY value LIMIT 10001")?
        .query_map([], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    if tags.len() > 10000 {
        return Err(invalid("This catalog exceeds the 10,000 tag-album limit."));
    }
    for tag in tags {
        label(&tag)?;
        all.push((
            format!("tag:{tag}"),
            tag.clone(),
            true,
            0,
            AssetQuery {
                tag,
                include_missing: true,
                ..AssetQuery::default()
            },
        ));
    }
    let search = search.to_lowercase();
    all.retain(|(_, name, ..)| name.to_lowercase().contains(&search));
    all.sort_by(|a, b| {
        a.1.to_lowercase()
            .cmp(&b.1.to_lowercase())
            .then(a.0.cmp(&b.0))
    });
    let total = all.len() as u32;
    let mut items = Vec::new();
    for (id, name, from_tags, version, query) in all.into_iter().skip(offset as usize).take(48) {
        let (scope, values) = store::query_where(&query)?;
        let photo_count = db.query_row(
            &format!("SELECT count(*) FROM assets WHERE {scope}"),
            params_from_iter(values.clone()),
            |r| r.get(0),
        )?;
        let covers = db
            .prepare(&format!(
                "SELECT {} FROM assets WHERE {scope} ORDER BY date DESC,id LIMIT 2",
                store::ASSET_COLUMNS
            ))?
            .query_map(params_from_iter(values), store::asset_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        items.push(AlbumSummary {
            id,
            name,
            from_tags,
            version,
            photo_count,
            query,
            covers,
        });
    }
    let end = offset + items.len() as u32;
    Ok(AlbumPage {
        items,
        total,
        next_offset: (end < total).then_some(end),
    })
}

pub(in crate::library) fn import_scanned_keywords(db: &Connection, id: &str) -> Result<bool> {
    let observed = notes(db, id)?;
    if !observed.keywords_available || observed.observed_keywords.is_empty() {
        return Ok(false);
    }
    let raw: String = db.query_row("SELECT tags FROM assets WHERE id=?", [id], |r| r.get(0))?;
    let mut merged: BTreeSet<String> = serde_json::from_str::<Vec<String>>(&raw)
        .map_err(|_| invalid("Invalid catalog tags."))?
        .into_iter()
        .collect();
    let before = merged.clone();
    let mut invalid_labels = observed.keywords_truncated;
    for value in observed.observed_keywords {
        let Ok(value) = label(&value) else {
            invalid_labels = true;
            continue;
        };
        if !db
            .prepare(
                "SELECT 1 FROM catalog_tag_overrides WHERE asset_id=? AND label=? AND removed=1",
            )?
            .exists(params![id, value])?
        {
            merged.insert(value);
        }
    }
    if invalid_labels || merged.len() > 64 {
        db.execute("UPDATE assets SET metadata_error=coalesce(metadata_error, 'Source keywords could not be imported automatically: invalid labels or 64-tag/128-byte limit. Review observed keywords in Info.') WHERE id=?", [id])?;
        return Ok(false);
    }
    if before == merged {
        return Ok(false);
    }
    db.execute(
        "UPDATE assets SET tags=? WHERE id=?",
        params![
            serde_json::to_string(&merged).map_err(|_| invalid("Invalid tags."))?,
            id
        ],
    )?;
    Ok(true)
}

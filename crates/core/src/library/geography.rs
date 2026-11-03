//! Local catalog observations only. R-tree candidates always receive exact
//! double-precision bounds checks; its float32 rounding cannot choose membership.
use super::{filters::invalid, model::*, store};
use crate::{catalog::Catalog, domain::Result};
use rusqlite::{Connection, params_from_iter, types::Value};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const MAX_CLUSTERS: usize = 256;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapBounds {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}
impl MapBounds {
    pub fn validate(&self) -> Result<()> {
        if ![self.west, self.east, self.south, self.north]
            .into_iter()
            .all(f64::is_finite)
            || !(-180.0..=180.0).contains(&self.west)
            || !(-180.0..=180.0).contains(&self.east)
            || !(-90.0..=90.0).contains(&self.south)
            || !(-90.0..=90.0).contains(&self.north)
            || self.south > self.north
        {
            return Err(invalid("Invalid geographic viewport."));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocationFilter {
    Known,
    Unavailable,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MapCluster {
    pub cell: MapCell,
    pub key: String,
    pub latitude: f64,
    pub longitude: f64,
    pub count: u32,
    pub representative: AssetSummary,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MapView {
    pub clusters: Vec<MapCluster>,
    pub in_view: u32,
    pub located: u32,
    /// Includes absent, invalid, and not-yet-extracted coordinates; not "missing files".
    pub unavailable: u32,
    pub cluster_zoom: u8,
    pub revision: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MapCell {
    pub zoom: u8,
    pub x: u32,
    pub y: u32,
}

pub(super) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch(
        r#"
        CREATE TABLE asset_locations (
          sequence INTEGER PRIMARY KEY REFERENCES assets(sequence) ON DELETE CASCADE,
          latitude REAL NOT NULL CHECK(latitude BETWEEN -90 AND 90),
          longitude REAL NOT NULL CHECK(longitude>=-180 AND longitude<180),
          source TEXT NOT NULL CHECK(source='exiftool-composite-numeric-v1')
        );
        CREATE VIRTUAL TABLE asset_spatial USING rtree(sequence,min_lon,max_lon,min_lat,max_lat);
        CREATE VIEW asset_location_observations AS
          SELECT sequence,
            json_extract(CASE WHEN json_valid(raw_metadata) THEN raw_metadata ELSE '[]' END,'$[0].GPSLatitude') AS latitude,
            json_extract(CASE WHEN json_valid(raw_metadata) THEN raw_metadata ELSE '[]' END,'$[0].GPSLongitude') AS longitude
          FROM assets WHERE is_media=1
            AND json_type(CASE WHEN json_valid(raw_metadata) THEN raw_metadata ELSE '[]' END,'$[0].GPSLatitude') IN ('integer','real')
            AND json_type(CASE WHEN json_valid(raw_metadata) THEN raw_metadata ELSE '[]' END,'$[0].GPSLongitude') IN ('integer','real');
        CREATE TRIGGER asset_locations_insert AFTER INSERT ON asset_locations BEGIN
          INSERT INTO asset_spatial VALUES(NEW.sequence,NEW.longitude,NEW.longitude,NEW.latitude,NEW.latitude);
        END;
        CREATE TRIGGER asset_locations_delete AFTER DELETE ON asset_locations BEGIN
          DELETE FROM asset_spatial WHERE sequence=OLD.sequence;
        END;
        CREATE TRIGGER assets_location_insert AFTER INSERT ON assets BEGIN
          INSERT INTO asset_locations SELECT sequence,latitude,CASE WHEN longitude=180 THEN -180 ELSE longitude END,'exiftool-composite-numeric-v1'
          FROM asset_location_observations WHERE sequence=NEW.sequence AND latitude BETWEEN -90 AND 90 AND longitude BETWEEN -180 AND 180;
        END;
        CREATE TRIGGER assets_location_update AFTER UPDATE OF raw_metadata,is_media ON assets BEGIN
          DELETE FROM asset_locations WHERE sequence=OLD.sequence;
          INSERT INTO asset_locations SELECT sequence,latitude,CASE WHEN longitude=180 THEN -180 ELSE longitude END,'exiftool-composite-numeric-v1'
          FROM asset_location_observations WHERE sequence=NEW.sequence AND latitude BETWEEN -90 AND 90 AND longitude BETWEEN -180 AND 180;
        END;
        CREATE TRIGGER assets_location_delete AFTER DELETE ON assets BEGIN
          DELETE FROM asset_locations WHERE sequence=OLD.sequence;
        END;
        INSERT INTO asset_locations SELECT sequence,latitude,CASE WHEN longitude=180 THEN -180 ELSE longitude END,'exiftool-composite-numeric-v1'
          FROM asset_location_observations WHERE latitude BETWEEN -90 AND 90 AND longitude BETWEEN -180 AND 180;
        "#,
    )?;
    Ok(())
}

pub(super) fn migrate_jsonq(db: &Connection) -> Result<()> {
    // Preserve the exact previously created v14 fingerprint. JSONQ support is
    // a distinct transactionally migrated schema, never same-version adoption.
    db.execute_batch(r#"
        DROP VIEW asset_location_observations;
        CREATE VIEW asset_location_observations AS
          WITH safe AS (SELECT sequence,CASE WHEN json_valid(raw_metadata) THEN raw_metadata ELSE '[]' END AS raw FROM assets WHERE is_media=1),
          observed AS (SELECT sequence,json_type(raw,'$[0].GPSLatitude') AS lat_type,json_type(raw,'$[0].GPSLongitude') AS lon_type,json_extract(raw,'$[0].GPSLatitude') AS lat,json_extract(raw,'$[0].GPSLongitude') AS lon FROM safe)
          SELECT sequence,
            CASE WHEN lat_type IN ('integer','real') THEN lat
              WHEN lat_type='text' THEN CASE WHEN json_valid(lat) THEN CASE WHEN json_type(lat) IN ('integer','real') THEN CAST(lat AS REAL) END END END AS latitude,
            CASE WHEN lon_type IN ('integer','real') THEN lon
              WHEN lon_type='text' THEN CASE WHEN json_valid(lon) THEN CASE WHEN json_type(lon) IN ('integer','real') THEN CAST(lon AS REAL) END END END AS longitude
          FROM observed;
        DELETE FROM asset_locations;
        INSERT INTO asset_locations SELECT sequence,latitude,CASE WHEN longitude=180 THEN -180 ELSE longitude END,'exiftool-composite-numeric-v1'
          FROM asset_location_observations WHERE latitude BETWEEN -90 AND 90 AND longitude BETWEEN -180 AND 180;
    "#)?;
    Ok(())
}

pub(super) fn query_filter(
    query: &AssetQuery,
    sql: &mut String,
    values: &mut Vec<Value>,
) -> Result<()> {
    if let Some(cell) = query.map_cell {
        if cell.zoom > 16 || cell.x >= 1_u32 << cell.zoom || cell.y >= 1_u32 << cell.zoom {
            return Err(invalid("Invalid geographic cluster."));
        }
        let cells = 1_u32 << cell.zoom;
        sql.push_str(&format!(" AND EXISTS(SELECT 1 FROM asset_locations p WHERE p.sequence=assets.sequence AND min({cells}-1,CAST((p.longitude+180)/360*{cells} AS INTEGER))=? AND min({cells}-1,CAST((p.latitude+90)/180*{cells} AS INTEGER))=?)"));
        values.extend([Value::Integer(cell.x.into()), Value::Integer(cell.y.into())]);
    }
    if let Some(location) = query.location {
        sql.push_str(if location == LocationFilter::Known {
            " AND EXISTS(SELECT 1 FROM asset_locations p WHERE p.sequence=assets.sequence)"
        } else {
            " AND NOT EXISTS(SELECT 1 FROM asset_locations p WHERE p.sequence=assets.sequence)"
        });
    }
    if let Some(bounds) = query.map_bounds {
        bounds.validate()?;
        sql.push_str(" AND assets.sequence IN (SELECT p.sequence FROM asset_locations p JOIN asset_spatial s ON s.sequence=p.sequence WHERE s.min_lat<=? AND s.max_lat>=? AND p.latitude>=? AND p.latitude<=?");
        values.extend([bounds.north, bounds.south, bounds.south, bounds.north].map(Value::Real));
        if bounds.east - bounds.west < 360.0 {
            let west = if bounds.west == 180.0 {
                -180.0
            } else {
                bounds.west
            };
            let east = if bounds.east == 180.0 {
                -180.0
            } else {
                bounds.east
            };
            if west > east {
                sql.push_str(
                    " AND ((s.max_lon>=? AND p.longitude>=?) OR (s.min_lon<=? AND p.longitude<=?))",
                );
            } else {
                sql.push_str(
                    " AND s.max_lon>=? AND p.longitude>=? AND s.min_lon<=? AND p.longitude<=?",
                );
            }
            values.extend([west, west, east, east].map(Value::Real));
        }
        sql.push(')');
    }
    Ok(())
}

pub fn query(
    catalog: &Catalog,
    mut query: AssetQuery,
    bounds: MapBounds,
    zoom: u8,
) -> Result<MapView> {
    bounds.validate()?;
    if zoom > 16 {
        return Err(invalid("Map zoom exceeds its limit."));
    }
    // Counts reflect the supplied catalog filters, independently of the viewport.
    query.map_bounds = None;
    query.map_cell = None;
    query.location = None;
    let (all_where, all_values) = store::query_where(&query)?;
    query.map_bounds = Some(bounds);
    let (where_sql, values) = store::query_where(&query)?;
    catalog.call(move |db| query_connection(db, &all_where, all_values, &where_sql, values, zoom))
}

fn query_connection(
    db: &Connection,
    all_where: &str,
    all_values: Vec<Value>,
    where_sql: &str,
    values: Vec<Value>,
    mut zoom: u8,
) -> Result<MapView> {
    let (total, located): (u32, u32) = db.query_row(
        &format!("SELECT count(*),coalesce(sum(EXISTS(SELECT 1 FROM asset_locations p WHERE p.sequence=assets.sequence)),0) FROM assets WHERE {all_where}"),
        params_from_iter(all_values), |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let in_view = db.query_row(
        &format!("SELECT count(*) FROM assets WHERE {where_sql}"),
        params_from_iter(values.clone()),
        |row| row.get(0),
    )?;
    let groups = loop {
        let cells = 1_u32 << zoom;
        // Fixed world-aligned angular grid; zoom is coarsened to bound markers.
        // No raw metadata or individual library-wide point list leaves SQLite.
        let sql = format!(
            "SELECT min({cells}-1,CAST((p.longitude+180)/360*{cells} AS INTEGER)) AS x,min({cells}-1,CAST((p.latitude+90)/180*{cells} AS INTEGER)) AS y,count(*),min(id) FROM assets JOIN asset_locations p ON p.sequence=assets.sequence WHERE {where_sql} GROUP BY x,y ORDER BY x,y LIMIT {}",
            MAX_CLUSTERS + 1
        );
        let groups = db
            .prepare(&sql)?
            .query_map(params_from_iter(values.clone()), |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, u32>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if groups.len() <= MAX_CLUSTERS || zoom == 0 {
            break groups;
        }
        zoom -= 1;
    };
    let mut clusters = Vec::with_capacity(groups.len());
    let mut asset_statement = db.prepare(&format!(
        "SELECT {} FROM assets WHERE id=?",
        store::ASSET_COLUMNS
    ))?;
    let mut point_statement = db.prepare("SELECT latitude,longitude FROM asset_locations WHERE sequence=(SELECT sequence FROM assets WHERE id=?)")?;
    for (x, y, count, id) in groups {
        let representative = asset_statement.query_row([&id], store::asset_row)?;
        let (latitude, longitude) =
            point_statement.query_row([&id], |row| Ok((row.get(0)?, row.get(1)?)))?;
        clusters.push(MapCluster {
            cell: MapCell { zoom, x, y },
            key: format!("v1:{zoom}:{x}:{y}"),
            latitude,
            longitude,
            count,
            representative,
        });
    }
    Ok(MapView {
        clusters,
        in_view,
        located,
        unavailable: total - located,
        cluster_zoom: zoom,
        revision: db.query_row(
            "SELECT CAST(revision AS TEXT) FROM library_state WHERE singleton=1",
            [],
            |row| row.get(0),
        )?,
    })
}

#[cfg(test)]
mod tests;

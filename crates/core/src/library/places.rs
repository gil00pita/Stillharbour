//! Bundled place lookup and GPS membership. No geocoding service or source I/O.
mod geometry;
use super::{filters::invalid, geography::MapBounds, model::AssetQuery};
use crate::domain::{AppError, ErrorCode, Result};
use geometry::{Polygon, city_bounds, city_contains, country_bounds};
use rusqlite::{Connection, functions::FunctionFlags, types::Value};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Read, sync::OnceLock};
use ts_rs::TS;
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlaceKind {
    City,
    Country,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlaceSummary {
    pub id: String,
    pub label: String,
    pub kind: PlaceKind,
    pub area: String,
    pub bounds: MapBounds,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlaceSearchReply {
    pub matches: Vec<PlaceSummary>,
    pub selected: Option<PlaceSummary>,
    pub more: bool,
    pub credits: String,
}

enum Shape {
    City(f64, f64),
    Country(Vec<Polygon>),
}
struct Place {
    summary: PlaceSummary,
    name: String,
    search: String,
    population: u64,
    shape: Shape,
}
impl Place {
    fn contains(&self, latitude: f64, longitude: f64) -> bool {
        if !latitude.is_finite()
            || !longitude.is_finite()
            || !(-90.0..=90.0).contains(&latitude)
            || !(-180.0..=180.0).contains(&longitude)
        {
            return false;
        }
        match &self.shape {
            Shape::City(lat, lon) => city_contains(latitude, longitude, *lat, *lon),
            Shape::Country(polygons) => polygons
                .iter()
                .any(|polygon| polygon.contains(latitude, longitude)),
        }
    }
}

fn normalize(value: &str) -> String {
    value
        .nfkd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}
fn data_error() -> AppError {
    AppError::new(
        ErrorCode::Unsupported,
        "The bundled offline place index could not be read.",
    )
}

#[derive(Deserialize)]
struct City {
    id: String,
    name: String,
    country: String,
    region: String,
    latitude: f64,
    longitude: f64,
    population: u64,
    aliases: Vec<String>,
}
#[derive(Deserialize)]
struct Country {
    id: String,
    name: String,
    aliases: Vec<String>,
    polygons: Vec<Vec<Vec<[f64; 2]>>>,
}

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let mut data = Vec::new();
    flate2::read::GzDecoder::new(bytes)
        .take(32_000_001)
        .read_to_end(&mut data)
        .map_err(|error| data_error().caused_by(error))?;
    if data.len() > 32_000_000 {
        return Err(data_error());
    }
    serde_json::from_slice(&data).map_err(|error| data_error().caused_by(error))
}

fn load() -> Result<BTreeMap<String, Place>> {
    let cities: Vec<City> = decode(include_bytes!("../../../../assets/places/cities.json.gz"))?;
    let countries: Vec<Country> = decode(include_bytes!(
        "../../../../assets/places/countries.json.gz"
    ))?;
    if cities.len() > 40_000 || countries.len() > 300 {
        return Err(data_error());
    }
    let mut places = BTreeMap::new();
    for city in cities {
        if !city.latitude.is_finite()
            || !city.longitude.is_finite()
            || !(-90.0..=90.0).contains(&city.latitude)
            || !(-180.0..=180.0).contains(&city.longitude)
        {
            return Err(data_error());
        }
        let label = [
            city.name.as_str(),
            city.region.as_str(),
            city.country.as_str(),
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
        let search = normalize(&format!("{label} {}", city.aliases.join(" ")));
        places.insert(
            city.id.clone(),
            Place {
                name: normalize(&city.name),
                search,
                population: city.population,
                summary: PlaceSummary {
                    id: city.id,
                    label,
                    kind: PlaceKind::City,
                    area: "Within 25 km of the city centre".into(),
                    bounds: city_bounds(city.latitude, city.longitude),
                },
                shape: Shape::City(city.latitude, city.longitude),
            },
        );
    }
    for country in countries {
        let polygons = country
            .polygons
            .into_iter()
            .map(Polygon::new)
            .collect::<Option<Vec<_>>>()
            .ok_or_else(data_error)?;
        let bounds = country_bounds(&polygons).ok_or_else(data_error)?;
        places.insert(
            country.id.clone(),
            Place {
                name: normalize(&country.name),
                search: normalize(&format!("{} {}", country.name, country.aliases.join(" "))),
                population: 0,
                summary: PlaceSummary {
                    id: country.id,
                    label: country.name,
                    kind: PlaceKind::Country,
                    area: "Approximate country or territory boundaries".into(),
                    bounds,
                },
                shape: Shape::Country(polygons),
            },
        );
    }
    Ok(places)
}
fn dataset() -> Result<&'static BTreeMap<String, Place>> {
    static DATA: OnceLock<Result<BTreeMap<String, Place>>> = OnceLock::new();
    DATA.get_or_init(load).as_ref().map_err(Clone::clone)
}
fn lookup(id: &str) -> Result<&'static Place> {
    if id.len() > 64 {
        return Err(invalid("Invalid place identity."));
    }
    dataset()?.get(id).ok_or_else(|| {
        invalid("This place is unavailable in the offline index. Search for it again.")
    })
}

pub fn search(text: &str, selected_id: Option<&str>) -> Result<PlaceSearchReply> {
    if text.len() > 256 || text.contains('\0') {
        return Err(invalid("Place search exceeds its limit."));
    }
    let selected = selected_id
        .map(|id| lookup(id).map(|place| place.summary.clone()))
        .transpose()?;
    let term = normalize(text.trim());
    let mut matches = Vec::new();
    if term.chars().count() >= 2 {
        let words: Vec<_> = term.split_whitespace().collect();
        for place in dataset()?.values() {
            if words
                .iter()
                .all(|word| place.search.contains(word.trim_matches(',')))
            {
                let rank = if place.name == term
                    || (place.summary.kind == PlaceKind::Country
                        && place.search.split_whitespace().any(|word| word == term))
                {
                    0
                } else if place.name.starts_with(&term) {
                    1
                } else {
                    2
                };
                matches.push((rank, place));
            }
        }
        matches.sort_by(|(a, p), (b, q)| {
            a.cmp(b)
                .then_with(|| q.population.cmp(&p.population))
                .then_with(|| p.summary.label.cmp(&q.summary.label))
        });
    }
    let more = matches.len() > 20;
    Ok(PlaceSearchReply {
        matches: matches
            .into_iter()
            .take(20)
            .map(|(_, p)| p.summary.clone())
            .collect(),
        selected,
        more,
        credits: include_str!("../../../../assets/places/NOTICE.md").into(),
    })
}

pub(crate) fn register(db: &Connection) -> rusqlite::Result<()> {
    db.create_scalar_function(
        "stillharbour_place_contains",
        3,
        FunctionFlags::SQLITE_UTF8
            | FunctionFlags::SQLITE_DETERMINISTIC
            | FunctionFlags::SQLITE_DIRECTONLY,
        |context| {
            let id: String = context.get(0)?;
            let place =
                lookup(&id).map_err(|error| rusqlite::Error::UserFunctionError(Box::new(error)))?;
            Ok(place.contains(context.get(1)?, context.get(2)?))
        },
    )
}

pub(super) fn query_filter(
    query: &AssetQuery,
    sql: &mut String,
    values: &mut Vec<Value>,
) -> Result<()> {
    let Some(id) = &query.place_id else {
        return Ok(());
    };
    let place = lookup(id)?;
    // Conservative indexed candidates, followed by exact distance/polygon
    // membership. Renderer-provided labels or bounds never define the place.
    super::geography::query_filter(
        &AssetQuery {
            map_bounds: Some(place.summary.bounds),
            ..Default::default()
        },
        sql,
        values,
    )?;
    sql.push_str(" AND EXISTS(SELECT 1 FROM asset_locations place_point WHERE place_point.sequence=assets.sequence AND stillharbour_place_contains(?,place_point.latitude,place_point.longitude))");
    values.push(id.clone().into());
    Ok(())
}

#[cfg(test)]
mod tests;

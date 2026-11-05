use super::super::geography::MapBounds;

pub(super) const CITY_RADIUS_KM: f64 = 25.0;
const EARTH_KM: f64 = 6371.0088;

pub(super) fn city_contains(lat: f64, lon: f64, centre_lat: f64, centre_lon: f64) -> bool {
    let a = ((lat - centre_lat).to_radians() / 2.0).sin().powi(2)
        + lat.to_radians().cos()
            * centre_lat.to_radians().cos()
            * ((lon - centre_lon).to_radians() / 2.0).sin().powi(2);
    2.0 * EARTH_KM * a.clamp(0.0, 1.0).sqrt().asin() <= CITY_RADIUS_KM
}

pub(super) fn city_bounds(lat: f64, lon: f64) -> MapBounds {
    let angle = CITY_RADIUS_KM / EARTH_KM;
    let south = (lat - angle.to_degrees()).max(-90.0);
    let north = (lat + angle.to_degrees()).min(90.0);
    let (west, east) = if south == -90.0 || north == 90.0 {
        (-180.0, 180.0)
    } else {
        let delta = (angle.sin() / lat.to_radians().cos())
            .clamp(-1.0, 1.0)
            .asin()
            .to_degrees();
        (
            (lon - delta + 180.0).rem_euclid(360.0) - 180.0,
            (lon + delta + 180.0).rem_euclid(360.0) - 180.0,
        )
    };
    MapBounds {
        west,
        east,
        south,
        north,
    }
}

#[derive(Debug)]
pub(super) struct Polygon {
    rings: Vec<Vec<[f64; 2]>>,
    pub bounds: MapBounds,
}

impl Polygon {
    pub fn new(rings: Vec<Vec<[f64; 2]>>) -> Option<Self> {
        if rings.is_empty()
            || rings.iter().any(|ring| {
                ring.len() < 4
                    || ring.first() != ring.last()
                    || ring.iter().any(|[x, y]| {
                        !x.is_finite()
                            || !y.is_finite()
                            || !(-180.0..=180.0).contains(x)
                            || !(-90.0..=90.0).contains(y)
                    })
            })
        {
            return None;
        }
        let mut bounds = MapBounds {
            west: 180.0,
            east: -180.0,
            south: 90.0,
            north: -90.0,
        };
        for [x, y] in &rings[0] {
            bounds.west = bounds.west.min(*x);
            bounds.east = bounds.east.max(*x);
            bounds.south = bounds.south.min(*y);
            bounds.north = bounds.north.max(*y);
        }
        Some(Self { rings, bounds })
    }

    pub fn contains(&self, lat: f64, lon: f64) -> bool {
        // Natural Earth cuts its polygons at the antimeridian. Test both
        // representations of that shared edge without unwrapping polar rings.
        [lon, if lon == -180.0 { 180.0 } else { lon }]
            .into_iter()
            .any(|x| {
                if x < self.bounds.west
                    || x > self.bounds.east
                    || lat < self.bounds.south
                    || lat > self.bounds.north
                {
                    return false;
                }
                ring_position(&self.rings[0], x, lat) != Position::Outside
                    && !self.rings[1..]
                        .iter()
                        .any(|ring| ring_position(ring, x, lat) == Position::Inside)
            })
    }
}

#[derive(PartialEq)]
enum Position {
    Outside,
    Inside,
    Boundary,
}

fn ring_position(ring: &[[f64; 2]], x: f64, y: f64) -> Position {
    let mut inside = false;
    for edge in ring.windows(2) {
        let ([ax, ay], [bx, by]) = (edge[0], edge[1]);
        let cross = (x - ax) * (by - ay) - (y - ay) * (bx - ax);
        if cross.abs() <= 1e-10
            && x >= ax.min(bx)
            && x <= ax.max(bx)
            && y >= ay.min(by)
            && y <= ay.max(by)
        {
            return Position::Boundary;
        }
        if (ay > y) != (by > y) && x < (bx - ax) * (y - ay) / (by - ay) + ax {
            inside = !inside;
        }
    }
    if inside {
        Position::Inside
    } else {
        Position::Outside
    }
}

pub(super) fn country_bounds(polygons: &[Polygon]) -> Option<MapBounds> {
    let mut intervals: Vec<_> = polygons
        .iter()
        .map(|p| (p.bounds.west, p.bounds.east))
        .collect();
    intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (west, east) in intervals {
        if let Some(last) = merged.last_mut().filter(|last| west <= last.1) {
            last.1 = last.1.max(east);
        } else {
            merged.push((west, east));
        }
    }
    let first = merged.first()?;
    let mut west = -180.0;
    let mut east = 180.0;
    let mut gap = 0.0;
    for (index, interval) in merged.iter().enumerate() {
        let next = merged.get(index + 1).map_or(first.0 + 360.0, |next| next.0);
        if next - interval.1 > gap {
            gap = next - interval.1;
            // Keep the source endpoint exact: adding/subtracting 360 can move
            // the western edge inward by one ULP and exclude a boundary point.
            west = merged.get(index + 1).map_or(first.0, |next| next.0);
            east = interval.1;
        }
    }
    Some(MapBounds {
        west,
        east,
        south: polygons.iter().map(|p| p.bounds.south).fold(90.0, f64::min),
        north: polygons
            .iter()
            .map(|p| p.bounds.north)
            .fold(-90.0, f64::max),
    })
}

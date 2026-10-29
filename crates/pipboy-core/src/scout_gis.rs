//! Reclamation Scout GIS bridge: read-only, offline GeoJSON inspection
//! and explicit waypoint/distance calculations. Independent code; no RustGIS
//! source vendoring, sockets, location sampling or data persistence.
use serde::Serialize;
use serde_json::{json, Value};
use std::io;

pub const MAX_GEOJSON_BYTES: usize = 1024 * 1024;
pub const MAX_FEATURES: usize = 1000;
pub const MAX_POINTS: usize = MAX_FEATURES;
pub const MAX_VERTICES: usize = 30_000;
const MEAN_EARTH_RADIUS_M: f64 = 6_371_008.8;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Point {
    pub latitude: f64,
    pub longitude: f64,
}

impl Point {
    pub fn new(latitude: f64, longitude: f64) -> io::Result<Self> {
        if !latitude.is_finite() || !longitude.is_finite() {
            return Err(invalid("coordinates must be finite numbers"));
        }
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(invalid("latitude must be in -90..=90"));
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(invalid("longitude must be in -180..=180"));
        }
        Ok(Self {
            latitude,
            longitude,
        })
    }

    pub fn from_text(lat: &str, lon: &str) -> io::Result<Self> {
        let lat = lat
            .parse::<f64>()
            .map_err(|_| invalid("latitude is not a number"))?;
        let lon = lon
            .parse::<f64>()
            .map_err(|_| invalid("longitude is not a number"))?;
        Self::new(lat, lon)
    }
}

/// Spherical great-circle distance, not an ellipsoidal survey-grade geodesic.
/// The longitude difference handles crossing the antimeridian automatically.
pub fn distance_meters(a: Point, b: Point) -> f64 {
    let dlat = (b.latitude - a.latitude).to_radians();
    let dlon = (b.longitude - a.longitude).to_radians();
    let lat_a = a.latitude.to_radians();
    let lat_b = b.latitude.to_radians();
    let h = (dlat / 2.0).sin().powi(2) + lat_a.cos() * lat_b.cos() * (dlon / 2.0).sin().powi(2);
    MEAN_EARTH_RADIUS_M * 2.0 * h.clamp(0.0, 1.0).sqrt().asin()
}

/// WGS84 GeoJSON per RFC 7946: positions are [longitude, latitude].
/// User-supplied name is JSON escaped; never interpolated as source text.
pub fn waypoint(name: &str, point: Point) -> io::Result<Value> {
    if name.is_empty()
        || name.trim() != name
        || name.chars().count() > 64
        || name.chars().any(char::is_control)
    {
        return Err(invalid("waypoint name must be 1..64 printable characters"));
    }
    Ok(json!({
        "type": "Feature",
        "geometry": {
            "type": "Point",
            "coordinates": [point.longitude, point.latitude]
        },
        "properties": {
            "name": name
        }
    }))
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GeoJsonSummary {
    pub format: &'static str,
    pub geometry: &'static str,
    pub features: usize,
    /// [west, south, east, north]; raw min/max, no antimeridian unwrapping.
    pub bbox: [f64; 4],
    pub coordinate_order: &'static str,
    pub antimeridian_note: &'static str,
    pub network_used: bool,
}

struct GeometryStats {
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    vertices: usize,
    geometries: usize,
    types: std::collections::BTreeSet<&'static str>,
}

impl GeometryStats {
    fn new() -> Self {
        Self {
            west: f64::INFINITY,
            south: f64::INFINITY,
            east: f64::NEG_INFINITY,
            north: f64::NEG_INFINITY,
            vertices: 0,
            geometries: 0,
            types: std::collections::BTreeSet::new(),
        }
    }

    fn position(&mut self, value: &Value) -> io::Result<()> {
        let coordinates = value
            .as_array()
            .filter(|coordinates| coordinates.len() >= 2)
            .ok_or_else(|| invalid("GeoJSON position requires longitude and latitude"))?;
        let lon = coordinates[0]
            .as_f64()
            .ok_or_else(|| invalid("longitude must be numeric"))?;
        let lat = coordinates[1]
            .as_f64()
            .ok_or_else(|| invalid("latitude must be numeric"))?;
        Point::new(lat, lon)?;
        self.vertices += 1;
        if self.vertices > MAX_VERTICES {
            return Err(invalid("GeoJSON exceeds 30,000 positions"));
        }
        self.west = self.west.min(lon);
        self.east = self.east.max(lon);
        self.south = self.south.min(lat);
        self.north = self.north.max(lat);
        Ok(())
    }

    fn positions(&mut self, value: &Value, minimum: usize) -> io::Result<()> {
        let values = value
            .as_array()
            .filter(|values| values.len() >= minimum)
            .ok_or_else(|| invalid("GeoJSON coordinate path is empty or too short"))?;
        for position in values {
            self.position(position)?;
        }
        Ok(())
    }

    fn geometry(&mut self, value: &Value, depth: usize) -> io::Result<()> {
        if depth > 6 {
            return Err(invalid("GeoJSON GeometryCollection nesting exceeds 6"));
        }
        let kind = value
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("GeoJSON geometry requires a type"))?;
        self.geometries += 1;
        match kind {
            "Point" => {
                self.types.insert("Point");
                self.position(&value["coordinates"])?;
            }
            "MultiPoint" => {
                self.types.insert("MultiPoint");
                self.positions(&value["coordinates"], 1)?;
            }
            "LineString" => {
                self.types.insert("LineString");
                self.positions(&value["coordinates"], 2)?;
            }
            "MultiLineString" => {
                self.types.insert("MultiLineString");
                let paths = value["coordinates"]
                    .as_array()
                    .filter(|paths| !paths.is_empty())
                    .ok_or_else(|| invalid("MultiLineString requires coordinate paths"))?;
                for path in paths {
                    self.positions(path, 2)?;
                }
            }
            "Polygon" => {
                self.types.insert("Polygon");
                let rings = value["coordinates"]
                    .as_array()
                    .filter(|rings| !rings.is_empty())
                    .ok_or_else(|| invalid("Polygon requires coordinate rings"))?;
                for ring in rings {
                    self.positions(ring, 4)?;
                }
            }
            "MultiPolygon" => {
                self.types.insert("MultiPolygon");
                let polygons = value["coordinates"]
                    .as_array()
                    .filter(|polygons| !polygons.is_empty())
                    .ok_or_else(|| invalid("MultiPolygon requires polygons"))?;
                for polygon in polygons {
                    let rings = polygon
                        .as_array()
                        .filter(|rings| !rings.is_empty())
                        .ok_or_else(|| invalid("MultiPolygon requires coordinate rings"))?;
                    for ring in rings {
                        self.positions(ring, 4)?;
                    }
                }
            }
            "GeometryCollection" => {
                let geometries = value
                    .get("geometries")
                    .and_then(Value::as_array)
                    .filter(|geometries| !geometries.is_empty())
                    .ok_or_else(|| invalid("GeometryCollection requires geometries"))?;
                for geometry in geometries {
                    self.geometry(geometry, depth + 1)?;
                }
            }
            _ => return Err(invalid("unsupported GeoJSON geometry type")),
        }
        Ok(())
    }

    fn geometry_label(&self) -> &'static str {
        if self.types.len() == 1 {
            self.types.iter().next().copied().unwrap_or("Unknown")
        } else {
            "Mixed"
        }
    }
}

/// Bounded read-only interoperability with RustGIS and other RFC 7946 tools.
/// Supports the geometry types consumed by MAP and rejects unsupported data.
pub fn inspect_geojson(bytes: &[u8]) -> io::Result<GeoJsonSummary> {
    if bytes.is_empty() || bytes.len() > MAX_GEOJSON_BYTES {
        return Err(invalid("GeoJSON must be 1 byte to 1 MiB"));
    }
    let root: Value =
        serde_json::from_slice(bytes).map_err(|_| invalid("invalid GeoJSON JSON document"))?;
    if root.get("crs").is_some() {
        return Err(invalid(
            "GeoJSON must use WGS84 longitude/latitude coordinates without a legacy CRS field",
        ));
    }
    let mut stats = GeometryStats::new();
    let features = match root.get("type").and_then(Value::as_str) {
        Some(
            "Point" | "MultiPoint" | "LineString" | "MultiLineString" | "Polygon" | "MultiPolygon"
            | "GeometryCollection",
        ) => {
            stats.geometry(&root, 0)?;
            1
        }
        Some("Feature") => {
            stats.geometry(
                root.get("geometry")
                    .filter(|geometry| !geometry.is_null())
                    .ok_or_else(|| invalid("GeoJSON Feature requires non-null geometry"))?,
                0,
            )?;
            1
        }
        Some("FeatureCollection") => {
            let features = root
                .get("features")
                .and_then(Value::as_array)
                .ok_or_else(|| invalid("FeatureCollection requires a features array"))?;
            if features.is_empty() || features.len() > MAX_FEATURES {
                return Err(invalid("FeatureCollection must contain 1..1000 Features"));
            }
            for feature in features {
                if feature.get("type").and_then(Value::as_str) != Some("Feature") {
                    return Err(invalid("FeatureCollection entries must be Features"));
                }
                stats.geometry(
                    feature
                        .get("geometry")
                        .filter(|geometry| !geometry.is_null())
                        .ok_or_else(|| invalid("GeoJSON Feature requires non-null geometry"))?,
                    0,
                )?;
            }
            features.len()
        }
        _ => {
            return Err(invalid(
                "expected GeoJSON geometry, Feature or FeatureCollection",
            ))
        }
    };
    if stats.vertices == 0 {
        return Err(invalid("GeoJSON contains no coordinate positions"));
    }
    Ok(GeoJsonSummary {
        format: "GeoJSON (RFC 7946)",
        geometry: stats.geometry_label(),
        features,
        bbox: [stats.west, stats.south, stats.east, stats.north],
        coordinate_order: "longitude,latitude",
        antimeridian_note: "Raw min/max bbox; crossing the antimeridian may create a wide extent",
        network_used: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geojson_coordinates_are_longitude_then_latitude() {
        let point = Point::new(42.0, -70.75).unwrap();
        let json = waypoint("Scout Camp", point).unwrap();
        assert_eq!(json["geometry"]["coordinates"], json!([-70.75, 42.0]));
        assert_eq!(json["properties"]["name"], "Scout Camp");
        let summary = inspect_geojson(json.to_string().as_bytes()).unwrap();
        assert_eq!(summary.features, 1);
        assert_eq!(summary.bbox, [-70.75, 42.0, -70.75, 42.0]);
        assert!(!summary.network_used);
    }

    #[test]
    fn distance_is_bounded_and_antimeridian_safe() {
        let a = Point::new(0.0, 0.0).unwrap();
        let b = Point::new(0.0, 1.0).unwrap();
        assert!((distance_meters(a, b) - 111_195.08).abs() < 1.0);
        assert_eq!(distance_meters(a, a), 0.0);
        let near_dateline = distance_meters(
            Point::new(0.0, 179.9).unwrap(),
            Point::new(0.0, -179.9).unwrap(),
        );
        assert!((near_dateline - 22_239.0).abs() < 5.0);
        let antipodes = distance_meters(a, Point::new(0.0, 180.0).unwrap());
        assert!(antipodes.is_finite() && antipodes > 20_000_000.0);
    }

    #[test]
    fn invalid_inputs_fail_closed_without_storage() {
        for (lat, lon) in [
            (91.0, 0.0),
            (-91.0, 0.0),
            (0.0, -181.0),
            (0.0, 181.0),
            (f64::NAN, 0.0),
            (f64::INFINITY, 0.0),
        ] {
            assert!(Point::new(lat, lon).is_err());
        }
        for name in ["", " bad", "bad ", "a\nb"] {
            assert!(waypoint(name, Point::new(0.0, 0.0).unwrap()).is_err());
        }
        assert!(waypoint(&"x".repeat(65), Point::new(0.0, 0.0).unwrap()).is_err());
        for invalid in [
            r#"{}"#,
            r#"{"type":"FeatureCollection","features":[]}"#,
            r#"{"type":"Point","coordinates":[0,0],"crs":{"type":"name","properties":{"name":"EPSG:3857"}}}"#,
            r#"{"type":"Point","coordinates":[0]}"#,
            r#"{"type":"Point","coordinates":[0,91]}"#,
            r#"{"type":"Point","coordinates":[null,1]}"#,
            r#"{"type":"Feature","geometry":null,"properties":{}}"#,
            r#"{"type":"FeatureCollection","features":[{"type":"Point","coordinates":[1,2]}]}"#,
            r#"{"type":"LineString","coordinates":[[0,0]]}"#,
            r#"{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1]]] }"#,
            r#"{"type":"GeometryCollection","geometries":[]}"#,
        ] {
            assert!(inspect_geojson(invalid.as_bytes()).is_err(), "{invalid}");
        }
        assert!(inspect_geojson(&vec![b' '; MAX_GEOJSON_BYTES + 1]).is_err());
        let oversized =
            json!({"type":"LineString","coordinates":vec![[0.0,0.0]; MAX_VERTICES + 1]});
        assert!(inspect_geojson(oversized.to_string().as_bytes()).is_err());
    }

    #[test]
    fn inspector_accepts_map_geometry_families_and_counts_vertices() {
        let cases = [
            (
                json!({"type":"Point","coordinates":[1,2]}),
                "Point",
                1,
                [1.0, 2.0, 1.0, 2.0],
            ),
            (
                json!({"type":"MultiPoint","coordinates":[[1,2],[3,4]]}),
                "MultiPoint",
                2,
                [1.0, 2.0, 3.0, 4.0],
            ),
            (
                json!({"type":"LineString","coordinates":[[-1,0],[1,2]]}),
                "LineString",
                2,
                [-1.0, 0.0, 1.0, 2.0],
            ),
            (
                json!({"type":"MultiLineString","coordinates":[[[0,0],[1,1]],[[2,2],[3,3]]]}),
                "MultiLineString",
                4,
                [0.0, 0.0, 3.0, 3.0],
            ),
            (
                json!({"type":"Polygon","coordinates":[[[0,0],[2,0],[2,2],[0,0]]]}),
                "Polygon",
                4,
                [0.0, 0.0, 2.0, 2.0],
            ),
            (
                json!({"type":"MultiPolygon","coordinates":[[[[0,0],[1,0],[1,1],[0,0]]],[[[2,2],[3,2],[3,3],[2,2]]]]}),
                "MultiPolygon",
                8,
                [0.0, 0.0, 3.0, 3.0],
            ),
        ];
        for (geometry, expected_type, vertices, bbox) in cases {
            let summary = inspect_geojson(geometry.to_string().as_bytes()).unwrap();
            assert_eq!(summary.geometry, expected_type);
            assert_eq!(summary.features, 1);
            assert_eq!(summary.bbox, bbox);
            assert!(!summary.network_used);
            let mut stats = GeometryStats::new();
            stats.geometry(&geometry, 0).unwrap();
            assert_eq!(stats.vertices, vertices);
        }
    }

    #[test]
    fn geometry_collections_and_feature_collections_preserve_feature_counts() {
        let mixed_feature = json!({"type":"Feature","properties":{},"geometry":{
            "type":"GeometryCollection","geometries":[
                {"type":"Point","coordinates":[-81,37]},
                {"type":"LineString","coordinates":[[-82,36],[-80,38]]}
            ]
        }});
        let summary = inspect_geojson(mixed_feature.to_string().as_bytes()).unwrap();
        assert_eq!(summary.geometry, "Mixed");
        assert_eq!(summary.features, 1);
        assert_eq!(summary.bbox, [-82.0, 36.0, -80.0, 38.0]);
        let mut stats = GeometryStats::new();
        stats.geometry(&mixed_feature["geometry"], 0).unwrap();
        assert_eq!(stats.geometries, 3);
        assert_eq!(stats.vertices, 3);

        let collection = json!({"type":"FeatureCollection","features":[
            waypoint("Point", Point::new(37.0,-81.0).unwrap()).unwrap(),
            {"type":"Feature","properties":{},"geometry":{"type":"Polygon","coordinates":[[[-82,36],[-80,36],[-80,38],[-82,36]]]}}
        ]});
        let summary = inspect_geojson(collection.to_string().as_bytes()).unwrap();
        assert_eq!(summary.geometry, "Mixed");
        assert_eq!(summary.features, 2);
        assert_eq!(summary.bbox, [-82.0, 36.0, -80.0, 38.0]);
    }

    #[test]
    fn feature_collection_counts_and_bounds_are_reproducible() {
        let payload = json!({"type":"FeatureCollection","features":[
            waypoint("East", Point::new(40.0,-70.0).unwrap()).unwrap(),
            waypoint("West", Point::new(41.0,-72.0).unwrap()).unwrap()
        ]});
        let summary = inspect_geojson(payload.to_string().as_bytes()).unwrap();
        assert_eq!(summary.features, 2);
        assert_eq!(summary.bbox, [-72.0, 40.0, -70.0, 41.0]);
        assert_eq!(summary.coordinate_order, "longitude,latitude");
        let json = serde_json::to_value(summary).unwrap();
        let keys: Vec<_> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            [
                "antimeridian_note",
                "bbox",
                "coordinate_order",
                "features",
                "format",
                "geometry",
                "network_used"
            ]
        );
    }

    #[test]
    fn geojson_export_escapes_untrusted_waypoint_text() {
        let p = Point::new(10.0, 20.0).unwrap();
        let feature = waypoint(r#"<script>&"hello"#, p).unwrap();
        let encoded = feature.to_string();
        let reparsed: Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(reparsed["properties"]["name"], r#"<script>&"hello"#);
        assert_eq!(inspect_geojson(encoded.as_bytes()).unwrap().features, 1);
    }
}

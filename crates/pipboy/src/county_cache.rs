//! Allowlisted, operator-triggered Census county archives for the local map.
//! Never accepts a URL or raw GPS position. Only a validated 5-digit GEOID and
//! a source ID from SOURCE_LIST can reach a fixed HTTPS Census ZIP path.
use pipboy_core::process_safety::isolated_command;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};

pub const MAX_ARCHIVE: u64 = 12 * 1024 * 1024;
const MAX_BOUNDARY: u64 = 1024 * 1024;
const MAX_BOUNDARY_VERTICES: usize = 12_000;
const MAX_BOUNDARY_RINGS: usize = 256;
const MAX_COUNTIES: usize = 4;
const VINTAGE: &str = "2025";
const SOURCE_LIST: [(&str, &str, &str); 4] = [
    ("tiger-roads", "ROADS", "roads"),
    ("tiger-areawater", "AREAWATER", "areawater"),
    ("tiger-linearwater", "LINEARWATER", "linearwater"),
    ("tiger-pointlm", "POINTLM", "pointlm"),
];

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn source(name: &str) -> io::Result<(&'static str, &'static str)> {
    SOURCE_LIST
        .iter()
        .find(|(id, _, _)| *id == name)
        .map(|(_, path, filename)| (*path, *filename))
        .ok_or_else(|| invalid("Unknown county source; only Census sources are supported"))
}

fn geoid_valid(geoid: &str) -> bool {
    geoid.len() == 5
        && geoid.bytes().all(|b| b.is_ascii_digit())
        && &geoid[..2] != "00"
        && &geoid[2..] != "000"
}

pub fn sources() -> Value {
    json!({
        "schema": 1,
        "provider": "U.S. Census Bureau TIGER/Line",
        "vintage": VINTAGE,
        "source_crs": "GCS_NAD83",
        "county_boundaries": "TIGERweb / ACS 2025 (separate county lookup)",
        "sources": [
            {"id":"tiger-roads","layer":"roads","kind":"line","origin":"www2.census.gov",
             "description":"County-wide TIGER All Roads centerlines"},
            {"id":"tiger-areawater","layer":"water","kind":"polygon","origin":"www2.census.gov",
             "description":"County-wide TIGER areal water polygons; not a surveyed shoreline"},
            {"id":"tiger-linearwater","layer":"waterways","kind":"line","origin":"www2.census.gov",
             "description":"County-wide TIGER linear rivers, streams and hydrography"},
            {"id":"tiger-pointlm","layer":"landmarks","kind":"point","origin":"www2.census.gov",
             "scope":"state","display_scope":"verified county polygon only",
             "description":"State point landmarks clipped to verified county geometry"}
        ],
        "limits": {"max_zip_bytes":MAX_ARCHIVE,"max_cached_counties":MAX_COUNTIES,
                   "position":"resolved county GEOID only","network":"explicit COUNTY action"}
    })
}

pub struct CountyCache {
    root: PathBuf,
}

static FETCH_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

impl CountyCache {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn name(geoid: &str, layer: &str) -> String {
        // Cache filenames remain county-isolated even when origin is state-based.
        format!("tl_{VINTAGE}_{geoid}_{layer}.zip")
    }

    fn remote_name(geoid: &str, layer: &str) -> String {
        // Census publishes POINTLM by state, not by county.
        if layer == "pointlm" {
            format!("tl_{VINTAGE}_{}_pointlm.zip", &geoid[..2])
        } else {
            Self::name(geoid, layer)
        }
    }

    fn path(&self, geoid: &str, layer: &str) -> PathBuf {
        self.root.join(Self::name(geoid, layer))
    }

    fn boundary_path(&self, geoid: &str) -> PathBuf {
        self.root
            .join(format!("boundary_{VINTAGE}_{geoid}.geojson"))
    }

    // Fixed Census TIGERweb layer 37 is ACS 2025 county geometry.
    // The server downloads by GEOID, not device/user coordinates.
    fn boundary_url(geoid: &str) -> String {
        format!(
            "https://tigerweb.geo.census.gov/arcgis/rest/services/TIGERweb/State_County/MapServer/37/query?f=geojson&where=GEOID%3D%27{geoid}%27&outFields=GEOID%2CNAME&returnGeometry=true&outSR=4326&maxAllowableOffset=0.00015"
        )
    }

    fn boundary_raw(path: &Path) -> io::Result<Vec<u8>> {
        let size = fs::metadata(path)?.len();
        if !(2..=MAX_BOUNDARY).contains(&size) {
            return Err(invalid("County boundary exceeds 1 MiB or is empty"));
        }
        let mut bytes = Vec::with_capacity(size as usize);
        File::open(path)?
            .take(MAX_BOUNDARY + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 != size {
            return Err(invalid("County boundary changed during read"));
        }
        Ok(bytes)
    }

    // A cached file must pass the SAME structural checks as a fresh
    // source response. Merely having a file or GEOID does not verify it.
    fn parse_boundary(geoid: &str, raw: &[u8], cache_hit: bool) -> io::Result<Value> {
        if !geoid_valid(geoid) || raw.len() as u64 > MAX_BOUNDARY {
            return Err(invalid("Invalid county or boundary size"));
        }
        let payload: Value =
            serde_json::from_slice(raw).map_err(|_| invalid("Census boundary JSON is invalid"))?;
        if payload.get("type").and_then(Value::as_str) != Some("FeatureCollection") {
            return Err(invalid(
                "Census boundary must be a GeoJSON FeatureCollection",
            ));
        }
        let features = payload
            .get("features")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("Missing Census county features"))?;
        if features.len() != 1 {
            return Err(invalid("Census county response must contain one feature"));
        }
        let feature = &features[0];
        if feature.pointer("/properties/GEOID").and_then(Value::as_str) != Some(geoid) {
            return Err(invalid("Census county GEOID does not match selection"));
        }
        let name = feature
            .pointer("/properties/NAME")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("Census county name is missing"))?;
        if name.is_empty() || name.len() > 100 || name.chars().any(char::is_control) {
            return Err(invalid("Census county name is invalid"));
        }
        let geometry = feature
            .get("geometry")
            .ok_or_else(|| invalid("Missing Census county geometry"))?;
        let parts = geometry
            .get("coordinates")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("Missing Census county coordinates"))?;
        // Both Polygon and MultiPolygon become a flat ring list; even-odd
        // parity in the viewer preserves holes for landmark clipping.
        let polygons: Vec<&Value> = match geometry.get("type").and_then(Value::as_str) {
            Some("Polygon") => vec![&geometry["coordinates"]],
            Some("MultiPolygon") => parts.iter().collect(),
            _ => return Err(invalid("Census county is not Polygon or MultiPolygon")),
        };
        let mut rings: Vec<Vec<[f64; 2]>> = Vec::new();
        let mut vertices = 0;
        for polygon in polygons {
            let paths = polygon
                .as_array()
                .ok_or_else(|| invalid("Invalid county polygon parts"))?;
            for path in paths {
                let points = path
                    .as_array()
                    .ok_or_else(|| invalid("Invalid county ring"))?;
                vertices += points.len();
                if points.len() < 4
                    || vertices > MAX_BOUNDARY_VERTICES
                    || rings.len() >= MAX_BOUNDARY_RINGS
                {
                    return Err(invalid("County boundary exceeds geometry bounds"));
                }
                let mut ring = Vec::with_capacity(points.len());
                for point in points {
                    let pair = point
                        .as_array()
                        .ok_or_else(|| invalid("Invalid county vertex"))?;
                    if pair.len() < 2 {
                        return Err(invalid("County vertex is not longitude/latitude"));
                    }
                    let lon = pair[0]
                        .as_f64()
                        .ok_or_else(|| invalid("Invalid county longitude"))?;
                    let lat = pair[1]
                        .as_f64()
                        .ok_or_else(|| invalid("Invalid county latitude"))?;
                    if !lon.is_finite() || !lat.is_finite() || lon.abs() > 180.0 || lat.abs() > 90.0
                    {
                        return Err(invalid("County coordinates outside geographic range"));
                    }
                    ring.push([lon, lat]);
                }
                if ring.first() != ring.last() {
                    return Err(invalid("County boundary ring must be closed"));
                }
                rings.push(ring);
            }
        }
        if rings.is_empty() {
            return Err(invalid("County boundary contains no rings"));
        }
        Ok(json!({
            "schema":1, "geoid":geoid, "name":name,
            "paths":rings,
            "source":"U.S. Census TIGERweb State_County ACS 2025 / January 1 2025",
            "vintage":"2025-01-01",
            "cache_hit":cache_hit,
            "network_used":!cache_hit
        }))
    }

    fn read_zip(path: &Path) -> io::Result<Vec<u8>> {
        let size = fs::metadata(path)?.len();
        if !(4..=MAX_ARCHIVE).contains(&size) {
            return Err(invalid("Cached county archive exceeds bound or is empty"));
        }
        let mut bytes = Vec::with_capacity(size as usize);
        File::open(path)?
            .take(MAX_ARCHIVE + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 != size || !bytes.starts_with(b"PK\x03\x04") {
            return Err(invalid("Cached county ZIP invalid; original preserved"));
        }
        Ok(bytes)
    }

    fn ensure_root(&self) -> io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&self.root)?;
        }
        #[cfg(not(unix))]
        fs::create_dir_all(&self.root)?;
        Ok(())
    }

    fn limit_new_county(&self, geoid: &str) -> io::Result<()> {
        if !self.root.exists() {
            return Ok(());
        }
        let mut counties = BTreeSet::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(&format!("tl_{VINTAGE}_")) && name.ends_with(".zip") {
                let split: Vec<_> = name.split('_').collect();
                if split.len() == 4 && geoid_valid(split[2]) {
                    counties.insert(split[2].to_owned());
                }
            } else if let Some(id) = name
                .strip_prefix(&format!("boundary_{VINTAGE}_"))
                .and_then(|n| n.strip_suffix(".geojson"))
            {
                if geoid_valid(id) {
                    counties.insert(id.to_owned());
                }
            }
        }
        if counties.len() >= MAX_COUNTIES && !counties.contains(geoid) {
            return Err(invalid(
                "County cache limit reached; choose an existing county",
            ));
        }
        Ok(())
    }

    fn remember(&self, geoid: &str) -> io::Result<()> {
        let mut tmp = tempfile::Builder::new()
            .prefix(".county-selection-")
            .tempfile_in(&self.root)?;
        tmp.write_all(geoid.as_bytes())?;
        tmp.as_file().sync_all()?;
        tmp.persist(self.root.join("active_geoid"))
            .map_err(|error| error.error)?;
        Ok(())
    }

    pub fn active(&self) -> io::Result<Value> {
        let geoid = match fs::read_to_string(self.root.join("active_geoid")) {
            Ok(value) if geoid_valid(&value) => Some(value),
            Ok(_) => None,
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        let mut cached = Vec::new();
        if let Some(ref id) = geoid {
            for (source_id, _, layer) in SOURCE_LIST {
                if let Ok(file) = fs::metadata(self.path(id, layer)) {
                    if (4..=MAX_ARCHIVE).contains(&file.len()) {
                        cached.push(source_id);
                    }
                }
            }
        }
        let boundary_cached = geoid.as_ref().is_some_and(|id| {
            Self::boundary_raw(&self.boundary_path(id))
                .and_then(|raw| Self::parse_boundary(id, &raw, true))
                .is_ok()
        });
        Ok(json!({
            "schema":1,"active_geoid":geoid,"cached_sources":cached,
            "boundary_cached":boundary_cached,
            "vintage":VINTAGE,"network_used":false
        }))
    }

    pub fn boundary(&self, geoid: &str, allow_network: bool) -> io::Result<Value> {
        if !geoid_valid(geoid) {
            return Err(invalid(
                "GEOID must be a valid five-digit county identifier",
            ));
        }
        let path = self.boundary_path(geoid);
        match Self::boundary_raw(&path) {
            Ok(raw) => {
                let boundary = Self::parse_boundary(geoid, &raw, true)?;
                if allow_network {
                    self.ensure_root()?;
                    self.remember(geoid)?;
                }
                return Ok(boundary);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        if !allow_network {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "County boundary not cached",
            ));
        }
        let lock = FETCH_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = lock.try_lock().map_err(|_| {
            io::Error::new(
                io::ErrorKind::WouldBlock,
                "Another county download is in progress",
            )
        })?;
        // A second process/request may have filled the cache while waiting.
        match Self::boundary_raw(&path) {
            Ok(raw) => {
                let boundary = Self::parse_boundary(geoid, &raw, true)?;
                self.remember(geoid)?;
                return Ok(boundary);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        self.limit_new_county(geoid)?;
        self.ensure_root()?;
        let tmp = tempfile::Builder::new()
            .prefix(".county-boundary-")
            .tempfile_in(&self.root)?;
        // No supplied URL, no GPS coordinates, redirects, insecure TLS,
        // shell invocation, arbitrary domains, or retries.
        let cmd = if cfg!(windows) { "curl.exe" } else { "curl" };
        let success = isolated_command(cmd)
            .args([
                "--fail",
                "--silent",
                "--show-error",
                "--proto",
                "=https",
                "--max-redirs",
                "0",
                "--connect-timeout",
                "8",
                "--max-time",
                "25",
                "--max-filesize",
                "1048576",
                "--retry",
                "0",
                "--output",
            ])
            .arg(tmp.path())
            .arg(Self::boundary_url(geoid))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !success.success() {
            return Err(io::Error::other(
                "Census boundary unavailable; existing cache preserved",
            ));
        }
        let raw = Self::boundary_raw(tmp.path())?;
        let boundary = Self::parse_boundary(geoid, &raw, false)?;
        tmp.as_file().sync_all()?;
        tmp.persist(&path).map_err(|error| error.error)?;
        self.remember(geoid)?;
        Ok(boundary)
    }

    pub fn fetch(&self, geoid: &str, source_id: &str, allow_network: bool) -> io::Result<Vec<u8>> {
        if !geoid_valid(geoid) {
            return Err(invalid(
                "GEOID must be a valid five-digit county identifier",
            ));
        }
        let (folder, layer) = source(source_id)?;
        let path = self.path(geoid, layer);
        // Cached immutable ZIPs are safe to read without the global download
        // gate. Atomic persist makes them visible only after full validation.
        // Previously even offline reads could fail WouldBlock when unrelated
        // county jobs/tests were running in parallel (notably on Windows).
        match Self::read_zip(&path) {
            Ok(bytes) => {
                if allow_network {
                    self.ensure_root()?;
                    self.remember(geoid)?;
                }
                return Ok(bytes);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        if !allow_network {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "County archive is not cached",
            ));
        }
        let lock = FETCH_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = lock.try_lock().map_err(|_| {
            io::Error::new(
                io::ErrorKind::WouldBlock,
                "Another county download is in progress",
            )
        })?;
        // Recheck after acquiring the exclusive download gate: another
        // caller may have completed this archive between the two reads.
        match Self::read_zip(&path) {
            Ok(bytes) => {
                self.remember(geoid)?;
                return Ok(bytes);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        self.limit_new_county(geoid)?;
        self.ensure_root()?;
        let url = format!(
            "https://www2.census.gov/geo/tiger/TIGER{VINTAGE}/{folder}/{}",
            Self::remote_name(geoid, layer)
        );
        let tmp = tempfile::Builder::new()
            .prefix(".county-download-")
            .tempfile_in(&self.root)?;
        // argv is always fixed by source(), geoid_valid(); no shell, arbitrary
        // target, redirects, retry loop, credentials, or custom user URLs.
        let cmd = if cfg!(windows) { "curl.exe" } else { "curl" };
        let ok = isolated_command(cmd)
            .args([
                "--fail",
                "--silent",
                "--show-error",
                "--proto",
                "=https",
                "--max-redirs",
                "0",
                "--connect-timeout",
                "8",
                "--max-time",
                "40",
                "--max-filesize",
                "12582912",
                "--retry",
                "0",
                "--output",
            ])
            .arg(tmp.path())
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !ok.success() {
            return Err(io::Error::other(
                "Census archive download failed; existing cache preserved",
            ));
        }
        let bytes = Self::read_zip(tmp.path())?;
        tmp.as_file().sync_all()?;
        tmp.persist(&path).map_err(|error| error.error)?;
        self.remember(geoid)?;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn county_fixture(geoid: &str) -> Vec<u8> {
        json!({
            "type":"FeatureCollection",
            "features":[{
                "type":"Feature",
                "properties":{"GEOID":geoid,"NAME":"Test County"},
                "geometry":{"type":"Polygon","coordinates":[
                    [[-71.2,42.3],[-71.0,42.3],[-71.0,42.5],[-71.2,42.3]],
                    [[-71.12,42.34],[-71.10,42.34],[-71.10,42.36],[-71.12,42.34]]
                ]}
            }]
        })
        .to_string()
        .into_bytes()
    }

    #[test]
    fn boundary_parser_rejects_unmatched_or_malformed_counties() {
        let value = CountyCache::parse_boundary("25025", &county_fixture("25025"), false).unwrap();
        assert_eq!(value["geoid"], "25025");
        assert_eq!(value["paths"].as_array().unwrap().len(), 2);
        assert_eq!(value["cache_hit"], false);
        assert_eq!(value["network_used"], true);
        assert!(CountyCache::parse_boundary("25023", &county_fixture("25025"), false).is_err());
        let mut invalid: Value = serde_json::from_slice(&county_fixture("25025")).unwrap();
        invalid["features"][0]["geometry"]["coordinates"][0][1][0] = json!(181.0);
        assert!(
            CountyCache::parse_boundary("25025", invalid.to_string().as_bytes(), false).is_err()
        );
        invalid["features"][0]["geometry"]["coordinates"][0][1][0] = json!(-71.0);
        invalid["features"][0]["geometry"]["coordinates"][0][3][0] = json!(-71.1);
        assert!(
            CountyCache::parse_boundary("25025", invalid.to_string().as_bytes(), false).is_err()
        );
        assert!(CountyCache::parse_boundary("25025", b"{}", false).is_err());
        assert!(CountyCache::parse_boundary("25025", b"{", false).is_err());
    }

    #[test]
    fn cached_boundary_restores_offline_without_location_or_side_effects() {
        let root = tempfile::tempdir().unwrap();
        let cache = CountyCache::new(root.path().to_path_buf());
        assert_eq!(
            cache.boundary("25025", false).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        assert!(cache.boundary("../25", false).is_err());
        assert!(cache.boundary("00000", true).is_err());
        assert!(!cache.boundary_path("25025").exists());
        fs::write(cache.boundary_path("25025"), county_fixture("25025")).unwrap();
        cache.remember("25025").unwrap();
        let data = cache.boundary("25025", false).unwrap();
        assert_eq!(data["geoid"], "25025");
        assert_eq!(data["cache_hit"], true);
        assert_eq!(data["network_used"], false);
        assert_eq!(cache.active().unwrap()["boundary_cached"], true);
        assert_eq!(cache.active().unwrap()["cached_sources"], json!([]));
        let original = fs::read(cache.boundary_path("25025")).unwrap();
        fs::write(cache.boundary_path("25025"), b"{\"features\":[]}").unwrap();
        assert!(cache.boundary("25025", false).is_err());
        assert!(cache.boundary("25025", true).is_err());
        assert_eq!(cache.active().unwrap()["boundary_cached"], false);
        assert_ne!(fs::read(cache.boundary_path("25025")).unwrap(), original);
    }

    #[test]
    fn boundary_urls_and_county_budget_are_fixed() {
        assert!(CountyCache::boundary_url("25025").contains("GEOID%3D%2725025%27"));
        assert!(CountyCache::boundary_url("25025").starts_with(
            "https://tigerweb.geo.census.gov/arcgis/rest/services/TIGERweb/State_County/"
        ));
        let tmp = tempfile::tempdir().unwrap();
        let cache = CountyCache::new(tmp.path().to_path_buf());
        for id in ["25001", "25003", "25005", "25007"] {
            fs::write(cache.boundary_path(id), county_fixture(id)).unwrap();
        }
        assert!(cache.limit_new_county("25009").is_err());
        assert!(cache.limit_new_county("25003").is_ok());
    }

    #[test]
    fn source_and_geoid_restrictions_do_not_create_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let store = CountyCache::new(tmp.path().join("absent"));
        for value in ["", "250", "250250", "00000", "25000", "../25", "25\n25"] {
            assert!(store.fetch(value, "tiger-roads", true).is_err());
        }
        assert!(store.fetch("25025", "https://evil.invalid", true).is_err());
        assert!(store.fetch("25025", "tiger-areawater", false).is_err());
        assert!(!store.root.exists());
    }

    #[test]
    fn cached_archives_are_offline_and_active_county_is_bounded() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = CountyCache::new(tmp.path().to_path_buf());
        let path = cache.path("25025", "roads");
        fs::write(&path, b"PK\x03\x04test").unwrap();
        assert_eq!(
            cache.fetch("25025", "tiger-roads", false).unwrap(),
            b"PK\x03\x04test"
        );
        assert!(cache.fetch("25025", "tiger-areawater", false).is_err());
        cache.remember("25025").unwrap();
        let active = cache.active().unwrap();
        assert_eq!(active["active_geoid"], "25025");
        assert_eq!(active["cached_sources"], json!(["tiger-roads"]));
        assert_eq!(active["network_used"], false);
    }

    #[test]
    fn cache_reads_do_not_contend_with_unrelated_downloads() {
        let temp = tempfile::tempdir().unwrap();
        let cache = CountyCache::new(temp.path().to_path_buf());
        fs::write(cache.path("25025", "roads"), b"PK\x03\x04local").unwrap();
        let gate = FETCH_LOCK.get_or_init(|| Mutex::new(()));
        let _held = gate.lock().unwrap();
        assert_eq!(
            cache.fetch("25025", "tiger-roads", false).unwrap(),
            b"PK\x03\x04local"
        );
        let missing = cache.fetch("25025", "tiger-areawater", false).unwrap_err();
        assert_eq!(missing.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn corrupt_archive_is_never_overwritten() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = CountyCache::new(tmp.path().to_path_buf());
        let p = cache.path("25025", "roads");
        fs::write(&p, b"broken").unwrap();
        assert!(cache.fetch("25025", "tiger-roads", true).is_err());
        assert_eq!(fs::read(&p).unwrap(), b"broken");
    }

    #[test]
    fn source_catalog_is_a_strict_pair() {
        let data = sources();
        assert_eq!(data["sources"].as_array().unwrap().len(), 4);
        assert_eq!(source("tiger-roads").unwrap().1, "roads");
        assert_eq!(source("tiger-areawater").unwrap().1, "areawater");
        assert_eq!(source("tiger-linearwater").unwrap().1, "linearwater");
        assert_eq!(source("tiger-pointlm").unwrap().1, "pointlm");
        assert_eq!(
            CountyCache::remote_name("25025", "pointlm"),
            "tl_2025_25_pointlm.zip"
        );
        assert_eq!(
            CountyCache::name("25025", "pointlm"),
            "tl_2025_25025_pointlm.zip"
        );
    }
}

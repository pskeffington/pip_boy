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

    fn validate_archive(bytes: &[u8], geoid: &str, layer: &str) -> io::Result<()> {
        validate_tiger_archive(bytes, geoid, layer)
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
                if let Ok(bytes) = Self::read_zip(&self.path(id, layer)) {
                    if Self::validate_archive(&bytes, id, layer).is_ok() {
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
                Self::validate_archive(&bytes, geoid, layer)?;
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
                Self::validate_archive(&bytes, geoid, layer)?;
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
        Self::validate_archive(&bytes, geoid, layer)?;
        tmp.as_file().sync_all()?;
        tmp.persist(&path).map_err(|error| error.error)?;
        Ok(bytes)
    }

    /// Make a county the cold-start selection only after the map client has
    /// parsed at least one cached base-layer archive successfully.
    pub fn activate(&self, geoid: &str) -> io::Result<Value> {
        if !geoid_valid(geoid) {
            return Err(invalid(
                "GEOID must be a valid five-digit county identifier",
            ));
        }
        let mut cached_sources = Vec::new();
        for (source_id, _, layer) in SOURCE_LIST {
            if source_id == "tiger-pointlm" {
                continue;
            }
            if Self::read_zip(&self.path(geoid, layer))
                .and_then(|bytes| Self::validate_archive(&bytes, geoid, layer))
                .is_ok()
            {
                cached_sources.push(source_id);
            }
        }
        if cached_sources.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "No cached base layer is available for activation",
            ));
        }
        self.ensure_root()?;
        self.remember(geoid)?;
        self.active()
    }
}

const MAX_UNPACKED_SHP: usize = 64 * 1024 * 1024;
const MAX_UNPACKED_DBF: usize = 24 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 24;
const MAX_SHAPE_RECORDS: u32 = 60_000;

struct ZipMember<'a> {
    method: u16,
    crc: u32,
    packed: &'a [u8],
    unpacked: usize,
}

fn archive_invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn slice_at(bytes: &[u8], offset: usize, len: usize) -> io::Result<&[u8]> {
    let end = offset
        .checked_add(len)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| archive_invalid("Census ZIP member is truncated"))?;
    Ok(&bytes[offset..end])
}

fn zip_u16(bytes: &[u8], offset: usize) -> io::Result<u16> {
    Ok(u16::from_le_bytes(
        slice_at(bytes, offset, 2)?
            .try_into()
            .expect("length checked"),
    ))
}

fn zip_u32(bytes: &[u8], offset: usize) -> io::Result<u32> {
    Ok(u32::from_le_bytes(
        slice_at(bytes, offset, 4)?
            .try_into()
            .expect("length checked"),
    ))
}

fn census_member_limit(extension: &str) -> Option<usize> {
    match extension {
        "shp" => Some(MAX_UNPACKED_SHP),
        "dbf" => Some(MAX_UNPACKED_DBF),
        _ => None,
    }
}

fn zip_crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

fn unpack_member(member: &ZipMember<'_>) -> io::Result<Vec<u8>> {
    let data = match member.method {
        0 if member.packed.len() == member.unpacked => member.packed.to_vec(),
        0 => return Err(archive_invalid("Census ZIP stored size mismatch")),
        8 => miniz_oxide::inflate::decompress_to_vec_with_limit(member.packed, member.unpacked)
            .map_err(|_| archive_invalid("Census ZIP deflate data is invalid"))?,
        _ => return Err(archive_invalid("Unsupported Census ZIP compression method")),
    };
    if data.len() != member.unpacked || zip_crc32(&data) != member.crc {
        return Err(archive_invalid("Census ZIP CRC or expanded size mismatch"));
    }
    Ok(data)
}

fn validate_shape_file(bytes: &[u8], layer: &str) -> io::Result<u32> {
    if bytes.len() < 100 || !bytes.len().is_multiple_of(2) {
        return Err(archive_invalid("Census SHP header or size is invalid"));
    }
    let file_code = u32::from_be_bytes(bytes[0..4].try_into().expect("header checked"));
    let file_words = u32::from_be_bytes(bytes[24..28].try_into().expect("header checked"));
    let version = u32::from_le_bytes(bytes[28..32].try_into().expect("header checked"));
    let shape_type = u32::from_le_bytes(bytes[32..36].try_into().expect("header checked"));
    let expected_type = match layer {
        "roads" | "linearwater" => 3,
        "areawater" => 5,
        "pointlm" => 1,
        _ => return Err(archive_invalid("Unknown Census geometry layer")),
    };
    if file_code != 9994
        || usize::try_from(file_words)
            .ok()
            .and_then(|words| words.checked_mul(2))
            != Some(bytes.len())
        || version != 1000
        || shape_type != expected_type
    {
        return Err(archive_invalid(
            "Census SHP header does not match its layer",
        ));
    }
    let mut offset = 100usize;
    let mut records = 0u32;
    while offset < bytes.len() {
        let header = slice_at(bytes, offset, 8)?;
        let record_number = u32::from_be_bytes(header[0..4].try_into().expect("header checked"));
        let content_words = u32::from_be_bytes(header[4..8].try_into().expect("header checked"));
        let content_len = usize::try_from(content_words)
            .ok()
            .and_then(|words| words.checked_mul(2))
            .filter(|len| *len >= 4)
            .ok_or_else(|| archive_invalid("Census SHP record length is invalid"))?;
        let content_start = offset + 8;
        let content = slice_at(bytes, content_start, content_len)?;
        let record_type = u32::from_le_bytes(content[0..4].try_into().expect("record checked"));
        records = records
            .checked_add(1)
            .filter(|count| *count <= MAX_SHAPE_RECORDS)
            .ok_or_else(|| archive_invalid("Census SHP has too many records"))?;
        if record_number != records || record_type != expected_type {
            return Err(archive_invalid(
                "Census SHP record sequence or type is invalid",
            ));
        }
        let minimum = if expected_type == 1 { 20 } else { 44 };
        if content_len < minimum {
            return Err(archive_invalid(
                "Census SHP record is shorter than its type",
            ));
        }
        offset = content_start
            .checked_add(content_len)
            .ok_or_else(|| archive_invalid("Census SHP record length overflow"))?;
    }
    if offset != bytes.len() {
        return Err(archive_invalid(
            "Census SHP records do not match file length",
        ));
    }
    Ok(records)
}

fn validate_dbf_file(bytes: &[u8]) -> io::Result<u32> {
    if bytes.len() < 65 {
        return Err(archive_invalid("Census DBF header is truncated"));
    }
    let records = u32::from_le_bytes(bytes[4..8].try_into().expect("header checked"));
    let header_len = usize::from(u16::from_le_bytes(
        bytes[8..10].try_into().expect("header checked"),
    ));
    let record_len = usize::from(u16::from_le_bytes(
        bytes[10..12].try_into().expect("header checked"),
    ));
    if records > MAX_SHAPE_RECORDS || header_len < 33 || header_len > bytes.len() || record_len < 1
    {
        return Err(archive_invalid("Census DBF dimensions exceed bounds"));
    }
    let data_end = header_len
        .checked_add(
            usize::try_from(records)
                .ok()
                .and_then(|count| count.checked_mul(record_len))
                .ok_or_else(|| archive_invalid("Census DBF dimensions overflow"))?,
        )
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| archive_invalid("Census DBF records exceed the file"))?;
    let mut offset = 32usize;
    let mut field_width = 1usize; // deletion marker
    let mut terminated = false;
    while offset < header_len {
        let marker = bytes[offset];
        if marker == 0x0d {
            terminated = true;
            break;
        }
        let descriptor = slice_at(bytes, offset, 32)?;
        let name = &descriptor[..11];
        if name.iter().all(|byte| *byte == 0)
            || !name
                .iter()
                .all(|byte| byte.is_ascii_graphic() || *byte == 0)
        {
            return Err(archive_invalid("Census DBF field name is invalid"));
        }
        field_width = field_width
            .checked_add(usize::from(descriptor[16]))
            .filter(|width| *width <= record_len)
            .ok_or_else(|| archive_invalid("Census DBF field widths exceed record"))?;
        offset += 32;
    }
    if !terminated || field_width > record_len || data_end > bytes.len() {
        return Err(archive_invalid("Census DBF field table is invalid"));
    }
    if bytes.len() > data_end && !(bytes.len() == data_end + 1 && bytes[data_end] == 0x1a) {
        return Err(archive_invalid("Census DBF has unexpected trailing data"));
    }
    Ok(records)
}

fn validate_tiger_archive(bytes: &[u8], geoid: &str, layer: &str) -> io::Result<()> {
    if !geoid_valid(geoid) || !(22..=MAX_ARCHIVE as usize).contains(&bytes.len()) {
        return Err(archive_invalid("Census ZIP size or county is invalid"));
    }
    let search_from = bytes.len().saturating_sub(65_557);
    let eocd = (search_from..=bytes.len() - 22)
        .rev()
        .find(|offset| bytes.get(*offset..*offset + 4) == Some(b"PK\x05\x06"))
        .ok_or_else(|| archive_invalid("Census ZIP directory is missing"))?;
    let disk = zip_u16(bytes, eocd + 4)?;
    let directory_disk = zip_u16(bytes, eocd + 6)?;
    let disk_count = usize::from(zip_u16(bytes, eocd + 8)?);
    let count = usize::from(zip_u16(bytes, eocd + 10)?);
    let directory_size = usize::try_from(zip_u32(bytes, eocd + 12)?)
        .map_err(|_| archive_invalid("Census ZIP directory is too large"))?;
    let directory_offset = usize::try_from(zip_u32(bytes, eocd + 16)?)
        .map_err(|_| archive_invalid("Census ZIP directory offset is invalid"))?;
    let comment_len = usize::from(zip_u16(bytes, eocd + 20)?);
    let directory_end = directory_offset
        .checked_add(directory_size)
        .ok_or_else(|| archive_invalid("Census ZIP directory length overflow"))?;
    if disk != 0
        || directory_disk != 0
        || count == 0
        || count > MAX_ARCHIVE_ENTRIES
        || disk_count != count
        || directory_end != eocd
        || eocd + 22 + comment_len != bytes.len()
    {
        return Err(archive_invalid(
            "Census ZIP must be a bounded single-volume archive",
        ));
    }

    let expected_state = if layer == "pointlm" {
        &geoid[..2]
    } else {
        geoid
    };
    let expected_stem = format!("tl_{VINTAGE}_{expected_state}_{layer}.");
    let mut offset = directory_offset;
    let mut seen = BTreeSet::new();
    let mut ranges = Vec::with_capacity(count);
    let mut shp = None;
    let mut dbf = None;
    for _ in 0..count {
        if zip_u32(bytes, offset)? != 0x0201_4b50 {
            return Err(archive_invalid(
                "Census ZIP central directory entry is invalid",
            ));
        }
        let flags = zip_u16(bytes, offset + 8)?;
        let method = zip_u16(bytes, offset + 10)?;
        let crc = zip_u32(bytes, offset + 16)?;
        let packed_len = usize::try_from(zip_u32(bytes, offset + 20)?)
            .map_err(|_| archive_invalid("Census ZIP member is too large"))?;
        let unpacked = usize::try_from(zip_u32(bytes, offset + 24)?)
            .map_err(|_| archive_invalid("Census ZIP member is too large"))?;
        let name_len = usize::from(zip_u16(bytes, offset + 28)?);
        let extra_len = usize::from(zip_u16(bytes, offset + 30)?);
        let comment_len = usize::from(zip_u16(bytes, offset + 32)?);
        let local_offset = usize::try_from(zip_u32(bytes, offset + 42)?)
            .map_err(|_| archive_invalid("Census ZIP local offset is invalid"))?;
        let entry_end = offset
            .checked_add(46)
            .and_then(|value| value.checked_add(name_len))
            .and_then(|value| value.checked_add(extra_len))
            .and_then(|value| value.checked_add(comment_len))
            .filter(|end| *end <= directory_end)
            .ok_or_else(|| archive_invalid("Census ZIP central directory is truncated"))?;
        let name = std::str::from_utf8(slice_at(bytes, offset + 46, name_len)?)
            .map_err(|_| archive_invalid("Census ZIP filename is not UTF-8"))?;
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || name.contains(':')
            || name.contains("..")
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
            || !seen.insert(name.to_ascii_lowercase())
            || flags & !0x080e != 0
            || !matches!(method, 0 | 8)
            || unpacked > MAX_UNPACKED_SHP
        {
            return Err(archive_invalid(
                "Census ZIP contains an unsafe or unsupported member",
            ));
        }
        let name_bytes = name.as_bytes();
        if zip_u32(bytes, local_offset)? != 0x0403_4b50 {
            return Err(archive_invalid("Census ZIP local member header is invalid"));
        }
        let local_flags = zip_u16(bytes, local_offset + 6)?;
        let local_method = zip_u16(bytes, local_offset + 8)?;
        let local_crc = zip_u32(bytes, local_offset + 14)?;
        let local_packed = zip_u32(bytes, local_offset + 18)?;
        let local_unpacked = zip_u32(bytes, local_offset + 22)?;
        let local_name_len = usize::from(zip_u16(bytes, local_offset + 26)?);
        let local_extra_len = usize::from(zip_u16(bytes, local_offset + 28)?);
        let local_name = slice_at(bytes, local_offset + 30, local_name_len)?;
        let data_start = local_offset
            .checked_add(30)
            .and_then(|value| value.checked_add(local_name_len))
            .and_then(|value| value.checked_add(local_extra_len))
            .ok_or_else(|| archive_invalid("Census ZIP member offset overflow"))?;
        let data_end = data_start
            .checked_add(packed_len)
            .filter(|end| *end <= directory_offset)
            .ok_or_else(|| archive_invalid("Census ZIP member data is truncated"))?;
        if local_name != name_bytes
            || local_flags != flags
            || local_method != method
            || (flags & 0x0008 == 0
                && (local_crc != crc
                    || usize::try_from(local_packed).ok() != Some(packed_len)
                    || usize::try_from(local_unpacked).ok() != Some(unpacked)))
        {
            return Err(archive_invalid(
                "Census ZIP local and central headers disagree",
            ));
        }
        ranges.push((local_offset, data_end));
        if let Some(extension) = name.strip_prefix(&expected_stem) {
            if let Some(member_limit) = census_member_limit(extension) {
                if unpacked > member_limit || (method == 0 && packed_len != unpacked) {
                    return Err(archive_invalid("Census SHP/DBF exceeds its size bound"));
                }
                let member = ZipMember {
                    method,
                    crc,
                    packed: slice_at(bytes, data_start, packed_len)?,
                    unpacked,
                };
                match extension {
                    "shp" if shp.is_none() => shp = Some(member),
                    "dbf" if dbf.is_none() => dbf = Some(member),
                    _ => return Err(archive_invalid("Census ZIP has duplicate data members")),
                }
            }
        }
        offset = entry_end;
    }
    if offset != directory_end {
        return Err(archive_invalid(
            "Census ZIP directory length does not match entries",
        ));
    }
    ranges.sort_unstable_by_key(|range| range.0);
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return Err(archive_invalid("Census ZIP members overlap"));
    }
    let shp = shp.ok_or_else(|| archive_invalid("Census ZIP has no matching SHP"))?;
    let dbf = dbf.ok_or_else(|| archive_invalid("Census ZIP has no matching DBF"))?;
    let shape_count = validate_shape_file(&unpack_member(&shp)?, layer)?;
    let dbf_count = validate_dbf_file(&unpack_member(&dbf)?)?;
    if shape_count != dbf_count {
        return Err(archive_invalid("Census SHP/DBF record counts do not match"));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn shape_file(layer: &str) -> Vec<u8> {
        let mut bytes = vec![0; 100];
        bytes[0..4].copy_from_slice(&9994u32.to_be_bytes());
        bytes[24..28].copy_from_slice(&50u32.to_be_bytes());
        bytes[28..32].copy_from_slice(&1000u32.to_le_bytes());
        let kind: u32 = match layer {
            "roads" | "linearwater" => 3,
            "areawater" => 5,
            "pointlm" => 1,
            _ => unreachable!(),
        };
        bytes[32..36].copy_from_slice(&kind.to_le_bytes());
        bytes
    }

    fn dbf_file() -> Vec<u8> {
        let mut bytes = vec![0; 65];
        bytes[0] = 3;
        bytes[8..10].copy_from_slice(&65u16.to_le_bytes());
        bytes[10..12].copy_from_slice(&1u16.to_le_bytes());
        bytes[32] = 0x0d;
        bytes
    }

    fn test_zip(geoid: &str, layer: &str) -> Vec<u8> {
        let stem = CountyCache::remote_name(geoid, layer)
            .strip_suffix(".zip")
            .unwrap()
            .to_owned();
        let files = [
            (format!("{stem}.shp"), shape_file(layer)),
            (format!("{stem}.dbf"), dbf_file()),
            (format!("{stem}.shx"), shape_file(layer)),
            (format!("{stem}.prj"), b"NAD83".to_vec()),
            (format!("{stem}.shp.xml"), b"<metadata/>".to_vec()),
        ];
        let entry_count = files.len() as u16;
        let mut bytes = Vec::new();
        let mut central = Vec::new();
        for (name, data) in files {
            let name_bytes = name.as_bytes();
            let offset = bytes.len() as u32;
            let crc = zip_crc32(&data);
            bytes.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            bytes.extend_from_slice(&20u16.to_le_bytes());
            bytes.extend_from_slice(&0u16.to_le_bytes());
            bytes.extend_from_slice(&0u16.to_le_bytes());
            bytes.extend_from_slice(&0u16.to_le_bytes());
            bytes.extend_from_slice(&0u16.to_le_bytes());
            bytes.extend_from_slice(&crc.to_le_bytes());
            bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            bytes.extend_from_slice(&0u16.to_le_bytes());
            bytes.extend_from_slice(name_bytes);
            bytes.extend_from_slice(&data);

            central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&(data.len() as u32).to_le_bytes());
            central.extend_from_slice(&(data.len() as u32).to_le_bytes());
            central.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u32.to_le_bytes());
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(name_bytes);
        }
        let central_offset = bytes.len() as u32;
        bytes.extend_from_slice(&central);
        bytes.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&entry_count.to_le_bytes());
        bytes.extend_from_slice(&entry_count.to_le_bytes());
        bytes.extend_from_slice(&(central.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&central_offset.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes
    }

    pub(crate) fn test_zip_archive(geoid: &str, layer: &str) -> Vec<u8> {
        test_zip(geoid, layer)
    }

    #[test]
    fn census_archive_validation_matches_source_and_checks_integrity() {
        for layer in ["roads", "areawater", "linearwater", "pointlm"] {
            let archive = test_zip("25025", layer);
            validate_tiger_archive(&archive, "25025", layer).unwrap();
        }

        let mut corrupted = test_zip("25025", "roads");
        let name_len = usize::from(u16::from_le_bytes([corrupted[26], corrupted[27]]));
        corrupted[30 + name_len] ^= 1;
        assert!(validate_tiger_archive(&corrupted, "25025", "roads").is_err());
        assert!(validate_tiger_archive(&test_zip("25025", "roads"), "25025", "areawater").is_err());
        assert!(validate_tiger_archive(&test_zip("25025", "pointlm"), "36061", "pointlm").is_err());
    }

    #[test]
    fn census_archive_validator_accepts_bounded_raw_deflate() {
        let plain = b"small TIGER member";
        let packed = miniz_oxide::deflate::compress_to_vec(plain, 6);
        let member = ZipMember {
            method: 8,
            crc: zip_crc32(plain),
            packed: &packed,
            unpacked: plain.len(),
        };
        assert_eq!(unpack_member(&member).unwrap(), plain);

        let oversized = ZipMember {
            unpacked: plain.len() - 1,
            ..member
        };
        assert!(unpack_member(&oversized).is_err());
    }

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
        let archive = test_zip("25025", "roads");
        fs::write(&path, &archive).unwrap();
        assert_eq!(cache.fetch("25025", "tiger-roads", false).unwrap(), archive);
        assert!(cache.fetch("25025", "tiger-areawater", false).is_err());
        cache.remember("25025").unwrap();
        let active = cache.active().unwrap();
        assert_eq!(active["active_geoid"], "25025");
        assert_eq!(active["cached_sources"], json!(["tiger-roads"]));
        assert_eq!(active["network_used"], false);
    }

    #[test]
    fn cache_fetch_does_not_activate_until_a_base_layer_is_committed() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = CountyCache::new(tmp.path().to_path_buf());
        fs::write(cache.boundary_path("25025"), county_fixture("25025")).unwrap();
        fs::write(cache.path("25025", "roads"), test_zip("25025", "roads")).unwrap();

        cache.boundary("25025", true).unwrap();
        cache.fetch("25025", "tiger-roads", true).unwrap();
        assert_eq!(cache.active().unwrap()["active_geoid"], Value::Null);

        let active = cache.activate("25025").unwrap();
        assert_eq!(active["active_geoid"], "25025");
        assert_eq!(active["cached_sources"], json!(["tiger-roads"]));
        assert_eq!(active["boundary_cached"], true);
        assert!(cache.activate("25023").is_err());
    }

    #[test]
    fn cache_reads_do_not_contend_with_unrelated_downloads() {
        let temp = tempfile::tempdir().unwrap();
        let cache = CountyCache::new(temp.path().to_path_buf());
        let archive = test_zip("25025", "roads");
        fs::write(cache.path("25025", "roads"), &archive).unwrap();
        let gate = FETCH_LOCK.get_or_init(|| Mutex::new(()));
        let _held = gate.lock().unwrap();
        assert_eq!(cache.fetch("25025", "tiger-roads", false).unwrap(), archive);
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
    fn structurally_invalid_cached_archive_is_preserved_and_not_activated() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = CountyCache::new(tmp.path().to_path_buf());
        let path = cache.path("25025", "roads");
        let mut archive = test_zip("25025", "roads");
        let name = b"tl_2025_25025_roads.shp";
        let local_name = archive
            .windows(name.len())
            .position(|window| window == name)
            .unwrap();
        let data_start = local_name + name.len();
        archive[data_start] ^= 1;
        fs::write(&path, &archive).unwrap();

        assert_eq!(
            cache
                .fetch("25025", "tiger-roads", true)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(fs::read(&path).unwrap(), archive);
        assert_eq!(cache.active().unwrap()["cached_sources"], json!([]));
        assert!(cache.activate("25025").is_err());
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

//! Allowlisted, operator-triggered Census county archives for the local map.
//! Never accepts a URL or raw GPS position. Only a validated 5-digit GEOID and
//! a source ID from SOURCE_LIST can reach a fixed HTTPS Census ZIP path.
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

pub const MAX_ARCHIVE: u64 = 12 * 1024 * 1024;
const MAX_COUNTIES: usize = 4;
const VINTAGE: &str = "2025";
const SOURCE_LIST: [(&str, &str, &str); 3] = [
    ("tiger-roads", "ROADS", "roads"),
    ("tiger-areawater", "AREAWATER", "areawater"),
    ("tiger-linearwater", "LINEARWATER", "linearwater"),
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
             "description":"County-wide TIGER linear rivers, streams and hydrography"}
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
        format!("tl_{VINTAGE}_{geoid}_{layer}.zip")
    }

    fn path(&self, geoid: &str, layer: &str) -> PathBuf {
        self.root.join(Self::name(geoid, layer))
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
        Ok(json!({
            "schema":1,"active_geoid":geoid,"cached_sources":cached,
            "vintage":VINTAGE,"network_used":false
        }))
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
            Self::name(geoid, layer)
        );
        let tmp = tempfile::Builder::new()
            .prefix(".county-download-")
            .tempfile_in(&self.root)?;
        // argv is always fixed by source(), geoid_valid(); no shell, arbitrary
        // target, redirects, retry loop, credentials, or custom user URLs.
        let cmd = if cfg!(windows) { "curl.exe" } else { "curl" };
        let ok = Command::new(cmd)
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
        assert_eq!(data["sources"].as_array().unwrap().len(), 3);
        assert_eq!(source("tiger-roads").unwrap().1, "roads");
        assert_eq!(source("tiger-areawater").unwrap().1, "areawater");
        assert_eq!(source("tiger-linearwater").unwrap().1, "linearwater");
    }
}

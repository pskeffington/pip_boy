//! Bounded, user-triggered 2020 Census ZCTA-to-county relationship index.
//! Statistical ZCTAs are NOT USPS postal delivery ZIP codes. There is no
//! one-to-one ZIP, town or county mapping, and county boundaries can change.
//! The only upstream file is a fixed 2020 Census pipe-delimited crosswalk.
use pipboy_core::process_safety::isolated_command;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};

const FILE_NAME: &str = "tab20_zcta520_county20_natl.txt";
const SOURCE_URL: &str =
    "https://www2.census.gov/geo/docs/maps-data/data/rel2020/zcta520/tab20_zcta520_county20_natl.txt";
const MAX_BYTES: u64 = 9 * 1024 * 1024;
const MAX_ROWS: usize = 80_000;
const PAGE_SIZE: usize = 12;
const HEADERS: [&str; 18] = [
    "OID_ZCTA5_20",
    "GEOID_ZCTA5_20",
    "NAMELSAD_ZCTA5_20",
    "AREALAND_ZCTA5_20",
    "AREAWATER_ZCTA5_20",
    "MTFCC_ZCTA5_20",
    "CLASSFP_ZCTA5_20",
    "FUNCSTAT_ZCTA5_20",
    "OID_COUNTY_20",
    "GEOID_COUNTY_20",
    "NAMELSAD_COUNTY_20",
    "AREALAND_COUNTY_20",
    "AREAWATER_COUNTY_20",
    "MTFCC_COUNTY_20",
    "CLASSFP_COUNTY_20",
    "FUNCSTAT_COUNTY_20",
    "AREALAND_PART",
    "AREAWATER_PART",
];
static FETCH_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

pub fn valid_zip(zip: &str) -> bool {
    zip.len() == 5 && zip.bytes().all(|x| x.is_ascii_digit())
}
pub fn valid_county(geoid: &str) -> bool {
    valid_zip(geoid) && &geoid[..2] != "00" && &geoid[2..] != "000"
}

#[derive(Clone, Debug)]
struct Intersection {
    zcta: String,
    geoid: String,
    county_name: String,
    part_land: u64,
    total_land: u64,
}

fn parse(raw: &[u8]) -> io::Result<Vec<Intersection>> {
    if raw.len() as u64 > MAX_BYTES || raw.len() < 80 {
        return Err(invalid("ZCTA relationship index is missing or oversized"));
    }
    let content = std::str::from_utf8(raw)
        .map_err(|_| invalid("ZCTA relationship index has invalid UTF-8"))?;
    let mut lines = content.trim_start_matches('\u{feff}').lines();
    let header = lines.next().ok_or_else(|| invalid("Missing ZCTA relationship header"))?;
    if header.trim_end_matches('\r').split('|').collect::<Vec<_>>() != HEADERS {
        return Err(invalid("Unexpected Census ZCTA relationship file schema"));
    }
    let mut parsed = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        if parsed.len() >= MAX_ROWS {
            return Err(invalid("ZCTA relationship row limit exceeded"));
        }
        let fields: Vec<&str> = line.trim_end_matches('\r').split('|').collect();
        if fields.len() != HEADERS.len() {
            return Err(invalid("Malformed Census ZCTA relationship row"));
        }
        let zcta = fields[1];
        if zcta.is_empty() {
            // Counties with no 2020 ZCTA have an intentionally empty ZCTA.
            continue;
        }
        let geoid = fields[9];
        let name = fields[10];
        if !valid_zip(zcta) || !valid_county(geoid) || name.is_empty()
            || name.len() > 100 || name.chars().any(char::is_control)
        {
            return Err(invalid("Invalid Census ZCTA/county identifier or name"));
        }
        let total_land = fields[3]
            .parse::<u64>()
            .map_err(|_| invalid("Invalid Census ZCTA total land area"))?;
        let part_land = fields[16]
            .parse::<u64>()
            .map_err(|_| invalid("Invalid Census ZCTA county intersection area"))?;
        if part_land > total_land && total_land != 0 {
            return Err(invalid("Census ZCTA intersection exceeds its total area"));
        }
        parsed.push(Intersection {
            zcta: zcta.to_owned(),
            geoid: geoid.to_owned(),
            county_name: name.to_owned(),
            part_land,
            total_land,
        });
    }
    if parsed.is_empty() {
        return Err(invalid("Census ZCTA relationship index has no relationships"));
    }
    Ok(parsed)
}

pub struct ZipIndex {
    root: PathBuf,
}

impl ZipIndex {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn path(&self) -> PathBuf {
        self.root.join(FILE_NAME)
    }

    fn read(&self) -> io::Result<Vec<Intersection>> {
        let file = File::open(self.path())?;
        let length = file.metadata()?.len();
        if !(80..=MAX_BYTES).contains(&length) {
            return Err(invalid("Invalid cached Census ZCTA index size"));
        }
        let mut bytes = Vec::with_capacity(length as usize);
        file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 != length {
            return Err(invalid("Census index changed or exceeds byte budget"));
        }
        parse(&bytes)
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

    fn entries(&self, allow_network: bool) -> io::Result<(Vec<Intersection>, bool)> {
        match self.read() {
            Ok(entries) => return Ok((entries, true)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error), // Never overwrite corrupt cache.
        }
        if !allow_network {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "2020 ZCTA county index is not cached",
            ));
        }
        let lock = FETCH_LOCK.get_or_init(|| Mutex::new(()));
        let _guard = lock.try_lock().map_err(|_| {
            io::Error::new(io::ErrorKind::WouldBlock, "Census index download in progress")
        })?;
        match self.read() {
            Ok(entries) => return Ok((entries, true)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        self.ensure_root()?;
        let temp = tempfile::Builder::new()
            .prefix(".zcta-index-")
            .tempfile_in(&self.root)?;
        // The URL, host, HTTPS policy and filesystem target are all fixed.
        // No user-provided ZIP, GEOID, coordinates or origin reaches curl.
        let cmd = if cfg!(windows) { "curl.exe" } else { "curl" };
        let ok = isolated_command(cmd)
            .args([
                "--fail", "--silent", "--show-error", "--proto", "=https",
                "--max-redirs", "0", "--connect-timeout", "8", "--max-time", "60",
                "--max-filesize", "9437184", "--retry", "0", "--output",
            ])
            .arg(temp.path())
            .arg(SOURCE_URL)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if !ok.success() {
            return Err(io::Error::other("Census ZCTA relationship download unavailable"));
        }
        let mut bytes = Vec::new();
        File::open(temp.path())?
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(invalid("Census ZCTA relationship download exceeds limit"));
        }
        let entries = parse(&bytes)?;
        // The production national source contains many thousands of rows.
        if entries.len() < 30_000 {
            return Err(invalid("Truncated national Census ZCTA relationship download"));
        }
        temp.as_file().sync_all()?;
        temp.persist(self.path()).map_err(|error| error.error)?;
        Ok((entries, false))
    }

    pub fn resolve_zip(&self, zip: &str, allow_network: bool) -> io::Result<Value> {
        if !valid_zip(zip) {
            return Err(invalid("ZIP entry must be five ASCII digits"));
        }
        let (entries, cache_hit) = self.entries(allow_network)?;
        let mut found: BTreeMap<String, (String, u64, u64)> = BTreeMap::new();
        for row in entries.iter().filter(|row| row.zcta == zip) {
            let item = found.entry(row.geoid.clone()).or_insert_with(|| {
                (row.county_name.clone(), 0, row.total_land)
            });
            item.1 = item.1.saturating_add(row.part_land);
            item.2 = item.2.max(row.total_land);
        }
        let mut candidates: Vec<_> = found
            .into_iter()
            .map(|(geoid, (name, land_part, land_total))| {
                let fraction = if land_total == 0 {
                    0.0
                } else {
                    (land_part as f64 / land_total as f64 * 1000.0).round() / 10.0
                };
                (geoid, name, land_part, fraction.min(100.0))
            })
            .collect();
        candidates.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        if candidates.len() > 24 {
            return Err(invalid("Unexpected number of intersecting Census counties"));
        }
        let count = candidates.len();
        Ok(json!({
            "schema":1, "zip":zip, "interpretation":"2020 Census ZCTA approximation (not USPS ZIP)",
            "relationship_vintage":"2020", "map_vintage":"2025",
            "status":if count==0 {"no_zcta"} else if count==1 {"single_county"} else {"multiple_counties"},
            "requires_selection":count!=1,
            "counties":candidates.into_iter().map(|(geoid,name,_,share)| json!({
                "geoid":geoid,"name":name,"zcta_land_share_pct":share
            })).collect::<Vec<_>>(),
            "cache_hit":cache_hit, "network_used":!cache_hit
        }))
    }

    pub fn county_zctas(&self, geoid: &str, offset: usize, allow_network: bool) -> io::Result<Value> {
        if !valid_county(geoid) || offset > MAX_ROWS {
            return Err(invalid("Invalid county GEOID or ZCTA offset"));
        }
        let (entries, cache_hit) = self.entries(allow_network)?;
        let mut found: BTreeMap<String, u64> = BTreeMap::new();
        for row in entries.iter().filter(|row| row.geoid == geoid) {
            *found.entry(row.zcta.clone()).or_default() += row.part_land;
        }
        let mut ordered: Vec<_> = found.into_iter().collect();
        ordered.sort_by(|a,b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let total = ordered.len();
        let zctas: Vec<_> = ordered
            .into_iter()
            .skip(offset)
            .take(PAGE_SIZE)
            .map(|(zip, _)| zip)
            .collect();
        Ok(json!({
            "schema":1,"geoid":geoid, "relationship_vintage":"2020",
            "interpretation":"Intersecting ZCTAs (not USPS ZIP inventory)",
            "total":total,"offset":offset,"page_size":PAGE_SIZE,
            "zctas":zctas,"cache_hit":cache_hit,"network_used":!cache_hit
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut s = HEADERS.join("|");
        s.push('\n');
        let mut row = |zip: &str, geoid: &str, name: &str, whole: &str, part: &str| {
            let mut cols = vec![""; HEADERS.len()];
            cols[1] = zip;
            cols[3] = whole;
            cols[9] = geoid;
            cols[10] = name;
            cols[16] = part;
            s.push_str(&cols.join("|"));
            s.push('\n');
        };
        row("02360", "25023", "Plymouth County", "100", "95");
        row("02360", "25001", "Barnstable County", "100", "5");
        row("02108", "25025", "Suffolk County", "50", "50");
        row("00501", "36059", "Nassau County", "200", "200");
        s.into_bytes()
    }

    #[test]
    fn county_crosswalk_is_bidirectional_without_choosing_a_multi_county_zip() {
        let temp = tempfile::tempdir().unwrap();
        let index = ZipIndex::new(temp.path().to_path_buf());
        fs::write(index.path(), fixture()).unwrap();
        let multi = index.resolve_zip("02360", false).unwrap();
        assert_eq!(multi["status"], "multiple_counties");
        assert_eq!(multi["requires_selection"], true);
        assert_eq!(multi["counties"][0]["geoid"], "25023");
        assert_eq!(multi["counties"][1]["geoid"], "25001");
        assert_eq!(multi["counties"][0]["zcta_land_share_pct"], 95.0);
        assert_eq!(multi["network_used"], false);
        let single = index.resolve_zip("02108", false).unwrap();
        assert_eq!(single["status"], "single_county");
        assert_eq!(single["requires_selection"], false);
        assert_eq!(single["counties"][0]["geoid"], "25025");
        assert_eq!(index.resolve_zip("99999", false).unwrap()["status"], "no_zcta");
        let reverse = index.county_zctas("25023", 0, false).unwrap();
        assert_eq!(reverse["zctas"], json!(["02360"]));
        assert_eq!(reverse["total"], 1);
        assert_eq!(index.county_zctas("25023", 12, false).unwrap()["zctas"], json!([]));
    }

    #[test]
    fn leading_zero_and_invalid_input_fail_closed() {
        let temp = tempfile::tempdir().unwrap();
        let index = ZipIndex::new(temp.path().join("missing"));
        for zip in ["", "2360", "023600", "../25", "02\n08", "０２３６０"] {
            assert!(index.resolve_zip(zip, true).is_err());
        }
        assert!(index.county_zctas("../25", 0, true).is_err());
        assert!(!index.root.exists());
        assert_eq!(
            index.resolve_zip("02360", false).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }

    #[test]
    fn damaged_cached_crosswalk_never_triggers_replacement_download() {
        let temp = tempfile::tempdir().unwrap();
        let index = ZipIndex::new(temp.path().to_path_buf());
        fs::write(index.path(), b"damaged").unwrap();
        assert_eq!(
            index.resolve_zip("02360", true).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(fs::read(index.path()).unwrap(), b"damaged");
    }

    #[test]
    fn crosswalk_parser_rejects_invalid_header_and_areas() {
        let mut bad = fixture();
        bad[0] = b'X';
        assert!(parse(&bad).is_err());
        let s = String::from_utf8(fixture()).unwrap();
        assert!(parse(s.replace("100||||||", "oops||||||").as_bytes()).is_err());
        assert!(SOURCE_URL.starts_with("https://www2.census.gov/geo/docs/maps-data/"));
    }
}

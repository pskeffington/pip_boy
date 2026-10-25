//! Local-first Reclamation Field Kit: bounded site observations and GeoJSON export.
//! Records are operator-entered, not GPS fixes or verified remediation decisions.
//! Reading or loading a profile never writes a file or starts network activity.

use crate::scout_gis::{waypoint, Point};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEMA: u32 = 1;
const MAX_SITES: usize = 256;
const MAX_BYTES: u64 = 512 * 1024;
const MAX_HISTORY: usize = 16;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}

fn text_ok(text: &str, limit: usize, required: bool) -> bool {
    text.len() <= limit
        && (!required || !text.is_empty())
        && text.trim() == text
        && !text.chars().any(char::is_control)
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SiteKind {
    Water,
    Sanitation,
    Infrastructure,
    Terrain,
    Supplies,
    Other,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    Unknown,
    Serviceable,
    Degraded,
    Unsafe,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Routine,
    High,
    Urgent,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SiteStatus {
    Logged,
    InProgress,
    NeedsAssistance,
    Completed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NewSite {
    pub name: String,
    pub kind: SiteKind,
    pub condition: Condition,
    pub priority: Priority,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_m: Option<f64>,
    pub notes: String,
}

impl NewSite {
    pub fn validate(&self) -> io::Result<()> {
        if !text_ok(&self.name, 64, true) || !text_ok(&self.notes, 512, false) {
            return Err(invalid("Invalid site name or notes"));
        }
        Point::new(self.latitude, self.longitude)?;
        if self
            .accuracy_m
            .is_some_and(|value| !value.is_finite() || !(0.0..=10_000.0).contains(&value))
        {
            return Err(invalid("Accuracy estimate must be between 0 and 10000 meters"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SiteEvent {
    pub at: u64,
    pub status: SiteStatus,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    pub id: u64,
    pub revision: u64,
    pub name: String,
    pub kind: SiteKind,
    pub condition: Condition,
    pub priority: Priority,
    pub status: SiteStatus,
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_m: Option<f64>,
    /// "manual" is a provenance label. No background GPS or inferred fix.
    pub position_source: String,
    pub notes: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub history: Vec<SiteEvent>,
}

impl Site {
    fn validate(&self) -> io::Result<()> {
        NewSite {
            name: self.name.clone(),
            kind: self.kind,
            condition: self.condition,
            priority: self.priority,
            latitude: self.latitude,
            longitude: self.longitude,
            accuracy_m: self.accuracy_m,
            notes: self.notes.clone(),
        }
        .validate()?;
        if self.id == 0
            || self.revision == 0
            || self.position_source != "manual"
            || self.updated_at < self.created_at
            || self.history.is_empty()
            || self.history.len() > MAX_HISTORY
            || self.history.last().is_none_or(|event| event.status != self.status)
        {
            return Err(invalid("Invalid reclamation site record"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SiteBook {
    pub schema_version: u32,
    pub next_id: u64,
    pub sites: Vec<Site>,
}

impl Default for SiteBook {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA,
            next_id: 1,
            sites: Vec::new(),
        }
    }
}

impl SiteBook {
    fn validate(&self) -> io::Result<()> {
        if self.schema_version != SCHEMA || self.sites.len() > MAX_SITES || self.next_id == 0 {
            return Err(invalid("Unsupported or oversized reclamation site book"));
        }
        let mut seen = HashSet::new();
        for site in &self.sites {
            site.validate()?;
            if site.id >= self.next_id || !seen.insert(site.id) {
                return Err(invalid("Duplicate or invalid reclamation site ID"));
            }
        }
        Ok(())
    }

    fn read(mut input: impl Read) -> io::Result<Self> {
        let mut bytes = Vec::new();
        input.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(invalid("Reclamation site book exceeds storage limit"));
        }
        let book: Self = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("Reclamation site book is invalid or from a newer schema"))?;
        book.validate()?;
        Ok(book)
    }

    pub fn geojson(&self) -> io::Result<Value> {
        self.validate()?;
        let features = self
            .sites
            .iter()
            .map(|site| {
                let point = Point::new(site.latitude, site.longitude)?;
                let mut feature = waypoint(&site.name, point)?;
                feature["properties"] = json!({
                    "id": site.id,
                    "name": site.name,
                    "kind": site.kind,
                    "condition": site.condition,
                    "priority": site.priority,
                    "status": site.status,
                    "accuracy_m": site.accuracy_m,
                    "position_source": site.position_source,
                    "notes": site.notes
                });
                Ok(feature)
            })
            .collect::<io::Result<Vec<_>>>()?;
        Ok(json!({
            "type": "FeatureCollection",
            "name": "Rust-Boy Reclamation Sites",
            "features": features
        }))
    }
}

pub struct SiteStore {
    path: PathBuf,
}

impl SiteStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            path: root.join("reclamation.json"),
        }
    }

    pub fn from_environment() -> io::Result<Self> {
        let root = match env::var_os("PIPBOY_HOME") {
            Some(value) if !value.is_empty() => PathBuf::from(value),
            _ => {
                let home = env::var_os("HOME")
                    .or_else(|| env::var_os("USERPROFILE"))
                    .ok_or_else(|| invalid("Home folder unavailable"))?;
                PathBuf::from(home).join(".pipboy")
            }
        };
        Ok(Self::new(root))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn directory(&self) -> &Path {
        self.path.parent().unwrap_or_else(|| Path::new("."))
    }

    fn writer_lock(&self) -> io::Result<fs::File> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(self.directory())?;
        }
        #[cfg(not(unix))]
        fs::create_dir_all(self.directory())?;
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(self.directory().join("reclamation.lock"))?;
        fs2::FileExt::try_lock_exclusive(&lock)?;
        Ok(lock)
    }

    pub fn load(&self) -> io::Result<SiteBook> {
        match fs::File::open(&self.path) {
            Ok(file) => SiteBook::read(file),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(SiteBook::default()),
            Err(error) => Err(error),
        }
    }

    fn save(&self, book: &SiteBook) -> io::Result<()> {
        book.validate()?;
        let encoded = serde_json::to_vec_pretty(book)?;
        if encoded.len() as u64 > MAX_BYTES {
            return Err(invalid("Reclamation site book storage limit reached"));
        }
        let mut temp = tempfile::Builder::new()
            .prefix(".reclamation-")
            .tempfile_in(self.directory())?;
        temp.as_file_mut().write_all(&encoded)?;
        temp.as_file().sync_all()?;
        temp.persist(&self.path).map_err(|error| error.error)?;
        #[cfg(unix)]
        fs::File::open(self.directory())?.sync_all()?;
        Ok(())
    }

    pub fn create(&self, input: NewSite) -> io::Result<Site> {
        input.validate()?;
        let _lock = self.writer_lock()?;
        let mut book = self.load()?;
        if book.sites.len() >= MAX_SITES {
            return Err(invalid("Reclamation site limit reached"));
        }
        let id = book.next_id;
        book.next_id = id.checked_add(1).ok_or_else(|| invalid("Site IDs exhausted"))?;
        let at = now();
        let site = Site {
            id,
            revision: 1,
            name: input.name,
            kind: input.kind,
            condition: input.condition,
            priority: input.priority,
            status: SiteStatus::Logged,
            latitude: input.latitude,
            longitude: input.longitude,
            accuracy_m: input.accuracy_m,
            position_source: "manual".to_owned(),
            notes: input.notes,
            created_at: at,
            updated_at: at,
            history: vec![SiteEvent {
                at,
                status: SiteStatus::Logged,
            }],
        };
        book.sites.push(site.clone());
        self.save(&book)?;
        Ok(site)
    }

    pub fn change(
        &self,
        id: u64,
        revision: u64,
        status: SiteStatus,
        notes: Option<&str>,
    ) -> io::Result<Site> {
        if id == 0 || revision == 0 || notes.is_some_and(|value| !text_ok(value, 512, false)) {
            return Err(invalid("Invalid site update"));
        }
        let _lock = self.writer_lock()?;
        let mut book = self.load()?;
        let site = book
            .sites
            .iter_mut()
            .find(|site| site.id == id)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Site not found"))?;
        if revision != site.revision {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Site changed since last refresh",
            ));
        }
        if site.status == status && notes.is_none_or(|value| value == site.notes) {
            return Err(invalid("Site update contains no changes"));
        }
        if site.history.len() == MAX_HISTORY {
            site.history.remove(0);
        }
        let at = now().max(site.updated_at);
        site.history.push(SiteEvent { at, status });
        site.status = status;
        if let Some(notes) = notes {
            site.notes = notes.to_owned();
        }
        site.revision = site.revision.checked_add(1).ok_or_else(|| invalid("Site revision exhausted"))?;
        site.updated_at = at;
        let changed = site.clone();
        self.save(&book)?;
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(latitude: f64, longitude: f64) -> NewSite {
        NewSite {
            name: "North Intake".to_owned(),
            kind: SiteKind::Water,
            condition: Condition::Degraded,
            priority: Priority::High,
            latitude,
            longitude,
            accuracy_m: None,
            notes: "Visual survey only".to_owned(),
        }
    }

    #[test]
    fn read_without_mutation_and_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("absent");
        let store = SiteStore::new(root.clone());
        assert!(store.load().unwrap().sites.is_empty());
        assert!(!root.exists());
        let created = store.create(site(42.0, -70.75)).unwrap();
        assert_eq!(created.id, 1);
        assert_eq!(created.position_source, "manual");
        let saved = store.load().unwrap();
        assert_eq!(saved.sites.len(), 1);
        let feature = &saved.geojson().unwrap()["features"][0];
        assert_eq!(feature["geometry"]["coordinates"], json!([-70.75, 42.0]));
        assert_eq!(feature["properties"]["status"], "logged");
        assert_eq!(feature["properties"]["position_source"], "manual");
        assert_eq!(store.path().file_name().unwrap(), "reclamation.json");
    }

    #[test]
    fn revision_guard_and_status_persist() {
        let tmp = tempfile::tempdir().unwrap();
        let store = SiteStore::new(tmp.path().to_owned());
        let first = store.create(site(0.0, 179.9)).unwrap();
        let next = store
            .change(first.id, 1, SiteStatus::NeedsAssistance, Some("Unsafe access"))
            .unwrap();
        assert_eq!(next.revision, 2);
        assert_eq!(next.history.len(), 2);
        assert_eq!(next.notes, "Unsafe access");
        assert_eq!(store.load().unwrap().sites[0].revision, 2);
        let before = fs::read(store.path()).unwrap();
        assert_eq!(
            store.change(first.id, 1, SiteStatus::Completed, None).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert!(store.change(first.id, 2, SiteStatus::NeedsAssistance, None).is_err());
        assert_eq!(fs::read(store.path()).unwrap(), before);
    }

    #[test]
    fn invalid_and_future_records_are_not_overwritten() {
        let tmp = tempfile::tempdir().unwrap();
        let store = SiteStore::new(tmp.path().to_owned());
        for (lat, lon) in [(91.0, 0.0), (0.0, -181.0), (f64::NAN, 0.0)] {
            assert!(store.create(site(lat, lon)).is_err());
        }
        let mut invalid_name = site(0.0, 0.0);
        invalid_name.name = " bad ".to_owned();
        assert!(store.create(invalid_name).is_err());
        assert!(!store.path().exists());
        for contents in [r#"{"schema_version":2,"next_id":1,"sites":[]}"#, "{bad"] {
            fs::write(store.path(), contents).unwrap();
            assert!(store.load().is_err());
            assert!(store.create(site(0.0, 0.0)).is_err());
            assert_eq!(fs::read_to_string(store.path()).unwrap(), contents);
        }
    }

    #[test]
    fn geojson_preserves_dateline_and_no_fix_claim() {
        let tmp = tempfile::tempdir().unwrap();
        let store = SiteStore::new(tmp.path().to_owned());
        store.create(site(0.0, 179.9)).unwrap();
        store.create(site(0.0, -179.9)).unwrap();
        let geojson = store.load().unwrap().geojson().unwrap();
        assert_eq!(geojson["features"].as_array().unwrap().len(), 2);
        assert_eq!(geojson["features"][0]["geometry"]["coordinates"], json!([179.9, 0.0]));
        assert_eq!(geojson["features"][1]["geometry"]["coordinates"], json!([-179.9, 0.0]));
        assert!(geojson.to_string().len() < 4096);
    }
}

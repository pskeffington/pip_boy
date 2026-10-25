//! Persisted adapter preferences. Requests are NOT active capability grants.
//! No hardware, socket, radio or device handle is opened by this module.

use crate::capability::Capability;
use crate::notes::NoteStore;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const PREFERENCES_SCHEMA_VERSION: u32 = 1;
pub const MAX_PREFERENCES_BYTES: u64 = 16 * 1024;

/// External adapter surfaces only. The local host/notes/library permissions
/// remain command-scoped; writing a preference never authorizes an adapter.
pub const ADAPTER_CAPABILITIES: &[Capability] = &[
    Capability::DeviceEnumerate,
    Capability::GpioRead,
    Capability::DisplayWrite,
    Capability::SerialRead,
    Capability::GnssRead,
    Capability::SdrReceive,
    Capability::BleDiscover,
    Capability::MeshTransmit,
    Capability::SensorRead,
    Capability::AudioPlay,
    Capability::NetworkFetch,
];

pub fn adapter_capability(key: &str) -> io::Result<Capability> {
    Capability::from_key(key)
        .filter(|capability| ADAPTER_CAPABILITIES.contains(capability))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown adapter capability; use permissions list",
            )
        })
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Preference {
    Requested,
    Denied,
}

impl Preference {
    pub fn label(self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::Denied => "denied",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreferenceEntry {
    pub capability: String,
    pub preference: Preference,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema_version: u32,
    entries: Vec<PreferenceEntry>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            schema_version: PREFERENCES_SCHEMA_VERSION,
            entries: Vec::new(),
        }
    }
}

impl Document {
    fn validate(&self) -> io::Result<()> {
        if self.schema_version != PREFERENCES_SCHEMA_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported preferences schema; original file preserved",
            ));
        }
        if self.entries.len() > ADAPTER_CAPABILITIES.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "too many adapter preferences; original file preserved",
            ));
        }
        let mut seen = HashSet::new();
        for entry in &self.entries {
            if adapter_capability(&entry.capability).is_err()
                || !seen.insert(entry.capability.as_str())
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid or repeated adapter preference; original file preserved",
                ));
            }
        }
        Ok(())
    }

    fn decode(reader: impl Read) -> io::Result<Self> {
        let mut bytes = Vec::new();
        reader
            .take(MAX_PREFERENCES_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_PREFERENCES_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "preferences document exceeds 16 KiB",
            ));
        }
        let document: Self = serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        document.validate()?;
        Ok(document)
    }

    fn encode(&self) -> io::Result<Vec<u8>> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        if bytes.len() as u64 > MAX_PREFERENCES_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "preferences document exceeds 16 KiB",
            ));
        }
        Ok(bytes)
    }
}

pub struct PreferenceStore {
    path: PathBuf,
}

impl PreferenceStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            path: root.join("permissions.json"),
        }
    }

    pub fn from_environment() -> io::Result<Self> {
        let notes = NoteStore::from_environment()?;
        let root = notes
            .path()
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid data directory"))?;
        Ok(Self::new(root.to_path_buf()))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn parent(&self) -> &Path {
        self.path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
    }

    fn load(&self) -> io::Result<Document> {
        match fs::File::open(&self.path) {
            Ok(file) => Document::decode(file),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Document::default()),
            Err(error) => Err(error),
        }
    }

    /// Reading preferences never creates a directory, file or lock.
    pub fn list(&self) -> io::Result<Vec<PreferenceEntry>> {
        Ok(self.load()?.entries)
    }

    fn ensure_directory(&self) -> io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(self.parent())?;
        }
        #[cfg(not(unix))]
        fs::create_dir_all(self.parent())?;
        Ok(())
    }

    /// Stable advisory lock for cooperating preference writers.
    fn writer_lock(&self) -> io::Result<fs::File> {
        self.ensure_directory()?;
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(self.parent().join("permissions.lock"))?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|error| {
            if error.kind() == io::ErrorKind::WouldBlock
                || error.raw_os_error() == fs2::lock_contended_error().raw_os_error()
            {
                io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "preferences are being updated by another writer; retry",
                )
            } else {
                error
            }
        })?;
        Ok(lock)
    }

    fn save(&self, document: &Document) -> io::Result<()> {
        let bytes = document.encode()?;
        let mut staging = tempfile::Builder::new()
            .prefix(".permissions-")
            .tempfile_in(self.parent())?;
        staging.as_file_mut().write_all(&bytes)?;
        staging.as_file().sync_all()?;
        staging.persist(&self.path).map_err(|error| error.error)?;
        #[cfg(unix)]
        fs::File::open(self.parent())
            .and_then(|directory| directory.sync_all())
            .map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!(
                        "preferences committed, but directory durability is uncertain; inspect before retrying: {error}"
                    ),
                )
            })?;
        Ok(())
    }

    /// Returns true only when the persisted preference changed.
    pub fn set(&self, capability: Capability, preference: Preference) -> io::Result<bool> {
        adapter_capability(capability.key())?;
        let _lock = self.writer_lock()?;
        let mut document = self.load()?;
        if let Some(entry) = document
            .entries
            .iter_mut()
            .find(|entry| entry.capability == capability.key())
        {
            if entry.preference == preference {
                return Ok(false);
            }
            entry.preference = preference;
        } else {
            document.entries.push(PreferenceEntry {
                capability: capability.key().to_owned(),
                preference,
            });
        }
        document
            .entries
            .sort_by(|left, right| left.capability.cmp(&right.capability));
        self.save(&document)?;
        Ok(true)
    }

    /// Remove a preference; the adapter remains disabled and ungranted.
    pub fn clear(&self, capability: Capability) -> io::Result<bool> {
        adapter_capability(capability.key())?;
        let _lock = self.writer_lock()?;
        let mut document = self.load()?;
        let before = document.entries.len();
        document
            .entries
            .retain(|entry| entry.capability != capability.key());
        if document.entries.len() == before {
            return Ok(false);
        }
        self.save(&document)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_preferences_are_read_only_and_fail_closed() {
        let temporary = tempfile::tempdir().unwrap();
        let home = temporary.path().join("absent");
        let store = PreferenceStore::new(home.clone());
        assert!(store.list().unwrap().is_empty());
        assert!(!home.exists());
        let host = crate::runtime::ModuleHost::new(
            crate::capability::PermissionSet::offline_read_only(),
        );
        for capability in ADAPTER_CAPABILITIES {
            assert!(!host.permissions().allows(*capability));
        }
    }

    #[test]
    fn request_deny_clear_round_trip_never_authorizes_a_device() {
        let temporary = tempfile::tempdir().unwrap();
        let store = PreferenceStore::new(temporary.path().join("data"));
        assert!(store
            .set(Capability::SdrReceive, Preference::Requested)
            .unwrap());
        let initial = fs::read(store.path()).unwrap();
        assert!(!store
            .set(Capability::SdrReceive, Preference::Requested)
            .unwrap());
        assert_eq!(fs::read(store.path()).unwrap(), initial);
        assert_eq!(store.list().unwrap()[0].preference, Preference::Requested);
        let host = crate::runtime::ModuleHost::new(
            crate::capability::PermissionSet::offline_read_only(),
        );
        assert!(!host.permissions().allows(Capability::SdrReceive));
        assert!(store
            .set(Capability::SdrReceive, Preference::Denied)
            .unwrap());
        assert_eq!(store.list().unwrap()[0].preference, Preference::Denied);
        assert!(store.clear(Capability::SdrReceive).unwrap());
        assert!(!store.clear(Capability::SdrReceive).unwrap());
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn unknown_keys_and_future_or_duplicate_documents_are_preserved() {
        let temporary = tempfile::tempdir().unwrap();
        let store = PreferenceStore::new(temporary.path().to_owned());
        assert!(adapter_capability("host.read").is_err());
        assert!(adapter_capability("unexpected.adapter").is_err());
        assert!(store.set(Capability::FilesWrite, Preference::Requested).is_err());
        assert!(!store.path().exists());
        let invalid = [
            "{broken",
            r#"{"schema_version":2,"entries":[]}"#,
            r#"{"schema_version":1,"entries":[{"capability":"network.fetch","preference":"requested"},{"capability":"network.fetch","preference":"denied"}]}"#,
            r#"{"schema_version":1,"entries":[{"capability":"bogus","preference":"requested"}]}"#,
            r#"{"schema_version":1,"entries":[],"future":true}"#,
        ];
        for item in invalid {
            fs::write(store.path(), item).unwrap();
            assert_eq!(store.list().unwrap_err().kind(), io::ErrorKind::InvalidData);
            assert!(store.set(Capability::SdrReceive, Preference::Denied).is_err());
            assert_eq!(fs::read_to_string(store.path()).unwrap(), item);
        }
    }

    #[test]
    fn lock_contention_does_not_mutate_preferences() {
        let temporary = tempfile::tempdir().unwrap();
        let store = PreferenceStore::new(temporary.path().to_owned());
        store
            .set(Capability::BleDiscover, Preference::Denied)
            .unwrap();
        let prior = fs::read(store.path()).unwrap();
        let lock = store.writer_lock().unwrap();
        assert_eq!(
            store
                .set(Capability::BleDiscover, Preference::Requested)
                .unwrap_err()
                .kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(fs::read(store.path()).unwrap(), prior);
        drop(lock);
        assert!(store
            .set(Capability::BleDiscover, Preference::Requested)
            .unwrap());
    }

    #[test]
    fn oversized_and_invalid_documents_are_rejected() {
        let temporary = tempfile::tempdir().unwrap();
        let store = PreferenceStore::new(temporary.path().to_owned());
        fs::write(store.path(), vec![b' '; (MAX_PREFERENCES_BYTES + 1) as usize])
            .unwrap();
        assert_eq!(store.list().unwrap_err().kind(), io::ErrorKind::InvalidData);
        let before = fs::read(store.path()).unwrap();
        assert!(store.clear(Capability::GnssRead).is_err());
        assert_eq!(fs::read(store.path()).unwrap(), before);
    }
}

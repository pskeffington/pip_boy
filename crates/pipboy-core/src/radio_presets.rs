//! Validated, local-only RADIO favorites and last-selected station.
//! The file contains approved station IDs only, never stream URLs or audio.

use crate::radio_directory::station_by_id;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_FAVORITES: usize = 12;
pub const MAX_BYTES: u64 = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadioPresets {
    pub schema_version: u32,
    pub favorites: Vec<String>,
    pub last_selected: Option<String>,
}

impl Default for RadioPresets {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            favorites: Vec::new(),
            last_selected: None,
        }
    }
}

impl RadioPresets {
    pub fn validate(&self) -> io::Result<()> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unsupported radio presets schema",
            ));
        }
        if self.favorites.len() > MAX_FAVORITES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Radio favorites exceed 12 approved station IDs",
            ));
        }
        let mut seen = HashSet::new();
        for id in &self.favorites {
            if station_by_id(id).is_none() || !seen.insert(id.as_str()) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Radio favorites contain duplicate or unapproved station IDs",
                ));
            }
        }
        if self
            .last_selected
            .as_deref()
            .is_some_and(|id| station_by_id(id).is_none())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Last-selected radio station is not curated",
            ));
        }
        Ok(())
    }

    fn read(reader: impl Read) -> io::Result<Self> {
        let mut data = Vec::new();
        reader.take(MAX_BYTES + 1).read_to_end(&mut data)?;
        if data.len() as u64 > MAX_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Radio presets exceed 4 KiB",
            ));
        }
        let document: Self = serde_json::from_slice(&data)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        document.validate()?;
        Ok(document)
    }

    fn encoded(&self) -> io::Result<Vec<u8>> {
        self.validate()?;
        let bytes =
            serde_json::to_vec_pretty(self).map_err(|error| io::Error::other(error.to_string()))?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Radio presets exceed 4 KiB",
            ));
        }
        Ok(bytes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDirection {
    Earlier,
    Later,
}

pub struct RadioPresetStore {
    path: PathBuf,
}

impl RadioPresetStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            path: root.join("radio-presets.json"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn parent(&self) -> &Path {
        self.path.parent().unwrap_or_else(|| Path::new("."))
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

    fn writer_lock(&self) -> io::Result<fs::File> {
        self.ensure_directory()?;
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(self.parent().join("radio-presets.lock"))?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|error| {
            if error.kind() == io::ErrorKind::WouldBlock
                || error.raw_os_error() == fs2::lock_contended_error().raw_os_error()
            {
                io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "Radio presets are in use by another writer",
                )
            } else {
                error
            }
        })?;
        Ok(lock)
    }

    /// Missing preferences are empty; GET never creates a folder or file.
    /// Corrupt, symlinked and incompatible documents are rejected, not reset.
    pub fn load(&self) -> io::Result<RadioPresets> {
        let metadata = match fs::symlink_metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(RadioPresets::default());
            }
            Err(error) => return Err(error),
        };
        if !metadata.file_type().is_file() || metadata.len() > MAX_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Radio presets must be a regular file smaller than 4 KiB",
            ));
        }
        RadioPresets::read(fs::File::open(&self.path)?)
    }

    fn mutate(
        &self,
        change: impl FnOnce(&mut RadioPresets) -> io::Result<()>,
    ) -> io::Result<RadioPresets> {
        let _lock = self.writer_lock()?;
        let mut next = self.load()?;
        let previous = next.clone();
        change(&mut next)?;
        next.validate()?;
        if next != previous {
            let bytes = next.encoded()?;
            let mut temporary = tempfile::Builder::new()
                .prefix(".radio-presets-")
                .tempfile_in(self.parent())?;
            temporary.as_file_mut().write_all(&bytes)?;
            temporary.as_file().sync_all()?;
            temporary.persist(&self.path).map_err(|error| error.error)?;
            #[cfg(unix)]
            fs::File::open(self.parent())?.sync_all()?;
        }
        Ok(next)
    }

    fn approved(id: &str) -> io::Result<()> {
        if station_by_id(id).is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Unknown curated radio station",
            ));
        }
        Ok(())
    }

    /// Selecting a station updates metadata only. It never starts audio.
    pub fn select(&self, id: &str) -> io::Result<RadioPresets> {
        Self::approved(id)?;
        self.mutate(|document| {
            document.last_selected = Some(id.to_owned());
            Ok(())
        })
    }

    /// Add/remove one validated station, retaining the user's order.
    pub fn favorite(&self, id: &str, enabled: bool) -> io::Result<RadioPresets> {
        Self::approved(id)?;
        self.mutate(|document| {
            let index = document.favorites.iter().position(|entry| entry == id);
            if enabled && index.is_none() {
                if document.favorites.len() >= MAX_FAVORITES {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Radio favorites limit reached (12)",
                    ));
                }
                document.favorites.push(id.to_owned());
            } else if !enabled {
                if let Some(index) = index {
                    document.favorites.remove(index);
                }
            }
            Ok(())
        })
    }

    pub fn move_favorite(&self, id: &str, direction: MoveDirection) -> io::Result<RadioPresets> {
        Self::approved(id)?;
        self.mutate(|document| {
            let index = document
                .favorites
                .iter()
                .position(|entry| entry == id)
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Station is not a favorite")
                })?;
            let other = match direction {
                MoveDirection::Earlier => index.saturating_sub(1),
                MoveDirection::Later => (index + 1).min(document.favorites.len() - 1),
            };
            document.favorites.swap(index, other);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_default_does_not_touch_the_disk() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("not-created");
        let store = RadioPresetStore::new(root.clone());
        assert_eq!(store.load().unwrap(), RadioPresets::default());
        assert!(!root.exists());
    }

    #[test]
    fn favorites_order_and_selection_survive_restart_without_urls() {
        let temporary = tempfile::tempdir().unwrap();
        let store = RadioPresetStore::new(temporary.path().to_path_buf());
        let mut document = store.favorite("skylab", true).unwrap();
        assert_eq!(document.favorites, vec!["skylab"]);
        document = store.favorite("dronezone", true).unwrap();
        assert_eq!(document.favorites, vec!["skylab", "dronezone"]);
        document = store.move_favorite("dronezone", MoveDirection::Earlier).unwrap();
        assert_eq!(document.favorites, vec!["dronezone", "skylab"]);
        document = store.select("groovesalad").unwrap();
        assert_eq!(document.last_selected.as_deref(), Some("groovesalad"));
        assert_eq!(
            RadioPresetStore::new(temporary.path().to_path_buf())
                .load()
                .unwrap(),
            document
        );
        let persisted = fs::read_to_string(store.path()).unwrap();
        assert!(!persisted.contains("https:"));
        assert!(!persisted.contains("uksoutha"));
        document = store.favorite("skylab", false).unwrap();
        assert_eq!(document.favorites, vec!["dronezone"]);
        assert_eq!(document.last_selected.as_deref(), Some("groovesalad"));
    }

    #[test]
    fn invalid_and_duplicate_ids_never_replace_saved_data() {
        let temporary = tempfile::tempdir().unwrap();
        let store = RadioPresetStore::new(temporary.path().to_path_buf());
        store.favorite("skylab", true).unwrap();
        let original = fs::read(store.path()).unwrap();
        assert!(store.select("https://unapproved.test/stream").is_err());
        assert!(store.favorite("malicious", true).is_err());
        assert!(store.move_favorite("groovesalad", MoveDirection::Later).is_err());
        assert_eq!(fs::read(store.path()).unwrap(), original);
        let invalid = RadioPresets {
            favorites: vec!["skylab".into(), "skylab".into()],
            ..RadioPresets::default()
        };
        assert!(invalid.validate().is_err());
        assert!(RadioPresets {
            schema_version: SCHEMA_VERSION + 1,
            ..RadioPresets::default()
        }
        .validate()
        .is_err());
        assert!(RadioPresets {
            last_selected: Some("https://other.example/".into()),
            ..RadioPresets::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn corrupted_existing_document_is_preserved_on_mutation() {
        let temporary = tempfile::tempdir().unwrap();
        let store = RadioPresetStore::new(temporary.path().to_path_buf());
        let corrupt = br#"{"schema_version":1,"favorites":["evil"],"last_selected":null}"#;
        fs::write(store.path(), corrupt).unwrap();
        assert!(store.load().is_err());
        assert!(store.favorite("skylab", true).is_err());
        assert_eq!(fs::read(store.path()).unwrap(), corrupt);
        fs::write(store.path(), vec![b'x'; MAX_BYTES as usize + 1]).unwrap();
        assert!(store.load().is_err());
        assert!(store.select("skylab").is_err());
        assert_eq!(fs::metadata(store.path()).unwrap().len(), MAX_BYTES + 1);
    }

    #[test]
    fn no_op_and_bounded_reordering_are_safe() {
        let temporary = tempfile::tempdir().unwrap();
        let store = RadioPresetStore::new(temporary.path().to_path_buf());
        store.favorite("skylab", true).unwrap();
        let original = fs::read(store.path()).unwrap();
        store.favorite("skylab", true).unwrap();
        store.move_favorite("skylab", MoveDirection::Earlier).unwrap();
        store.move_favorite("skylab", MoveDirection::Later).unwrap();
        assert_eq!(fs::read(store.path()).unwrap(), original);
    }

    #[test]
    fn writer_lock_prevents_concurrent_replacement() {
        let temporary = tempfile::tempdir().unwrap();
        let store = RadioPresetStore::new(temporary.path().to_path_buf());
        let _lock = store.writer_lock().unwrap();
        let error = store.favorite("skylab", true).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        assert!(!store.path().exists());
    }

    #[cfg(unix)]
    #[test]
    fn preset_file_symlink_is_not_followed() {
        use std::os::unix::fs::symlink;
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        let target = root.join("target");
        fs::write(&target, b"sensitive data").unwrap();
        let store = RadioPresetStore::new(root.to_path_buf());
        symlink(&target, store.path()).unwrap();
        assert!(store.load().is_err());
        assert!(store.favorite("skylab", true).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"sensitive data");
    }
}

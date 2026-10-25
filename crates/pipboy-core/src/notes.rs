//! Local plaintext journal. No implicit sync, encryption or network access.

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Note {
    pub id: u64,
    pub created_unix_s: u64,
    pub text: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Document {
    schema_version: u32,
    notes: Vec<Note>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            schema_version: 1,
            notes: Vec::new(),
        }
    }
}

pub struct NoteStore {
    path: PathBuf,
}

impl NoteStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            path: root.join("notes.json"),
        }
    }

    pub fn from_environment() -> io::Result<Self> {
        let root = match env::var_os("PIPBOY_HOME") {
            Some(value) if !value.is_empty() => PathBuf::from(value),
            _ => {
                let home = env::var_os("HOME")
                    .or_else(|| env::var_os("USERPROFILE"))
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::NotFound, "home directory not available")
                    })?;
                PathBuf::from(home).join(".pipboy")
            }
        };
        Ok(Self::new(root))
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    fn load(&self) -> io::Result<Document> {
        if !self.path.exists() {
            return Ok(Document::default());
        }
        let bytes = fs::read(&self.path)?;
        let document: Document = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        if document.schema_version != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported notes schema",
            ));
        }
        Ok(document)
    }

    pub fn list(&self) -> io::Result<Vec<Note>> {
        Ok(self.load()?.notes)
    }

    pub fn add(&self, text: &str) -> io::Result<Note> {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > 1024 || text.chars().any(char::is_control) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "notes require 1-1024 printable characters",
            ));
        }

        let mut document = self.load()?;
        let next_id = document
            .notes
            .iter()
            .map(|note| note.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| io::Error::other("note id overflow"))?;

        let note = Note {
            id: next_id,
            created_unix_s: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            text: text.to_owned(),
        };
        document.notes.push(note.clone());
        self.save(&document)?;
        Ok(note)
    }

    fn save(&self, document: &Document) -> io::Result<()> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid notes path"))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            let mut builder = fs::DirBuilder::new();
            builder.recursive(true).mode(0o700);
            builder.create(parent)?;
        }
        #[cfg(not(unix))]
        fs::create_dir_all(parent)?;

        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temporary = self
            .path
            .with_extension(format!("{}.{}.tmp", std::process::id(), suffix));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        let data = serde_json::to_vec_pretty(document).map_err(io::Error::other)?;
        file.write_all(&data)?;
        file.sync_all()?;
        drop(file);

        // Windows does not uniformly replace a pre-existing target via rename.
        // Its replacement is best-effort and must not be described as atomic.
        #[cfg(windows)]
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }

        match fs::rename(&temporary, &self.path) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = fs::remove_file(&temporary);
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_reloads_notes_locally() {
        let root = env::temp_dir().join(format!(
            "pipboy-notes-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = NoteStore::new(root.clone());

        assert!(store.list().unwrap().is_empty());
        let first = store.add("first observation").unwrap();
        let second = store.add("second observation").unwrap();
        assert_eq!(first.id, 1);
        assert_eq!(second.id, 2);
        assert_eq!(store.list().unwrap().len(), 2);
        assert!(store.add("\n").is_err());
        assert!(store.add(&"x".repeat(1025)).is_err());

        fs::remove_dir_all(root).unwrap();
    }
}

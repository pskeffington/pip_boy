//! Local plaintext journal with bounded reads, locked writes and staged replacement.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub id: u64,
    pub created_unix_s: u64,
    pub text: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema_version: u32,
    notes: Vec<Note>,
}

pub const NOTES_SCHEMA_VERSION: u32 = 1;
pub const MAX_NOTES: usize = 10_000;
pub const MAX_DOCUMENT_BYTES: u64 = 8 * 1024 * 1024;

impl Default for Document {
    fn default() -> Self {
        Self {
            schema_version: NOTES_SCHEMA_VERSION,
            notes: Vec::new(),
        }
    }
}

fn valid_text(text: &str) -> bool {
    !text.trim().is_empty() && text.chars().count() <= 1024 && !text.chars().any(char::is_control)
}

impl Document {
    fn validate(&self) -> io::Result<()> {
        if self.schema_version != NOTES_SCHEMA_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported notes schema; original file preserved",
            ));
        }
        if self.notes.len() > MAX_NOTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "notes document exceeds 10000 records",
            ));
        }
        let mut ids = HashSet::new();
        for note in &self.notes {
            if note.id == 0 || !ids.insert(note.id) || !valid_text(&note.text) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "notes contain duplicate/zero IDs or invalid text; original file preserved",
                ));
            }
        }
        Ok(())
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

    fn parent(&self) -> &Path {
        self.path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
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

    /// Keep this file in place: deleting it could split writers onto different locks.
    /// Closing the handle releases the OS lock, including after process termination.
    fn writer_lock(&self) -> io::Result<fs::File> {
        self.ensure_directory()?;
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(self.parent().join("notes.lock"))?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|error| {
            if error.kind() == io::ErrorKind::WouldBlock
                || error.raw_os_error() == fs2::lock_contended_error().raw_os_error()
            {
                io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "notes are being updated by another writer; retry",
                )
            } else {
                error
            }
        })?;
        Ok(lock)
    }

    fn load(&self) -> io::Result<Document> {
        let file = match fs::File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Document::default()),
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        file.take(MAX_DOCUMENT_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "notes document exceeds 8 MiB",
            ));
        }
        let document: Document = serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        document.validate()?;
        Ok(document)
    }

    /// Reads are side-effect free and see either the old or replaced document.
    pub fn list(&self) -> io::Result<Vec<Note>> {
        Ok(self.load()?.notes)
    }

    pub fn search(&self, query: &str) -> io::Result<Vec<Note>> {
        let query = query.trim().to_lowercase();
        if query.is_empty() || query.chars().count() > 1024 || query.chars().any(char::is_control) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "search requires 1-1024 printable characters",
            ));
        }
        Ok(self
            .list()?
            .into_iter()
            .filter(|note| note.text.to_lowercase().contains(&query))
            .collect())
    }

    pub fn add(&self, text: &str) -> io::Result<Note> {
        let text = text.trim();
        if !valid_text(text) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "notes require 1-1024 printable characters",
            ));
        }
        let _lock = self.writer_lock()?;
        // Lock the entire read-modify-replace transaction, not just its write.
        let mut document = self.load()?;
        if document.notes.len() >= MAX_NOTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "notes limit reached (10000 records)",
            ));
        }
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
        self.save_with(&document, |file, bytes| file.write_all(bytes))?;
        Ok(note)
    }

    fn save_with(
        &self,
        document: &Document,
        write: impl FnOnce(&mut fs::File, &[u8]) -> io::Result<()>,
    ) -> io::Result<()> {
        document.validate()?;
        let data = serde_json::to_vec_pretty(document).map_err(io::Error::other)?;
        if data.len() as u64 > MAX_DOCUMENT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "notes document exceeds 8 MiB",
            ));
        }
        let mut temporary = tempfile::Builder::new()
            .prefix(".notes-")
            .tempfile_in(self.parent())?;
        write(temporary.as_file_mut(), &data)?;
        temporary.as_file().sync_all()?;
        // Persist replaces the destination without deleting it first. Its RAII
        // owner removes temporary files on ordinary write/persist error paths.
        temporary.persist(&self.path).map_err(|error| error.error)?;
        #[cfg(unix)]
        fs::File::open(self.parent()).and_then(|directory| directory.sync_all()).map_err(|error| {
            io::Error::new(error.kind(), format!("notes committed, but directory durability could not be confirmed; list before retrying: {error}"))
        })?;
        Ok(())
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
    #[test]
    fn reads_and_search_do_not_create_directories() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("absent");
        let store = NoteStore::new(root.clone());
        assert!(store.list().unwrap().is_empty());
        assert!(store.search("fixture").unwrap().is_empty());
        assert!(store.add("\n").is_err());
        assert!(!root.exists());
    }

    #[test]
    fn case_insensitive_search_preserves_original_text_and_ids() {
        let temporary = tempfile::tempdir().unwrap();
        let store = NoteStore::new(temporary.path().to_owned());
        store.add("Battery checked at Plymouth").unwrap();
        store.add("Receiver inspection").unwrap();
        let before = fs::read(store.path()).unwrap();
        let found = store.search(" BATTERY ").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, 1);
        assert_eq!(found[0].text, "Battery checked at Plymouth");
        assert_eq!(fs::read(store.path()).unwrap(), before);
        assert!(store.search("").is_err());
        assert!(store.search("a\tb").is_err());
    }

    #[test]
    fn invalid_or_future_documents_are_preserved_on_read_and_add() {
        let temporary = tempfile::tempdir().unwrap();
        let store = NoteStore::new(temporary.path().to_owned());
        let note = r#"{"id":1,"created_unix_s":1234,"text":"fixture"}"#;
        let documents = [
            "{broken".to_owned(),
            r#"{"schema_version":2,"notes":[]}"#.to_owned(),
            format!(r#"{{"schema_version":1,"notes":[{note},{note}]}}"#),
            r#"{"schema_version":1,"notes":[{"id":0,"created_unix_s":1,"text":"fixture"}]}"#.to_owned(),
            r#"{"schema_version":1,"notes":[{"id":1,"created_unix_s":1,"text":"a\u001bb"}]}"#.to_owned(),
            r#"{"schema_version":1,"notes":[],"future_field":true}"#.to_owned(),
            r#"{"schema_version":1,"notes":[{"id":1,"created_unix_s":1,"text":"fixture","future":1}]}"#.to_owned(),
        ];
        for document in documents {
            fs::write(store.path(), &document).unwrap();
            assert_eq!(store.list().unwrap_err().kind(), io::ErrorKind::InvalidData);
            assert!(store.add("must not overwrite").is_err());
            assert_eq!(fs::read_to_string(store.path()).unwrap(), document);
        }
    }

    #[test]
    fn lock_covers_transaction_and_is_released_by_handle_drop() {
        let temporary = tempfile::tempdir().unwrap();
        let store = NoteStore::new(temporary.path().to_owned());
        store.add("original").unwrap();
        let before = fs::read(store.path()).unwrap();
        let lock = store.writer_lock().unwrap();
        assert_eq!(
            store.add("blocked").unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(store.list().unwrap().len(), 1); // Readers remain usable.
        assert_eq!(fs::read(store.path()).unwrap(), before);
        drop(lock);
        assert_eq!(store.add("after release").unwrap().id, 2);
        assert!(temporary.path().join("notes.lock").exists());
    }

    #[test]
    fn interrupted_staging_preserves_original_and_removes_temporary() {
        let temporary = tempfile::tempdir().unwrap();
        let store = NoteStore::new(temporary.path().to_owned());
        store.add("original").unwrap();
        let before = fs::read(store.path()).unwrap();
        let _lock = store.writer_lock().unwrap();
        let error = store
            .save_with(&store.load().unwrap(), |file, _| {
                file.write_all(b"partial")?;
                Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "fixture interruption",
                ))
            })
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert_eq!(fs::read(store.path()).unwrap(), before);
        assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 2); // Notes and persistent lock only.
    }

    #[test]
    fn failed_replacement_preserves_target_and_removes_temporary() {
        let temporary = tempfile::tempdir().unwrap();
        let store = NoteStore::new(temporary.path().to_owned());
        fs::create_dir(store.path()).unwrap(); // Unreplaceable destination.
        fs::write(store.path().join("sentinel"), b"preserve").unwrap();
        assert!(store
            .save_with(&Document::default(), |file, bytes| file.write_all(bytes))
            .is_err());
        assert_eq!(
            fs::read(store.path().join("sentinel")).unwrap(),
            b"preserve"
        );
        assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 1);
    }

    #[test]
    fn size_count_and_identifier_limits_fail_without_overwrite() {
        let temporary = tempfile::tempdir().unwrap();
        let store = NoteStore::new(temporary.path().to_owned());
        let large = fs::File::create(store.path()).unwrap();
        large.set_len(MAX_DOCUMENT_BYTES + 1).unwrap();
        drop(large);
        assert!(store.list().is_err());
        assert!(store.add("must not overwrite").is_err());
        assert_eq!(
            fs::metadata(store.path()).unwrap().len(),
            MAX_DOCUMENT_BYTES + 1
        );
        let overflow = format!(
            r#"{{"schema_version":1,"notes":[{{"id":{},"created_unix_s":1,"text":"fixture"}}]}}"#,
            u64::MAX
        );
        fs::write(store.path(), &overflow).unwrap();
        assert!(store.add("overflow").is_err());
        assert_eq!(fs::read_to_string(store.path()).unwrap(), overflow);
        let document = Document {
            schema_version: 1,
            notes: (1..=MAX_NOTES as u64)
                .map(|id| Note {
                    id,
                    created_unix_s: 1,
                    text: "fixture".into(),
                })
                .collect(),
        };
        fs::write(store.path(), serde_json::to_vec(&document).unwrap()).unwrap();
        let before = fs::read(store.path()).unwrap();
        assert!(store.add("too many").is_err());
        assert_eq!(fs::read(store.path()).unwrap(), before);
    }

    #[cfg(unix)]
    #[test]
    fn new_notes_and_lock_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("private");
        let store = NoteStore::new(root.clone());
        store.add("private fixture").unwrap();
        assert_eq!(
            fs::metadata(root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        for path in [store.path().to_owned(), store.parent().join("notes.lock")] {
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    #[test]
    fn abandoned_staging_file_is_never_promoted_or_used_as_notes() {
        let temporary = tempfile::tempdir().unwrap();
        let store = NoteStore::new(temporary.path().to_owned());
        store.add("committed").unwrap();
        let orphan = temporary.path().join(".notes-abandoned");
        fs::write(&orphan, b"partial uncommitted document").unwrap();
        assert_eq!(store.list().unwrap()[0].text, "committed");
        assert_eq!(store.add("next committed").unwrap().id, 2);
        assert_eq!(store.list().unwrap().len(), 2);
        assert_eq!(fs::read(orphan).unwrap(), b"partial uncommitted document");
    }
}

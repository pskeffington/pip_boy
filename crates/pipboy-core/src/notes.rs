//! Local plaintext journal with bounded reads, locked writes and staged replacement.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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
    fn read(reader: impl Read) -> io::Result<Self> {
        let mut bytes = Vec::new();
        reader
            .take(MAX_DOCUMENT_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "notes document exceeds 8 MiB",
            ));
        }
        let document: Self = serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        document.validate()?;
        Ok(document)
    }

    fn encoded(&self) -> io::Result<Vec<u8>> {
        self.validate()?;
        let data = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        if data.len() as u64 > MAX_DOCUMENT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "notes document exceeds 8 MiB",
            ));
        }
        Ok(data)
    }

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImportReport {
    pub imported: usize,
    pub skipped: usize,
    pub total: usize,
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
        Document::read(file)
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

    /// Validated schema-1 JSON; only the caller's writer receives output.
    /// No export file, data directory or lock is opened by this operation.
    pub fn export_to(&self, mut output: impl Write) -> io::Result<()> {
        output.write_all(&self.load()?.encoded()?)?;
        output.flush()
    }

    /// Merge validated records without replacing existing notes. Source IDs are
    /// local to the backup and are reassigned when records are appended here.
    /// For equal timestamp/text pairs retain the greater occurrence count from
    /// either collection, preserving repeats while making re-import idempotent.
    pub fn import_from(&self, input: impl Read) -> io::Result<ImportReport> {
        let incoming = Document::read(input)?; // Reject bad input before any write.
        if incoming.notes.is_empty() {
            return Ok(ImportReport {
                imported: 0,
                skipped: 0,
                total: self.load()?.notes.len(),
            });
        }
        let _lock = self.writer_lock()?;
        let mut document = self.load()?;
        let mut existing = HashMap::new();
        for note in &document.notes {
            *existing
                .entry((note.created_unix_s, note.text.clone()))
                .or_insert(0usize) += 1;
        }
        let mut seen = HashMap::new();
        let mut next_id = document.notes.iter().map(|note| note.id).max().unwrap_or(0);
        let mut imported = 0;
        let mut skipped = 0;
        for mut note in incoming.notes {
            let key = (note.created_unix_s, note.text.clone());
            let ordinal = seen.entry(key.clone()).or_insert(0usize);
            *ordinal += 1;
            if existing.get(&key).copied().unwrap_or(0) >= *ordinal {
                skipped += 1;
                continue;
            }
            if document.notes.len() >= MAX_NOTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "import would exceed 10000 notes; original file preserved",
                ));
            }
            next_id = next_id
                .checked_add(1)
                .ok_or_else(|| io::Error::other("note id overflow; original file preserved"))?;
            note.id = next_id;
            document.notes.push(note);
            imported += 1;
        }
        if imported > 0 {
            self.save_with(&document, |file, bytes| file.write_all(bytes))?;
        }
        Ok(ImportReport {
            imported,
            skipped,
            total: document.notes.len(),
        })
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
        let data = document.encoded()?;
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
    fn backup(notes: Vec<Note>) -> Vec<u8> {
        Document {
            schema_version: NOTES_SCHEMA_VERSION,
            notes,
        }
        .encoded()
        .unwrap()
    }

    #[test]
    fn export_import_preserves_content_and_reimport_does_not_rewrite() {
        let temporary = tempfile::tempdir().unwrap();
        let source = NoteStore::new(temporary.path().join("source"));
        source.add("alpha café").unwrap();
        source.add("beta").unwrap();
        let source_before = fs::read(source.path()).unwrap();
        let mut output = Vec::new();
        source.export_to(&mut output).unwrap();
        assert_eq!(fs::read(source.path()).unwrap(), source_before);
        let target = NoteStore::new(temporary.path().join("target"));
        target.add("existing unrelated note").unwrap();
        assert_eq!(
            target.import_from(output.as_slice()).unwrap(),
            ImportReport {
                imported: 2,
                skipped: 0,
                total: 3
            }
        );
        let merged = target.list().unwrap();
        assert_eq!(merged[1].id, 2);
        assert_eq!(merged[2].id, 3);
        assert_eq!(merged[1].text, "alpha café");
        assert_eq!(
            merged[1].created_unix_s,
            source.list().unwrap()[0].created_unix_s
        );
        let before = fs::read(target.path()).unwrap();
        assert_eq!(
            target.import_from(output.as_slice()).unwrap(),
            ImportReport {
                imported: 0,
                skipped: 2,
                total: 3
            }
        );
        assert_eq!(fs::read(target.path()).unwrap(), before);
    }

    #[test]
    fn merge_preserves_repeated_identical_notes_and_ignores_backup_ids() {
        let temporary = tempfile::tempdir().unwrap();
        let store = NoteStore::new(temporary.path().to_owned());
        let repeated = |id| Note {
            id,
            created_unix_s: 42,
            text: "same timestamp and text".into(),
        };
        fs::write(store.path(), backup(vec![repeated(1), repeated(9)])).unwrap();
        let input = backup(vec![repeated(100), repeated(200), repeated(u64::MAX)]);
        assert_eq!(
            store.import_from(input.as_slice()).unwrap(),
            ImportReport {
                imported: 1,
                skipped: 2,
                total: 3
            }
        );
        assert_eq!(
            store
                .list()
                .unwrap()
                .iter()
                .map(|note| note.id)
                .collect::<Vec<_>>(),
            vec![1, 9, 10]
        );
        assert_eq!(store.import_from(input.as_slice()).unwrap().imported, 0);
    }

    #[test]
    fn empty_exports_and_invalid_or_empty_imports_have_no_write_side_effects() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("absent");
        let store = NoteStore::new(root.clone());
        let mut empty = Vec::new();
        store.export_to(&mut empty).unwrap();
        assert!(Document::read(empty.as_slice()).unwrap().notes.is_empty());
        assert_eq!(store.import_from(empty.as_slice()).unwrap().total, 0);
        for input in [
            b"{broken".as_slice(),
            br#"{"schema_version":2,"notes":[]}"#.as_slice(),
            br#"{"schema_version":1,"notes":[{"id":0,"created_unix_s":1,"text":"bad"}]}"#
                .as_slice(),
        ] {
            assert!(store.import_from(input).is_err());
        }
        assert!(store
            .import_from(io::repeat(0).take(MAX_DOCUMENT_BYTES + 10))
            .is_err());
        assert!(!root.exists());
    }

    #[test]
    fn imports_obey_existing_writer_lock_and_preserve_canonical_errors() {
        let temporary = tempfile::tempdir().unwrap();
        let store = NoteStore::new(temporary.path().to_owned());
        store.add("original").unwrap();
        let input = backup(vec![Note {
            id: 1,
            created_unix_s: 2,
            text: "new".into(),
        }]);
        let before = fs::read(store.path()).unwrap();
        let lock = store.writer_lock().unwrap();
        assert_eq!(
            store.import_from(input.as_slice()).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(fs::read(store.path()).unwrap(), before);
        drop(lock);
        fs::write(store.path(), b"{broken").unwrap();
        assert!(store.import_from(input.as_slice()).is_err());
        assert_eq!(fs::read(store.path()).unwrap(), b"{broken");
    }

    #[test]
    fn import_capacity_and_id_overflow_are_all_or_nothing() {
        let temporary = tempfile::tempdir().unwrap();
        let store = NoteStore::new(temporary.path().to_owned());
        let note = |id| Note {
            id,
            created_unix_s: 1,
            text: "existing".into(),
        };
        let input = backup(vec![Note {
            id: 1,
            created_unix_s: 2,
            text: "new".into(),
        }]);
        for notes in [
            vec![note(u64::MAX)],
            (1..=MAX_NOTES as u64).map(note).collect(),
        ] {
            fs::write(store.path(), backup(notes)).unwrap();
            let before = fs::read(store.path()).unwrap();
            assert!(store.import_from(input.as_slice()).is_err());
            assert_eq!(fs::read(store.path()).unwrap(), before);
        }
    }

    #[test]
    fn combined_import_size_limit_preserves_previous_notes() {
        let temporary = tempfile::tempdir().unwrap();
        let store = NoteStore::new(temporary.path().to_owned());
        let records = |text: &str, count| {
            (1..=count)
                .map(|id| Note {
                    id,
                    created_unix_s: 1,
                    text: text.repeat(180),
                })
                .collect()
        };
        let original = backup(records("local", 5000));
        let input = backup(records("other", 4500));
        fs::write(store.path(), &original).unwrap();
        assert!(store.import_from(input.as_slice()).is_err());
        assert_eq!(fs::read(store.path()).unwrap(), original);
    }

    #[test]
    fn export_output_failures_never_change_notes() {
        let temporary = tempfile::tempdir().unwrap();
        let store = NoteStore::new(temporary.path().to_owned());
        store.add("original").unwrap();
        let before = fs::read(store.path()).unwrap();
        let mut short = [0u8; 4];
        assert_eq!(
            store.export_to(&mut short[..]).unwrap_err().kind(),
            io::ErrorKind::WriteZero
        );
        assert_eq!(fs::read(store.path()).unwrap(), before);
    }
}

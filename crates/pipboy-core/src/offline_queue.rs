//! P2 prototype: bounded, crash-atomic local queue for synthetic public-demo events.
//! NOT encrypted, network-facing, a trust ledger, or suitable for regulated data.
#![forbid(unsafe_code)]
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::PathBuf;
const LIMIT: u64 = 1_048_576;
const MAX_ITEMS: usize = 1024;
const MAX_BODY: usize = 2048;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QueuedEvent {
    pub source: String,
    pub sequence: u64,
    pub payload: String,
}
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema_version: u32,
    items: Vec<QueuedEvent>,
}
pub struct OfflineQueue {
    root: PathBuf,
}
impl OfflineQueue {
    pub fn new(root: PathBuf) -> Self { Self { root } }
    fn lock(&self) -> io::Result<File> {
        fs::create_dir_all(&self.root)?;
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.root, fs::Permissions::from_mode(0o700))?;
        }
        let file = OpenOptions::new().read(true).write(true).create(true)
            .open(self.root.join("offline-queue.lock"))?;
        file.lock_exclusive()?;
        Ok(file)
    }
    fn read(&self) -> io::Result<Journal> {
        let path = self.root.join("offline-queue.json");
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Journal { schema_version: 1, items: vec![] }),
            Err(e) => return Err(e),
        };
        if file.metadata()?.len() > LIMIT { return Err(io::Error::new(io::ErrorKind::InvalidData, "queue oversized")); }
        let mut data = Vec::new();
        file.take(LIMIT + 1).read_to_end(&mut data)?;
        if data.len() as u64 > LIMIT { return Err(io::Error::new(io::ErrorKind::InvalidData, "queue oversized")); }
        let journal: Journal = serde_json::from_slice(&data)?;
        if journal.schema_version != 1 || journal.items.len() > MAX_ITEMS || journal.items.iter().any(|e| !valid(e)) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "queue invalid"));
        }
        Ok(journal)
    }
    fn save(&self, journal: &Journal) -> io::Result<()> {
        let bytes = serde_json::to_vec(journal)?;
        if bytes.len() as u64 > LIMIT { return Err(io::Error::new(io::ErrorKind::OutOfMemory, "queue full")); }
        // Unique temporary file per write avoids corrupting the previous snapshot.
        let mut temp = tempfile::NamedTempFile::new_in(&self.root)?;
        temp.write_all(&bytes)?;
        temp.as_file().sync_all()?;
        temp.persist(self.root.join("offline-queue.json")).map_err(|e| e.error)?;
        File::open(&self.root)?.sync_all()?;
        Ok(())
    }
    pub fn enqueue(&self, event: QueuedEvent) -> io::Result<bool> {
        if !valid(&event) { return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid event")); }
        let _guard = self.lock()?;
        let mut journal = self.read()?;
        if let Some(found) = journal.items.iter().find(|e| e.source == event.source && e.sequence == event.sequence) {
            if found != &event { return Err(io::Error::new(io::ErrorKind::AlreadyExists, "event identity reused")); }
            return Ok(false);
        }
        if journal.items.len() == MAX_ITEMS { return Err(io::Error::new(io::ErrorKind::OutOfMemory, "queue full")); }
        journal.items.push(event);
        self.save(&journal)?;
        Ok(true)
    }
    pub fn pending(&self) -> io::Result<Vec<QueuedEvent>> {
        let _guard = self.lock()?;
        Ok(self.read()?.items)
    }
}
fn valid(e: &QueuedEvent) -> bool {
    (1..=48).contains(&e.source.len())
        && e.source.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        && e.sequence > 0 && !e.payload.is_empty() && e.payload.len() <= MAX_BODY
}
#[cfg(test)]
mod tests {
    use super::*;
    fn event(n: u64) -> QueuedEvent { QueuedEvent { source: "rb-one".into(), sequence: n, payload: "demo".into() } }
    #[test]
    fn restart_duplicate_and_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let q = OfflineQueue::new(dir.path().to_path_buf());
        assert!(q.enqueue(event(1)).unwrap());
        let fresh = OfflineQueue::new(dir.path().to_path_buf());
        assert_eq!(fresh.pending().unwrap(), vec![event(1)]);
        assert!(!fresh.enqueue(event(1)).unwrap());
        let mut changed = event(1);
        changed.payload = "conflicting".into();
        assert_eq!(fresh.enqueue(changed).unwrap_err().kind(), io::ErrorKind::AlreadyExists);
    }
    #[test]
    fn damaged_journal_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let q = OfflineQueue::new(dir.path().to_path_buf());
        q.enqueue(event(1)).unwrap();
        fs::write(dir.path().join("offline-queue.json"), b"{corrupt").unwrap();
        assert!(q.enqueue(event(2)).is_err());
        assert_eq!(fs::read(dir.path().join("offline-queue.json")).unwrap(), b"{corrupt");
    }
    #[test]
    fn rejects_bad_events() {
        let dir = tempfile::tempdir().unwrap();
        let q = OfflineQueue::new(dir.path().to_path_buf());
        assert!(q.enqueue(QueuedEvent { source: "../bad".into(), ..event(1) }).is_err());
        assert!(q.enqueue(QueuedEvent { payload: "x".repeat(MAX_BODY + 1), ..event(1) }).is_err());
    }
}

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
    #[serde(default)]
    receipts: Vec<ReceiptRecord>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReceiptRecord {
    pub source: String,
    pub sequence: u64,
    pub receiver: String,
    pub remote_id: u64,
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
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Journal { schema_version: 1, items: vec![], receipts: vec![] }),
            Err(e) => return Err(e),
        };
        if file.metadata()?.len() > LIMIT { return Err(io::Error::new(io::ErrorKind::InvalidData, "queue oversized")); }
        let mut data = Vec::new();
        file.take(LIMIT + 1).read_to_end(&mut data)?;
        if data.len() as u64 > LIMIT { return Err(io::Error::new(io::ErrorKind::InvalidData, "queue oversized")); }
        let journal: Journal = serde_json::from_slice(&data)?;
        if journal.schema_version != 1 || journal.items.len() > MAX_ITEMS || journal.receipts.len() > MAX_ITEMS
            || journal.items.iter().any(|e| !valid(e))
            || journal.receipts.iter().any(|r| !valid_receiver(&r.receiver) || r.remote_id == 0
                || !journal.items.iter().any(|e| e.source == r.source && e.sequence == r.sequence))
            || journal.receipts.iter().enumerate().any(|(i, r)| journal.receipts[..i].iter()
                .any(|prior| prior.source == r.source && prior.sequence == r.sequence)) {
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
        #[cfg(unix)]
        File::open(&self.root)?.sync_all()?;
        // Windows directory durability requires a separate field acceptance gate.
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
    /// Record a storage receipt only AFTER an authenticated transport independently
    /// validates peer identity and receipt binding. This method does not verify
    /// cryptographic signatures, so it is NOT an authorization or proof API.
    /// Retaining the event avoids reusing a sequence after restart.
    pub fn record_receipt(&self, source: &str, sequence: u64,
        receiver: &str, remote_id: u64) -> io::Result<bool> {
        if !valid_receiver(receiver) || remote_id == 0 || sequence == 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid receipt fields"));
        }
        let _guard = self.lock()?;
        let mut journal = self.read()?;
        if !journal.items.iter().any(|e| e.source == source && e.sequence == sequence) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "event not queued"));
        }
        if let Some(old) = journal.receipts.iter().find(|r| r.source == source && r.sequence == sequence) {
            if old.receiver != receiver || old.remote_id != remote_id {
                return Err(io::Error::new(io::ErrorKind::AlreadyExists, "conflicting receipt"));
            }
            return Ok(false);
        }
        journal.receipts.push(ReceiptRecord {source: source.to_owned(), sequence,
            receiver: receiver.to_owned(), remote_id});
        self.save(&journal)?;
        Ok(true)
    }
    pub fn receipts(&self) -> io::Result<Vec<ReceiptRecord>> {
        let _guard = self.lock()?;
        Ok(self.read()?.receipts)
    }
    pub fn pending(&self) -> io::Result<Vec<QueuedEvent>> {
        let _guard = self.lock()?;
        Ok(self.read()?.items)
    }
}
fn valid_receiver(s: &str) -> bool {
    (1..=48).contains(&s.len())
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
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
    fn receipt_records_survive_restart_and_reject_conflicts() {
        let dir = tempfile::tempdir().unwrap();
        let q = OfflineQueue::new(dir.path().to_path_buf());
        q.enqueue(event(1)).unwrap();
        assert!(q.record_receipt("rb-one", 1, "field-two", 13).unwrap());
        let restarted = OfflineQueue::new(dir.path().to_path_buf());
        assert_eq!(restarted.receipts().unwrap().len(), 1);
        assert!(!restarted.record_receipt("rb-one", 1, "field-two", 13).unwrap());
        assert_eq!(restarted.record_receipt("rb-one", 1, "field-two", 14)
            .unwrap_err().kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(restarted.record_receipt("rb-one", 2, "field-two", 14)
            .unwrap_err().kind(), io::ErrorKind::NotFound);
        assert!(!restarted.enqueue(event(1)).unwrap());
    }
    #[test]
    fn receipts_cannot_claim_invalid_peer_or_zero_remote_id() {
        let dir = tempfile::tempdir().unwrap();
        let q = OfflineQueue::new(dir.path().to_path_buf());
        q.enqueue(event(1)).unwrap();
        assert!(q.record_receipt("rb-one", 1, "../unsafe", 1).is_err());
        assert!(q.record_receipt("rb-one", 1, "field-two", 0).is_err());
        assert!(q.receipts().unwrap().is_empty());
    }
    #[test]
    fn rejects_bad_events() {
        let dir = tempfile::tempdir().unwrap();
        let q = OfflineQueue::new(dir.path().to_path_buf());
        assert!(q.enqueue(QueuedEvent { source: "../bad".into(), ..event(1) }).is_err());
        assert!(q.enqueue(QueuedEvent { payload: "x".repeat(MAX_BODY + 1), ..event(1) }).is_err());
    }
}

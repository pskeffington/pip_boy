//! Local-only synthetic event ledger for future RB sync v2.
//! No peer input, authorization, or encryption. File locking applies only to
//! cooperating writers on a trusted local filesystem.
//! Use only under a trusted per-device PIPBOY_HOME directory.
use crate::sync_delta::{EventHeader, PlanError, MAX_ORIGINS};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::Path;

pub const MAX_EVENTS: usize = 4096;
pub const MAX_LEDGER_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ledger {
    pub schema_version: u32,
    pub events: Vec<StoredEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredEvent {
    pub origin: String,
    pub sequence: u64,
    pub event_id: String,
}

impl From<&StoredEvent> for EventHeader {
    fn from(event: &StoredEvent) -> Self {
        Self {
            origin: event.origin.clone(),
            sequence: event.sequence,
            event_id: event.event_id.clone(),
        }
    }
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(ErrorKind::InvalidData, message)
}

impl Ledger {
    pub fn empty() -> Self {
        Self {
            schema_version: 1,
            events: Vec::new(),
        }
    }

    pub fn validate(&self) -> io::Result<()> {
        if self.schema_version != 1 || self.events.len() > MAX_EVENTS {
            return Err(invalid("unsupported or oversized ledger"));
        }
        let mut last = BTreeMap::<&str, u64>::new();
        let mut ids = BTreeSet::new();
        for event in &self.events {
            let header = EventHeader::from(event);
            // Reuse planner's ID validation, without assuming any transport trust.
            if crate::sync_delta::plan_batch(std::slice::from_ref(&header), &Default::default(), 1)
                .is_err_and(|e| {
                    matches!(e, PlanError::InvalidIdentifier | PlanError::InvalidSequence)
                })
            {
                return Err(invalid("invalid event header"));
            }
            if !ids.insert(event.event_id.as_str()) {
                return Err(invalid("duplicate event ID"));
            }
            let previous = last.get(event.origin.as_str()).copied().unwrap_or(0);
            if event.sequence != previous.saturating_add(1) || previous == u64::MAX {
                return Err(invalid("noncontiguous origin sequence"));
            }
            last.insert(&event.origin, event.sequence);
            if last.len() > MAX_ORIGINS {
                return Err(invalid("too many origins"));
            }
        }
        Ok(())
    }

    /// Append only an expected next sequence. Caller must serialize writers.
    pub fn append(&mut self, event: StoredEvent) -> io::Result<()> {
        if self.events.len() >= MAX_EVENTS {
            return Err(invalid("ledger capacity exceeded"));
        }
        let mut candidate = self.clone();
        candidate.events.push(event);
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    /// Refuses symlinks and malformed/truncated snapshots; never silently resets.
    pub fn load(path: &Path) -> io::Result<Self> {
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.file_type().is_file() || metadata.len() > MAX_LEDGER_BYTES {
            return Err(invalid("ledger file type or size rejected"));
        }
        let bytes = fs::read(path)?;
        if bytes.len() as u64 > MAX_LEDGER_BYTES {
            return Err(invalid("ledger too large"));
        }
        let ledger: Self =
            serde_json::from_slice(&bytes).map_err(|_| invalid("malformed ledger JSON"))?;
        ledger.validate()?;
        Ok(ledger)
    }

    /// Atomic replacement, not a multi-writer transaction. Parent directory must
    /// already exist and be operator-controlled; writer coordination is external.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        self.validate()?;
        let parent = path
            .parent()
            .ok_or_else(|| invalid("missing parent directory"))?;
        if !fs::symlink_metadata(parent)?.file_type().is_dir() {
            return Err(invalid("parent must be a directory"));
        }
        match fs::symlink_metadata(path) {
            Ok(m) if !m.file_type().is_file() => {
                return Err(invalid("existing target is not regular file"))
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(e),
            _ => {}
        }
        let bytes = serde_json::to_vec(self).map_err(|_| invalid("ledger serialization failed"))?;
        if bytes.len() as u64 > MAX_LEDGER_BYTES {
            return Err(invalid("ledger too large"));
        }
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(&bytes)?;
        temp.as_file().sync_all()?;
        temp.persist(path).map_err(|e| e.error)?;
        // Directory fsync makes the rename durable on supported Unix filesystems.
        #[cfg(unix)]
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    }
}

/// Append transaction for cooperating processes. A dedicated adjacent lock is
/// kept across load, validation, append and atomic save; never lock the snapshot
/// inode, because save replaces that inode. Existing corrupt ledgers fail closed.
/// This is not a remote lock, a protected key store, or an append-only audit log.
pub fn append_locked(path: &Path, event: StoredEvent, initialize: bool) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid("missing parent directory"))?;
    if !fs::symlink_metadata(parent)?.file_type().is_dir() {
        return Err(invalid("invalid parent directory"));
    }
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| invalid("invalid ledger filename"))?;
    if filename.is_empty() || filename == "." || filename == ".." {
        return Err(invalid("invalid ledger filename"));
    }
    let lock_path = parent.join(format!("{filename}.lock"));
    if let Ok(meta) = fs::symlink_metadata(&lock_path) {
        if !meta.file_type().is_file() {
            return Err(invalid("unsafe lock file type"));
        }
    }
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;
    if !lock.metadata()?.file_type().is_file() {
        return Err(invalid("unsafe lock file"));
    }
    lock.lock_exclusive()?;
    let result = (|| {
        let mut ledger = match Ledger::load(path) {
            Ok(existing) => existing,
            Err(error) if error.kind() == ErrorKind::NotFound && initialize => Ledger::empty(),
            Err(error) => return Err(error),
        };
        ledger.append(event)?;
        ledger.save(path)
    })();
    let unlock_result = FileExt::unlock(&lock);
    result.and(unlock_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn e(n: u64) -> StoredEvent {
        StoredEvent {
            origin: "unit-a".into(),
            sequence: n,
            event_id: format!("ev-{n}"),
        }
    }
    #[test]
    fn restart_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ledger.json");
        let mut ledger = Ledger::empty();
        ledger.append(e(1)).unwrap();
        ledger.save(&path).unwrap();
        let mut restored = Ledger::load(&path).unwrap();
        restored.append(e(2)).unwrap();
        restored.save(&path).unwrap();
        assert_eq!(Ledger::load(&path).unwrap().events, vec![e(1), e(2)]);
    }
    #[test]
    fn sequence_gap_and_duplicate_fail_without_mutation() {
        let mut ledger = Ledger::empty();
        assert!(ledger.append(e(2)).is_err());
        assert!(ledger.events.is_empty());
        ledger.append(e(1)).unwrap();
        assert!(ledger.append(e(1)).is_err());
        assert_eq!(ledger.events, vec![e(1)]);
    }
    #[test]
    fn truncated_snapshot_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ledger.json");
        fs::write(&path, b"{\"schema_version\":1,\"events\":[").unwrap();
        assert!(Ledger::load(&path).is_err());
    }
    #[test]
    fn missing_snapshot_is_not_initialized_implicitly() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            Ledger::load(&dir.path().join("missing"))
                .unwrap_err()
                .kind(),
            ErrorKind::NotFound
        );
    }
    #[test]
    fn locked_append_survives_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ledger.json");
        append_locked(&path, e(1), true).unwrap();
        append_locked(&path, e(2), false).unwrap();
        assert_eq!(Ledger::load(&path).unwrap().events, vec![e(1), e(2)]);
    }
    #[test]
    fn locked_append_never_repairs_corruption_implicitly() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ledger.json");
        fs::write(&path, b"not-json").unwrap();
        assert!(append_locked(&path, e(1), true).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"not-json");
    }
    #[test]
    fn locked_append_requires_explicit_initialization() {
        let dir = tempfile::tempdir().unwrap();
        assert!(append_locked(&dir.path().join("ledger.json"), e(1), false).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn symlink_snapshot_rejected() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("real"), b"{}").unwrap();
        symlink(dir.path().join("real"), dir.path().join("link")).unwrap();
        assert!(Ledger::load(&dir.path().join("link")).is_err());
    }
}

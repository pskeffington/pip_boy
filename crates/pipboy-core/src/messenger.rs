//! Rust-Boy Messenger: bounded, local device-to-device message log.
//!
//! Transport authentication and encryption are delegated to pinned OpenSSH.
//! Existing journals are plaintext; fresh journals can enroll age + OS key storage.
//! Neither mode is an end-to-end encryption protocol.
//! Peer identity must be enforced by the restricted SSH bridge, not labels alone.
use crate::message_keys;
use crate::tasks::valid_device_id;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;

pub const SCHEMA: u32 = 1;
const MAX_FILE: u64 = 256 * 1024;
const MAX_MESSAGES: usize = 192;
pub const MAX_TEXT_BYTES: usize = 1024;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

pub fn valid_text(body: &str) -> bool {
    let trimmed = body.trim();
    !trimmed.is_empty()
        && body.len() <= MAX_TEXT_BYTES
        && !body.chars().any(|ch| {
            (ch.is_control() && !matches!(ch, '\n' | '\t'))
                || matches!(ch,
                    '\u{061c}' | '\u{200e}' | '\u{200f}' |
                    '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
}

pub fn unix_time() -> io::Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_secs())
        .map_err(|_| invalid("System clock precedes Unix epoch"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Outgoing,
    Incoming,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Delivery {
    Queued,
    Stored,
    Received,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub id: u64,
    pub peer: String,
    pub direction: Direction,
    pub body: String,
    pub created_at: u64,
    pub delivery: Delivery,
    pub remote_origin_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_sent_at: Option<u64>,
    pub remote_stored_id: Option<u64>,
}

impl Message {
    fn valid(&self) -> bool {
        self.id > 0
            && valid_device_id(&self.peer)
            && valid_text(&self.body)
            && match (self.direction, self.delivery) {
                (Direction::Incoming, Delivery::Received) => {
                    self.remote_origin_id.is_some_and(|id| id > 0)
                        && self.remote_stored_id.is_none()
                }
                (Direction::Outgoing, Delivery::Queued) => {
                    self.remote_origin_id.is_none() && self.remote_stored_id.is_none()
                }
                (Direction::Outgoing, Delivery::Stored) => {
                    self.remote_origin_id.is_none()
                        && self.remote_stored_id.is_some_and(|id| id > 0)
                }
                _ => false,
            }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessageBook {
    pub schema_version: u32,
    pub next_id: u64,
    pub messages: Vec<Message>,
}

impl Default for MessageBook {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA,
            next_id: 1,
            messages: Vec::new(),
        }
    }
}

impl MessageBook {
    fn validate(&self) -> io::Result<()> {
        if self.schema_version != SCHEMA || self.next_id == 0 || self.messages.len() > MAX_MESSAGES
        {
            return Err(invalid(
                "Unsupported or oversized Messenger log; original preserved",
            ));
        }
        let mut seen = HashSet::new();
        let mut incoming = HashSet::new();
        for message in &self.messages {
            if !message.valid() || !seen.insert(message.id) || message.id >= self.next_id {
                return Err(invalid("Invalid Messenger message; original preserved"));
            }
            if let Some(origin) = message.remote_origin_id {
                if !incoming.insert((message.peer.as_str(), origin)) {
                    return Err(invalid("Duplicate incoming message identity"));
                }
            }
        }
        Ok(())
    }

    fn read(input: impl Read) -> io::Result<Self> {
        let mut bytes = Zeroizing::new(Vec::new());
        input.take(MAX_FILE + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_FILE {
            return Err(invalid("Messenger journal exceeds bound"));
        }
        let result: Self = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("Messenger journal invalid; original preserved"))?;
        result.validate()?;
        Ok(result)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteMessage {
    pub schema_version: u32,
    pub from_device: String,
    pub to_device: String,
    pub origin_id: u64,
    pub body: String,
    pub sent_at: u64,
}

impl RemoteMessage {
    pub fn valid(&self) -> bool {
        self.schema_version == SCHEMA
            && valid_device_id(&self.from_device)
            && valid_device_id(&self.to_device)
            && self.from_device != self.to_device
            && self.origin_id > 0
            && valid_text(&self.body)
    }
}

pub struct MessageStore {
    path: PathBuf,
    key: Option<age::x25519::Identity>,
}

impl MessageStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            path: root.join("messages.json"),
            key: None,
        }
    }

    /// Explicit identity injection for a caller with a secure key provider.
    pub fn with_key(root: PathBuf, identity: age::x25519::Identity) -> Self {
        Self {
            path: root.join("messages.json"),
            key: Some(identity),
        }
    }

    fn identity(&self) -> io::Result<Option<age::x25519::Identity>> {
        match &self.key {
            Some(key) => Ok(Some(key.clone())),
            None => message_keys::unlock(self.directory()),
        }
    }

    pub fn storage_mode(&self) -> io::Result<&'static str> {
        Ok(if self.identity()?.is_some() {
            "age_os_keyring"
        } else {
            "plaintext_local_only"
        })
    }

    pub fn enroll_encryption(&self) -> io::Result<String> {
        message_keys::enroll(self.directory())
    }

    pub fn from_environment() -> io::Result<Self> {
        let root = if let Some(home) = env::var_os("PIPBOY_HOME").filter(|x| !x.is_empty()) {
            PathBuf::from(home)
        } else {
            let home = env::var_os("HOME")
                .or_else(|| env::var_os("USERPROFILE"))
                .ok_or_else(|| invalid("Home directory unavailable"))?;
            PathBuf::from(home).join(".pipboy")
        };
        Ok(Self::new(root))
    }

    fn directory(&self) -> &Path {
        self.path.parent().unwrap_or_else(|| Path::new("."))
    }

    pub fn load(&self) -> io::Result<MessageBook> {
        let identity = self.identity()?;
        match File::open(&self.path) {
            Ok(mut file) => {
                if let Some(identity) = identity {
                    let mut encrypted = Vec::new();
                    (&mut file)
                        .take(MAX_FILE + 4097)
                        .read_to_end(&mut encrypted)?;
                    if encrypted.len() as u64 > MAX_FILE + 4096 {
                        return Err(invalid("Encrypted journal exceeds bound"));
                    }
                    let plaintext = message_keys::decrypt(&identity, &encrypted, MAX_FILE)?;
                    MessageBook::read(plaintext.as_slice())
                } else {
                    MessageBook::read(file)
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(MessageBook::default()),
            Err(error) => Err(error),
        }
    }

    fn lock(&self) -> io::Result<File> {
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
        options.create(true).read(true).write(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(self.directory().join("messages.lock"))?;
        fs2::FileExt::try_lock_exclusive(&file)?;
        Ok(file)
    }

    fn save(&self, book: &MessageBook) -> io::Result<()> {
        book.validate()?;
        let plaintext = Zeroizing::new(serde_json::to_vec(book)?);
        let bytes = match self.identity()? {
            Some(identity) => message_keys::encrypt(&identity, &plaintext)?,
            None => plaintext.to_vec(),
        };
        if plaintext.len() as u64 > MAX_FILE {
            return Err(invalid("Messenger storage limit reached"));
        }
        let mut tmp = tempfile::Builder::new()
            .prefix(".messages-")
            .tempfile_in(self.directory())?;
        tmp.write_all(&bytes)?;
        tmp.as_file().sync_all()?;
        tmp.persist(&self.path).map_err(|error| error.error)?;
        #[cfg(unix)]
        File::open(self.directory())?.sync_all()?;
        Ok(())
    }

    pub fn queue(&self, peer: &str, body: &str) -> io::Result<Message> {
        if !valid_device_id(peer) || !valid_text(body) {
            return Err(invalid("Invalid message recipient or text"));
        }
        let _guard = self.lock()?;
        let mut book = self.load()?;
        if book.messages.len() == MAX_MESSAGES || book.next_id == u64::MAX {
            return Err(invalid(
                "Messenger journal full; export/retention controls not released",
            ));
        }
        let message = Message {
            id: book.next_id,
            peer: peer.to_owned(),
            direction: Direction::Outgoing,
            body: body.to_owned(),
            created_at: unix_time()?,
            delivery: Delivery::Queued,
            remote_origin_id: None,
            remote_sent_at: None,
            remote_stored_id: None,
        };
        book.next_id += 1;
        book.messages.push(message.clone());
        self.save(&book)?;
        Ok(message)
    }

    pub fn outgoing(&self, id: u64, peer: &str) -> io::Result<Message> {
        if !valid_device_id(peer) || id == 0 {
            return Err(invalid("Invalid outgoing message selector"));
        }
        self.load()?
            .messages
            .into_iter()
            .find(|message| {
                message.id == id && message.peer == peer && message.direction == Direction::Outgoing
            })
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Queued message not found"))
    }

    pub fn mark_stored(&self, id: u64, peer: &str, remote_id: u64) -> io::Result<Message> {
        if id == 0 || remote_id == 0 || !valid_device_id(peer) {
            return Err(invalid("Invalid remote storage acknowledgment"));
        }
        let _guard = self.lock()?;
        let mut book = self.load()?;
        let message = book
            .messages
            .iter_mut()
            .find(|entry| {
                entry.id == id && entry.peer == peer && entry.direction == Direction::Outgoing
            })
            .ok_or_else(|| invalid("Message to acknowledge not found"))?;
        if message.delivery == Delivery::Stored {
            if message.remote_stored_id == Some(remote_id) {
                return Ok(message.clone());
            }
            return Err(invalid("Conflicting remote message receipt"));
        }
        if message.delivery != Delivery::Queued {
            return Err(invalid("Message is not pending transport"));
        }
        message.delivery = Delivery::Stored;
        message.remote_stored_id = Some(remote_id);
        let result = message.clone();
        self.save(&book)?;
        Ok(result)
    }

    pub fn receive(
        &self,
        frame: &RemoteMessage,
        local_device: &str,
    ) -> io::Result<(Message, bool)> {
        if !frame.valid() || frame.to_device != local_device {
            return Err(invalid("Invalid remote message or recipient device"));
        }
        let _guard = self.lock()?;
        let mut book = self.load()?;
        if let Some(existing) = book.messages.iter().find(|entry| {
            entry.direction == Direction::Incoming
                && entry.peer == frame.from_device
                && entry.remote_origin_id == Some(frame.origin_id)
        }) {
            if existing.body == frame.body
                && existing
                    .remote_sent_at
                    .is_none_or(|time| time == frame.sent_at)
            {
                return Ok((existing.clone(), false)); // idempotent retry
            }
            return Err(invalid("Conflicting retransmission of remote message"));
        }
        if book.messages.len() == MAX_MESSAGES || book.next_id == u64::MAX {
            return Err(invalid(
                "Message journal full; receiver did not store frame",
            ));
        }
        let message = Message {
            id: book.next_id,
            peer: frame.from_device.clone(),
            direction: Direction::Incoming,
            body: frame.body.clone(),
            created_at: unix_time()?, // local arrival timestamp; sender time untrusted
            delivery: Delivery::Received,
            remote_origin_id: Some(frame.origin_id),
            remote_sent_at: Some(frame.sent_at),
            remote_stored_id: None,
        };
        book.next_id += 1;
        book.messages.push(message.clone());
        self.save(&book)?;
        Ok((message, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypted_restart_wrong_key_tampering_and_no_plaintext_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let key = age::x25519::Identity::generate();
        let store = MessageStore::with_key(dir.path().into(), key.clone());
        store.queue("scout-one", "secret rendezvous").unwrap();
        let bytes = fs::read(&store.path).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("secret rendezvous"));
        let reopened = MessageStore::with_key(dir.path().into(), key);
        assert_eq!(
            reopened.load().unwrap().messages[0].body,
            "secret rendezvous"
        );
        let wrong = MessageStore::with_key(dir.path().into(), age::x25519::Identity::generate());
        assert!(wrong.load().is_err());
        assert!(wrong.queue("scout-one", "cannot overwrite").is_err());
        assert_eq!(fs::read(&store.path).unwrap(), bytes);
        let mut corrupt = bytes;
        let last = corrupt.len() - 1;
        corrupt[last] ^= 1;
        fs::write(&store.path, &corrupt).unwrap();
        assert!(reopened.load().is_err());
        assert!(MessageStore::new(dir.path().into()).load().is_err());
        assert_eq!(fs::read(&store.path).unwrap(), corrupt);
    }

    #[test]
    fn existing_plaintext_and_bad_marker_refuse_enrollment_or_downgrade() {
        let dir = tempfile::tempdir().unwrap();
        let store = MessageStore::new(dir.path().into());
        store.queue("scout-one", "existing").unwrap();
        assert!(store.enroll_encryption().is_err());
        fs::write(dir.path().join("messages.recipient"), "invalid").unwrap();
        assert!(store.load().is_err());
        assert!(store.queue("scout-one", "no fallback").is_err());
    }

    #[test]
    fn strictly_bounded_text_no_terminal_controls_or_spoofed_direction() {
        assert!(valid_text("Hello world\nREADY"));
        assert!(valid_text("中文 • Привет"));
        for s in ["", "  ", "\u{1b}[31mspoof", "hello\u{0007}", "x\u{202e}txt"] {
            assert!(!valid_text(s));
        }
        assert!(!valid_text(&"x".repeat(MAX_TEXT_BYTES + 1)));
    }

    #[test]
    fn outbound_requires_explicit_ack_and_inbound_retransmits_idempotently() {
        let dir = tempfile::tempdir().unwrap();
        let store = MessageStore::new(dir.path().join("private"));
        assert_eq!(store.load().unwrap().next_id, 1);
        assert!(!dir.path().join("private").exists());
        let pending = store.queue("scout-one", "North station checked").unwrap();
        assert_eq!(pending.delivery, Delivery::Queued);
        assert_eq!(pending.id, 1);
        assert_eq!(store.outgoing(1, "scout-one").unwrap().body, pending.body);
        assert!(store.outgoing(1, "not-the-peer").is_err());
        let marked = store.mark_stored(1, "scout-one", 8).unwrap();
        assert_eq!(marked.delivery, Delivery::Stored);
        assert_eq!(marked.remote_stored_id, Some(8));
        assert_eq!(store.mark_stored(1, "scout-one", 8).unwrap().id, 1);
        assert!(store.mark_stored(1, "scout-one", 9).is_err());

        let frame = RemoteMessage {
            schema_version: SCHEMA,
            from_device: "overseer-a".into(),
            to_device: "scout-one".into(),
            origin_id: 15,
            body: "Arrived".into(),
            sent_at: 1,
        };
        assert!(store.receive(&frame, "not-scout").is_err());
        let (received, new) = store.receive(&frame, "scout-one").unwrap();
        assert!(new);
        assert_eq!(received.delivery, Delivery::Received);
        assert_eq!(received.remote_origin_id, Some(15));
        assert!(!store.receive(&frame, "scout-one").unwrap().1);
        let mut changed_time = frame.clone();
        changed_time.sent_at += 1;
        assert!(store.receive(&changed_time, "scout-one").is_err());
        let mut altered = frame;
        altered.body = "Changed after first delivery".into();
        assert!(store.receive(&altered, "scout-one").is_err());
        assert_eq!(store.load().unwrap().messages.len(), 2);
    }

    #[test]
    fn invalid_existing_file_is_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let store = MessageStore::new(dir.path().to_path_buf());
        fs::write(&store.path, b"{corrupted").unwrap();
        assert!(store.load().is_err());
        assert!(store.queue("scout-one", "Do not overwrite").is_err());
        assert_eq!(fs::read(&store.path).unwrap(), b"{corrupted");
        fs::write(
            &store.path,
            b"{\"schema_version\":9,\"next_id\":1,\"messages\":[]}",
        )
        .unwrap();
        assert!(store.load().is_err());
    }

    #[test]
    fn invalid_frame_and_rejected_premature_receipts_do_not_modify_store() {
        let dir = tempfile::tempdir().unwrap();
        let store = MessageStore::new(dir.path().join("vault"));
        let mut packet = RemoteMessage {
            schema_version: 1,
            from_device: "overseer-a".into(),
            to_device: "scout-one".into(),
            origin_id: 1,
            body: "Command received".into(),
            sent_at: 1,
        };
        packet.from_device = "scout-one".into();
        assert!(store.receive(&packet, "scout-one").is_err());
        packet.from_device = "overseer-a".into();
        packet.body = "\u{1b}bad".into();
        assert!(store.receive(&packet, "scout-one").is_err());
        assert!(!dir.path().join("vault/messages.json").exists());
    }
}

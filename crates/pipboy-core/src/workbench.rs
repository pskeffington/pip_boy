//! User-owned, declarative workbench. Pinning a tool never runs it or grants permissions.
//! A missing file represents a deliberately empty workbench, without disk writes.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_SLOTS: usize = 24;
pub const MAX_BYTES: u64 = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Ready,
    Simulated,
    ExplicitConnection,
    Planned,
}

impl Availability {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready / local",
            Self::Simulated => "simulated only",
            Self::ExplicitConnection => "explicit opt-in",
            Self::Planned => "planned / not available",
        }
    }

    pub fn can_pin(self) -> bool {
        self != Self::Planned
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Tool {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub availability: Availability,
    pub command: &'static str,
}

/// Built-in descriptors are navigation/metadata, not automatically executed plugins.
pub const PALETTE: &[Tool] = &[
    Tool {
        id: "byte-lab",
        name: "BYTE LAB",
        description: "Bounded hexadecimal inspection and UTF-8 to hex conversion. No I/O.",
        availability: Availability::Ready,
        command: "pipboy workbench bytes inspect 48656c6c6f",
    },
    Tool {
        id: "host",
        name: "HOST STAT",
        description: "Opt-in local CPU/RAM and field quality. No collection by pinning.",
        availability: Availability::Ready,
        command: "pipboy status --details",
    },
    Tool {
        id: "library",
        name: "LIBRARY",
        description: "Bundled research and open-source references. Fully offline.",
        availability: Availability::Ready,
        command: "pipboy sources rust",
    },
    Tool {
        id: "notes",
        name: "NOTES",
        description: "Local plaintext research journal; do not store sensitive material.",
        availability: Availability::Ready,
        command: "pipboy notes list",
    },
    Tool {
        id: "simulator",
        name: "DEVICE SIM",
        description: "Fixed synthetic fixtures for experiment and adapter development.",
        availability: Availability::Simulated,
        command: "pipboy simulate list",
    },
    Tool {
        id: "game",
        name: "GAME LINK",
        description: "Fallout 4 companion browser. Direct LAN connection needs --confirm.",
        availability: Availability::ExplicitConnection,
        command: "pipboy game --ps4 IP --confirm",
    },
    Tool {
        id: "gpio",
        name: "GPIO",
        description: "Physical pin access requires a future OS-enforced broker.",
        availability: Availability::Planned,
        command: "not implemented",
    },
    Tool {
        id: "radio",
        name: "RADIO/SDR",
        description: "Physical RF receive/transmit requires explicit hardware gates.",
        availability: Availability::Planned,
        command: "not implemented",
    },
    Tool {
        id: "ble",
        name: "BLE",
        description: "Bluetooth discovery and transport are not available.",
        availability: Availability::Planned,
        command: "not implemented",
    },
    Tool {
        id: "nfc",
        name: "NFC/RFID",
        description: "Card/radio access is not available; no device I/O is enabled.",
        availability: Availability::Planned,
        command: "not implemented",
    },
    Tool {
        id: "mesh",
        name: "MESH",
        description: "Mesh transport and physical radio gateways remain unimplemented.",
        availability: Availability::Planned,
        command: "not implemented",
    },
];

pub fn tool(id: &str) -> Option<&'static Tool> {
    PALETTE.iter().find(|entry| entry.id == id)
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub id: u64,
    pub tool: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Workbench {
    pub schema_version: u32,
    pub title: String,
    pub slots: Vec<Slot>,
}

impl Default for Workbench {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            title: "My Rust-Boy Workbench".to_string(),
            slots: Vec::new(),
        }
    }
}

fn valid_title(title: &str) -> bool {
    !title.is_empty()
        && title.trim() == title
        && title.chars().count() <= 64
        && !title.chars().any(char::is_control)
}

impl Workbench {
    pub fn validate(&self) -> io::Result<()> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported workbench schema; original file preserved",
            ));
        }
        if !valid_title(&self.title) || self.slots.len() > MAX_SLOTS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid title or too many workbench slots; original file preserved",
            ));
        }
        let mut ids = HashSet::new();
        let mut tools = HashSet::new();
        for slot in &self.slots {
            if slot.id == 0
                || !ids.insert(slot.id)
                || !tools.insert(slot.tool.as_str())
                || !tool(&slot.tool).is_some_and(|entry| entry.availability.can_pin())
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "unknown, duplicate or unavailable workbench slot; original file preserved",
                ));
            }
        }
        Ok(())
    }

    fn read(reader: impl Read) -> io::Result<Self> {
        let mut bytes = Vec::new();
        reader.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "workbench exceeds 64 KiB"));
        }
        let board: Self = serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        board.validate()?;
        Ok(board)
    }

    fn encoded(&self) -> io::Result<Vec<u8>> {
        self.validate()?;
        let data = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        if data.len() as u64 > MAX_BYTES {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "workbench exceeds 64 KiB"));
        }
        Ok(data)
    }
}

pub struct WorkbenchStore {
    path: PathBuf,
}

impl WorkbenchStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            path: root.join("workbench.json"),
        }
    }

    pub fn from_environment() -> io::Result<Self> {
        // Reuse the established application data root without opening notes.
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

    pub fn path(&self) -> &Path {
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

    fn writer_lock(&self) -> io::Result<fs::File> {
        self.ensure_directory()?;
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(self.parent().join("workbench.lock"))?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|error| {
            if error.kind() == io::ErrorKind::WouldBlock
                || error.raw_os_error() == fs2::lock_contended_error().raw_os_error()
            {
                io::Error::new(io::ErrorKind::WouldBlock, "workbench is in use by another writer")
            } else {
                error
            }
        })?;
        Ok(lock)
    }

    /// A missing file is the empty default; reading never creates files.
    pub fn load(&self) -> io::Result<Workbench> {
        match fs::File::open(&self.path) {
            Ok(file) => Workbench::read(file),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Workbench::default()),
            Err(error) => Err(error),
        }
    }

    pub fn add(&self, id: &str) -> io::Result<Slot> {
        if !tool(id).is_some_and(|entry| entry.availability.can_pin()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown or unavailable tool; inspect workbench palette",
            ));
        }
        let _lock = self.writer_lock()?;
        let mut board = self.load()?;
        if board.slots.iter().any(|slot| slot.tool == id) {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, "tool is already pinned"));
        }
        if board.slots.len() >= MAX_SLOTS {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "workbench is full"));
        }
        let next_id = board
            .slots
            .iter()
            .map(|slot| slot.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| io::Error::other("workbench slot ID overflow"))?;
        let slot = Slot {
            id: next_id,
            tool: id.to_owned(),
        };
        board.slots.push(slot.clone());
        self.save(&board)?;
        Ok(slot)
    }

    pub fn remove(&self, id: u64) -> io::Result<bool> {
        if !self.load()?.slots.iter().any(|slot| slot.id == id) {
            return Ok(false);
        }
        let _lock = self.writer_lock()?;
        let mut board = self.load()?;
        let before = board.slots.len();
        board.slots.retain(|slot| slot.id != id);
        if board.slots.len() == before {
            return Ok(false);
        }
        self.save(&board)?;
        Ok(true)
    }

    pub fn rename(&self, title: &str) -> io::Result<bool> {
        if !valid_title(title) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "workbench title must be 1-64 printable characters without edge spaces",
            ));
        }
        if self.load()?.title == title {
            return Ok(false);
        }
        let _lock = self.writer_lock()?;
        let mut board = self.load()?;
        if board.title == title {
            return Ok(false);
        }
        board.title = title.to_string();
        self.save(&board)?;
        Ok(true)
    }

    pub fn clear(&self) -> io::Result<bool> {
        if self.load()?.slots.is_empty() {
            return Ok(false);
        }
        let _lock = self.writer_lock()?;
        let mut board = self.load()?;
        if board.slots.is_empty() {
            return Ok(false);
        }
        board.slots.clear();
        self.save(&board)?;
        Ok(true)
    }

    fn save(&self, board: &Workbench) -> io::Result<()> {
        let bytes = board.encoded()?;
        let mut staging = tempfile::Builder::new()
            .prefix(".workbench-")
            .tempfile_in(self.parent())?;
        staging.as_file_mut().write_all(&bytes)?;
        staging.as_file().sync_all()?;
        staging.persist(&self.path).map_err(|error| error.error)?;
        #[cfg(unix)]
        fs::File::open(self.parent())?.sync_all()?;
        Ok(())
    }
}

/// A bounded, printable-only hex dump suitable for terminal inspection.
pub fn inspect_hex(input: &str) -> io::Result<String> {
    let mut data = Vec::new();
    for token in input.split_whitespace() {
        let part = token
            .strip_prefix("0x")
            .or_else(|| token.strip_prefix("0X"))
            .unwrap_or(token);
        if part.is_empty() || part.len() % 2 != 0 || !part.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "enter paired hex bytes, optionally separated by spaces or prefixed 0x",
            ));
        }
        for pair in part.as_bytes().chunks_exact(2) {
            if data.len() >= 4096 {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "max 4096 bytes"));
            }
            let digit = std::str::from_utf8(pair).expect("validated ASCII");
            data.push(u8::from_str_radix(digit, 16).expect("validated hex digits"));
        }
    }
    if data.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "hex input cannot be empty"));
    }
    let mut output = format!("BYTE LAB | {} bytes\nOFFSET    HEX                                              ASCII\n", data.len());
    for (index, chunk) in data.chunks(16).enumerate() {
        let _ = write!(&mut output, "{:08x}  ", index * 16);
        for byte in chunk {
            let _ = write!(&mut output, "{byte:02x} ");
        }
        for _ in chunk.len()..16 {
            output.push_str("   ");
        }
        output.push(' ');
        for byte in chunk {
            output.push(if (0x20..=0x7e).contains(byte) {
                *byte as char
            } else {
                '.'
            });
        }
        output.push('\n');
    }
    Ok(output)
}

pub fn encode_hex(text: &str) -> io::Result<String> {
    if text.is_empty() || text.len() > 4096 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "text must be 1..4096 bytes"));
    }
    Ok(text
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_slate_is_empty_and_has_no_read_side_effects() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("new");
        let store = WorkbenchStore::new(root.clone());
        let board = store.load().unwrap();
        assert_eq!(board.schema_version, 1);
        assert!(board.slots.is_empty());
        assert!(!root.exists());
        assert!(!store.remove(77).unwrap());
        assert!(!store.clear().unwrap());
        assert!(!root.exists());
    }

    #[test]
    fn pin_unpin_and_rename_are_local_and_persistent() {
        let temporary = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temporary.path().join("state"));
        assert!(store.add("radio").is_err());
        assert!(!store.path().exists());
        let pinned = store.add("byte-lab").unwrap();
        assert_eq!(pinned.id, 1);
        assert_eq!(store.load().unwrap().slots, vec![pinned.clone()]);
        assert!(store.add("byte-lab").is_err());
        assert!(store.rename("Field Lab").unwrap());
        assert!(!store.rename("Field Lab").unwrap());
        assert!(store.remove(pinned.id).unwrap());
        assert!(!store.remove(pinned.id).unwrap());
        assert!(store.load().unwrap().slots.is_empty());
        assert_eq!(store.load().unwrap().title, "Field Lab");
        let again = store.add("game").unwrap();
        assert_eq!(again.id, 1);
        assert!(store.clear().unwrap());
        assert!(!store.clear().unwrap());
    }

    #[test]
    fn corrupt_future_and_unauthorized_manifests_remain_untouched() {
        let temporary = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temporary.path().to_path_buf());
        for payload in [
            "{oops".to_string(),
            r#"{"schema_version":100,"title":"Bad","slots":[]}"#.to_string(),
            r#"{"schema_version":1,"title":"Bad","slots":[{"id":1,"tool":"gpio"}]}"#.to_string(),
            r#"{"schema_version":1,"title":"Bad","slots":[{"id":1,"tool":"notes"},{"id":1,"tool":"notes"}]}"#.to_string(),
            r#"{"schema_version":1,"title":"Bad","slots":[],"execute":"sh"}"#.to_string(),
        ] {
            fs::write(store.path(), &payload).unwrap();
            assert!(store.load().is_err());
            assert!(store.add("notes").is_err());
            assert_eq!(fs::read_to_string(store.path()).unwrap(), payload);
        }
    }

    #[test]
    fn file_and_lock_are_private_on_unix() {
        let temporary = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temporary.path().join("private"));
        store.add("simulator").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(store.path()).unwrap().permissions().mode() & 0o777, 0o600);
            assert_eq!(
                fs::metadata(store.path().parent().unwrap())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
        }
    }

    #[test]
    fn hex_tools_are_bounded_and_never_emit_escape_bytes() {
        let dump = inspect_hex("0x48 65 6c6c6f 00 1b").unwrap();
        assert!(dump.contains("Hello.."));
        assert!(dump.contains("00000000"));
        assert_eq!(encode_hex("Hi").unwrap(), "48 69");
        assert_eq!(encode_hex("é").unwrap(), "c3 a9");
        for invalid in ["", "1", "0x", "gg", "0x1", "00 0Xf"] {
            assert!(inspect_hex(invalid).is_err(), "{invalid}");
        }
        assert!(inspect_hex(&"aa".repeat(4097)).is_err());
        assert!(encode_hex(&"x".repeat(4097)).is_err());
        assert!(!dump.contains('\u{1b}'));
    }
}

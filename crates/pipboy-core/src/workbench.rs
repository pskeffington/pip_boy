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
        id: "net-lab",
        name: "NET LAB",
        description: "Offline IPv4/CIDR planner for LAN setup; no probing or sockets.",
        availability: Availability::Ready,
        command: "pipboy workbench net cidr 192.168.1.50/24",
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

/// A role is a complete, ready-to-use board composed only of implemented tools.
/// Lore anchors are traceable in docs/VAULT_TEC_LORE_MATRIX.md; workflow mapping
/// is our own modern interpretation, not a claim about game mechanics.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Preset {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub lore_evidence: &'static str,
    pub role_fidelity: &'static str,
    pub tools: &'static [&'static str],
}

pub const PRESETS: &[Preset] = &[
    Preset {
        id: "overseer",
        title: "Overseer Operations",
        description: "Vault oversight: host condition, mission notes, reference library and optional game link.",
        lore_evidence: "VT-01",
        role_fidelity: "canonical_job",
        tools: &["host", "notes", "library", "game"],
    },
    Preset {
        id: "head-engineer",
        title: "Head Engineer Station",
        description: "Systems engineering: health telemetry, subnet calculations, byte analysis and synthetic rigs.",
        lore_evidence: "VT-01",
        role_fidelity: "canonical_job",
        tools: &["host", "net-lab", "byte-lab", "simulator", "notes"],
    },
    Preset {
        id: "security-chief",
        title: "Security Chief Station",
        description: "Defensive triage: local status, offline network planning, packet bytes and incident notes.",
        lore_evidence: "VT-01",
        role_fidelity: "canonical_job",
        tools: &["host", "net-lab", "byte-lab", "notes"],
    },
    Preset {
        id: "vault-doctor",
        title: "Vault Doctor Station",
        description: "Scientific reference, research notes and workstation condition; not medical diagnosis.",
        lore_evidence: "VT-01",
        role_fidelity: "canonical_job",
        tools: &["library", "notes", "host"],
    },
    Preset {
        id: "university-researcher",
        title: "University Research Station",
        description: "Reproducible experimentation: bytes, references, notes, fixtures and network mathematics.",
        lore_evidence: "VT-01",
        role_fidelity: "role_inspired",
        tools: &["byte-lab", "library", "notes", "simulator", "net-lab"],
    },
    Preset {
        id: "reclamation-scout",
        title: "Reclamation Field Kit",
        description: "Offline field records, network setup, computer condition and explicit game companion.",
        lore_evidence: "VT-02",
        role_fidelity: "role_inspired",
        tools: &["game", "net-lab", "notes", "host"],
    },
    Preset {
        id: "records-custodian",
        title: "Records Custodian Station",
        description: "Research archives, local writing, and noncryptographic byte verification.",
        lore_evidence: "VT-03",
        role_fidelity: "role_inspired",
        tools: &["library", "notes", "byte-lab"],
    },
    Preset {
        id: "custom",
        title: "Custom Workbench",
        description: "Advanced builder: start empty, then choose every tool yourself.",
        lore_evidence: "RB-01",
        role_fidelity: "rust_boy_original",
        tools: &[],
    },
];

pub fn preset(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|profile| profile.id == id)
}

fn legacy_profile() -> String {
    // Existing v1 workbench files predate profiles. Preserve their pins as custom.
    "custom".to_owned()
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
    #[serde(default = "legacy_profile")]
    pub profile: String,
    pub slots: Vec<Slot>,
}

impl Default for Workbench {
    fn default() -> Self {
        Self::from_preset(&PRESETS[0])
    }
}

impl Workbench {
    pub fn from_preset(selected: &Preset) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            title: selected.title.to_owned(),
            profile: selected.id.to_owned(),
            slots: selected
                .tools
                .iter()
                .enumerate()
                .map(|(index, name)| Slot {
                    id: index as u64 + 1,
                    tool: (*name).to_owned(),
                })
                .collect(),
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
        if preset(&self.profile).is_none()
            || !valid_title(&self.title)
            || self.slots.len() > MAX_SLOTS
        {
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
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "workbench exceeds 64 KiB",
            ));
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
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "workbench exceeds 64 KiB",
            ));
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
                io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "workbench is in use by another writer",
                )
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
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "tool is already pinned",
            ));
        }
        if board.slots.len() >= MAX_SLOTS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "workbench is full",
            ));
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
        board.profile = "custom".into();
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
        board.profile = "custom".into();
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
        board.profile = "custom".into();
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
        board.profile = "custom".into();
        self.save(&board)?;
        Ok(true)
    }

    /// Apply an entire role configuration. Callers must independently confirm
    /// destructive replacement; installation never executes tools.
    pub fn apply_preset(&self, key: &str) -> io::Result<bool> {
        let chosen = preset(key).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "unknown workbench profile")
        })?;
        let next = Workbench::from_preset(chosen);
        if self.load()? == next {
            return Ok(false);
        }
        let _lock = self.writer_lock()?;
        if self.load()? == next {
            return Ok(false);
        }
        self.save(&next)?;
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
fn parse_hex(input: &str) -> io::Result<Vec<u8>> {
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
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "max 4096 bytes",
                ));
            }
            let digit = std::str::from_utf8(pair).expect("validated ASCII");
            data.push(u8::from_str_radix(digit, 16).expect("validated hex digits"));
        }
    }
    if data.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "hex input cannot be empty",
        ));
    }
    Ok(data)
}

/// A bounded, printable-only hex dump suitable for terminal inspection.
pub fn inspect_hex(input: &str) -> io::Result<String> {
    let data = parse_hex(input)?;
    let mut output = format!(
        "BYTE LAB | {} bytes\nOFFSET    HEX                                              ASCII\n",
        data.len()
    );
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

/// Offline IPv4/CIDR arithmetic only. Does not inspect interfaces or send packets.
pub fn ipv4_cidr(value: &str) -> io::Result<String> {
    use std::net::Ipv4Addr;

    let (address, prefix) = value.split_once('/').ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected IPv4/prefix, e.g. 192.168.1.50/24",
        )
    })?;
    if address.is_empty()
        || address.trim() != address
        || prefix.is_empty()
        || !prefix.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid IPv4/CIDR input",
        ));
    }
    let ip: Ipv4Addr = address
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid IPv4 address"))?;
    let bits: u8 = prefix
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "prefix must be 0..32"))?;
    if bits > 32 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "prefix must be 0..32",
        ));
    }
    let mask = if bits == 0 {
        0
    } else {
        u32::MAX << (32 - bits)
    };
    let network = u32::from(ip) & mask;
    let broadcast = network | !mask;
    let (first, last, usable) = if bits <= 30 {
        (network + 1, broadcast - 1, (1u64 << (32 - bits)) - 2)
    } else if bits == 31 {
        (network, broadcast, 2)
    } else {
        (network, network, 1)
    };
    Ok(format!(
        "NET LAB / OFFLINE IPv4\nInput: {ip}/{bits}\nNetwork: {}/{}\nMask: {}\nBroadcast: {}\nHost range: {} - {}\nUsable addresses: {usable}\nNo network packets sent.",
        Ipv4Addr::from(network),
        bits,
        Ipv4Addr::from(mask),
        Ipv4Addr::from(broadcast),
        Ipv4Addr::from(first),
        Ipv4Addr::from(last)
    ))
}

/// CRC32/IEEE detects accidental corruption; it is not cryptographic integrity.
pub fn crc32_hex(input: &str) -> io::Result<String> {
    let bytes = parse_hex(input)?;
    let mut state = !0u32;
    for byte in &bytes {
        state ^= u32::from(*byte);
        for _ in 0..8 {
            state = if state & 1 != 0 {
                (state >> 1) ^ 0xedb8_8320
            } else {
                state >> 1
            };
        }
    }
    Ok(format!(
        "CRC32/IEEE: {:08x}\nBytes: {}\nNONCRYPTOGRAPHIC: not suitable for security or authentication.",
        !state,
        bytes.len()
    ))
}

pub fn encode_hex(text: &str) -> io::Result<String> {
    if text.is_empty() || text.len() > 4096 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "text must be 1..4096 bytes",
        ));
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
    fn default_overseer_board_is_ready_without_read_side_effects() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("new");
        let store = WorkbenchStore::new(root.clone());
        let board = store.load().unwrap();
        assert_eq!(board.schema_version, 1);
        assert_eq!(board.profile, "overseer");
        assert_eq!(board.slots.len(), 4);
        assert!(!root.exists());
        assert!(!store.remove(77).unwrap());
        assert!(!root.exists());
    }

    #[test]
    fn pin_unpin_and_rename_are_local_and_persistent() {
        let temporary = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temporary.path().join("state"));
        assert!(store.add("radio").is_err());
        assert!(!store.path().exists());
        assert!(store.apply_preset("custom").unwrap());
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
            assert_eq!(
                fs::metadata(store.path()).unwrap().permissions().mode() & 0o777,
                0o600
            );
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
    #[test]
    fn canonical_profiles_are_full_ready_boards_and_custom_is_opt_in() {
        let board = Workbench::default();
        assert_eq!(board.profile, "overseer");
        assert_eq!(board.slots.len(), 4);
        for profile in PRESETS {
            let board = Workbench::from_preset(profile);
            board.validate().unwrap();
            assert_eq!(board.profile, profile.id);
            assert_eq!(board.slots.len(), profile.tools.len());
            for slot in &board.slots {
                assert!(tool(&slot.tool).unwrap().availability.can_pin());
            }
        }
        assert!(Workbench::from_preset(preset("custom").unwrap())
            .slots
            .is_empty());
        assert!(PRESETS.iter().all(|entry| entry.lore_evidence.len() == 5));
    }

    #[test]
    fn applying_complete_profiles_does_not_run_tools_and_preserves_legacy_v1() {
        let temporary = tempfile::tempdir().unwrap();
        let store = WorkbenchStore::new(temporary.path().join("profiles"));
        assert_eq!(store.load().unwrap().profile, "overseer");
        assert!(!store.path().exists());
        assert!(store.apply_preset("head-engineer").unwrap());
        assert!(!store.apply_preset("head-engineer").unwrap());
        assert_eq!(store.load().unwrap().slots.len(), 5);
        assert_eq!(store.load().unwrap().profile, "head-engineer");
        assert!(store.apply_preset("custom").unwrap());
        assert!(store.load().unwrap().slots.is_empty());
        assert!(!store.apply_preset("custom").unwrap());
        assert!(store.add("byte-lab").is_ok());
        assert_eq!(store.load().unwrap().profile, "custom");
        let existing = fs::read(store.path()).unwrap();
        assert!(store.apply_preset("invalid").is_err());
        assert_eq!(fs::read(store.path()).unwrap(), existing);

        // Existing version-1 boards lacked a profile and must remain readable.
        fs::write(
            store.path(),
            r#"{"schema_version":1,"title":"Legacy","slots":[{"id":1,"tool":"notes"}]}"#,
        )
        .unwrap();
        let legacy = store.load().unwrap();
        assert_eq!(legacy.profile, "custom");
        assert_eq!(legacy.slots[0].tool, "notes");
    }

    #[test]
    fn network_math_and_crc32_are_real_bounded_offline_tools() {
        let result = ipv4_cidr("192.168.1.50/24").unwrap();
        assert!(result.contains("192.168.1.0/24"));
        assert!(result.contains("192.168.1.255"));
        assert!(result.contains("192.168.1.1 - 192.168.1.254"));
        assert!(ipv4_cidr("10.0.0.4/31")
            .unwrap()
            .contains("Usable addresses: 2"));
        assert!(ipv4_cidr("10.0.0.4/32")
            .unwrap()
            .contains("Usable addresses: 1"));
        assert!(ipv4_cidr("0.0.0.0/0").unwrap().contains("0.0.0.0/0"));
        for bad in [
            "",
            "host/24",
            "192.168.1.5/33",
            "192.168.1.5/a",
            "1.2.3.4",
            "1.2.3.4/-1",
        ] {
            assert!(ipv4_cidr(bad).is_err(), "{bad}");
        }
        assert!(crc32_hex("313233343536373839")
            .unwrap()
            .contains("cbf43926"));
        assert!(crc32_hex("gg").is_err());
        assert!(crc32_hex(&"01".repeat(4097)).is_err());
    }
}

//! Structured medical handoffs and a durable local emergency latch.
//! No transport, location acquisition, diagnosis, or automatic retry lives here.
use crate::message_pace::Tier;
use crate::messenger::{unix_time, valid_text};
use crate::tasks::valid_device_id;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;

pub const PREFIX: &str = "RB-AID/1 ";
const MAX_FILE: u64 = 8192;
fn invalid(text: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, text)
}
fn text(value: &str, max: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= max
        && valid_text(value)
        && !value.chars().any(char::is_control)
}
fn optional_peer(value: &str) -> bool {
    value.is_empty() || valid_device_id(value)
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub doctor_peer: String,
    pub emergency_peer: String,
    pub location: String,
    pub tier: Tier,
}
impl Config {
    fn valid(&self) -> bool {
        optional_peer(&self.doctor_peer)
            && optional_peer(&self.emergency_peer)
            && (self.location.is_empty() || text(&self.location, 80))
            && matches!(self.tier, Tier::Primary | Tier::Alternate)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Peacetime,
    Wartime,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Packet {
    NineLine { mode: Mode, lines: [String; 9] },
    Alarm { incident: u64, location: String },
    Cancel { incident: u64 },
    Ack { incident: u64 },
}
impl Packet {
    pub fn encode(&self) -> io::Result<String> {
        let valid = match self {
            Self::NineLine { lines, .. } => lines.iter().all(|line| text(line, 48)),
            Self::Alarm { incident, location } => *incident > 0 && text(location, 80),
            Self::Cancel { incident } | Self::Ack { incident } => *incident > 0,
        };
        if !valid {
            return Err(invalid("Invalid aid packet fields"));
        }
        let body = format!("{PREFIX}{}", serde_json::to_string(self)?);
        if !valid_text(&body) {
            return Err(invalid("Aid packet exceeds Messenger limit"));
        }
        Ok(body)
    }
    pub fn decode(body: &str) -> io::Result<Self> {
        if !valid_text(body) {
            return Err(invalid("Invalid aid packet"));
        }
        let packet: Self = serde_json::from_str(
            body.strip_prefix(PREFIX)
                .ok_or_else(|| invalid("Not an aid packet"))?,
        )
        .map_err(|_| invalid("Invalid aid packet schema"))?;
        packet.encode()?;
        Ok(packet)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Beacon {
    pub id: u64,
    pub active: bool,
    pub created_at: u64,
    pub peer: String,
    pub location: String,
    pub tier: Tier,
}
impl Beacon {
    pub fn packet(&self) -> Packet {
        if self.active {
            Packet::Alarm { incident: self.id, location: self.location.clone() }
        } else {
            Packet::Cancel { incident: self.id }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AidState {
    pub schema_version: u32,
    pub next_id: u64,
    pub config: Config,
    pub beacon: Option<Beacon>,
}
impl Default for AidState {
    fn default() -> Self {
        Self { schema_version: 1, next_id: 1, config: Config::default(), beacon: None }
    }
}
impl AidState {
    fn validate(&self) -> io::Result<()> {
        if self.schema_version != 1 || self.next_id == 0 || !self.config.valid() {
            return Err(invalid("Unsupported aid state; original preserved"));
        }
        if let Some(beacon) = &self.beacon {
            if beacon.id == 0 || beacon.id >= self.next_id || !optional_peer(&beacon.peer)
                || !text(&beacon.location, 80)
                || !matches!(beacon.tier, Tier::Primary | Tier::Alternate)
            {
                return Err(invalid("Invalid emergency latch; original preserved"));
            }
        }
        Ok(())
    }
}

pub struct AidStore {
    root: PathBuf,
}
impl AidStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn load(&self) -> io::Result<AidState> {
        let file = match File::open(self.root.join("aid-state.json")) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(AidState::default()),
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        file.take(MAX_FILE + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_FILE {
            return Err(invalid("Aid state exceeds bound"));
        }
        let state: AidState = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("Invalid aid state; original preserved"))?;
        state.validate()?;
        Ok(state)
    }
    fn lock(&self, name: &str) -> io::Result<File> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new().recursive(true).mode(0o700).create(&self.root)?;
        }
        #[cfg(not(unix))]
        fs::create_dir_all(&self.root)?;
        let mut options = fs::OpenOptions::new();
        options.create(true).read(true).write(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let guard = options.open(self.root.join(name))?;
        fs2::FileExt::try_lock_exclusive(&guard)?;
        Ok(guard)
    }
    /// Serialize explicit aid operations, including queue lookup and transport.
    pub fn dispatch_lock(&self) -> io::Result<File> {
        self.lock("aid-dispatch.lock")
    }
    fn update(&self, change: impl FnOnce(&mut AidState) -> io::Result<()>) -> io::Result<AidState> {
        let _guard = self.lock("aid-state.lock")?;
        let mut state = self.load()?;
        change(&mut state)?;
        state.validate()?;
        let bytes = serde_json::to_vec(&state)?;
        if bytes.len() as u64 > MAX_FILE {
            return Err(invalid("Aid state exceeds bound"));
        }
        let mut staging = tempfile::NamedTempFile::new_in(&self.root)?;
        staging.write_all(&bytes)?;
        staging.as_file().sync_all()?;
        staging.persist(self.root.join("aid-state.json")).map_err(|e| e.error)?;
        #[cfg(unix)]
        File::open(&self.root)?.sync_all()?;
        Ok(state)
    }
    pub fn configure(&self, config: Config) -> io::Result<AidState> {
        if !config.valid() {
            return Err(invalid("Invalid aid configuration"));
        }
        self.update(|state| {
            if let Some(beacon) = state.beacon.as_mut().filter(|b| b.active && b.peer.is_empty()) {
                beacon.peer = config.emergency_peer.clone();
                beacon.tier = config.tier;
            }
            state.config = config;
            Ok(())
        })
    }
    pub fn activate(&self) -> io::Result<AidState> {
        self.update(|state| {
            if state.beacon.as_ref().is_some_and(|b| b.active) {
                return Ok(());
            }
            let id = state.next_id;
            state.next_id = id.checked_add(1).ok_or_else(|| invalid("Incident IDs exhausted"))?;
            state.beacon = Some(Beacon {
                id, active: true, created_at: unix_time()?,
                peer: state.config.emergency_peer.clone(),
                location: if state.config.location.is_empty() { "UNKNOWN / NOT PROVIDED".into() }
                    else { state.config.location.clone() },
                tier: state.config.tier,
            });
            Ok(())
        })
    }
    pub fn cancel(&self, id: u64) -> io::Result<AidState> {
        self.update(|state| {
            let beacon = state.beacon.as_mut().filter(|b| b.id == id)
                .ok_or_else(|| invalid("Emergency incident changed; refresh before cancelling"))?;
            beacon.active = false;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packets_are_typed_bounded_and_not_clinical_inferences() {
        let packet = Packet::NineLine { mode: Mode::Peacetime, lines: std::array::from_fn(|_| "UNKNOWN".into()) };
        assert_eq!(Packet::decode(&packet.encode().unwrap()).unwrap(), packet);
        for body in ["RB-AID/2 {}", "RB-AID/1 {\"kind\":\"ack\",\"incident\":0}",
            "RB-AID/1 {\"kind\":\"ack\",\"incident\":1,\"extra\":true}"] {
            assert!(Packet::decode(body).is_err());
        }
        let bad = Packet::NineLine { mode: Mode::Wartime, lines: std::array::from_fn(|_| "é".repeat(25)) };
        assert!(bad.encode().is_err());
        assert!(Packet::Alarm { incident: 1, location: "\u{202e}unknown".into() }.encode().is_err());
    }
    #[test]
    fn latch_survives_restart_and_configuration_changes_without_retargeting() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("new");
        let store = AidStore::new(root.clone());
        assert!(store.load().unwrap().beacon.is_none());
        assert!(!root.exists());
        store.configure(Config { emergency_peer: "responder-one".into(), location: "Training site".into(), ..Config::default() }).unwrap();
        let first = store.activate().unwrap().beacon.unwrap();
        assert_eq!(store.activate().unwrap().beacon.unwrap(), first);
        store.configure(Config::default()).unwrap();
        let restarted = AidStore::new(root);
        assert_eq!(restarted.load().unwrap().beacon.unwrap().peer, "responder-one");
        assert!(restarted.cancel(first.id + 1).is_err());
        assert!(!restarted.cancel(first.id).unwrap().beacon.unwrap().active);
        assert!(restarted.activate().unwrap().beacon.unwrap().id > first.id);
    }
    #[test]
    fn corrupt_state_and_contention_preserve_the_latch() {
        let dir = tempfile::tempdir().unwrap();
        let store = AidStore::new(dir.path().into());
        let guard = store.dispatch_lock().unwrap();
        assert!(store.dispatch_lock().is_err());
        drop(guard);
        store.dispatch_lock().unwrap();
        let file = dir.path().join("aid-state.json");
        fs::write(&file, b"invalid").unwrap();
        assert!(store.activate().is_err());
        assert_eq!(fs::read(file).unwrap(), b"invalid");
    }
}

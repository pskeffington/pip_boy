//! Explicit, file-backed peer roster for OpenSSH task forwarding.
//!
//! No address is probed, no listener is opened, and storing a peer never
//! performs a connection. Host-key and client-key verification are delegated
//! to OpenSSH with a strictly configured known_hosts file on each send.
use crate::tasks::valid_device_id;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

const VERSION: u32 = 1;
const MAX_PEERS: usize = 16;
const MAX_FILE: u64 = 16 * 1024;

fn bad(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn valid_target(value: &str) -> bool {
    let Some((user, host)) = value.split_once('@') else {
        return false;
    };
    !user.is_empty()
        && user.len() <= 32
        && host.len() >= 3
        && host.len() <= 128
        && !host.contains('@')
        && !host.contains("..")
        && !host.starts_with('-')
        && !host.ends_with('-')
        && !host.starts_with('.')
        && !host.ends_with('.')
        && user
            .bytes()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == b'_')
        && user
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, b'-' | b'_'))
        && host
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, b'.' | b'-'))
}

fn valid_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && !value.contains('\0')
        && value.bytes().all(|ch| ch.is_ascii_graphic()
            && !matches!(ch, b'%' | b'"' | b'\'' | b'\\' | b';'))
        && Path::new(value).is_absolute()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Peer {
    pub alias: String,
    pub target: String,
    pub identity_file: String,
    pub known_hosts_file: String,
}

impl Peer {
    fn valid(&self) -> bool {
        valid_device_id(&self.alias)
            && valid_target(&self.target)
            && valid_path(&self.identity_file)
            && valid_path(&self.known_hosts_file)
    }

    fn files_exist(&self) -> bool {
        Path::new(&self.identity_file).is_file() && Path::new(&self.known_hosts_file).is_file()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PeerConfig {
    pub schema_version: u32,
    pub device_id: Option<String>,
    pub trusted_senders: Vec<String>,
    pub peers: Vec<Peer>,
}

impl Default for PeerConfig {
    fn default() -> Self {
        Self {
            schema_version: VERSION,
            device_id: None,
            trusted_senders: Vec::new(),
            peers: Vec::new(),
        }
    }
}

impl PeerConfig {
    fn validate(&self) -> io::Result<()> {
        if self.schema_version != VERSION
            || self.peers.len() > MAX_PEERS
            || self.trusted_senders.len() > MAX_PEERS
            || self
                .device_id
                .as_deref()
                .is_some_and(|id| !valid_device_id(id))
        {
            return Err(bad("Invalid peer configuration; existing file preserved"));
        }
        let mut names = HashSet::new();
        if self
            .peers
            .iter()
            .any(|peer| !peer.valid() || !names.insert(&peer.alias))
        {
            return Err(bad("Invalid or duplicate peer destination"));
        }
        let mut trusted = HashSet::new();
        if self
            .trusted_senders
            .iter()
            .any(|id| !valid_device_id(id) || !trusted.insert(id))
        {
            return Err(bad("Invalid trusted sender list"));
        }
        Ok(())
    }
}

pub struct PeerStore {
    path: PathBuf,
}

impl PeerStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            path: root.join("peers.json"),
        }
    }

    pub fn from_environment() -> io::Result<Self> {
        let root = match env::var_os("PIPBOY_HOME") {
            Some(value) if !value.is_empty() => PathBuf::from(value),
            _ => {
                let home = env::var_os("HOME")
                    .or_else(|| env::var_os("USERPROFILE"))
                    .ok_or_else(|| bad("Home directory unavailable"))?;
                PathBuf::from(home).join(".pipboy")
            }
        };
        Ok(Self::new(root))
    }

    fn directory(&self) -> &Path {
        self.path.parent().unwrap_or_else(|| Path::new("."))
    }

    pub fn load(&self) -> io::Result<PeerConfig> {
        match fs::File::open(&self.path) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(MAX_FILE + 1).read_to_end(&mut bytes)?;
                if bytes.len() as u64 > MAX_FILE {
                    return Err(bad("Peer configuration exceeds size limit"));
                }
                let config: PeerConfig = serde_json::from_slice(&bytes)
                    .map_err(|_| bad("Unrecognized peer configuration; not reset"))?;
                config.validate()?;
                Ok(config)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(PeerConfig::default()),
            Err(error) => Err(error),
        }
    }

    fn lock(&self) -> io::Result<fs::File> {
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
        let mut opts = fs::OpenOptions::new();
        opts.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let file = opts.open(self.directory().join("peers.lock"))?;
        fs2::FileExt::try_lock_exclusive(&file).map_err(|e| {
            if e.kind() == io::ErrorKind::WouldBlock {
                bad("Peer registry already in use")
            } else {
                e
            }
        })?;
        Ok(file)
    }

    fn save(&self, config: &PeerConfig) -> io::Result<()> {
        config.validate()?;
        let data = serde_json::to_vec_pretty(config)?;
        if data.len() as u64 > MAX_FILE {
            return Err(bad("Peer roster exceeds size limit"));
        }
        let mut staging = tempfile::Builder::new()
            .prefix(".peers-")
            .tempfile_in(self.directory())?;
        staging.as_file_mut().write_all(&data)?;
        staging.as_file_mut().sync_all()?;
        staging.persist(&self.path).map_err(|e| e.error)?;
        #[cfg(unix)]
        fs::File::open(self.directory())?.sync_all()?;
        Ok(())
    }

    pub fn init(&self, device_id: &str) -> io::Result<()> {
        if !valid_device_id(device_id) {
            return Err(bad(
                "Device ID must be 2-32 lowercase letters, numbers or hyphens",
            ));
        }
        let _lock = self.lock()?;
        let mut config = self.load()?;
        if config.device_id.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Device ID is already enrolled; never silently rotate identity",
            ));
        }
        config.device_id = Some(device_id.to_owned());
        self.save(&config)
    }

    pub fn trust_sender(&self, device_id: &str) -> io::Result<()> {
        if !valid_device_id(device_id) {
            return Err(bad("Invalid sender device ID"));
        }
        let _lock = self.lock()?;
        let mut config = self.load()?;
        if config.device_id.as_deref() == Some(device_id) {
            return Err(bad("Cannot trust the local device as a remote sender"));
        }
        if config.trusted_senders.iter().any(|id| id == device_id) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Sender already trusted",
            ));
        }
        if config.trusted_senders.len() >= MAX_PEERS {
            return Err(bad("Trusted sender list full"));
        }
        config.trusted_senders.push(device_id.to_owned());
        self.save(&config)
    }

    pub fn add(&self, peer: Peer) -> io::Result<()> {
        if !peer.valid() || !peer.files_exist() {
            return Err(bad(
                "Peer requires valid alias, user@host, absolute existing key and known_hosts paths",
            ));
        }
        let _lock = self.lock()?;
        let mut config = self.load()?;
        if config.device_id.is_none() {
            return Err(bad("Initialize this Rust-Boy device before adding peers"));
        }
        if config.peers.iter().any(|entry| entry.alias == peer.alias) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Duplicate peer alias",
            ));
        }
        if config.peers.len() >= MAX_PEERS {
            return Err(bad("Peer roster full"));
        }
        config.peers.push(peer);
        self.save(&config)
    }

    pub fn get(&self, alias: &str) -> io::Result<Peer> {
        self.load()?
            .peers
            .into_iter()
            .find(|peer| peer.alias == alias)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Device alias is not paired on this Rust-Boy",
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_file_created_by_read_and_strict_config_validation() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("vault");
        let store = PeerStore::new(path.clone());
        assert!(store.load().unwrap().device_id.is_none());
        assert!(!path.exists());
        assert!(!valid_target("-o@evil"));
        assert!(!valid_target("root@-host"));
        assert!(!valid_target("root@bad..host"));
        assert!(!valid_target("root@host;echo"));
        assert!(valid_target("pi@10.0.0.20"));
        assert!(valid_target("rustboy@scout.local"));
        assert!(!valid_path("../keys/id_ed25519"));
        store.init("command-one").unwrap();
        assert!(store.init("another").is_err());
        assert!(store.trust_sender("command-one").is_err());
        store.trust_sender("scout-one").unwrap();
        assert!(store.trust_sender("scout-one").is_err());
        assert_eq!(store.load().unwrap().trusted_senders, vec!["scout-one"]);
        fs::write(path.join("peers.json"), "{bad").unwrap();
        assert!(store.load().is_err());
        assert!(store.trust_sender("scout-two").is_err());
        assert_eq!(fs::read_to_string(path.join("peers.json")).unwrap(), "{bad");
    }

    #[test]
    fn add_peer_requires_explicit_key_and_pinned_known_hosts() {
        let temp = tempfile::tempdir().unwrap();
        let store = PeerStore::new(temp.path().join("vault"));
        store.init("overseer-a").unwrap();
        let key = temp.path().join("ssh_key");
        let hosts = temp.path().join("known_hosts");
        fs::write(&key, "test key placeholder").unwrap();
        fs::write(&hosts, "test hostkey placeholder").unwrap();
        let peer = Peer {
            alias: "scout-one".into(),
            target: "pi@scout.local".into(),
            identity_file: key.to_string_lossy().into(),
            known_hosts_file: hosts.to_string_lossy().into(),
        };
        store.add(peer.clone()).unwrap();
        assert_eq!(store.get("scout-one").unwrap().target, "pi@scout.local");
        assert!(store.add(peer).is_err());
        assert!(store.get("other").is_err());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(temp.path().join("vault/peers.json"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }
}

//! Explicit per-peer PACE routing. Plans are configuration, never reachability
//! evidence or authorization. No discovery, retry loop or radio transmitter.
use crate::tasks::valid_device_id;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};

const MAX_PLANS: usize = 16;
const MAX_FILE: u64 = 8192;
fn invalid(text: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, text)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    #[default]
    Primary,
    Alternate,
    Contingency,
    Emergency,
}
impl Tier {
    pub fn parse(value: &str) -> io::Result<Self> {
        match value {
            "primary" => Ok(Self::Primary),
            "alternate" => Ok(Self::Alternate),
            "contingency" => Ok(Self::Contingency),
            "emergency" => Ok(Self::Emergency),
            _ => Err(invalid("Unknown PACE tier")),
        }
    }
}

pub fn private_ipv4(value: &str) -> io::Result<Ipv4Addr> {
    let address: Ipv4Addr = value
        .parse()
        .map_err(|_| invalid("Expected a private IPv4 literal"))?;
    if !address.is_private() {
        return Err(invalid("Alternate route requires RFC1918 IPv4"));
    }
    Ok(address)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeerPlan {
    pub peer: String,
    pub alternate_ipv4: Ipv4Addr,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanBook {
    schema_version: u32,
    plans: Vec<PeerPlan>,
}
impl PlanBook {
    fn empty() -> Self {
        Self {
            schema_version: 1,
            plans: Vec::new(),
        }
    }
    fn validate(&self) -> io::Result<()> {
        if self.schema_version != 1 || self.plans.len() > MAX_PLANS {
            return Err(invalid("Unsupported PACE plan book"));
        }
        let mut seen = HashSet::new();
        for plan in &self.plans {
            if !valid_device_id(&plan.peer)
                || !plan.alternate_ipv4.is_private()
                || !seen.insert(&plan.peer)
            {
                return Err(invalid("Invalid or duplicate PACE peer route"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    PairedSsh,
    LocalSsh(Ipv4Addr),
    Hold,
}
#[derive(Clone, Debug, Serialize)]
pub struct Lane {
    pub tier: Tier,
    pub transport: &'static str,
    pub status: &'static str,
    pub selectable: bool,
    pub can_transmit: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct PlanView {
    pub peer: String,
    pub alternate_ipv4: Option<Ipv4Addr>,
    pub lanes: [Lane; 4],
    pub independent_bearers: bool,
    pub automatic_failover: bool,
}

pub struct PaceStore {
    path: PathBuf,
}
impl PaceStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            path: root.join("message-pace.json"),
        }
    }
    fn directory(&self) -> &Path {
        self.path.parent().unwrap_or_else(|| Path::new("."))
    }
    fn load(&self) -> io::Result<PlanBook> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(PlanBook::empty()),
            Err(e) => return Err(e),
        };
        let mut bytes = Vec::new();
        file.take(MAX_FILE + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_FILE {
            return Err(invalid("PACE plan file too large"));
        }
        let book: PlanBook = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("Invalid PACE plan; original preserved"))?;
        book.validate()?;
        Ok(book)
    }
    pub fn set_alternate(&self, peer: &str, address: Option<Ipv4Addr>) -> io::Result<()> {
        if !valid_device_id(peer) || address.is_some_and(|ip| !ip.is_private()) {
            return Err(invalid("Invalid PACE alternate"));
        }
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
        options.create(true).truncate(false).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let guard = options.open(self.directory().join("message-pace.lock"))?;
        fs2::FileExt::try_lock_exclusive(&guard)?;
        let mut book = self.load()?;
        book.plans.retain(|plan| plan.peer != peer);
        if let Some(ip) = address {
            book.plans.push(PeerPlan {
                peer: peer.into(),
                alternate_ipv4: ip,
            });
        }
        book.validate()?;
        let bytes = serde_json::to_vec(&book)?;
        if bytes.len() as u64 > MAX_FILE {
            return Err(invalid("PACE plan file too large"));
        }
        let mut staging = tempfile::NamedTempFile::new_in(self.directory())?;
        staging.write_all(&bytes)?;
        staging.as_file().sync_all()?;
        staging.persist(&self.path).map_err(|e| e.error)?;
        #[cfg(unix)]
        File::open(self.directory())?.sync_all()?;
        Ok(())
    }
    pub fn view(&self, peer: &str, paired: bool) -> io::Result<PlanView> {
        if !valid_device_id(peer) {
            return Err(invalid("Invalid PACE peer"));
        }
        let alternate_ipv4 = self
            .load()?
            .plans
            .into_iter()
            .find(|plan| plan.peer == peer)
            .map(|p| p.alternate_ipv4);
        Ok(PlanView {
            peer: peer.into(),
            alternate_ipv4,
            independent_bearers: false,
            automatic_failover: false,
            lanes: [
                Lane {
                    tier: Tier::Primary,
                    transport: "pinned_ssh",
                    status: if paired {
                        "configured_unprobed"
                    } else {
                        "unpaired"
                    },
                    selectable: paired,
                    can_transmit: paired,
                },
                Lane {
                    tier: Tier::Alternate,
                    transport: "pinned_ssh_local_ip",
                    status: if !paired {
                        "unpaired"
                    } else if alternate_ipv4.is_some() {
                        "configured_unprobed"
                    } else {
                        "not_configured"
                    },
                    selectable: paired && alternate_ipv4.is_some(),
                    can_transmit: paired && alternate_ipv4.is_some(),
                },
                Lane {
                    tier: Tier::Contingency,
                    transport: "meshtastic_passive",
                    status: "receive_only_unvalidated",
                    selectable: false,
                    can_transmit: false,
                },
                Lane {
                    tier: Tier::Emergency,
                    transport: "local_hold",
                    status: "local_only_no_delivery",
                    selectable: paired,
                    can_transmit: false,
                },
            ],
        })
    }
    pub fn resolve(&self, peer: &str, tier: Tier, paired: bool) -> io::Result<Route> {
        let view = self.view(peer, paired)?;
        if !paired {
            return Err(invalid("PACE recipient must be paired"));
        }
        match tier {
            Tier::Primary => Ok(Route::PairedSsh),
            Tier::Alternate => view
                .alternate_ipv4
                .map(Route::LocalSsh)
                .ok_or_else(|| invalid("Alternate IP not configured; no fallback attempted")),
            Tier::Contingency => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Meshtastic is passive; no authenticated transmit route",
            )),
            Tier::Emergency => Ok(Route::Hold),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plans_are_persistent_bounded_and_never_imply_transport_independence() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("home");
        let store = PaceStore::new(root.clone());
        let initial = store.view("scout-one", true).unwrap();
        assert!(!root.exists());
        assert!(!initial.automatic_failover && !initial.independent_bearers);
        assert!(store.resolve("scout-one", Tier::Alternate, true).is_err());
        let ip = private_ipv4("192.168.1.20").unwrap();
        store.set_alternate("scout-one", Some(ip)).unwrap();
        let reopened = PaceStore::new(root);
        assert_eq!(
            reopened
                .resolve("scout-one", Tier::Alternate, true)
                .unwrap(),
            Route::LocalSsh(ip)
        );
        assert!(reopened.resolve("scout-one", Tier::Primary, false).is_err());
        assert!(reopened
            .resolve("scout-one", Tier::Contingency, true)
            .is_err());
        assert_eq!(
            reopened
                .resolve("scout-one", Tier::Emergency, true)
                .unwrap(),
            Route::Hold
        );
        reopened.set_alternate("scout-one", None).unwrap();
        assert!(reopened
            .resolve("scout-one", Tier::Alternate, true)
            .is_err());
    }
    #[test]
    fn invalid_private_routes_and_corrupt_configuration_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let store = PaceStore::new(dir.path().into());
        for bad in [
            "127.0.0.1",
            "8.8.8.8",
            "10.0.0.1;exit",
            "scout.local",
            "::1",
        ] {
            assert!(private_ipv4(bad).is_err());
        }
        for bytes in [
            r#"{"schema_version":2,"plans":[]}"#,
            r#"{"schema_version":1,"plans":[{"peer":"scout","alternate_ipv4":"8.8.8.8"}]}"#,
        ] {
            fs::write(&store.path, bytes).unwrap();
            assert!(store.view("scout", true).is_err());
            assert!(store
                .set_alternate("scout", Some("10.0.0.2".parse().unwrap()))
                .is_err());
            assert_eq!(fs::read_to_string(&store.path).unwrap(), bytes);
        }
    }
}

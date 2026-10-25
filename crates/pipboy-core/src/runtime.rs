//! Testable in-process module polymorphism and capability prechecks.
 //!
 //! Important: PermissionSet is an application policy check, **not** an
 //! operating system sandbox. No untrusted native code may be loaded here.

use crate::registry::{Readiness, MODULES};
use serde::Serialize;
use std::collections::HashSet;

/// Explicitly named future I/O surfaces, with no implicit network/device grant.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    HostRead,
    FilesRead,
    FilesWrite,
    DeviceEnumerate,
    GpioRead,
    DisplayWrite,
    SerialRead,
    GnssRead,
    SdrReceive,
    BleDiscover,
    MeshTransmit,
    SensorRead,
    AudioPlay,
    NetworkFetch,
}

impl Capability {
    pub fn key(self) -> &'static str {
        match self {
            Self::HostRead => "host.read",
            Self::FilesRead => "files.read",
            Self::FilesWrite => "files.write",
            Self::DeviceEnumerate => "device.enumerate",
            Self::GpioRead => "gpio.read",
            Self::DisplayWrite => "display.write",
            Self::SerialRead => "serial.read",
            Self::GnssRead => "gnss.read",
            Self::SdrReceive => "sdr.receive",
            Self::BleDiscover => "ble.discover",
            Self::MeshTransmit => "mesh.transmit",
            Self::SensorRead => "sensor.read",
            Self::AudioPlay => "audio.play",
            Self::NetworkFetch => "network.fetch",
        }
    }

    pub fn from_key(value: &str) -> Option<Self> {
        match value {
            "host.read" => Some(Self::HostRead),
            "files.read" => Some(Self::FilesRead),
            "files.write" => Some(Self::FilesWrite),
            "device.enumerate" => Some(Self::DeviceEnumerate),
            "gpio.read" => Some(Self::GpioRead),
            "display.write" => Some(Self::DisplayWrite),
            "serial.read" => Some(Self::SerialRead),
            "gnss.read" => Some(Self::GnssRead),
            "sdr.receive" => Some(Self::SdrReceive),
            "ble.discover" => Some(Self::BleDiscover),
            "mesh.transmit" => Some(Self::MeshTransmit),
            "sensor.read" => Some(Self::SensorRead),
            "audio.play" => Some(Self::AudioPlay),
            "network.fetch" => Some(Self::NetworkFetch),
            _ => None,
        }
    }
}

/// Capability grants must originate from a later explicit consent flow.
#[derive(Default)]
pub struct PermissionSet {
    grants: HashSet<Capability>,
}

impl PermissionSet {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Only read-only local operations; files.write is deliberately absent.
    pub fn offline_read_only() -> Self {
        Self {
            grants: [Capability::HostRead, Capability::FilesRead]
                .into_iter()
                .collect(),
        }
    }

    pub fn grant(&mut self, capability: Capability) {
        self.grants.insert(capability);
    }

    pub fn revoke(&mut self, capability: Capability) {
        self.grants.remove(&capability);
    }

    pub fn allows(&self, capability: Capability) -> bool {
        self.grants.contains(&capability)
    }
}

/// A domain sample; future versioned Observation<T> models add type/units/quality.
#[derive(Clone, Debug, Serialize)]
pub struct ModuleSample {
    pub source: String,
    pub detail: String,
}

/// dyn-compatible interface for heterogeneous, **trusted built-in** modules.
/// This interface does not give untrusted code OS-level isolation.
pub trait RuntimeModule: Send {
    fn id(&self) -> &'static str;
    fn capabilities(&self) -> &'static [Capability];
    fn sample(&mut self) -> Result<ModuleSample, String>;
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PollOutcome {
    Sample { data: ModuleSample },
    Denied { missing: Vec<Capability> },
    Failed { reason: String },
}

#[derive(Clone, Debug, Serialize)]
pub struct ModulePoll {
    pub module_id: &'static str,
    pub outcome: PollOutcome,
}

/// A small OO host that makes prechecks before calling any module behavior.
///
/// Register only trusted built-in modules. The host is **not** used as a
/// sandbox and currently does not own hardware handles.
pub struct ModuleHost {
    grants: PermissionSet,
    modules: Vec<Box<dyn RuntimeModule>>,
}

impl ModuleHost {
    pub fn new(grants: PermissionSet) -> Self {
        Self {
            grants,
            modules: Vec::new(),
        }
    }

    pub fn permissions(&self) -> &PermissionSet {
        &self.grants
    }

    /// Caller is responsible for authenticating user consent to changes.
    pub fn permissions_mut(&mut self) -> &mut PermissionSet {
        &mut self.grants
    }

    pub fn register(&mut self, module: Box<dyn RuntimeModule>) -> Result<(), String> {
        let id = module.id();
        let declared = MODULES
            .iter()
            .find(|item| item.id == id)
            .ok_or_else(|| format!("unknown module {id}"))?;

        if declared.readiness != Readiness::Ready {
            return Err(format!("module {id} is not implemented"));
        }
        if self.modules.iter().any(|item| item.id() == id) {
            return Err(format!("duplicate module {id}"));
        }
        let mut seen = HashSet::new();
        if !module.capabilities().iter().all(|capability| seen.insert(*capability)) {
            return Err(format!("module {id} declares duplicate capabilities"));
        }
        self.modules.push(module);
        Ok(())
    }

    pub fn poll_once(&mut self) -> Vec<ModulePoll> {
        self.modules
            .iter_mut()
            .map(|module| {
                let missing: Vec<Capability> = module
                    .capabilities()
                    .iter()
                    .copied()
                    .filter(|capability| !self.grants.allows(*capability))
                    .collect();

                let outcome = if !missing.is_empty() {
                    PollOutcome::Denied { missing }
                } else {
                    match module.sample() {
                        Ok(data) => PollOutcome::Sample { data },
                        Err(reason) => PollOutcome::Failed { reason },
                    }
                };
                ModulePoll {
                    module_id: module.id(),
                    outcome,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    const READ: &[Capability] = &[Capability::HostRead];
    const WRITE: &[Capability] = &[Capability::FilesWrite];
    const RADIO: &[Capability] = &[Capability::SdrReceive];

    struct MockModule {
        id: &'static str,
        required: &'static [Capability],
        calls: Arc<AtomicUsize>,
    }

    impl RuntimeModule for MockModule {
        fn id(&self) -> &'static str {
            self.id
        }

        fn capabilities(&self) -> &'static [Capability] {
            self.required
        }

        fn sample(&mut self) -> Result<ModuleSample, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ModuleSample {
                source: self.id.to_string(),
                detail: "fixture".to_string(),
            })
        }
    }

    fn mock(id: &'static str, required: &'static [Capability], calls: &Arc<AtomicUsize>) -> Box<dyn RuntimeModule> {
        Box::new(MockModule {
            id,
            required,
            calls: Arc::clone(calls),
        })
    }

    #[test]
    fn denies_without_invoking_and_allows_after_grant() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut host = ModuleHost::new(PermissionSet::offline_read_only());
        host.register(mock("notes", WRITE, &calls)).unwrap();

        assert!(matches!(host.poll_once()[0].outcome, PollOutcome::Denied { .. }));
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        host.permissions_mut().grant(Capability::FilesWrite);
        assert!(matches!(host.poll_once()[0].outcome, PollOutcome::Sample { .. }));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        host.permissions_mut().revoke(Capability::FilesWrite);
        assert!(matches!(host.poll_once()[0].outcome, PollOutcome::Denied { .. }));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn two_concrete_modules_share_one_host() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut host = ModuleHost::new(PermissionSet::offline_read_only());
        host.register(mock("stat", READ, &calls)).unwrap();
        host.register(mock("library", &[], &calls)).unwrap();

        let output = host.poll_once();
        assert_eq!(output.len(), 2);
        assert!(output.iter().all(|entry| matches!(entry.outcome, PollOutcome::Sample { .. })));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn unimplemented_and_duplicate_modules_are_rejected() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut host = ModuleHost::new(PermissionSet::empty());
        assert!(host.register(mock("radio", RADIO, &calls)).is_err());
        assert!(host.register(mock("unknown", &[], &calls)).is_err());
        host.register(mock("stat", READ, &calls)).unwrap();
        assert!(host.register(mock("stat", READ, &calls)).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn permission_vocabulary_does_not_accept_undeclared_operations() {
        assert_eq!(Capability::from_key("mesh.transmit"), Some(Capability::MeshTransmit));
        assert_eq!(Capability::from_key("network.scan"), None);
        assert!(!PermissionSet::offline_read_only().allows(Capability::NetworkFetch));
    }
}

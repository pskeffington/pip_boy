//! Declared capabilities and in-process permission prechecks.
//!
//! This is not an operating-system sandbox and does not authorize device I/O.

use serde::Serialize;
use std::collections::HashSet;

/// Explicitly named future I/O surfaces, with no implicit network/device grant.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub enum Capability {
    #[serde(rename = "host.read")]
    HostRead,
    #[serde(rename = "files.read")]
    FilesRead,
    #[serde(rename = "files.write")]
    FilesWrite,
    #[serde(rename = "device.enumerate")]
    DeviceEnumerate,
    #[serde(rename = "gpio.read")]
    GpioRead,
    #[serde(rename = "display.write")]
    DisplayWrite,
    #[serde(rename = "serial.read")]
    SerialRead,
    #[serde(rename = "gnss.read")]
    GnssRead,
    #[serde(rename = "sdr.receive")]
    SdrReceive,
    #[serde(rename = "ble.discover")]
    BleDiscover,
    #[serde(rename = "mesh.transmit")]
    MeshTransmit,
    #[serde(rename = "sensor.read")]
    SensorRead,
    #[serde(rename = "audio.play")]
    AudioPlay,
    #[serde(rename = "network.fetch")]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emitted_permission_keys_match_manifest_vocabulary() {
        for key in [
            "host.read", "files.read", "files.write", "device.enumerate",
            "gpio.read", "display.write", "serial.read", "gnss.read",
            "sdr.receive", "ble.discover", "mesh.transmit", "sensor.read",
            "audio.play", "network.fetch",
        ] {
            let capability = Capability::from_key(key).expect("permission must exist");
            assert_eq!(capability.key(), key);
            assert_eq!(serde_json::to_string(&capability).unwrap(), format!("\"{key}\""));
        }
    }

    #[test]
    fn default_permissions_deny_sensitive_interfaces() {
        let permissions = PermissionSet::offline_read_only();
        assert!(permissions.allows(Capability::FilesRead));
        assert!(permissions.allows(Capability::HostRead));
        for denied in [
            Capability::FilesWrite,
            Capability::GnssRead,
            Capability::SdrReceive,
            Capability::BleDiscover,
            Capability::MeshTransmit,
            Capability::NetworkFetch,
        ] {
            assert!(!permissions.allows(denied));
        }
    }
}

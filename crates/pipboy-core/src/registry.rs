//! Declared module readiness is distinct from a working device connection.

use crate::capability::Capability;

use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Readiness {
    Ready,
    Planned,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Module {
    pub id: &'static str,
    pub title: &'static str,
    pub readiness: Readiness,
    pub description: &'static str,
    pub required_capabilities: &'static [Capability],
}

pub const MODULES: &[Module] = &[
    Module {
        id: "stat",
        title: "Host status",
        readiness: Readiness::Ready,
        description: "Read-only CPU, RAM, OS and uptime metrics",
        required_capabilities: &[Capability::HostRead],
    },
    Module {
        id: "library",
        title: "Reference library",
        readiness: Readiness::Ready,
        description: "Bundled GitHub source metadata and license reviews",
        required_capabilities: &[Capability::FilesRead],
    },
    Module {
        id: "notes",
        title: "Local notes",
        readiness: Readiness::Ready,
        description: "User-initiated offline JSON journal",
        required_capabilities: &[Capability::FilesWrite],
    },
    Module {
        id: "map",
        title: "Offline map",
        readiness: Readiness::Planned,
        description: "Optional GNSS and separately licensed offline tiles",
        required_capabilities: &[Capability::FilesRead],
    },
    Module {
        id: "radio",
        title: "Radio receiver",
        readiness: Readiness::Planned,
        description: "Opt-in receive-only SDR/audio adapter",
        required_capabilities: &[Capability::SdrReceive],
    },
    Module {
        id: "mesh",
        title: "Mesh communication",
        readiness: Readiness::Planned,
        description: "Manual peer selection and authenticated local transport",
        required_capabilities: &[Capability::MeshTransmit],
    },
    Module {
        id: "gpio",
        title: "Physical controls",
        readiness: Readiness::Planned,
        description: "Raspberry Pi rotary encoder and button adapter",
        required_capabilities: &[Capability::GpioRead],
    },
    Module {
        id: "devices",
        title: "Device I/O",
        readiness: Readiness::Planned,
        description: "Explicit serial/BLE device selection and access gate",
        required_capabilities: &[Capability::DeviceEnumerate],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planned_features_are_not_reported_ready() {
        for id in ["map", "radio", "mesh", "gpio", "devices"] {
            let module = MODULES.iter().find(|item| item.id == id).unwrap();
            assert_eq!(module.readiness, Readiness::Planned);
        }
    }
}

//! Deterministic, in-memory device-broker simulator for G1-D.
//!
//! No OS device, network, RF, location, file, or system measurement API is used.
//! A simulated confirmation is NOT an OS grant or authorization for a real
//! adapter. Only trusted fixtures may be dispatched through this broker.

use crate::capability::Capability;
use crate::permissions::{adapter_capability, Preference, PreferenceEntry, ADAPTER_CAPABILITIES};
use serde::Serialize;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

/// A fixed virtual fixture, never a discovered physical device.
#[derive(Clone, Copy, Debug)]
pub struct SimulatedDevice {
    pub id: &'static str,
    pub capability: Capability,
    pub description: &'static str,
}

/// A fixture for every external capability, with no physical backend.
pub const SIMULATED_DEVICES: &[SimulatedDevice] = &[
    SimulatedDevice {
        id: "sim:enumerate",
        capability: Capability::DeviceEnumerate,
        description: "virtual device inventory",
    },
    SimulatedDevice {
        id: "sim:gpio",
        capability: Capability::GpioRead,
        description: "virtual input pin",
    },
    SimulatedDevice {
        id: "sim:display",
        capability: Capability::DisplayWrite,
        description: "virtual display sink",
    },
    SimulatedDevice {
        id: "sim:serial",
        capability: Capability::SerialRead,
        description: "virtual serial input",
    },
    SimulatedDevice {
        id: "sim:gnss",
        capability: Capability::GnssRead,
        description: "virtual location source, no coordinates",
    },
    SimulatedDevice {
        id: "sim:sdr",
        capability: Capability::SdrReceive,
        description: "virtual RF receiver, no samples",
    },
    SimulatedDevice {
        id: "sim:ble",
        capability: Capability::BleDiscover,
        description: "virtual BLE discovery, no nearby scan",
    },
    SimulatedDevice {
        id: "sim:mesh",
        capability: Capability::MeshTransmit,
        description: "virtual mesh output, no transmission",
    },
    SimulatedDevice {
        id: "sim:sensor",
        capability: Capability::SensorRead,
        description: "virtual sensor, no measurements",
    },
    SimulatedDevice {
        id: "sim:audio",
        capability: Capability::AudioPlay,
        description: "virtual audio sink, no sound",
    },
    SimulatedDevice {
        id: "sim:network",
        capability: Capability::NetworkFetch,
        description: "virtual HTTP source, no requests",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionConfirmation {
    /// Default: no operation, even if a saved preference was requested.
    Absent,
    /// Caller explicitly confirmed a *simulation* for this session.
    Confirmed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrokerError {
    UnknownDevice,
    ConfirmationRequired,
    DeniedPreference,
    Busy,
    InvalidLease,
    InvalidPreference,
    CounterExhausted,
}

impl fmt::Display for BrokerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnknownDevice => "unknown simulated device",
            Self::ConfirmationRequired => "explicit simulation confirmation required",
            Self::DeniedPreference => "saved adapter preference denies this capability",
            Self::Busy => "simulated broker already has an active session",
            Self::InvalidLease => "simulated session is absent, revoked, or belongs to another broker",
            Self::InvalidPreference => "invalid or duplicate adapter preference; no session authorized",
            Self::CounterExhausted => "simulated session counter exhausted",
        };
        formatter.write_str(message)
    }
}

impl Error for BrokerError {}

/// A non-serializable, non-clonable token tied to exactly one broker instance.
/// Its private fields cannot be created from a stored preference or CLI input.
#[derive(Debug)]
pub struct SimulationLease {
    owner: Arc<()>,
    generation: u64,
    device_id: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct ActiveSession {
    device: &'static SimulatedDevice,
    generation: u64,
}

/// A truthful fixture event with no fabricated sensor value.
#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct SimulationEvent {
    pub schema_version: u8,
    pub simulated: bool,
    pub source: &'static str,
    pub device_id: &'static str,
    pub capability: &'static str,
    pub event: &'static str,
    pub sequence: u64,
}

/// All state is local to the broker instance and disappears when it is dropped.
/// This is an in-process test harness, not a privileged device broker.
pub struct SimulatedBroker {
    owner: Arc<()>,
    preferences: HashMap<Capability, Preference>,
    policy_valid: bool,
    active: Option<ActiveSession>,
    generation: u64,
    sequence: u64,
}

impl SimulatedBroker {
    pub fn new(entries: &[PreferenceEntry]) -> Result<Self, BrokerError> {
        Ok(Self {
            owner: Arc::new(()),
            preferences: Self::validated_preferences(entries)?,
            policy_valid: true,
            active: None,
            generation: 0,
            sequence: 0,
        })
    }

    fn validated_preferences(
        entries: &[PreferenceEntry],
    ) -> Result<HashMap<Capability, Preference>, BrokerError> {
        if entries.len() > ADAPTER_CAPABILITIES.len() {
            return Err(BrokerError::InvalidPreference);
        }
        let mut result = HashMap::new();
        for entry in entries {
            let capability = adapter_capability(&entry.capability)
                .map_err(|_| BrokerError::InvalidPreference)?;
            if result.insert(capability, entry.preference).is_some() {
                return Err(BrokerError::InvalidPreference);
            }
        }
        Ok(result)
    }

    /// Built-in inventory only: listing never discovers an OS or radio device.
    pub fn fixtures() -> &'static [SimulatedDevice] {
        SIMULATED_DEVICES
    }

    pub fn active_device(&self) -> Option<&'static str> {
        self.active.map(|session| session.device.id)
    }

    /// Require explicit, current-session simulation confirmation. A saved
    /// Requested preference does not satisfy this check; Denied blocks it.
    pub fn open(
        &mut self,
        id: &str,
        confirmation: SessionConfirmation,
    ) -> Result<SimulationLease, BrokerError> {
        if !self.policy_valid {
            return Err(BrokerError::InvalidPreference);
        }
        let device = SIMULATED_DEVICES
            .iter()
            .find(|device| device.id == id)
            .ok_or(BrokerError::UnknownDevice)?;
        if confirmation != SessionConfirmation::Confirmed {
            return Err(BrokerError::ConfirmationRequired);
        }
        if self.preferences.get(&device.capability) == Some(&Preference::Denied) {
            return Err(BrokerError::DeniedPreference);
        }
        if self.active.is_some() {
            return Err(BrokerError::Busy);
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(BrokerError::CounterExhausted)?;
        self.generation = generation;
        self.active = Some(ActiveSession { device, generation });
        Ok(SimulationLease {
            owner: Arc::clone(&self.owner),
            generation,
            device_id: device.id,
        })
    }

    fn authorized(&self, lease: &SimulationLease) -> Result<&ActiveSession, BrokerError> {
        if !Arc::ptr_eq(&self.owner, &lease.owner) {
            return Err(BrokerError::InvalidLease);
        }
        let active = self.active.as_ref().ok_or(BrokerError::InvalidLease)?;
        if active.generation != lease.generation || active.device.id != lease.device_id {
            return Err(BrokerError::InvalidLease);
        }
        if self.preferences.get(&active.device.capability) == Some(&Preference::Denied) {
            return Err(BrokerError::DeniedPreference);
        }
        Ok(active)
    }

    /// Emit a labeled fixture event; never touch hardware or invent readings.
    pub fn sample(&mut self, lease: &SimulationLease) -> Result<SimulationEvent, BrokerError> {
        let device = self.authorized(lease)?.device;
        let next = self
            .sequence
            .checked_add(1)
            .ok_or(BrokerError::CounterExhausted)?;
        self.sequence = next;
        Ok(SimulationEvent {
            schema_version: 1,
            simulated: true,
            source: "rust-boy/in-memory-simulator",
            device_id: device.id,
            capability: device.capability.key(),
            event: "fixture_only_no_hardware_io",
            sequence: next,
        })
    }

    pub fn close(&mut self, lease: &SimulationLease) -> Result<(), BrokerError> {
        self.authorized(lease)?;
        self.active = None;
        Ok(())
    }

    /// The next call using a revoked lease will fail, even after re-confirming.
    pub fn revoke(&mut self, capability: Capability) -> bool {
        if self
            .active
            .is_some_and(|session| session.device.capability == capability)
        {
            self.active = None;
            true
        } else {
            false
        }
    }

    /// The caller must reload a validated preference store before this step.
    /// On malformed updates, revoke the active session rather than fall back
    /// to the previous potentially permissive configuration.
    pub fn replace_preferences(
        &mut self,
        entries: &[PreferenceEntry],
    ) -> Result<(), BrokerError> {
        let validated = match Self::validated_preferences(entries) {
            Ok(validated) => validated,
            Err(error) => {
                self.active = None;
                self.preferences.clear();
                self.policy_valid = false;
                return Err(error);
            }
        };
        if self.active.is_some_and(|session| {
            validated.get(&session.device.capability) == Some(&Preference::Denied)
        }) {
            self.active = None;
        }
        self.preferences = validated;
        self.policy_valid = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn choice(capability: &str, preference: Preference) -> PreferenceEntry {
        PreferenceEntry {
            capability: capability.to_owned(),
            preference,
        }
    }

    #[test]
    fn fixture_inventory_is_unique_and_covers_all_external_capabilities() {
        let mut ids = HashSet::new();
        let mut capabilities = HashSet::new();
        for fixture in SimulatedBroker::fixtures() {
            assert!(fixture.id.starts_with("sim:"));
            assert!(!fixture.description.is_empty());
            assert!(ids.insert(fixture.id));
            assert!(capabilities.insert(fixture.capability));
        }
        assert_eq!(capabilities.len(), ADAPTER_CAPABILITIES.len());
        assert!(ADAPTER_CAPABILITIES
            .iter()
            .all(|capability| capabilities.contains(capability)));
    }

    #[test]
    fn a_saved_request_alone_never_authorizes_a_simulator() {
        let mut broker = SimulatedBroker::new(&[choice(
            "sdr.receive",
            Preference::Requested,
        )])
        .unwrap();
        assert_eq!(
            broker.open("sim:sdr", SessionConfirmation::Absent).unwrap_err(),
            BrokerError::ConfirmationRequired
        );
        assert!(broker.active_device().is_none());
    }

    #[test]
    fn denied_preference_overrides_explicit_simulation_confirmation() {
        let mut broker =
            SimulatedBroker::new(&[choice("mesh.transmit", Preference::Denied)]).unwrap();
        assert_eq!(
            broker.open("sim:mesh", SessionConfirmation::Confirmed).unwrap_err(),
            BrokerError::DeniedPreference
        );
        assert!(broker.active_device().is_none());
    }

    #[test]
    fn confirmed_fixture_emits_only_labeled_simulated_events() {
        let mut broker = SimulatedBroker::new(&[]).unwrap();
        let lease = broker
            .open("sim:gnss", SessionConfirmation::Confirmed)
            .unwrap();
        let event = broker.sample(&lease).unwrap();
        assert_eq!(event.schema_version, 1);
        assert!(event.simulated);
        assert_eq!(event.source, "rust-boy/in-memory-simulator");
        assert_eq!(event.capability, "gnss.read");
        assert_eq!(event.event, "fixture_only_no_hardware_io");
        assert_eq!(event.sequence, 1);
        let json = serde_json::to_value(&event).unwrap();
        assert!(json.get("latitude").is_none());
        assert!(json.get("longitude").is_none());
        assert_eq!(broker.sample(&lease).unwrap().sequence, 2);
        broker.close(&lease).unwrap();
        assert_eq!(broker.sample(&lease).unwrap_err(), BrokerError::InvalidLease);
    }

    #[test]
    fn revoke_invalidates_old_token_and_never_restarts_implicitly() {
        let mut broker = SimulatedBroker::new(&[]).unwrap();
        let old = broker
            .open("sim:sdr", SessionConfirmation::Confirmed)
            .unwrap();
        assert_eq!(broker.sample(&old).unwrap().sequence, 1);
        assert!(broker.revoke(Capability::SdrReceive));
        assert!(!broker.revoke(Capability::SdrReceive));
        assert_eq!(broker.sample(&old).unwrap_err(), BrokerError::InvalidLease);
        assert_eq!(
            broker.open("sim:sdr", SessionConfirmation::Absent).unwrap_err(),
            BrokerError::ConfirmationRequired
        );
        let fresh = broker
            .open("sim:sdr", SessionConfirmation::Confirmed)
            .unwrap();
        assert_eq!(broker.sample(&old).unwrap_err(), BrokerError::InvalidLease);
        assert_eq!(broker.sample(&fresh).unwrap().sequence, 2);
    }

    #[test]
    fn policy_reload_revokes_denied_capabilities_before_next_sample() {
        let mut broker = SimulatedBroker::new(&[]).unwrap();
        let token = broker
            .open("sim:ble", SessionConfirmation::Confirmed)
            .unwrap();
        broker
            .replace_preferences(&[choice("ble.discover", Preference::Denied)])
            .unwrap();
        assert_eq!(broker.sample(&token).unwrap_err(), BrokerError::InvalidLease);
        assert_eq!(
            broker.open("sim:ble", SessionConfirmation::Confirmed).unwrap_err(),
            BrokerError::DeniedPreference
        );
        broker
            .replace_preferences(&[choice("ble.discover", Preference::Requested)])
            .unwrap();
        assert_eq!(
            broker.open("sim:ble", SessionConfirmation::Absent).unwrap_err(),
            BrokerError::ConfirmationRequired
        );
    }

    #[test]
    fn invalid_policy_replacement_fails_closed_and_clears_current_session() {
        let mut broker = SimulatedBroker::new(&[]).unwrap();
        let token = broker
            .open("sim:gpio", SessionConfirmation::Confirmed)
            .unwrap();
        assert_eq!(
            broker
                .replace_preferences(&[
                    choice("gpio.read", Preference::Requested),
                    choice("gpio.read", Preference::Denied),
                ])
                .unwrap_err(),
            BrokerError::InvalidPreference
        );
        assert_eq!(broker.sample(&token).unwrap_err(), BrokerError::InvalidLease);
        assert_eq!(
            broker
                .open("sim:gpio", SessionConfirmation::Confirmed)
                .unwrap_err(),
            BrokerError::InvalidPreference
        );
        broker.replace_preferences(&[]).unwrap();
        assert!(broker
            .open("sim:gpio", SessionConfirmation::Confirmed)
            .is_ok());
        assert_eq!(
            SimulatedBroker::new(&[choice("files.write", Preference::Requested)])
                .err()
                .unwrap(),
            BrokerError::InvalidPreference
        );
    }

    #[test]
    fn leases_are_broker_scoped_and_unknown_devices_fail_closed() {
        let mut first = SimulatedBroker::new(&[]).unwrap();
        let mut second = SimulatedBroker::new(&[]).unwrap();
        let first_lease = first
            .open("sim:gpio", SessionConfirmation::Confirmed)
            .unwrap();
        let second_lease = second
            .open("sim:gpio", SessionConfirmation::Confirmed)
            .unwrap();
        assert_eq!(
            second.sample(&first_lease).unwrap_err(),
            BrokerError::InvalidLease
        );
        assert_eq!(
            first.open("real:gpio", SessionConfirmation::Confirmed).unwrap_err(),
            BrokerError::UnknownDevice
        );
        assert_eq!(
            first.open("sim:sdr", SessionConfirmation::Confirmed).unwrap_err(),
            BrokerError::Busy
        );
        assert!(second.sample(&second_lease).is_ok());
    }
}

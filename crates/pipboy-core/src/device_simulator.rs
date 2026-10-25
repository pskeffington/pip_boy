//! Deterministic, in-memory device-broker simulator for G1-D.
//!
//! No OS device, network, RF, location, file, or system measurement API is used.
//! A simulated confirmation is NOT an OS grant or authorization for a real
//! adapter. Only trusted fixtures may be dispatched through this broker.

use crate::capability::Capability;
use crate::permissions::{adapter_capability, Preference, PreferenceEntry, ADAPTER_CAPABILITIES};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::sync::Arc;

/// Maximum simulated lease duration; no physical timer or device timeout exists.
pub const MAX_VIRTUAL_TIMEOUT_MS: u64 = 60_000;

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
    Unavailable,
    Busy,
    InvalidLease,
    InvalidPreference,
    InvalidTimeout,
    ClockReversed,
    CounterExhausted,
    ReplayInvariant,
}

impl fmt::Display for BrokerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnknownDevice => "unknown simulated device",
            Self::ConfirmationRequired => "explicit simulation confirmation required",
            Self::DeniedPreference => "saved adapter preference denies this capability",
            Self::Unavailable => "simulated device is disconnected",
            Self::Busy => "simulated broker already has an active session",
            Self::InvalidLease => {
                "simulated session is absent, revoked, or belongs to another broker"
            }
            Self::InvalidPreference => {
                "invalid or duplicate adapter preference; no session authorized"
            }
            Self::InvalidTimeout => "simulated timeout must be between 1 and 60000 ms",
            Self::ClockReversed => "simulated logical clock moved backwards; lease invalidated",
            Self::CounterExhausted => "simulated session counter exhausted",
            Self::ReplayInvariant => "deterministic replay fixture invariant violated",
        };
        formatter.write_str(message)
    }
}

impl Error for BrokerError {}

impl BrokerError {
    /// Stable machine-readable error identifier for deterministic fixture traces.
    pub fn code(self) -> &'static str {
        match self {
            Self::UnknownDevice => "unknown_device",
            Self::ConfirmationRequired => "confirmation_required",
            Self::DeniedPreference => "denied_preference",
            Self::Unavailable => "unavailable",
            Self::Busy => "busy",
            Self::InvalidLease => "invalid_lease",
            Self::InvalidPreference => "invalid_preference",
            Self::InvalidTimeout => "invalid_timeout",
            Self::ClockReversed => "clock_reversed",
            Self::CounterExhausted => "counter_exhausted",
            Self::ReplayInvariant => "replay_invariant",
        }
    }
}

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
    expires_at_ms: Option<u64>,
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
    disconnected: HashSet<&'static str>,
    generation: u64,
    sequence: u64,
    logical_ms: u64,
}

impl SimulatedBroker {
    pub fn new(entries: &[PreferenceEntry]) -> Result<Self, BrokerError> {
        Ok(Self {
            owner: Arc::new(()),
            preferences: Self::validated_preferences(entries)?,
            policy_valid: true,
            active: None,
            disconnected: HashSet::new(),
            generation: 0,
            sequence: 0,
            logical_ms: 0,
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
        self.open_internal(id, confirmation, None)
    }

    /// A bounded mock-only deadline measured by explicit logical clock ticks.
    /// No wall-clock timer, actual device handle, or background task is created.
    pub fn open_with_timeout(
        &mut self,
        id: &str,
        confirmation: SessionConfirmation,
        ttl_ms: u64,
    ) -> Result<SimulationLease, BrokerError> {
        if ttl_ms == 0 || ttl_ms > MAX_VIRTUAL_TIMEOUT_MS {
            return Err(BrokerError::InvalidTimeout);
        }
        let deadline = self
            .logical_ms
            .checked_add(ttl_ms)
            .ok_or(BrokerError::CounterExhausted)?;
        self.open_internal(id, confirmation, Some(deadline))
    }

    fn open_internal(
        &mut self,
        id: &str,
        confirmation: SessionConfirmation,
        expires_at_ms: Option<u64>,
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
        if self.disconnected.contains(device.id) {
            return Err(BrokerError::Unavailable);
        }
        if self.active.is_some() {
            return Err(BrokerError::Busy);
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(BrokerError::CounterExhausted)?;
        self.generation = generation;
        self.active = Some(ActiveSession {
            device,
            generation,
            expires_at_ms,
        });
        Ok(SimulationLease {
            owner: Arc::clone(&self.owner),
            generation,
            device_id: device.id,
        })
    }

    /// Advance an injected, deterministic logical clock. Returns true only
    /// when crossing an active session deadline, invalidating that lease.
    /// A backwards tick fails closed and invalidates any active session.
    pub fn advance_to(&mut self, logical_ms: u64) -> Result<bool, BrokerError> {
        if logical_ms < self.logical_ms {
            self.active = None;
            return Err(BrokerError::ClockReversed);
        }
        self.logical_ms = logical_ms;
        let expired = self
            .active
            .is_some_and(|session| session.expires_at_ms.is_some_and(|end| logical_ms >= end));
        if expired {
            self.active = None;
        }
        Ok(expired)
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

    /// Simulate loss of one fixed fixture. Any active session for that device
    /// is invalidated before returning. No physical device is ever contacted.
    /// Returns true only when the virtual device changes to disconnected.
    pub fn disconnect(&mut self, id: &str) -> Result<bool, BrokerError> {
        let device = SIMULATED_DEVICES
            .iter()
            .find(|device| device.id == id)
            .ok_or(BrokerError::UnknownDevice)?;
        if self
            .active
            .is_some_and(|session| session.device.id == device.id)
        {
            self.active = None;
        }
        Ok(self.disconnected.insert(device.id))
    }

    /// Simulate recovery of a fixture. Does NOT reactivate a stale session;
    /// a subsequent open still requires explicit confirmation.
    pub fn reconnect(&mut self, id: &str) -> Result<bool, BrokerError> {
        let device = SIMULATED_DEVICES
            .iter()
            .find(|device| device.id == id)
            .ok_or(BrokerError::UnknownDevice)?;
        Ok(self.disconnected.remove(device.id))
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

    /// Explicitly cancel an authorized simulated lease. Cancellation cannot
    /// authorize a fresh lease; subsequent opens require new confirmation.
    pub fn cancel(&mut self, lease: &SimulationLease) -> Result<(), BrokerError> {
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
    pub fn replace_preferences(&mut self, entries: &[PreferenceEntry]) -> Result<(), BrokerError> {
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
        let mut broker =
            SimulatedBroker::new(&[choice("sdr.receive", Preference::Requested)]).unwrap();
        assert_eq!(
            broker
                .open("sim:sdr", SessionConfirmation::Absent)
                .unwrap_err(),
            BrokerError::ConfirmationRequired
        );
        assert!(broker.active_device().is_none());
    }

    #[test]
    fn denied_preference_overrides_explicit_simulation_confirmation() {
        let mut broker =
            SimulatedBroker::new(&[choice("mesh.transmit", Preference::Denied)]).unwrap();
        assert_eq!(
            broker
                .open("sim:mesh", SessionConfirmation::Confirmed)
                .unwrap_err(),
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
        assert_eq!(
            broker.sample(&lease).unwrap_err(),
            BrokerError::InvalidLease
        );
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
            broker
                .open("sim:sdr", SessionConfirmation::Absent)
                .unwrap_err(),
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
        assert_eq!(
            broker.sample(&token).unwrap_err(),
            BrokerError::InvalidLease
        );
        assert_eq!(
            broker
                .open("sim:ble", SessionConfirmation::Confirmed)
                .unwrap_err(),
            BrokerError::DeniedPreference
        );
        broker
            .replace_preferences(&[choice("ble.discover", Preference::Requested)])
            .unwrap();
        assert_eq!(
            broker
                .open("sim:ble", SessionConfirmation::Absent)
                .unwrap_err(),
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
        assert_eq!(
            broker.sample(&token).unwrap_err(),
            BrokerError::InvalidLease
        );
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
    fn simulated_disconnect_requires_new_session_after_reconnect() {
        let mut broker = SimulatedBroker::new(&[]).unwrap();
        let old = broker
            .open("sim:serial", SessionConfirmation::Confirmed)
            .unwrap();
        assert_eq!(broker.sample(&old).unwrap().sequence, 1);
        assert!(broker.disconnect("sim:serial").unwrap());
        assert!(!broker.disconnect("sim:serial").unwrap());
        assert_eq!(broker.active_device(), None);
        assert_eq!(broker.sample(&old).unwrap_err(), BrokerError::InvalidLease);
        assert_eq!(
            broker
                .open("sim:serial", SessionConfirmation::Confirmed)
                .unwrap_err(),
            BrokerError::Unavailable
        );
        assert!(broker.reconnect("sim:serial").unwrap());
        assert!(!broker.reconnect("sim:serial").unwrap());
        assert_eq!(broker.sample(&old).unwrap_err(), BrokerError::InvalidLease);
        assert_eq!(
            broker
                .open("sim:serial", SessionConfirmation::Absent)
                .unwrap_err(),
            BrokerError::ConfirmationRequired
        );
        let fresh = broker
            .open("sim:serial", SessionConfirmation::Confirmed)
            .unwrap();
        assert_eq!(broker.sample(&fresh).unwrap().sequence, 2);
        assert_eq!(broker.sample(&old).unwrap_err(), BrokerError::InvalidLease);
    }

    #[test]
    fn disconnect_and_reconnect_reject_unknown_devices() {
        let mut broker = SimulatedBroker::new(&[]).unwrap();
        assert_eq!(
            broker.disconnect("real:sdr").unwrap_err(),
            BrokerError::UnknownDevice
        );
        assert_eq!(
            broker.reconnect("real:sdr").unwrap_err(),
            BrokerError::UnknownDevice
        );
        assert_eq!(broker.active_device(), None);
        assert_eq!(BrokerError::Unavailable.code(), "unavailable");
    }

    #[test]
    fn cancellation_drops_lease_without_auto_restart() {
        let mut broker = SimulatedBroker::new(&[]).unwrap();
        let old = broker
            .open("sim:ble", SessionConfirmation::Confirmed)
            .unwrap();
        assert_eq!(broker.sample(&old).unwrap().sequence, 1);
        broker.cancel(&old).unwrap();
        assert!(broker.active_device().is_none());
        assert_eq!(broker.sample(&old).unwrap_err(), BrokerError::InvalidLease);
        assert_eq!(broker.cancel(&old).unwrap_err(), BrokerError::InvalidLease);
        assert_eq!(
            broker
                .open("sim:ble", SessionConfirmation::Absent)
                .unwrap_err(),
            BrokerError::ConfirmationRequired
        );
        let fresh = broker
            .open("sim:ble", SessionConfirmation::Confirmed)
            .unwrap();
        assert_eq!(broker.sample(&fresh).unwrap().sequence, 2);
    }

    #[test]
    fn logical_timeout_exact_boundary_invalidates_lease() {
        let mut broker = SimulatedBroker::new(&[]).unwrap();
        let old = broker
            .open_with_timeout("sim:serial", SessionConfirmation::Confirmed, 200)
            .unwrap();
        assert!(!broker.advance_to(100).unwrap());
        assert_eq!(broker.sample(&old).unwrap().sequence, 1);
        assert!(!broker.advance_to(199).unwrap());
        assert!(broker.advance_to(200).unwrap());
        assert!(!broker.advance_to(200).unwrap());
        assert_eq!(broker.sample(&old).unwrap_err(), BrokerError::InvalidLease);
        assert!(broker.active_device().is_none());
        assert_eq!(
            broker
                .open("sim:serial", SessionConfirmation::Absent)
                .unwrap_err(),
            BrokerError::ConfirmationRequired
        );
    }

    #[test]
    fn invalid_timeouts_and_reverse_ticks_fail_without_activity() {
        let mut broker = SimulatedBroker::new(&[]).unwrap();
        assert_eq!(
            broker
                .open_with_timeout("sim:gpio", SessionConfirmation::Confirmed, 0)
                .unwrap_err(),
            BrokerError::InvalidTimeout
        );
        assert_eq!(
            broker
                .open_with_timeout(
                    "sim:gpio",
                    SessionConfirmation::Confirmed,
                    MAX_VIRTUAL_TIMEOUT_MS + 1,
                )
                .unwrap_err(),
            BrokerError::InvalidTimeout
        );
        assert!(broker.active_device().is_none());
        assert!(!broker.advance_to(100).unwrap());
        let lease = broker
            .open("sim:gpio", SessionConfirmation::Confirmed)
            .unwrap();
        assert_eq!(
            broker.advance_to(99).unwrap_err(),
            BrokerError::ClockReversed
        );
        assert_eq!(
            broker.sample(&lease).unwrap_err(),
            BrokerError::InvalidLease
        );
        assert_eq!(BrokerError::ClockReversed.code(), "clock_reversed");
        assert!(!broker.advance_to(u64::MAX - 5).unwrap());
        assert_eq!(
            broker
                .open_with_timeout("sim:gpio", SessionConfirmation::Confirmed, 10)
                .unwrap_err(),
            BrokerError::CounterExhausted
        );
        assert!(broker.active_device().is_none());
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
            first
                .open("real:gpio", SessionConfirmation::Confirmed)
                .unwrap_err(),
            BrokerError::UnknownDevice
        );
        assert_eq!(
            first
                .open("sim:sdr", SessionConfirmation::Confirmed)
                .unwrap_err(),
            BrokerError::Busy
        );
        assert!(second.sample(&second_lease).is_ok());
    }
}

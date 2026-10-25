//! Deterministic, bounded replay of fixed in-memory device scenarios.
//!
//! Logical times are relative fixture labels, never wall-clock observations.
//! Scenarios do not use OS devices, sockets, timers, real measurements, or I/O.

use crate::device_simulator::{BrokerError, SessionConfirmation, SimulatedBroker, SimulationEvent};
use crate::permissions::PreferenceEntry;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayScenario {
    Nominal,
    DisconnectRecover,
    Revoke,
    Cancel,
    Timeout,
}

impl ReplayScenario {
    pub fn from_key(value: &str) -> Option<Self> {
        match value {
            "nominal" => Some(Self::Nominal),
            "disconnect-recover" => Some(Self::DisconnectRecover),
            "revoke" => Some(Self::Revoke),
            "cancel" => Some(Self::Cancel),
            "timeout" => Some(Self::Timeout),
            _ => None,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Nominal => "nominal",
            Self::DisconnectRecover => "disconnect-recover",
            Self::Revoke => "revoke",
            Self::Cancel => "cancel",
            Self::Timeout => "timeout",
        }
    }
}

/// A bounded, explicitly simulated step; never a physical sensor reading.
#[derive(Debug, Serialize)]
pub struct ReplayStep {
    pub logical_ms: u64,
    pub action: &'static str,
    pub state: &'static str,
    pub outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<SimulationEvent>,
}

/// Entirely process-local fixture output; no timestamps or live devices.
#[derive(Debug, Serialize)]
pub struct ReplayTrace {
    pub schema_version: u8,
    pub simulated: bool,
    pub source: &'static str,
    pub clock: &'static str,
    pub scenario: &'static str,
    pub device_id: &'static str,
    pub capability: &'static str,
    pub steps: Vec<ReplayStep>,
}

impl ReplayTrace {
    fn ok(
        &mut self,
        logical_ms: u64,
        action: &'static str,
        state: &'static str,
        event: Option<SimulationEvent>,
    ) {
        self.steps.push(ReplayStep {
            logical_ms,
            action,
            state,
            outcome: "ok",
            error: None,
            event,
        });
    }

    fn rejected(
        &mut self,
        logical_ms: u64,
        action: &'static str,
        state: &'static str,
        error: BrokerError,
    ) {
        self.steps.push(ReplayStep {
            logical_ms,
            action,
            state,
            outcome: "rejected",
            error: Some(error.code()),
            event: None,
        });
    }
}

/// Only exact expected rejections are accepted. An unexpected success is an
/// error, not a fabricated successful simulation or a silently dropped event.
fn expect_rejection<T>(
    observed: Result<T, BrokerError>,
    expected: BrokerError,
) -> Result<BrokerError, BrokerError> {
    match observed {
        Err(actual) if actual == expected => Ok(actual),
        _ => Err(BrokerError::ReplayInvariant),
    }
}

/// Replay one named, fixed scenario against a fresh simulated broker.
///
/// Confirmation authorizes just this *mock scenario*, including its initial
/// open. No scenario automatically opens a new lease after losing one; a
/// later simulation still requires a separate explicit confirmation.
/// No OS device, socket, or background timer is created.
pub fn replay_fixture(
    id: &str,
    scenario: ReplayScenario,
    preferences: &[PreferenceEntry],
    confirmation: SessionConfirmation,
) -> Result<ReplayTrace, BrokerError> {
    let device = SimulatedBroker::fixtures()
        .iter()
        .find(|device| device.id == id)
        .ok_or(BrokerError::UnknownDevice)?;
    let mut broker = SimulatedBroker::new(preferences)?;
    let lease = if scenario == ReplayScenario::Timeout {
        broker.open_with_timeout(id, confirmation, 200)?
    } else {
        broker.open(id, confirmation)?
    };
    let mut trace = ReplayTrace {
        schema_version: 1,
        simulated: true,
        source: "rust-boy/deterministic-simulator",
        clock: "synthetic_logical_ms",
        scenario: scenario.key(),
        device_id: device.id,
        capability: device.capability.key(),
        steps: Vec::with_capacity(8),
    };
    trace.ok(0, "confirm_open", "fixture_active", None);
    if broker.advance_to(100)? {
        return Err(BrokerError::ReplayInvariant);
    }
    let first = broker.sample(&lease)?;
    trace.ok(100, "sample_fixture", "fixture_active", Some(first));

    match scenario {
        ReplayScenario::Nominal => {
            broker.close(&lease)?;
            trace.ok(200, "close", "inactive", None);
        }
        ReplayScenario::DisconnectRecover => {
            if !broker.disconnect(id)? {
                return Err(BrokerError::ReplayInvariant);
            }
            trace.ok(200, "disconnect", "disconnected", None);
            let error = expect_rejection(broker.sample(&lease), BrokerError::InvalidLease)?;
            trace.rejected(300, "sample_stale_lease", "disconnected", error);
            let error = expect_rejection(
                broker.open(id, SessionConfirmation::Confirmed),
                BrokerError::Unavailable,
            )?;
            trace.rejected(400, "open_disconnected", "disconnected", error);
            if !broker.reconnect(id)? {
                return Err(BrokerError::ReplayInvariant);
            }
            trace.ok(500, "reconnect", "inactive", None);
            let error = expect_rejection(broker.sample(&lease), BrokerError::InvalidLease)?;
            trace.rejected(600, "sample_stale_lease", "inactive", error);
            let error = expect_rejection(
                broker.open(id, SessionConfirmation::Absent),
                BrokerError::ConfirmationRequired,
            )?;
            trace.rejected(700, "open_without_new_confirmation", "inactive", error);
        }
        ReplayScenario::Revoke => {
            if !broker.revoke(device.capability) {
                return Err(BrokerError::ReplayInvariant);
            }
            trace.ok(200, "revoke", "inactive", None);
            let error = expect_rejection(broker.sample(&lease), BrokerError::InvalidLease)?;
            trace.rejected(300, "sample_stale_lease", "inactive", error);
            let error = expect_rejection(
                broker.open(id, SessionConfirmation::Absent),
                BrokerError::ConfirmationRequired,
            )?;
            trace.rejected(400, "open_without_new_confirmation", "inactive", error);
        }
        ReplayScenario::Cancel => {
            broker.cancel(&lease)?;
            trace.ok(200, "cancel", "inactive", None);
            let error = expect_rejection(broker.sample(&lease), BrokerError::InvalidLease)?;
            trace.rejected(300, "sample_stale_lease", "inactive", error);
            let error = expect_rejection(
                broker.open(id, SessionConfirmation::Absent),
                BrokerError::ConfirmationRequired,
            )?;
            trace.rejected(400, "open_without_new_confirmation", "inactive", error);
        }
        ReplayScenario::Timeout => {
            if !broker.advance_to(200)? {
                return Err(BrokerError::ReplayInvariant);
            }
            trace.ok(200, "advance_to_deadline", "inactive", None);
            let error = expect_rejection(broker.sample(&lease), BrokerError::InvalidLease)?;
            trace.rejected(300, "sample_stale_lease", "inactive", error);
            let error = expect_rejection(
                broker.open(id, SessionConfirmation::Absent),
                BrokerError::ConfirmationRequired,
            )?;
            trace.rejected(400, "open_without_new_confirmation", "inactive", error);
        }
    }
    if broker.active_device().is_some() {
        return Err(BrokerError::ReplayInvariant);
    }
    Ok(trace)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::Preference;

    fn preference(key: &str, value: Preference) -> PreferenceEntry {
        PreferenceEntry {
            capability: key.to_owned(),
            preference: value,
        }
    }

    #[test]
    fn only_known_replay_scenarios_are_accepted() {
        assert_eq!(
            ReplayScenario::from_key("nominal"),
            Some(ReplayScenario::Nominal)
        );
        assert_eq!(
            ReplayScenario::from_key("disconnect-recover"),
            Some(ReplayScenario::DisconnectRecover)
        );
        assert_eq!(
            ReplayScenario::from_key("revoke"),
            Some(ReplayScenario::Revoke)
        );
        assert_eq!(ReplayScenario::from_key("cancel"), Some(ReplayScenario::Cancel));
        assert_eq!(ReplayScenario::from_key("timeout"), Some(ReplayScenario::Timeout));
        assert_eq!(ReplayScenario::from_key("hardware"), None);
    }

    #[test]
    fn nominal_trace_has_a_stable_small_truthful_schema() {
        let trace = replay_fixture(
            "sim:gpio",
            ReplayScenario::Nominal,
            &[],
            SessionConfirmation::Confirmed,
        )
        .unwrap();
        assert_eq!(trace.steps.len(), 3);
        assert_eq!(
            trace
                .steps
                .iter()
                .map(|step| step.logical_ms)
                .collect::<Vec<_>>(),
            [0, 100, 200]
        );
        assert_eq!(trace.clock, "synthetic_logical_ms");
        assert!(trace.simulated);
        assert_eq!(trace.steps[0].action, "confirm_open");
        assert_eq!(trace.steps[1].action, "sample_fixture");
        assert_eq!(trace.steps[1].event.as_ref().unwrap().sequence, 1);
        assert_eq!(trace.steps[2].state, "inactive");

        let json = serde_json::to_value(trace).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["simulated"], true);
        assert_eq!(json["steps"][1]["event"]["simulated"], true);
        assert!(json["steps"][1]["event"].get("measurement").is_none());
        assert!(json["steps"][1]["event"].get("latitude").is_none());
    }

    #[test]
    fn disconnect_recover_never_reactivates_or_reuses_old_lease() {
        let trace = replay_fixture(
            "sim:serial",
            ReplayScenario::DisconnectRecover,
            &[],
            SessionConfirmation::Confirmed,
        )
        .unwrap();
        assert_eq!(trace.steps.len(), 8);
        assert_eq!(
            trace.steps.iter().map(|s| s.logical_ms).collect::<Vec<_>>(),
            [0, 100, 200, 300, 400, 500, 600, 700]
        );
        assert_eq!(trace.steps[2].state, "disconnected");
        assert_eq!(trace.steps[3].error, Some("invalid_lease"));
        assert_eq!(trace.steps[4].error, Some("unavailable"));
        assert_eq!(trace.steps[5].action, "reconnect");
        assert_eq!(trace.steps[5].state, "inactive");
        assert_eq!(trace.steps[6].error, Some("invalid_lease"));
        assert_eq!(trace.steps[7].error, Some("confirmation_required"));
        assert!(trace.steps.iter().skip(2).all(|step| step.event.is_none()));
    }

    #[test]
    fn revoked_session_does_not_restart_without_fresh_confirmation() {
        let trace = replay_fixture(
            "sim:sdr",
            ReplayScenario::Revoke,
            &[],
            SessionConfirmation::Confirmed,
        )
        .unwrap();
        assert_eq!(trace.steps.len(), 5);
        assert_eq!(trace.steps[2].action, "revoke");
        assert_eq!(trace.steps[3].error, Some("invalid_lease"));
        assert_eq!(trace.steps[4].error, Some("confirmation_required"));
        assert_eq!(trace.steps[4].state, "inactive");
    }

    #[test]
    fn cancellation_trace_rejects_stale_lease_and_requires_new_confirmation() {
        let trace = replay_fixture(
            "sim:ble",
            ReplayScenario::Cancel,
            &[],
            SessionConfirmation::Confirmed,
        )
        .unwrap();
        assert_eq!(trace.steps.len(), 5);
        assert_eq!(trace.steps[2].action, "cancel");
        assert_eq!(trace.steps[2].state, "inactive");
        assert_eq!(trace.steps[3].error, Some("invalid_lease"));
        assert_eq!(trace.steps[4].error, Some("confirmation_required"));
    }

    #[test]
    fn timeout_trace_expires_at_injected_deadline_without_real_clock() {
        let trace = replay_fixture(
            "sim:network",
            ReplayScenario::Timeout,
            &[],
            SessionConfirmation::Confirmed,
        )
        .unwrap();
        assert_eq!(trace.steps.len(), 5);
        assert_eq!(
            trace.steps.iter().map(|step| step.logical_ms).collect::<Vec<_>>(),
            [0, 100, 200, 300, 400]
        );
        assert_eq!(trace.steps[2].action, "advance_to_deadline");
        assert_eq!(trace.steps[2].state, "inactive");
        assert_eq!(trace.steps[3].error, Some("invalid_lease"));
        assert_eq!(trace.steps[4].error, Some("confirmation_required"));
        assert!(trace.steps.iter().skip(2).all(|step| step.event.is_none()));
    }

    #[test]
    fn stored_requested_preference_cannot_replace_confirmation() {
        let preferences = [preference("sdr.receive", Preference::Requested)];
        let err = replay_fixture(
            "sim:sdr",
            ReplayScenario::Nominal,
            &preferences,
            SessionConfirmation::Absent,
        )
        .unwrap_err();
        assert_eq!(err, BrokerError::ConfirmationRequired);
    }

    #[test]
    fn stored_denial_blocks_even_confirmed_replay() {
        let preferences = [preference("mesh.transmit", Preference::Denied)];
        let err = replay_fixture(
            "sim:mesh",
            ReplayScenario::DisconnectRecover,
            &preferences,
            SessionConfirmation::Confirmed,
        )
        .unwrap_err();
        assert_eq!(err, BrokerError::DeniedPreference);
    }

    #[test]
    fn replay_output_is_byte_identical_for_repeated_runs() {
        for scenario in [
            ReplayScenario::Nominal,
            ReplayScenario::DisconnectRecover,
            ReplayScenario::Revoke,
            ReplayScenario::Cancel,
            ReplayScenario::Timeout,
        ] {
            let run = || {
                serde_json::to_vec(
                    &replay_fixture("sim:gnss", scenario, &[], SessionConfirmation::Confirmed)
                        .unwrap(),
                )
                .unwrap()
            };
            assert_eq!(
                run(),
                run(),
                "replay is nondeterministic: {}",
                scenario.key()
            );
        }
    }

    #[test]
    fn replay_is_bounded_and_unknown_device_fails_closed() {
        let err = replay_fixture(
            "real:radio",
            ReplayScenario::Nominal,
            &[],
            SessionConfirmation::Confirmed,
        )
        .unwrap_err();
        assert_eq!(err, BrokerError::UnknownDevice);
        let trace = replay_fixture(
            "sim:network",
            ReplayScenario::DisconnectRecover,
            &[],
            SessionConfirmation::Confirmed,
        )
        .unwrap();
        assert!(trace.steps.len() <= 8);
        assert!(trace.steps.iter().all(|step| step.logical_ms <= 700));
        assert_eq!(
            expect_rejection(Ok::<_, BrokerError>(()), BrokerError::InvalidLease),
            Err(BrokerError::ReplayInvariant)
        );
    }
}

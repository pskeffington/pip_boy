//! Pure state transitions for future adapters, with no physical I/O.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Planned,
    Disabled,
    Authorized,
    Running,
    Degraded,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    GrantConfirmed,
    StartSucceeded,
    DeviceUnavailable,
    DataDegraded,
    DataRecovered,
    Stop,
    Revoke,
}

#[derive(Clone, Debug, Serialize)]
pub struct ModuleLifecycle {
    module_id: String,
    state: State,
    transitions: u64,
}

impl ModuleLifecycle {
    pub fn new(id: &str, implemented: bool) -> Result<Self, String> {
        if id.trim().is_empty() {
            return Err("module id cannot be empty".into());
        }
        Ok(Self {
            module_id: id.to_owned(),
            state: if implemented {
                State::Disabled
            } else {
                State::Planned
            },
            transitions: 0,
        })
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn module_id(&self) -> &str {
        &self.module_id
    }

    pub fn transitions(&self) -> u64 {
        self.transitions
    }

    /// GrantConfirmed must follow a separate, trusted permission check.
    /// This state machine is not a security or OS authorization boundary.
    pub fn apply(&mut self, event: Event) -> Result<State, String> {
        let next = match (self.state, event) {
            (State::Disabled, Event::GrantConfirmed) => State::Authorized,
            (State::Authorized, Event::StartSucceeded) => State::Running,
            (State::Authorized, Event::DeviceUnavailable) => State::Unavailable,
            (State::Running, Event::DataDegraded) => State::Degraded,
            (State::Degraded, Event::DataRecovered) => State::Running,
            (State::Running, Event::DeviceUnavailable)
            | (State::Degraded, Event::DeviceUnavailable) => State::Unavailable,
            (
                State::Authorized
                | State::Running
                | State::Degraded
                | State::Unavailable
                | State::Disabled,
                Event::Stop | Event::Revoke,
            ) => State::Disabled,
            _ => {
                return Err(format!(
                    "invalid {event:?} while {:?} ({})",
                    self.state, self.module_id
                ))
            }
        };
        if self.state != next {
            self.transitions = self.transitions.saturating_add(1);
        }
        self.state = next;
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planned_modules_cannot_run() {
        let mut item = ModuleLifecycle::new("radio", false).unwrap();
        assert_eq!(item.state(), State::Planned);
        assert!(item.apply(Event::GrantConfirmed).is_err());
        assert!(item.apply(Event::StartSucceeded).is_err());
    }

    #[test]
    fn transitions_require_grant_and_revocation_stops_activity() {
        let mut item = ModuleLifecycle::new("stat", true).unwrap();
        assert!(item.apply(Event::StartSucceeded).is_err());
        assert_eq!(
            item.apply(Event::GrantConfirmed).unwrap(),
            State::Authorized
        );
        assert_eq!(item.apply(Event::StartSucceeded).unwrap(), State::Running);
        assert_eq!(item.apply(Event::DataDegraded).unwrap(), State::Degraded);
        assert_eq!(item.apply(Event::DataRecovered).unwrap(), State::Running);
        assert_eq!(item.apply(Event::Revoke).unwrap(), State::Disabled);
        assert!(item.apply(Event::StartSucceeded).is_err());
        assert_eq!(item.transitions(), 5);
        item.apply(Event::Revoke).unwrap();
        assert_eq!(item.transitions(), 5);
    }

    #[test]
    fn stop_and_revoke_share_fail_closed_transitions() {
        for event in [Event::Stop, Event::Revoke] {
            for initial in [
                State::Disabled,
                State::Authorized,
                State::Running,
                State::Degraded,
                State::Unavailable,
            ] {
                let mut item = ModuleLifecycle::new("fixture", true).unwrap();
                if initial != State::Disabled {
                    item.apply(Event::GrantConfirmed).unwrap();
                }
                if matches!(initial, State::Running | State::Degraded) {
                    item.apply(Event::StartSucceeded).unwrap();
                }
                if initial == State::Degraded {
                    item.apply(Event::DataDegraded).unwrap();
                }
                if initial == State::Unavailable {
                    item.apply(Event::DeviceUnavailable).unwrap();
                }
                assert_eq!(item.state(), initial);
                let before = item.transitions();
                assert_eq!(item.apply(event).unwrap(), State::Disabled);
                assert_eq!(
                    item.transitions(),
                    before + u64::from(initial != State::Disabled)
                );
                assert_eq!(item.apply(event).unwrap(), State::Disabled);
                assert_eq!(
                    item.transitions(),
                    before + u64::from(initial != State::Disabled)
                );
            }
            let mut planned = ModuleLifecycle::new("planned", false).unwrap();
            assert!(planned.apply(event).is_err());
            assert_eq!(planned.state(), State::Planned);
        }
    }

    #[test]
    fn unavailability_requires_explicit_recovery() {
        let mut item = ModuleLifecycle::new("gpio", true).unwrap();
        item.apply(Event::GrantConfirmed).unwrap();
        assert_eq!(
            item.apply(Event::DeviceUnavailable).unwrap(),
            State::Unavailable
        );
        assert!(item.apply(Event::DataRecovered).is_err());
        assert_eq!(item.apply(Event::Stop).unwrap(), State::Disabled);
    }
}

//! Host-owned exclusive output lease shared by independent foreground modules.
//!
//! A module cannot preempt another claimant. The lease drops only after the
//! output owner has stopped or failed closed; callers keep it beside the active
//! output session so all exits release the same host-wide claim.
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioModule {
    Radio,
    Music,
    Television,
}

#[derive(Clone, Default)]
pub struct AudioFocusBroker {
    state: Arc<Mutex<FocusState>>,
}

#[derive(Default)]
struct FocusState {
    owner: Option<AudioModule>,
    generation: u64,
}

/// Exclusive output claim. Dropping it releases the owner only if its token
/// still identifies the current claim.
pub struct AudioFocusLease {
    state: Arc<Mutex<FocusState>>,
    owner: AudioModule,
    generation: u64,
    release_on_drop: bool,
}

/// One coordinator for all in-process host modules. A module-specific adapter
/// must use this exact broker instance rather than constructing its own.
pub static HOST_AUDIO_FOCUS: LazyLock<AudioFocusBroker> = LazyLock::new(AudioFocusBroker::default);

impl AudioFocusBroker {
    /// Claims output for one module. Returns `None` on contention or generation
    /// exhaustion; this path never stops or steals another module's output.
    pub fn try_acquire(&self, owner: AudioModule) -> Option<AudioFocusLease> {
        let mut state = self.state.lock().ok()?;
        if state.owner.is_some() {
            return None;
        }
        let generation = state.generation.checked_add(1)?;
        state.generation = generation;
        state.owner = Some(owner);
        Some(AudioFocusLease {
            state: Arc::clone(&self.state),
            owner,
            generation,
            release_on_drop: true,
        })
    }

    pub fn owner(&self) -> Option<AudioModule> {
        self.state.lock().ok().and_then(|state| state.owner)
    }
}

impl Drop for AudioFocusLease {
    fn drop(&mut self) {
        if !self.release_on_drop {
            return;
        }
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.owner == Some(self.owner) && state.generation == self.generation {
            state.owner = None;
        }
    }
}

impl AudioFocusLease {
    /// Preserve this claim after uncertain adapter shutdown. The shared broker
    /// remains unavailable to other modules until the host process exits.
    pub(crate) fn retain_claim_on_drop(&mut self) {
        self.release_on_drop = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_module_owns_output_until_its_lease_drops() {
        let radio_broker = AudioFocusBroker::default();
        let music_broker = radio_broker.clone();
        let radio = radio_broker.try_acquire(AudioModule::Radio).unwrap();
        assert_eq!(radio_broker.owner(), Some(AudioModule::Radio));
        assert!(music_broker.try_acquire(AudioModule::Music).is_none());
        drop(radio);
        let music = music_broker.try_acquire(AudioModule::Music).unwrap();
        assert_eq!(radio_broker.owner(), Some(AudioModule::Music));
        drop(music);
        assert_eq!(music_broker.owner(), None);
    }

    #[test]
    fn independent_brokers_do_not_claim_to_coordinate() {
        let radio = AudioFocusBroker::default();
        let music = AudioFocusBroker::default();
        let _radio_lease = radio.try_acquire(AudioModule::Radio).unwrap();
        let _music_lease = music.try_acquire(AudioModule::Music).unwrap();
        assert_eq!(radio.owner(), Some(AudioModule::Radio));
        assert_eq!(music.owner(), Some(AudioModule::Music));
    }

    #[test]
    fn uncertain_cleanup_can_retain_the_broker_claim() {
        let broker = AudioFocusBroker::default();
        let mut lease = broker.try_acquire(AudioModule::Television).unwrap();
        lease.retain_claim_on_drop();
        drop(lease);
        assert_eq!(broker.owner(), Some(AudioModule::Television));
        assert!(broker.try_acquire(AudioModule::Radio).is_none());
    }
}

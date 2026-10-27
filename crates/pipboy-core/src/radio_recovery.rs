//! Host compatibility path for the standalone rust_radio module.
pub use rust_radio::recovery::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn output_selection_never_autoplays_stopped_or_unrelated_failures() {
        use crate::radio_controller::PlaybackState;
        assert!(!restart_after_output_change(PlaybackState::Stopped, false));
        assert!(!restart_after_output_change(PlaybackState::Stopped, true));
        assert!(!restart_after_output_change(PlaybackState::Failed, false));
        assert!(restart_after_output_change(PlaybackState::Failed, true));
        assert!(restart_after_output_change(PlaybackState::Playing, false));
        assert!(restart_after_output_change(
            PlaybackState::Recovering,
            false
        ));
    }

    #[test]
    fn bounded_retry_delays() {
        assert_eq!(
            decide(FailureClass::NetworkTransient, 0, false),
            RecoveryAction::RetryAfter(Duration::from_secs(1))
        );
        assert_eq!(
            decide(FailureClass::NetworkTransient, 1, false),
            RecoveryAction::RetryAfter(Duration::from_secs(2))
        );
        assert_eq!(
            decide(FailureClass::NetworkTransient, 2, false),
            RecoveryAction::RetryAfter(Duration::from_secs(4))
        );
        assert_eq!(
            decide(FailureClass::NetworkTransient, 3, false),
            RecoveryAction::Fail
        );
        assert_eq!(
            decide(FailureClass::NetworkTransient, 255, false),
            RecoveryAction::Fail
        );
    }

    #[test]
    fn output_recovery_requires_explicit_selection() {
        for attempt in 0..=3 {
            assert_eq!(
                decide(FailureClass::OutputUnavailable, attempt, false),
                RecoveryAction::SelectOutput
            );
        }
    }

    #[test]
    fn no_retry_for_permanent_failures() {
        for error in [
            ReceiverError::HttpRejected,
            ReceiverError::InvalidPlaylist,
            ReceiverError::UnsupportedCodec,
            ReceiverError::AudioFormatRejected,
        ] {
            assert_eq!(error.class(), FailureClass::Permanent);
            assert_eq!(decide(error.class(), 0, false), RecoveryAction::Fail);
        }
    }

    #[test]
    fn stop_overrides_all_states_including_a_retry_slot() {
        for class in [
            FailureClass::NetworkTransient,
            FailureClass::OutputUnavailable,
            FailureClass::Permanent,
        ] {
            assert_eq!(decide(class, 0, true), RecoveryAction::Stop);
            assert_eq!(decide(class, 3, true), RecoveryAction::Stop);
        }
    }

    #[test]
    fn typed_classification_does_not_merge_device_and_network() {
        assert_eq!(
            ReceiverError::NetworkTimeout.class(),
            FailureClass::NetworkTransient
        );
        assert_eq!(
            ReceiverError::NetworkDisconnected.class(),
            FailureClass::NetworkTransient
        );
        assert_eq!(
            ReceiverError::OutputDeviceLost.class(),
            FailureClass::OutputUnavailable
        );
    }
}

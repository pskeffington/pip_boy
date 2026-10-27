//! Host compatibility path for the standalone rust_radio module.
pub use rust_radio::controller::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_station_starts_only_connecting_not_playing() {
        let mut radio = RadioController::default();
        radio
            .apply(RadioCommand::PlayInternet {
                station_id: "skylab".into(),
            })
            .unwrap();
        assert_eq!(radio.state().state, PlaybackState::Connecting);
        assert_eq!(radio.state().input, Some(InputKind::Internet));
    }

    #[test]
    fn no_untrusted_station_or_invalid_gain() {
        let mut radio = RadioController::default();
        assert_eq!(
            radio.apply(RadioCommand::PlayInternet {
                station_id: "bad".into()
            }),
            Err(ControlError::UnknownStation)
        );
        assert_eq!(
            radio.apply(RadioCommand::SetVolume(101)),
            Err(ControlError::InvalidVolume)
        );
        assert_eq!(radio.state().generation, 0);
    }

    #[test]
    fn stale_worker_cannot_restart_after_stop() {
        let mut radio = RadioController::default();
        let old = radio
            .apply(RadioCommand::PlayInternet {
                station_id: "skylab".into(),
            })
            .unwrap();
        assert!(radio.event(old, PlaybackEvent::Connected));
        radio.apply(RadioCommand::Stop).unwrap();
        assert!(!radio.event(old, PlaybackEvent::PlaybackStarted));
        assert_eq!(radio.state().state, PlaybackState::Stopped);
    }

    #[test]
    fn disconnected_stream_reenters_buffering_on_same_generation() {
        let mut radio = RadioController::default();
        let generation = radio
            .apply(RadioCommand::PlayInternet {
                station_id: "skylab".into(),
            })
            .unwrap();
        assert!(radio.event(generation, PlaybackEvent::ConnectionLost));
        assert_eq!(radio.state().state, PlaybackState::Recovering);
        assert!(radio.event(generation, PlaybackEvent::Connected));
        assert!(radio.event(generation, PlaybackEvent::Buffered));
        assert!(radio.event(generation, PlaybackEvent::PlaybackStarted));
        assert_eq!(radio.state().state, PlaybackState::Playing);
        radio.apply(RadioCommand::Stop).unwrap();
        assert!(!radio.event(generation, PlaybackEvent::Connected));
    }

    #[test]
    fn playback_needs_actual_worker_events() {
        let mut radio = RadioController::default();
        let gen = radio
            .apply(RadioCommand::PlayInternet {
                station_id: "dronezone".into(),
            })
            .unwrap();
        assert!(!radio.event(gen, PlaybackEvent::PlaybackStarted));
        assert!(radio.event(gen, PlaybackEvent::Connected));
        assert!(radio.event(gen, PlaybackEvent::Buffered));
        assert!(radio.event(gen, PlaybackEvent::PlaybackStarted));
        assert_eq!(radio.state().state, PlaybackState::Playing);
    }
}

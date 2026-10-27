//! Host compatibility path for the standalone rust_radio module.
pub use rust_radio::sdr::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Tuning {
        Tuning {
            frequency_hz: 100_000_000,
            sample_rate_hz: 1_024_000,
            demodulation: Demodulation::Wfm,
            gain_db_tenths: None,
        }
    }

    #[test]
    fn tune_and_read_simulated_iq() {
        let mut tuner = RadioTuner::new(SimulatedRadio::default()).unwrap();
        assert_eq!(tuner.read_iq(&mut [0; 4]), Err(RadioError::NotConfigured));
        tuner.tune(settings()).unwrap();
        let mut buffer = [4i16; 8];
        assert_eq!(tuner.read_iq(&mut buffer).unwrap(), 8);
        assert_eq!(buffer, [0; 8]);
    }

    #[test]
    fn rejects_frequency_and_sample_rate_without_changing_state() {
        let mut tuner = RadioTuner::new(SimulatedRadio::default()).unwrap();
        let mut invalid = settings();
        invalid.frequency_hz = 1;
        assert_eq!(
            tuner.tune(invalid),
            Err(RadioError::UnsupportedFrequency(1))
        );
        invalid = settings();
        invalid.sample_rate_hz = 1;
        assert_eq!(
            tuner.tune(invalid),
            Err(RadioError::UnsupportedSampleRate(1))
        );
        assert_eq!(tuner.active_tuning(), None);
    }

    #[test]
    fn rejects_unavailable_gain_and_odd_iq_buffer() {
        let mut tuner = RadioTuner::new(SimulatedRadio::default()).unwrap();
        let mut invalid = settings();
        invalid.gain_db_tenths = Some(600);
        assert_eq!(tuner.tune(invalid), Err(RadioError::UnsupportedGain(600)));
        tuner.tune(settings()).unwrap();
        assert!(tuner.read_iq(&mut [0; 3]).is_err());
    }
}

//! Host compatibility path for the standalone rust_radio module.
pub use rust_radio::pcm::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_mapping_preserves_expected_samples() {
        let mut resampler = PcmConverter::new(48_000, 48_000, 2, 2).unwrap();
        let mut out = Vec::new();
        assert!(resampler
            .convert(&[0.2, 0.8, 0.4, 0.6], |frame| {
                out.extend_from_slice(frame);
                true
            })
            .unwrap());
        assert_eq!(out.len(), 4);
        assert!((out[0] - 0.2).abs() < 0.0001);
        assert!((out[3] - 0.6).abs() < 0.0001);
    }

    #[test]
    fn mono_to_stereo_and_surround_silence() {
        let mut resampler = PcmConverter::new(48_000, 48_000, 1, 4).unwrap();
        let mut out = Vec::new();
        resampler
            .convert(&[0.5, 0.5], |frame| {
                out.extend_from_slice(frame);
                true
            })
            .unwrap();
        assert_eq!(out, [0.5, 0.5, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0]);
    }

    #[test]
    fn common_radio_rate_resamples_to_device_rate() {
        let mut resampler = PcmConverter::new(44_100, 48_000, 1, 2).unwrap();
        let mut frames = 0usize;
        let samples = vec![0.4; 44_100];
        assert!(resampler
            .convert(&samples, |_| {
                frames += 1;
                true
            })
            .unwrap());
        assert!((47_998..=48_001).contains(&frames));
    }

    #[test]
    fn preserves_phase_across_packet_boundaries() {
        let mut one = PcmConverter::new(44_100, 48_000, 1, 1).unwrap();
        let mut two = PcmConverter::new(44_100, 48_000, 1, 1).unwrap();
        let signal: Vec<_> = (0..100).map(|n| n as f32 / 100.0).collect();
        let mut single = Vec::new();
        let mut split = Vec::new();
        one.convert(&signal, |v| {
            single.extend_from_slice(v);
            true
        })
        .unwrap();
        for chunk in signal.chunks(17) {
            two.convert(chunk, |v| {
                split.extend_from_slice(v);
                true
            })
            .unwrap();
        }
        assert_eq!(single, split);
    }

    #[test]
    fn invalid_format_and_cancellation() {
        assert!(matches!(
            PcmConverter::new(0, 48_000, 2, 2),
            Err(PcmError::UnsupportedRate)
        ));
        assert!(matches!(
            PcmConverter::new(44_100, 48_000, 0, 2),
            Err(PcmError::InvalidChannelCount)
        ));
        let mut r = PcmConverter::new(44_100, 48_000, 1, 2).unwrap();
        let mut calls = 0;
        assert!(!r
            .convert(&[0.2; 32], |_| {
                calls += 1;
                calls < 3
            })
            .unwrap());
    }

    #[test]
    fn non_finite_inputs_do_not_leak() {
        let mut r = PcmConverter::new(48_000, 48_000, 1, 1).unwrap();
        let mut out = Vec::new();
        r.convert(&[f32::NAN, f32::INFINITY], |v| {
            out.extend_from_slice(v);
            true
        })
        .unwrap();
        assert_eq!(out, vec![0.0; 2]);
    }
}

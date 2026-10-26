//! Explicit, offline demonstration of the receive-only radio abstraction.
//! No hardware capture is performed by this command.
use pipboy_core::radio::{Demodulation, RadioTuner, SimulatedRadio, Tuning};
use std::error::Error;

fn modulation(value: &str) -> Option<Demodulation> {
    match value.to_ascii_lowercase().as_str() {
        "am" => Some(Demodulation::Am),
        "nfm" => Some(Demodulation::Nfm),
        "wfm" => Some(Demodulation::Wfm),
        "usb" => Some(Demodulation::Usb),
        "lsb" => Some(Demodulation::Lsb),
        "iq" => Some(Demodulation::RawIq),
        _ => None,
    }
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    match args.as_slice() {
        [action] if action == "capabilities" => {
            let tuner = RadioTuner::new(SimulatedRadio::default())?;
            let caps = tuner.capabilities();
            println!("RADIO / {} / simulation only", caps.name);
            for range in &caps.frequency_ranges {
                println!("RF RANGE: {}..={} Hz", range.min_hz, range.max_hz);
            }
            println!("SAMPLE RATES: {:?} Hz", caps.sample_rates_hz);
            println!("No RF device is opened, sampled, or claimed.");
            Ok(())
        }
        [action, hz, mode] if action == "simulate" => {
            let frequency_hz: u64 = hz.parse()?;
            let demodulation = modulation(mode).ok_or("mode must be am, nfm, wfm, usb, lsb, or iq")?;
            let mut tuner = RadioTuner::new(SimulatedRadio::default())?;
            tuner.tune(Tuning {
                frequency_hz,
                sample_rate_hz: 1_024_000,
                demodulation,
                gain_db_tenths: None,
            })?;
            let mut buffer = [0i16; 256];
            let count = tuner.read_iq(&mut buffer)?;
            println!("RADIO SIM / {} Hz / {:?}", frequency_hz, demodulation);
            println!("I/Q: {count} interleaved samples / deterministic silence");
            println!("SIMULATED ONLY: no antenna, live RF or audio decoder connected.");
            Ok(())
        }
        _ => Err("usage: pipboy radio <capabilities|simulate FREQ_HZ am|nfm|wfm|usb|lsb|iq>".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_inputs() {
        assert!(run(vec!["simulate".into(), "0".into(), "am".into()]).is_err());
        assert!(run(vec!["simulate".into(), "100000000".into(), "bad".into()]).is_err());
    }
}

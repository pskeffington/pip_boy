//! Thin operator-command adapter; receiver ownership and capture live in rust_radio.
use std::error::Error;
use std::time::Duration;

enum Request<'a> {
    Devices,
    Capture {
        id: &'a str,
        tuning: rust_radio::sdr::Tuning,
        duration: Duration,
    },
}

fn parse(args: &[String]) -> Result<Request<'_>, Box<dyn Error>> {
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["sdr-devices", "--confirm"] => Ok(Request::Devices),
        ["sdr-capture", id, frequency, rate, "--milliseconds", milliseconds, "--confirm"] => {
            if !id.starts_with("soapy-") || id.len() != 22 || !id[6..].bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("Copy a receiver ID from radio sdr-devices --confirm".into());
            }
            let frequency_hz: u64 = frequency.parse()?;
            let sample_rate_hz: u32 = rate.parse()?;
            let milliseconds: u64 = milliseconds.parse()?;
            if frequency_hz == 0 || sample_rate_hz == 0 || !(1..=5000).contains(&milliseconds) {
                return Err("Frequency/rate must be positive Hz; duration must be 1..5000 ms".into());
            }
            Ok(Request::Capture {
                id,
                tuning: rust_radio::sdr::Tuning {
                    frequency_hz,
                    sample_rate_hz,
                    demodulation: rust_radio::sdr::Demodulation::RawIq,
                    gain_db_tenths: None,
                },
                duration: Duration::from_millis(milliseconds),
            })
        }
        _ => Err("Use radio sdr-devices --confirm or radio sdr-capture DEVICE_ID FREQ_HZ RATE_HZ --milliseconds 1..5000 --confirm".into()),
    }
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    execute(parse(args)?)
}

#[cfg(not(feature = "soapy-sdr"))]
fn execute(request: Request<'_>) -> Result<(), Box<dyn Error>> {
    // Keep parsing/validation identical in SDK-free builds.
    if let Request::Capture {
        id,
        tuning,
        duration,
    } = request
    {
        let _ = (id, tuning, duration);
    }
    Err("Hardware SDR is optional: install SoapySDR and a receiver driver, then build pipboy with --features soapy-sdr. IP radio remains available.".into())
}

#[cfg(feature = "soapy-sdr")]
fn execute(request: Request<'_>) -> Result<(), Box<dyn Error>> {
    match request {
        Request::Devices => {
            let devices = rust_radio::receiver::discover()?;
            if devices.is_empty() {
                println!(
                    "No physical SDR receivers found. IP radio remains available; nothing started."
                );
            }
            for device in devices {
                println!("{} | {} | {}", device.id, device.driver, device.label);
            }
        }
        Request::Capture {
            id,
            tuning,
            duration,
        } => {
            let receiver = rust_radio::receiver::SoapyReceiver::open(id, 0, tuning.sample_rate_hz)?;
            let report = rust_radio::receiver::capture(
                receiver,
                tuning,
                duration,
                &std::sync::atomic::AtomicBool::new(false),
            )?;
            println!(
                "RECEIVE ONLY / {:?} / {} complex samples / {:?} / stopped",
                report.receiver_kind, report.complex_samples, report.elapsed
            );
            println!(
                "I/Q ingress only; no RF audio demodulation or signal verification performed."
            );
            if report.complex_samples == 0 {
                return Err("No I/Q samples received before the deadline".into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(raw: &[&str]) -> Vec<String> {
        raw.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn invalid_or_unconfirmed_requests_never_reach_the_sdk() {
        for raw in [
            vec!["sdr-devices"],
            vec!["sdr-devices", "--confirm", "extra"],
            vec![
                "sdr-capture",
                "soapy-0000000000000000",
                "100100000",
                "1024000",
                "--milliseconds",
                "500",
            ],
            vec![
                "sdr-capture",
                "driver=remote",
                "100100000",
                "1024000",
                "--milliseconds",
                "500",
                "--confirm",
            ],
            vec![
                "sdr-capture",
                "soapy-0000000000000000",
                "0",
                "1024000",
                "--milliseconds",
                "500",
                "--confirm",
            ],
            vec![
                "sdr-capture",
                "soapy-0000000000000000",
                "100100000",
                "0",
                "--milliseconds",
                "500",
                "--confirm",
            ],
            vec![
                "sdr-capture",
                "soapy-0000000000000000",
                "100100000",
                "1024000",
                "--milliseconds",
                "5001",
                "--confirm",
            ],
        ] {
            assert!(run(&args(&raw)).is_err());
        }
    }

    #[test]
    fn valid_requests_parse_without_discovery_or_open() {
        assert!(matches!(
            parse(&args(&["sdr-devices", "--confirm"])).unwrap(),
            Request::Devices
        ));
        let raw = args(&[
            "sdr-capture",
            "soapy-0000000000000000",
            "100100000",
            "1024000",
            "--milliseconds",
            "500",
            "--confirm",
        ]);
        match parse(&raw).unwrap() {
            Request::Capture {
                tuning, duration, ..
            } => {
                assert_eq!(tuning.demodulation, rust_radio::sdr::Demodulation::RawIq);
                assert_eq!(duration, Duration::from_millis(500));
            }
            _ => panic!("expected capture"),
        }
    }
}

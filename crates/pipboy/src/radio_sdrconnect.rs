//! Host command presentation only; networking/protocol/cleanup live in rust_radio.
use std::error::Error;

pub fn validate_peer(peer: &str) -> Result<String, String> {
    #[cfg(feature = "sdrconnect")]
    {
        rust_radio::receiver::sdrconnect::Endpoint::parse(peer)
            .map(|p| p.address().to_string())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(feature = "sdrconnect"))]
    {
        let _ = peer;
        Err("Build with the sdrconnect feature to enable the network receiver".into())
    }
}

pub fn start_native(
    peer: &str,
    volume: u8,
    output: &str,
) -> Result<crate::radio_playback::NativeRadioEngine, String> {
    #[cfg(feature = "sdrconnect")]
    {
        crate::radio_playback::NativeRadioEngine::start_sdrconnect(peer, volume, output)
    }
    #[cfg(not(feature = "sdrconnect"))]
    {
        let _ = (peer, volume, output);
        Err("Build with the sdrconnect feature to enable the network receiver".into())
    }
}

pub fn receiver_info(player: &crate::radio_playback::NativeRadioEngine) -> serde_json::Value {
    #[cfg(feature = "sdrconnect")]
    {
        player.receiver_status().map(|s|serde_json::json!({"name":s.active_device,"frequency_hz":s.center_frequency_hz,"demodulator":s.demodulator,"antenna":s.active_antenna})).unwrap_or(serde_json::Value::Null)
    }
    #[cfg(not(feature = "sdrconnect"))]
    {
        let _ = player;
        serde_json::Value::Null
    }
}

pub fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    // Confirmation and bounded command syntax are enforced even in SDK-free
    // default builds. Never construct a client before validating operator input.
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["sdrconnect-status", address, "--confirm"] => {
            let _: std::net::SocketAddr = address.parse()?;
        }
        ["sdrconnect-capture", address, kind, "--milliseconds", ms, "--confirm"] => {
            let _: std::net::SocketAddr = address.parse()?;
            let milliseconds: u64 = ms.parse()?;
            if !matches!(*kind, "iq" | "audio") || !(1..=5000).contains(&milliseconds) {
                return Err("Use iq|audio and a duration of 1..5000 milliseconds".into());
            }
        }
        _ => return Err("Use radio sdrconnect-status IP:PORT --confirm or radio sdrconnect-capture IP:PORT iq|audio --milliseconds 1..5000 --confirm".into()),
    }
    execute(args)
}

#[cfg(not(feature = "sdrconnect"))]
fn execute(_: &[String]) -> Result<(), Box<dyn Error>> {
    Err("Enable pipboy's optional sdrconnect feature for nRSP-ST network reception. Connect SDRconnect to the receiver and enable its WebSocket API; no USB SoapySDR driver is required.".into())
}

#[cfg(feature = "sdrconnect")]
fn execute(args: &[String]) -> Result<(), Box<dyn Error>> {
    use rust_radio::receiver::sdrconnect::{Client, Endpoint, StreamKind};
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;
    let endpoint = Endpoint::parse(&args[1])?;
    let cancel = AtomicBool::new(false);
    let mut client = Client::connect(endpoint, &cancel)?;
    if args[0] == "sdrconnect-status" {
        let state = client.status(&cancel)?;
        println!(
            "SDRconnect {} / {} / started: {} / control: {}",
            state.api_version, state.active_device, state.started, state.can_control
        );
        println!("Devices: {:?}", state.valid_devices);
        println!(
            "{} Hz / {} samples/s / {} / antenna: {}",
            state.center_frequency_hz,
            state.sample_rate_hz,
            state.demodulator,
            state.active_antenna
        );
        println!(
            "Antennas: {:?} / read-only status; no stream activated",
            state.antennas
        );
    } else {
        let kind = if args[2] == "iq" {
            StreamKind::Iq
        } else {
            StreamKind::Audio
        };
        let duration = Duration::from_millis(args[4].parse()?);
        let report = client.receive_for(kind, duration, &cancel, |_| {})?;
        println!(
            "SDRconnect {:?} / {} sample pairs / {:?} / unsubscribed",
            report.kind, report.sample_pairs, report.elapsed
        );
        println!("Network ingress only; no native audio output or physical signal verification performed.");
        if report.sample_pairs == 0 {
            return Err("No sample pairs received before the deadline".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_and_unconfirmed_network_requests_never_connect() {
        for raw in [
            vec!["sdrconnect-status", "127.0.0.1:9"],
            vec!["sdrconnect-status", "not-an-address", "--confirm"],
            vec![
                "sdrconnect-capture",
                "127.0.0.1:9",
                "iq",
                "--milliseconds",
                "0",
                "--confirm",
            ],
            vec![
                "sdrconnect-capture",
                "127.0.0.1:9",
                "audio",
                "--milliseconds",
                "5001",
                "--confirm",
            ],
            vec![
                "sdrconnect-capture",
                "127.0.0.1:9",
                "unknown",
                "--milliseconds",
                "500",
                "--confirm",
            ],
            vec![
                "sdrconnect-capture",
                "127.0.0.1:9",
                "audio",
                "--milliseconds",
                "500",
            ],
        ] {
            let args: Vec<_> = raw.into_iter().map(str::to_owned).collect();
            assert!(run(&args).is_err());
        }
    }
}

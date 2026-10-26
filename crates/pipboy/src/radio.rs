//! Explicit receive-only radio CLI; no external media players are launched.
use crate::radio_http::NativeRadioHttp;
use pipboy_core::radio::{Demodulation, RadioTuner, SimulatedRadio, Tuning};
use pipboy_core::radio_directory::{
    curated_stations, select_source, station_by_id, station_tcp_reachable, NoHardwareEnumerator,
    PublisherTcpCheck, RadioSource, UncheckedConnectivity,
};
use pipboy_core::radio_stream::open_station;
use std::error::Error;
use std::io::Read;
use std::time::Duration;

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
        [action, id, confirm] if action == "probe" && confirm == "--confirm" => {
            let _ = station_by_id(id).ok_or("unknown curated radio station ID")?;
            let transport = NativeRadioHttp::for_probe()?;
            let mut stream = open_station(&transport, id)?;
            let mut sample = [0u8; 8192];
            let count = stream.reader.read(&mut sample)?;
            if count == 0 {
                return Err("stream ended before audio bytes arrived".into());
            }
            println!("RADIO / {} / HTTPS AUDIO BYTES: {}", id, count);
            println!("STREAM CHECK: bytes received, audio decoding/output not implemented.");
            Ok(())
        }
        [action, id, confirm] if action == "check" && confirm == "--confirm" => {
            let station = station_by_id(id).ok_or("unknown curated radio station ID")?;
            let reachability = station_tcp_reachable(id, Duration::from_secs(2))
                .ok_or("station is not allowlisted")?;
            println!("RADIO / {} / TCP: {:?}", station.name, reachability);
            println!("TCP reachability does not prove stream data or audio playback.");
            Ok(())
        }
        [action] if action == "stations" => {
            println!("RADIO / CURATED DIRECTORY / published metadata, stream unchecked");
            for station in curated_stations() {
                println!(
                    "{} | {} | {} | {}",
                    station.id, station.name, station.genre, station.playlist_url
                );
            }
            println!("No external player launches. Native audio worker is the next integration gate.");
            Ok(())
        }
        [action, confirm] if action == "stations" && confirm == "--check-ip" => {
            let check = PublisherTcpCheck::default();
            let source = select_source(&NoHardwareEnumerator, &check);
            println!("RADIO SOURCE: {:?}", source);
            if source == RadioSource::InternetDirectory {
                println!("Publisher TCP reachable; stream health and playback NOT yet verified.");
                for station in curated_stations() {
                    println!(
                        "{} | {} | {} | {}",
                        station.id, station.name, station.genre, station.playlist_url
                    );
                }
            } else {
                println!("No approved provider route detected. Offline radio mode.");
            }
            println!("Physical SDR autodetection is not yet implemented; no actual receiver was queried.");
            Ok(())
        }
        [action] if action == "source" => {
            let source = select_source(&NoHardwareEnumerator, &UncheckedConnectivity);
            println!("RADIO SOURCE: {:?} (network not checked; no physical SDR enumerator)", source);
            Ok(())
        }
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
        _ => Err("usage: pipboy radio <capabilities|source|stations [--check-ip]|check STATION_ID --confirm|probe STATION_ID --confirm|simulate FREQ_HZ am|nfm|wfm|usb|lsb|iq>".into()),
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

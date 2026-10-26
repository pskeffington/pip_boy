//! Explicit, offline demonstration of the receive-only radio abstraction.
//! No hardware capture is performed by this command.
use pipboy_core::radio::{Demodulation, RadioTuner, SimulatedRadio, Tuning};
use std::error::Error;
use std::process::Command;
use pipboy_core::radio_directory::{curated_stations, station_by_id, select_source, NoHardwareEnumerator, PublisherTcpCheck, UncheckedConnectivity, RadioSource};

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

/// User-initiated listening through an installed player. No shell interpolation.
fn launch_station(id: &str, player: &str) -> Result<(), Box<dyn Error>> {
    let station = station_by_id(id).ok_or("unknown curated radio station ID")?;
    let url = station.playlist_url;
    let mut command = match player {
        "vlc" => {
            let mut c = Command::new("vlc");
            c.arg("--no-video").arg(url);
            c
        }
        "ffplay" => {
            let mut c = Command::new("ffplay");
            c.arg("-nodisp").arg("-autoexit").arg(url);
            c
        }
        "system" => {
            if cfg!(target_os = "macos") {
                let mut c = Command::new("open");
                c.arg(url);
                c
            } else if cfg!(target_os = "windows") {
                let mut c = Command::new("explorer");
                c.arg(url);
                c
            } else {
                let mut c = Command::new("xdg-open");
                c.arg(url);
                c
            }
        }
        _ => return Err("player must be system, vlc, or ffplay".into()),
    };
    command.spawn().map_err(|e| format!("failed to start {player} player: {e}"))?;
    println!("RADIO / opening {} via {} (playback not verified)", station.name, player);
    Ok(())
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    match args.as_slice() {
        [action, id, confirm] if action == "listen" && confirm == "--confirm" => {
            launch_station(id, "system")
        }
        [action, id, confirm, player_flag, player]
            if action == "listen" && confirm == "--confirm" && player_flag == "--player" => {
            launch_station(id, player)
        }
        [action] if action == "stations" => {
            println!("RADIO / CURATED DIRECTORY / published metadata, stream unchecked");
            for station in curated_stations() {
                println!("{} | {} | {} | {}", station.id, station.name, station.genre, station.playlist_url);
            }
            println!("No playback initiated. To start: pipboy radio listen skylab --confirm --player vlc");
            Ok(())
        }
        [action, confirm] if action == "stations" && confirm == "--check-ip" => {
            let check = PublisherTcpCheck::default();
            let source = select_source(&NoHardwareEnumerator, &check);
            println!("RADIO SOURCE: {:?}", source);
            if source == RadioSource::InternetDirectory {
                println!("Publisher TCP reachable; stream health and playback NOT yet verified.");
                for station in curated_stations() {
                    println!("{} | {} | {} | {}", station.id, station.name, station.genre, station.playlist_url);
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
        _ => Err("usage: pipboy radio <capabilities|source|stations [--check-ip]|listen STATION_ID --confirm [--player system|vlc|ffplay]|simulate FREQ_HZ am|nfm|wfm|usb|lsb|iq>".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_inputs() {
        assert!(run(vec!["simulate".into(), "0".into(), "am".into()]).is_err());
        assert!(run(vec!["simulate".into(), "100000000".into(), "bad".into()]).is_err());
        assert!(launch_station("unknown", "system").is_err());
        assert!(launch_station("skylab", "invalid").is_err());
    }
}

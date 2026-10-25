//! Explicit direct LAN connection; no discovery, background service or saved game data.
use crate::game_view::{Action, Browser};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use pipboy_core::game::{Database, Frames};
use pipboy_core::permissions::{Preference, PreferenceStore};
use ratatui::prelude::*;
use std::{
    error::Error,
    io::{self, Read, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    time::{Duration, Instant},
};

fn playstation_address(raw: &str) -> Result<SocketAddr, std::net::AddrParseError> {
    raw.parse::<SocketAddr>()
        .or_else(|_| raw.parse::<IpAddr>().map(|ip| SocketAddr::new(ip, 27000)))
}

fn game_handshake(payload: &[u8]) -> Result<serde_json::Value, Box<dyn Error>> {
    if payload.len() > 4096 {
        return Err("Fallout 4 handshake exceeds 4096 bytes".into());
    }
    let hello: serde_json::Value = serde_json::from_slice(payload)?;
    for field in ["lang", "version"] {
        if !hello
            .get(field)
            .and_then(|value| value.as_str())
            .is_some_and(|value| {
                !value.trim().is_empty()
                    && value.len() <= 256
                    && !value.chars().any(char::is_control)
            })
        {
            return Err(
                format!("invalid Fallout 4 handshake: expected a nonempty {field} string").into(),
            );
        }
    }
    Ok(hello)
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let usage = "usage: pipboy game (--connect IP:PORT | --playstation IP[:PORT]) --confirm [--snapshot | --diagnose]";
    let mut address = None;
    let mut confirm = false;
    let mut snapshot = false;
    let mut diagnose = false;
    let mut playstation = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--connect" if address.is_none() => {
                address = Some(iter.next().ok_or(usage)?.parse::<SocketAddr>()?)
            }
            "--playstation" if address.is_none() => {
                let raw = iter.next().ok_or(usage)?;
                address = Some(playstation_address(raw)?);
                playstation = true;
            }
            "--confirm" if !confirm => confirm = true,
            "--snapshot" if !snapshot && !diagnose => snapshot = true,
            "--diagnose" if !snapshot && !diagnose => {
                snapshot = true;
                diagnose = true;
            }
            _ => return Err(usage.into()),
        }
    }
    let address = address.ok_or(usage)?;
    if !confirm {
        return Err("game connection requires explicit --confirm".into());
    }
    if PreferenceStore::from_environment()?
        .list()?
        .iter()
        .any(|entry| entry.capability == "network.fetch" && entry.preference == Preference::Denied)
    {
        return Err("game connection denied by saved network.fetch preference; clear it explicitly with pipboy permissions clear network.fetch".into());
    }
    let target = if playstation {
        "playstation"
    } else {
        "generic"
    };
    let advice = if playstation {
        "check the console IP, same LAN (avoid guest isolation/VPN), loaded Fallout 4 save, and companion setting on that exact PS4/PS5 game build; allow macOS Local Network access if prompted; PC companion-enabler mods do not apply to PlayStation"
    } else {
        "check game edition/service and firewall; Anniversary builds may require a game-side companion enabler"
    };
    // Literal addresses avoid DNS activity and any network discovery.
    let mut socket =
        TcpStream::connect_timeout(&address, Duration::from_secs(3)).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "cannot reach Fallout 4 at {address}: {error}; {advice} (docs/GAME_MODE.md)"
                ),
            )
        })?;
    socket.set_read_timeout(Some(Duration::from_millis(50)))?;
    socket.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut decoder = Frames::default();
    let mut db = Database::default();
    let started = Instant::now();
    let mut last_data = Instant::now();
    let mut received = Instant::now();
    let mut handshake = None;
    let mut received_root = false;
    let mut browser = Browser::default();
    let _guard = if snapshot {
        None
    } else {
        Some(crate::tui::TerminalGuard::enter()?)
    };
    let mut terminal = if snapshot {
        None
    } else {
        Some(Terminal::new(CrosstermBackend::new(io::stdout()))?)
    };
    loop {
        let mut bytes = [0u8; 65536];
        match socket.read(&mut bytes) {
            Ok(0) => return Err("Fallout 4 disconnected; game data is no longer live".into()),
            Ok(n) => {
                received = Instant::now();
                for (channel, payload) in decoder.feed(&bytes[..n])? {
                    match channel {
                        0 if payload.is_empty() => socket.write_all(&[0; 5])?,
                        0 => return Err("malformed game heartbeat".into()),
                        1 => {
                            handshake = Some(game_handshake(&payload)?);
                        }
                        2 => {
                            return Err(
                                "Fallout 4 is busy: disconnect the other Pip-Boy companion".into()
                            )
                        }
                        3 => {
                            db.update(&payload)?;
                            last_data = Instant::now();
                        }
                        4 | 6 => {} // No map-image or RPC request is sent by this read-only browser.
                        _ => return Err(format!("unsupported game channel {channel}").into()),
                    }
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(e) => return Err(e.into()),
        }
        if received.elapsed() > Duration::from_secs(15) {
            return Err(format!("Fallout 4 connection timed out at {address}; {advice}").into());
        }
        if handshake.is_none() && started.elapsed() > Duration::from_secs(10) {
            return Err(format!(
                "no Fallout 4 language/version handshake received at {address}; {advice}"
            )
            .into());
        }
        let database_ready = db.ready();
        received_root |= database_ready;
        if !received_root && started.elapsed() > Duration::from_secs(10) {
            return Err(format!(
                "no complete Fallout 4 root database received at {address}; {advice}"
            )
            .into());
        }
        let ready = handshake.is_some() && database_ready;
        if diagnose && ready {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"status":"ready","target_profile":target,"endpoint":address.to_string(),"handshake":handshake,"root_fields":db.children(0).iter().map(|entry|&entry.0).collect::<Vec<_>>(),"game_actions_enabled":false,"console_identity_verified":false})
                )?
            );
            return Ok(());
        }
        if snapshot && ready {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"game":"fallout4","target_profile":target,"source":address.to_string(),"game_handshake":handshake.as_ref().unwrap().to_string(),"view_limits":{"depth":32,"nodes":5000},"data":db.view(0)})
                )?
            );
            return Ok(());
        }
        if snapshot {
            continue;
        }
        let version = handshake
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "awaiting game handshake".to_owned());
        let header = format!(
            "Rust-Boy GAME / Fallout 4 / {target} / {address}\n{} | update age {} s | read-only\n{version}",
            if ready {
                "CONNECTED"
            } else if handshake.is_none() {
                "WAITING FOR GAME HANDSHAKE"
            } else {
                "WAITING FOR COMPLETE GAME DATA"
            },
            last_data.elapsed().as_secs()
        );
        terminal
            .as_mut()
            .unwrap()
            .draw(|frame| browser.render(frame, &db, &header))?;
        if event::poll(Duration::from_millis(20))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q' | 'Q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('c' | 'C') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(())
                    }
                    KeyCode::Down => browser.apply(Action::Down, &db),
                    KeyCode::Up => browser.apply(Action::Up, &db),
                    KeyCode::Enter | KeyCode::Right => browser.apply(Action::Open, &db),
                    KeyCode::Backspace | KeyCode::Left => browser.apply(Action::Back, &db),
                    KeyCode::Char('r' | 'R') => browser.apply(Action::Root, &db),
                    KeyCode::PageDown => browser.apply(Action::PageDown, &db),
                    KeyCode::PageUp => browser.apply(Action::PageUp, &db),
                    KeyCode::Home => browser.apply(Action::Home, &db),
                    KeyCode::End => browser.apply(Action::End, &db),
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handshake_requires_bounded_language_and_version_but_allows_extra_fields() {
        let hello = game_handshake(br#"{"version":"1.1.30.0","lang":"de","extra":true}"#).unwrap();
        assert_eq!(hello["version"], "1.1.30.0");
        assert_eq!(hello["extra"], true);
        for invalid in [
            "null",
            "[]",
            "{}",
            "{",
            r#"{"version":1,"lang":"en"}"#,
            r#"{"version":"1","lang":""}"#,
            r#"{"version":" ","lang":"en"}"#,
            r#"{"version":"1","lang":"en\u001b"}"#,
        ] {
            assert!(game_handshake(invalid.as_bytes()).is_err(), "{invalid}");
        }
        let too_long = serde_json::json!({"version":"1".repeat(257),"lang":"en"}).to_string();
        assert!(game_handshake(too_long.as_bytes()).is_err());
        assert!(game_handshake(&vec![b' '; 4097]).is_err());
    }

    #[test]
    fn playstation_preset_defaults_to_companion_port_without_dns() {
        assert_eq!(
            playstation_address("192.168.1.50").unwrap().to_string(),
            "192.168.1.50:27000"
        );
        assert_eq!(
            playstation_address("192.168.1.50:12345").unwrap().port(),
            12345
        );
        assert_eq!(playstation_address("fd00::1").unwrap().port(), 27000);
        assert_eq!(
            playstation_address("[fd00::1]:12345").unwrap().port(),
            12345
        );
        assert!(playstation_address("playstation.local").is_err());
        assert!(playstation_address("192.168.1.50;touch /tmp/example").is_err());
    }
}

//! Explicit direct LAN connection; no discovery, background service or saved game data.
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use pipboy_core::game::{Database, Frames};
use pipboy_core::permissions::{Preference, PreferenceStore};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};
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
    let mut updated = false;
    let mut received = Instant::now();
    let mut version = String::from("awaiting game handshake");
    let mut path = vec![("Fallout 4".to_owned(), 0_u32)];
    let mut selected = 0usize;
    let mut scroll = 0u16;
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
                            let hello: serde_json::Value = serde_json::from_slice(&payload)?;
                            version = hello.to_string();
                        }
                        2 => {
                            return Err(
                                "Fallout 4 is busy: disconnect the other Pip-Boy companion".into()
                            )
                        }
                        3 => {
                            db.update(&payload)?;
                            last_data = Instant::now();
                            updated = true;
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
            return Err("Fallout 4 connection timed out".into());
        }
        if !db.ready() && started.elapsed() > Duration::from_secs(10) {
            return Err("no Fallout 4 root database received; enable Pip-Boy App in the game and load a save".into());
        }
        if diagnose && db.ready() {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"status":"ready","target_profile":target,"endpoint":address.to_string(),"handshake":serde_json::from_str::<serde_json::Value>(&version).ok(),"root_fields":db.children(0).iter().map(|entry|&entry.0).collect::<Vec<_>>(),"game_actions_enabled":false,"console_identity_verified":false})
                )?
            );
            return Ok(());
        }
        if snapshot && db.ready() {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"game":"fallout4","target_profile":target,"source":address.to_string(),"game_handshake":version,"view_limits":{"depth":32,"nodes":5000},"data":db.view(0)})
                )?
            );
            return Ok(());
        }
        if snapshot {
            continue;
        }
        let id = path.last().unwrap().1;
        let children = db.children(id);
        selected = selected.min(children.len().saturating_sub(1));
        let title = path
            .iter()
            .map(|x| x.0.as_str())
            .collect::<Vec<_>>()
            .join(" / ");
        let mut body = format!(
            "GAME MODE / Fallout 4 / {target} / {address}\n{version}\n{} | update age {} s | read-only\n\n",
            if updated {
                "CONNECTED"
            } else {
                "WAITING FOR GAME DATA"
            },
            last_data.elapsed().as_secs()
        );
        for (index, (name, child)) in children.iter().enumerate() {
            body.push_str(&format!(
                "{} {} (object {})\n",
                if index == selected { ">" } else { " " },
                name,
                child
            ));
        }
        body.push_str("\nSELECTED VIEW\n");
        body.push_str(&serde_json::to_string_pretty(
            &db.view(children.get(selected).map(|x| x.1).unwrap_or(id)),
        )?);
        // Never interpret terminal control bytes received from the game.
        let body: String = body
            .chars()
            .filter(|c| !c.is_control() || *c == '\n')
            .take(262144)
            .collect();
        terminal.as_mut().unwrap().draw(|frame| {let area=frame.area();frame.render_widget(Paragraph::new(body.as_str()).wrap(Wrap{trim:false}).scroll((scroll,0)).block(Block::default().borders(Borders::ALL).title(title.chars().filter(|c|!c.is_control()).take(160).collect::<String>()).title_bottom("↑↓ select | Enter open | Backspace back | PgUp/PgDn scroll | Q/Ctrl+C exit")),area);})?;
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
                    KeyCode::Down => {
                        selected = (selected + 1).min(children.len().saturating_sub(1));
                        scroll = 0;
                    }
                    KeyCode::Up => {
                        selected = selected.saturating_sub(1);
                        scroll = 0;
                    }
                    KeyCode::Enter => {
                        if let Some(child) = children.get(selected) {
                            if path.len() < 32 {
                                path.push(child.clone());
                                selected = 0;
                                scroll = 0;
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        if path.len() > 1 {
                            path.pop();
                            selected = 0;
                            scroll = 0;
                        }
                    }
                    KeyCode::PageDown => scroll = scroll.saturating_add(10),
                    KeyCode::PageUp => scroll = scroll.saturating_sub(10),
                    KeyCode::Home => scroll = 0,
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

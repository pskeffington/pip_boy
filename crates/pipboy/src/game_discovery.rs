//! Opt-in PS4 legacy companion discovery. A single bounded UDP broadcast,
//! never automatic connection or authenticated identification.
use crate::game::ensure_network_allowed;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    error::Error,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::{Duration, Instant},
};

const DISCOVERY_PORT: u16 = 28000;
const COMPANION_PORT: u16 = 27000;
const MAX_REPLY: usize = 4096;
const DISCOVERY_WINDOW: Duration = Duration::from_millis(1300);
const REQUEST: &[u8] = br#"{"cmd":"autodiscover"}"#;

fn candidate(reply: &[u8], source: SocketAddr) -> Option<(Ipv4Addr, bool)> {
    if reply.len() > MAX_REPLY {
        return None;
    }
    let IpAddr::V4(ip) = source.ip() else {
        return None;
    };
    if ip.is_unspecified() || ip.is_multicast() || ip.is_broadcast() {
        return None;
    }
    let announcement: Value = serde_json::from_slice(reply).ok()?;
    if announcement.get("MachineType")?.as_str()? != "PS4" {
        return None;
    }
    let busy = announcement.get("IsBusy")?.as_bool()?;
    Some((ip, busy))
}

/// The target is injectable solely for deterministic loopback tests.
/// Production always sends one fixed local-network broadcast to UDP 28000.
fn discover_to(target: SocketAddr, window: Duration) -> io::Result<Vec<(Ipv4Addr, bool)>> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    socket.set_broadcast(true)?;
    socket.send_to(REQUEST, target)?;
    let deadline = Instant::now() + window;
    let mut found = BTreeMap::new();
    let mut buffer = [0u8; MAX_REPLY + 1];
    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        socket.set_read_timeout(Some(remaining.min(Duration::from_millis(100)).max(Duration::from_millis(1))))?;
        match socket.recv_from(&mut buffer) {
            Ok((len, from)) => {
                if let Some((ip, busy)) = candidate(&buffer[..len], from) {
                    found.insert(ip, busy);
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(found.into_iter().collect())
}

pub(crate) fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let usage = "usage: pipboy game --discover-ps4 --confirm [--json]";
    let (mut discover, mut confirm, mut as_json) = (false, false, false);
    for arg in args {
        match arg.as_str() {
            "--discover-ps4" if !discover => discover = true,
            "--confirm" if !confirm => confirm = true,
            "--json" if !as_json => as_json = true,
            _ => return Err(usage.into()),
        }
    }
    if !discover {
        return Err(usage.into());
    }
    if !confirm {
        return Err("PS4 local-network discovery requires explicit --confirm".into());
    }
    ensure_network_allowed()?;
    let destination = SocketAddr::new(Ipv4Addr::BROADCAST.into(), DISCOVERY_PORT);
    let devices = discover_to(destination, DISCOVERY_WINDOW).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("PS4 UDP discovery failed: {error}; check same LAN, client isolation and macOS Local Network permissions"),
        )
    })?;
    if as_json {
        let candidates: Vec<_> = devices
            .iter()
            .map(|(ip, busy)| {
                json!({
                    "ip": ip.to_string(),
                    "tcp_port": COMPANION_PORT,
                    "advertised_machine_type": "PS4",
                    "is_busy": busy
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "status": if candidates.is_empty() { "not_found" } else { "found" },
                "discovery": "one_opt_in_udp_broadcast",
                "console_identity_verified": false,
                "candidates": candidates
            }))?
        );
    } else if devices.is_empty() {
        println!("No PS4 companion advertisement received. Check that Fallout 4 is running with its companion service enabled on the same LAN; UDP broadcasts may be blocked.");
    } else {
        for (ip, busy) in devices {
            println!(
                "PS4 advertised at {ip} (TCP {COMPANION_PORT}) / {}",
                if busy { "busy" } else { "available" }
            );
            println!("  To connect: pipboy game --ps4 {ip} --confirm");
        }
        println!("INFO: a discovery advertisement is not authenticated console identity.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn discovery_accepts_only_bounded_ps4_advertisements() {
        let ps4: SocketAddr = "192.168.1.50:28000".parse().unwrap();
        let valid = br#"{"IsBusy":false,"MachineType":"PS4","extra":1}"#;
        assert_eq!(candidate(valid, ps4), Some((Ipv4Addr::new(192, 168, 1, 50), false)));
        assert_eq!(
            candidate(br#"{"IsBusy":true,"MachineType":"PS4"}"#, ps4),
            Some((Ipv4Addr::new(192, 168, 1, 50), true))
        );
        for invalid in [
            br#"{"IsBusy":false,"MachineType":"PC"}"#.as_slice(),
            br#"{"IsBusy":"false","MachineType":"PS4"}"#,
            br#"{"MachineType":"PS4"}"#,
            br#"{"IsBusy":false,"MachineType":"PS5"}"#,
            b"invalid",
            b"[]",
        ] {
            assert_eq!(candidate(invalid, ps4), None);
        }
        assert_eq!(candidate(&vec![b'x'; MAX_REPLY + 1], ps4), None);
        assert_eq!(candidate(valid, "[::1]:28000".parse().unwrap()), None);
        assert_eq!(candidate(valid, "255.255.255.255:28000".parse().unwrap()), None);
    }

    #[test]
    fn one_udp_request_discovers_a_mock_ps4_without_connecting_to_game() {
        let responder = UdpSocket::bind("127.0.0.1:0").unwrap();
        responder.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let destination = responder.local_addr().unwrap();
        let reply = thread::spawn(move || {
            let mut buffer = [0u8; 256];
            let (len, sender) = responder.recv_from(&mut buffer).unwrap();
            assert_eq!(&buffer[..len], REQUEST);
            responder.send_to(br#"{"IsBusy":false,"MachineType":"PC"}"#, sender).unwrap();
            responder.send_to(br#"{"IsBusy":true,"MachineType":"PS4"}"#, sender).unwrap();
            responder.send_to(br#"{"IsBusy":true,"MachineType":"PS4"}"#, sender).unwrap();
        });
        let found = discover_to(destination, Duration::from_millis(200)).unwrap();
        reply.join().unwrap();
        assert_eq!(found, vec![(Ipv4Addr::LOCALHOST, true)]);
    }

    #[test]
    fn discovery_rejects_missing_consent_and_invalid_flags_without_network() {
        for args in [
            vec!["--discover-ps4".to_owned()],
            vec!["--discover-ps4".into(), "--confirm".into(), "--confirm".into()],
            vec!["--discover-ps4".into(), "--confirm".into(), "--snapshot".into()],
        ] {
            assert!(run(&args).is_err());
        }
    }
}

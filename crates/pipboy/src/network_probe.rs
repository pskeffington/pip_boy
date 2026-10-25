//! Bounded, operator-confirmed single-endpoint TCP diagnostic.
//! No DNS, port-range loops, output data, packet payloads or background scans.
use pipboy_core::network_health::{self, HealthReport};
use pipboy_core::network_suite::tcp_target;
use std::io;
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn tcp_once(address: &str, port: &str, confirmed: bool) -> io::Result<String> {
    if !confirmed {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "TCP diagnostics require an explicit operator confirmation",
        ));
    }
    let target = tcp_target(address, port)?;
    let start = Instant::now();
    let status = match TcpStream::connect_timeout(&target, Duration::from_millis(900)) {
        Ok(stream) => {
            drop(stream);
            "CONNECTED (handshake complete)"
        }
        Err(error) => match error.kind() {
            io::ErrorKind::ConnectionRefused => "REFUSED / NO TCP LISTENER",
            io::ErrorKind::TimedOut => "TIMEOUT / NO TCP RESPONSE",
            _ => "UNREACHABLE / CONNECT FAILED",
        },
    };
    Ok(format!(
        "TCP CHECK / EXPLICIT\nTarget: {target}\nStatus: {status}\nElapsed: {} ms\nOne TCP connection attempt; no payload sent or scan performed.",
        start.elapsed().as_millis(),
    ))
}

/// Finite, explicitly confirmed set of TCP handshake measurements.
/// Connection timing includes local scheduling; it is not ICMP RTT.
pub fn tcp_health(
    address: &str,
    port: &str,
    samples: u8,
    confirmed: bool,
) -> io::Result<HealthReport> {
    if !confirmed {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Network health tests require explicit confirmation",
        ));
    }
    if !(network_health::MIN_SAMPLES..=network_health::MAX_SAMPLES).contains(&samples) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Sample count must be 3..8",
        ));
    }
    let target = tcp_target(address, port)?;
    let mut observations = Vec::with_capacity(samples as usize);
    for attempt in 0..samples {
        let start = Instant::now();
        let reading = TcpStream::connect_timeout(&target, Duration::from_millis(450))
            .ok()
            .map(|stream| {
                drop(stream);
                start.elapsed().as_secs_f64() * 1000.0
            });
        observations.push(reading);
        if attempt + 1 < samples {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    network_health::report(&target.to_string(), &observations)
}

/// Numeric-only OS ping: 3..8 probes to one literal IP, bounded runtime.
/// This reports actual ICMP RTT and response loss, not TCP reachability.
/// A missing or unauthorized OS ping produces an error, never fake metrics.
pub fn icmp_health(address: &str, samples: u8, confirmed: bool) -> io::Result<HealthReport> {
    if !confirmed {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "ICMP test needs explicit confirmation",
        ));
    }
    if !(network_health::MIN_SAMPLES..=network_health::MAX_SAMPLES).contains(&samples) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "ICMP sample count must be 3..8",
        ));
    }
    // Reuse the same literal, non-multicast, non-link-local IP validator.
    let target = tcp_target(address, "7")?.ip();
    let mut command = Command::new("ping");
    let count = samples.to_string();
    if cfg!(target_os = "windows") {
        command.args(["-n", &count, "-w", "900", &target.to_string()]);
    } else if cfg!(target_os = "macos") {
        command.args([
            "-n",
            "-c",
            &count,
            "-i",
            "0.25",
            "-W",
            "900",
            &target.to_string(),
        ]);
    } else {
        command.args([
            "-n",
            "-c",
            &count,
            "-i",
            "0.25",
            "-W",
            "1",
            "-w",
            "8",
            &target.to_string(),
        ]);
    }
    command.env("LANG", "C").env("LC_ALL", "C");
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command.spawn().map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Native ping executable is unavailable",
        )
    })?;
    let started = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if started.elapsed() > Duration::from_secs(9) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Native ping exceeded its 9-second limit",
            ));
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    let output = child.wait_with_output()?;
    if output.stdout.len() > 16 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Native ping output exceeded its 16-KiB limit",
        ));
    }
    parse_icmp(
        &String::from_utf8_lossy(&output.stdout),
        &target.to_string(),
        samples,
    )
}

/// Parses only native ping output from a complete count-bounded invocation.
/// The process exit status alone cannot mean 100% loss or justify inventing RTT.
fn parse_icmp(output: &str, address: &str, expected: u8) -> io::Result<HealthReport> {
    let sent_received = output.lines().find_map(|line| {
        if line.contains("packets transmitted") {
            let parts: Vec<_> = line.split(',').collect();
            let sent = parts
                .first()?
                .trim()
                .split_whitespace()
                .next()?
                .parse::<usize>()
                .ok()?;
            let received = parts
                .get(1)?
                .trim()
                .split_whitespace()
                .next()?
                .parse::<usize>()
                .ok()?;
            Some((sent, received))
        } else if line.contains("Packets: Sent = ") {
            let get = |needle: &str| -> Option<usize> {
                let text = line.split_once(needle)?.1.trim_start();
                text.split(|c: char| !c.is_ascii_digit())
                    .next()?
                    .parse::<usize>()
                    .ok()
            };
            Some((get("Sent = ")?, get("Received = ")?))
        } else {
            None
        }
    });
    let (sent, received) = sent_received.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Native ping did not provide a complete packet summary",
        )
    })?;
    if sent != expected as usize || received > sent {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Native ping packet summary did not match the requested count",
        ));
    }
    let mut rtts = Vec::with_capacity(received);
    for line in output.lines() {
        if !(line.contains("icmp_seq=") || line.trim_start().starts_with("Reply from ")) {
            continue;
        }
        let reading = if let Some((_, value)) = line.split_once("time=") {
            let number: String = value
                .chars()
                .take_while(|ch| ch.is_ascii_digit() || *ch == '.')
                .collect();
            number.parse::<f64>().ok()
        } else if line.contains("time<1ms") {
            // Windows reports a sub-millisecond bound, not an exact RTT.
            // Use the midpoint of the reported [0,1)ms interval and label it.
            Some(0.5)
        } else {
            None
        };
        if let Some(time) = reading {
            if time.is_finite() && time >= 0.0 {
                rtts.push(time);
            }
        }
    }
    if rtts.len() != received {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Ping RTT lines do not match received replies",
        ));
    }
    network_health::icmp_report(address, expected, &rtts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn no_connection_without_confirmation_and_local_loopback_probe() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port().to_string();
        assert_eq!(
            tcp_once("127.0.0.1", &port, false).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        let output = tcp_once("127.0.0.1", &port, true).unwrap();
        assert!(output.contains("CONNECTED"), "{output}");
        assert!(output.contains("One TCP connection attempt"));
        assert!(tcp_once("example.org", &port, true).is_err());
        assert!(tcp_once("127.0.0.1", "0", true).is_err());
    }

    #[test]
    fn health_requires_confirmation_and_only_samples_the_chosen_socket() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port().to_string();
        assert_eq!(
            tcp_health("127.0.0.1", &port, 4, false).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        let report = tcp_health("127.0.0.1", &port, 4, true).unwrap();
        assert_eq!(report.attempts, 4);
        assert_eq!(report.connected, 4);
        assert!(report.mean_ms.is_some());
        assert!(report.console().contains("NOT packet loss"));
        assert!(tcp_health("127.0.0.1", &port, 9, true).is_err());
        assert!(tcp_health("test.example", &port, 3, true).is_err());
    }

    #[test]
    fn parses_native_macos_linux_and_windows_ping_without_network() {
        let mac = "64 bytes from 192.168.1.1: icmp_seq=0 ttl=64 time=1.200 ms\n\
                   64 bytes from 192.168.1.1: icmp_seq=1 ttl=64 time=2.500 ms\n\
                   3 packets transmitted, 2 packets received, 33.3% packet loss";
        let r = parse_icmp(mac, "192.168.1.1", 3).unwrap();
        assert_eq!(r.connected, 2);
        assert_eq!(r.failed, 1);
        assert!(r.console().contains("ICMP packet loss"));
        let linux = "64 bytes from 127.0.0.1: icmp_seq=1 ttl=64 time=0.044 ms\n\
                     64 bytes from 127.0.0.1: icmp_seq=2 ttl=64 time=0.023 ms\n\
                     64 bytes from 127.0.0.1: icmp_seq=3 ttl=64 time=0.051 ms\n\
                     3 packets transmitted, 3 received, 0% packet loss";
        assert_eq!(parse_icmp(linux, "127.0.0.1", 3).unwrap().failed, 0);
        let windows = "Reply from 127.0.0.1: bytes=32 time<1ms TTL=128\n\
                       Reply from 127.0.0.1: bytes=32 time=1ms TTL=128\n\
                       Packets: Sent = 3, Received = 2, Lost = 1 (33% loss)";
        assert_eq!(parse_icmp(windows, "127.0.0.1", 3).unwrap().connected, 2);
        assert!(parse_icmp("timed out", "127.0.0.1", 3).is_err());
    }
}

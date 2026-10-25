//! Bounded, operator-confirmed single-endpoint TCP diagnostic.
//! No DNS, port-range loops, output data, packet payloads or background scans.
use pipboy_core::network_health::{self, HealthReport};
use pipboy_core::network_suite::tcp_target;
use std::io;
use std::net::TcpStream;
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
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Sample count must be 3..8"));
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
            tcp_health("127.0.0.1", &port, 4, false)
                .unwrap_err().kind(), io::ErrorKind::PermissionDenied
        );
        let report = tcp_health("127.0.0.1", &port, 4, true).unwrap();
        assert_eq!(report.attempts, 4);
        assert_eq!(report.connected, 4);
        assert!(report.mean_ms.is_some());
        assert!(report.console().contains("NOT packet loss"));
        assert!(tcp_health("127.0.0.1", &port, 9, true).is_err());
        assert!(tcp_health("test.example", &port, 3, true).is_err());
    }

}

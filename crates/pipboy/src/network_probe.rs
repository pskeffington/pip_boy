//! Bounded, operator-confirmed single-endpoint TCP diagnostic.
//! No DNS, port-range loops, output data, packet payloads or background scans.
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
}

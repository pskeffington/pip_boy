//! Deadline-bounded Unix-domain transport for the package socket protocol.
//!
//! This module provides framing and protocol validation only. A package ID in
//! the hello is an assertion, not authentication, and this transport does not
//! authorize capabilities, launch processes, or provide OS isolation.

use crate::package_socket::{
    read_protocol_frame, write_protocol_frame, AdapterHello, AdapterMessage, AdapterResponse,
    CorrelationError, FrameError, HandshakeAccepted, HandshakeError, ProtocolError,
    RequestCorrelator, RequestSequence, SOCKET_API_VERSION,
};
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum TransportError {
    Io(io::Error),
    Frame(FrameError),
    Protocol(ProtocolError),
    Handshake(HandshakeError),
    Correlation(CorrelationError),
    PeerClosed,
    PackageMismatch,
    SessionFailed,
    UnsupportedNegotiatedVersion(u32),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "Package socket I/O failed: {error}"),
            Self::Frame(error) => write!(f, "Package socket frame failed: {error}"),
            Self::Protocol(error) => write!(f, "Package socket protocol failed: {error}"),
            Self::Handshake(error) => write!(f, "Package socket handshake failed: {error:?}"),
            Self::Correlation(error) => write!(f, "Package socket response failed: {error:?}"),
            Self::PeerClosed => write!(f, "Package socket closed before completing the exchange"),
            Self::PackageMismatch => write!(f, "Package socket identity did not match the session"),
            Self::SessionFailed => {
                write!(f, "Package socket session has failed and cannot be reused")
            }
            Self::UnsupportedNegotiatedVersion(version) => {
                write!(
                    f,
                    "Negotiated package socket version is unsupported: {version}"
                )
            }
        }
    }
}

impl std::error::Error for TransportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Frame(error) => Some(error),
            Self::Protocol(error) => Some(error),
            _ => None,
        }
    }
}

/// A single negotiated local stream. Only one request may be outstanding at a
/// time; callers must still establish package provenance and capability policy
/// before constructing a session.
pub struct UnixAdapterSession {
    stream: UnixStream,
    timeout: Duration,
    package_id: String,
    accepted: HandshakeAccepted,
    sequence: RequestSequence,
    correlator: RequestCorrelator,
    failed: bool,
}

impl UnixAdapterSession {
    /// Completes the adapter's hello claim on an already accepted stream.
    /// The entire hello read must finish before `timeout` elapses.
    /// `timeout` must be greater than zero.
    pub fn from_stream(
        stream: UnixStream,
        expected_package_id: &str,
        supported_versions: &[u32],
        timeout: Duration,
    ) -> Result<Self, TransportError> {
        if timeout.is_zero() {
            return Err(TransportError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "package socket timeout must be nonzero",
            )));
        }
        stream.set_nonblocking(true).map_err(TransportError::Io)?;
        let stream = stream;
        let deadline = deadline_after(timeout)?;
        let frame = read_protocol_frame(&mut DeadlineIo::new(&stream, deadline))
            .map_err(TransportError::Frame)?
            .ok_or(TransportError::PeerClosed)?;
        let hello = AdapterHello::decode(&frame).map_err(TransportError::Protocol)?;
        let accepted = hello
            .negotiate(expected_package_id, supported_versions)
            .map_err(TransportError::Handshake)?;
        if accepted.protocol_version != SOCKET_API_VERSION {
            return Err(TransportError::UnsupportedNegotiatedVersion(
                accepted.protocol_version,
            ));
        }
        Ok(Self {
            stream,
            timeout,
            package_id: expected_package_id.to_owned(),
            accepted,
            sequence: RequestSequence::default(),
            correlator: RequestCorrelator::default(),
            failed: false,
        })
    }

    /// Negotiated wire version after validating the adapter's hello.
    pub fn protocol_version(&self) -> u32 {
        self.accepted.protocol_version
    }

    /// Sends one host request and waits for its strictly correlated response.
    /// Any transport/protocol failure should be treated as a failed session;
    /// the caller must not infer that the adapter performed the requested work.
    pub fn exchange(
        &mut self,
        request: &AdapterMessage,
    ) -> Result<AdapterResponse, TransportError> {
        if self.failed {
            return Err(TransportError::SessionFailed);
        }
        if request.package_id != self.package_id {
            return Err(TransportError::PackageMismatch);
        }
        if request.protocol_version != self.accepted.protocol_version {
            return Err(TransportError::UnsupportedNegotiatedVersion(
                request.protocol_version,
            ));
        }
        self.sequence
            .accept(request)
            .map_err(TransportError::Protocol)?;
        self.correlator
            .register(request)
            .map_err(TransportError::Correlation)?;
        let frame = request.encode().map_err(TransportError::Protocol)?;
        let result = (|| {
            let deadline = deadline_after(self.timeout)?;
            let mut io = DeadlineIo::new(&self.stream, deadline);
            write_protocol_frame(&mut io, &frame).map_err(TransportError::Frame)?;
            let frame = read_protocol_frame(&mut io)
                .map_err(TransportError::Frame)?
                .ok_or(TransportError::PeerClosed)?;
            let response = AdapterResponse::decode(&frame).map_err(TransportError::Protocol)?;
            self.correlator
                .complete(&response)
                .map_err(TransportError::Correlation)?;
            Ok(response)
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}

/// Adapts a Unix stream so every partial read/write shares one absolute
/// deadline. This prevents a peer from extending a frame indefinitely by
/// sending one byte just before each inactivity timeout.
struct DeadlineIo<'a> {
    stream: &'a UnixStream,
    deadline: Instant,
}

impl<'a> DeadlineIo<'a> {
    fn new(stream: &'a UnixStream, deadline: Instant) -> Self {
        Self { stream, deadline }
    }

    fn remaining(&self) -> io::Result<Duration> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            // Some Unix kernels reject sub-millisecond timeval values. The
            // next call rechecks the absolute deadline before performing I/O.
            .map(|remaining| remaining.max(Duration::from_millis(1)))
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::TimedOut, "package socket deadline elapsed")
            })
    }
}

impl Read for DeadlineIo<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            let remaining = self.remaining()?;
            match self.stream.read(buffer) {
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    std::thread::sleep(remaining.min(Duration::from_millis(1)));
                }
                result => return result,
            }
        }
    }
}

impl Write for DeadlineIo<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        loop {
            let remaining = self.remaining()?;
            match self.stream.write(buffer) {
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    std::thread::sleep(remaining.min(Duration::from_millis(1)));
                }
                result => return result,
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

fn deadline_after(timeout: Duration) -> Result<Instant, TransportError> {
    Instant::now().checked_add(timeout).ok_or_else(|| {
        TransportError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "package socket timeout is too large",
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package_socket::{AdapterCommand, AdapterResult};
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    use std::thread;

    const PACKAGE: &str = "org.rustboy.radio";
    const TIMEOUT: Duration = Duration::from_millis(300);

    fn send_frame(stream: &mut UnixStream, frame: &[u8]) {
        write_protocol_frame(stream, frame).unwrap();
    }

    fn hello_frame(package_id: &str) -> Vec<u8> {
        AdapterHello {
            package_id: package_id.into(),
            supported_versions: vec![1],
        }
        .encode()
        .unwrap()
    }

    fn request() -> AdapterMessage {
        AdapterMessage {
            protocol_version: 1,
            request_id: 1,
            package_id: PACKAGE.into(),
            command: AdapterCommand::Activate,
        }
    }

    #[test]
    fn unix_session_negotiates_and_correlates_one_exchange() {
        let (host, mut adapter) = UnixStream::pair().unwrap();
        let peer = thread::spawn(move || {
            send_frame(&mut adapter, &hello_frame(PACKAGE));
            let mut request_frame = read_protocol_frame(&mut adapter).unwrap().unwrap();
            let request = AdapterMessage::decode(&request_frame).unwrap();
            request_frame.fill(0);
            let response = AdapterResponse {
                protocol_version: 1,
                request_id: request.request_id,
                package_id: request.package_id,
                result: AdapterResult::Completed,
            }
            .encode()
            .unwrap();
            send_frame(&mut adapter, &response);
        });
        let mut session = UnixAdapterSession::from_stream(host, PACKAGE, &[1], TIMEOUT).unwrap();
        assert_eq!(session.protocol_version(), 1);
        let response = session.exchange(&request()).unwrap();
        assert_eq!(response.request_id, 1);
        assert_eq!(response.result, AdapterResult::Completed);
        peer.join().unwrap();
    }

    #[test]
    fn hello_identity_mismatch_fails_before_any_request() {
        let (host, mut adapter) = UnixStream::pair().unwrap();
        let peer =
            thread::spawn(move || send_frame(&mut adapter, &hello_frame("org.other.module")));
        let error = UnixAdapterSession::from_stream(host, PACKAGE, &[1], TIMEOUT)
            .err()
            .unwrap();
        assert!(
            matches!(
                error,
                TransportError::Handshake(HandshakeError::PackageIdentityMismatch)
            ),
            "unexpected handshake error: {error:?}"
        );
        peer.join().unwrap();
    }

    #[test]
    fn missing_hello_expires_at_the_configured_deadline() {
        let (host, _peer) = UnixStream::pair().unwrap();
        let started = std::time::Instant::now();
        let error = UnixAdapterSession::from_stream(host, PACKAGE, &[1], Duration::from_millis(25))
            .err()
            .unwrap();
        assert!(
            matches!(error, TransportError::Frame(FrameError::Io(ref e)) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut))
        );
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn slow_partial_hello_cannot_extend_the_absolute_deadline() {
        let (host, mut adapter) = UnixStream::pair().unwrap();
        let peer = thread::spawn(move || {
            let hello = hello_frame(PACKAGE);
            let header = (hello.len() as u32).to_be_bytes();
            adapter.write_all(&header[..1]).unwrap();
            for byte in &header[1..] {
                thread::sleep(Duration::from_millis(15));
                if adapter.write_all(std::slice::from_ref(byte)).is_err() {
                    return;
                }
            }
            let _ = adapter.write_all(&hello);
        });
        let error = UnixAdapterSession::from_stream(host, PACKAGE, &[1], Duration::from_millis(40))
            .err()
            .unwrap();
        assert!(matches!(
            error,
            TransportError::Frame(FrameError::Io(ref error))
                if matches!(error.kind(), io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock)
        ));
        peer.join().unwrap();
    }

    #[test]
    fn exchange_rejects_wrong_package_version_and_duplicate_request_id() {
        let (host, mut adapter) = UnixStream::pair().unwrap();
        let peer = thread::spawn(move || {
            send_frame(&mut adapter, &hello_frame(PACKAGE));
            let first = read_protocol_frame(&mut adapter).unwrap().unwrap();
            let first = AdapterMessage::decode(&first).unwrap();
            let response = AdapterResponse {
                protocol_version: 1,
                request_id: first.request_id,
                package_id: first.package_id,
                result: AdapterResult::Completed,
            }
            .encode()
            .unwrap();
            send_frame(&mut adapter, &response);
        });
        let mut session = UnixAdapterSession::from_stream(host, PACKAGE, &[1], TIMEOUT).unwrap();
        let mut wrong_package = request();
        wrong_package.package_id = "org.other.module".into();
        assert!(matches!(
            session.exchange(&wrong_package),
            Err(TransportError::PackageMismatch)
        ));
        let mut invalid_version = request();
        invalid_version.protocol_version = 2;
        assert!(matches!(
            session.exchange(&invalid_version),
            Err(TransportError::UnsupportedNegotiatedVersion(2))
        ));
        session.exchange(&request()).unwrap();
        assert!(matches!(
            session.exchange(&request()),
            Err(TransportError::Protocol(ProtocolError::InvalidRequestId))
        ));
        peer.join().unwrap();
    }

    #[test]
    fn response_timeout_and_truncation_are_reported_without_success_claim() {
        let (host, mut adapter) = UnixStream::pair().unwrap();
        let peer = thread::spawn(move || {
            send_frame(&mut adapter, &hello_frame(PACKAGE));
            let _ = read_protocol_frame(&mut adapter).unwrap();
            adapter.write_all(&[0, 0]).unwrap();
            adapter.shutdown(std::net::Shutdown::Write).unwrap();
        });
        let mut session = UnixAdapterSession::from_stream(host, PACKAGE, &[1], TIMEOUT).unwrap();
        assert!(matches!(
            session.exchange(&request()),
            Err(TransportError::Frame(FrameError::TruncatedHeader))
        ));
        let mut next = request();
        next.request_id = 2;
        assert!(matches!(
            session.exchange(&next),
            Err(TransportError::SessionFailed)
        ));
        peer.join().unwrap();
    }
}

//! Deadline-bounded Unix stream transport for the portable adapter SDK.
//!
//! The package ID remains an unauthenticated peer claim. This module provides
//! framing and correlation only; it does not authorize, launch, or isolate code.

use crate::package_listener::VerifiedUnixStream;
use crate::package_socket::FrameError;
use rust_boy_adapter_sdk::{
    AdapterEnvelope, AdapterError, AdapterHello, AdapterReplyEnvelope, ADAPTER_PROTOCOL,
    MAX_WIRE_BYTES,
};
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum TransportError {
    Io(io::Error),
    Frame(FrameError),
    Protocol(AdapterError),
    PeerClosed,
    PackageMismatch,
    SessionFailed,
    InvalidRequestId,
    UnsupportedNegotiatedVersion(u32),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "Adapter transport I/O failed: {error}"),
            Self::Frame(error) => write!(f, "Adapter frame failed: {error}"),
            Self::Protocol(error) => write!(f, "Adapter protocol failed: {error}"),
            Self::PeerClosed => write!(f, "Adapter closed before completing the exchange"),
            Self::PackageMismatch => {
                write!(f, "Adapter package identity did not match the session")
            }
            Self::SessionFailed => write!(f, "Adapter session failed and cannot be reused"),
            Self::InvalidRequestId => write!(f, "Adapter request IDs must increase strictly"),
            Self::UnsupportedNegotiatedVersion(version) => {
                write!(f, "Unsupported adapter protocol version: {version}")
            }
        }
    }
}

impl std::error::Error for TransportError {}

/// One negotiated local stream. Only one request may be outstanding; callers
/// must establish peer provenance and capability policy before constructing it.
pub struct UnixAdapterSession {
    stream: UnixStream,
    timeout: Duration,
    package_id: String,
    protocol: u32,
    last_request_id: u64,
    failed: bool,
}

impl UnixAdapterSession {
    /// Creates a session only from a peer admitted by `PrivateUnixListener`.
    /// The package ID hello remains a claim; process provenance comes from
    /// the operating system credentials captured during listener admission.
    pub fn from_verified_peer(
        stream: VerifiedUnixStream,
        expected_package_id: &str,
        supported_protocols: &[u32],
        timeout: Duration,
    ) -> Result<Self, TransportError> {
        Self::from_stream(
            stream.into_stream(),
            expected_package_id,
            supported_protocols,
            timeout,
        )
    }

    /// Accepts an already-open stream after a bounded SDK hello. The hello's
    /// package ID is a claim and is not peer authentication. This constructor
    /// is private so production callers must use `from_verified_peer`.
    fn from_stream(
        stream: UnixStream,
        expected_package_id: &str,
        supported_protocols: &[u32],
        timeout: Duration,
    ) -> Result<Self, TransportError> {
        if timeout.is_zero() {
            return Err(TransportError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "adapter timeout must be nonzero",
            )));
        }
        stream.set_nonblocking(true).map_err(TransportError::Io)?;
        let deadline = deadline_after(timeout)?;
        let frame = read_sdk_frame(&mut DeadlineIo::new(&stream, deadline))?
            .ok_or(TransportError::PeerClosed)?;
        let hello = AdapterHello::decode(&frame).map_err(TransportError::Protocol)?;
        let protocol = hello
            .negotiate(expected_package_id, supported_protocols)
            .map_err(TransportError::Protocol)?;
        if protocol != ADAPTER_PROTOCOL {
            return Err(TransportError::UnsupportedNegotiatedVersion(protocol));
        }
        Ok(Self {
            stream,
            timeout,
            package_id: expected_package_id.to_owned(),
            protocol,
            last_request_id: 0,
            failed: false,
        })
    }

    pub fn protocol_version(&self) -> u32 {
        self.protocol
    }

    /// Sends one SDK request and accepts only a reply envelope matching its
    /// package, protocol, and request ID. A reply is not proof of domain work.
    pub fn exchange(
        &mut self,
        request: &AdapterEnvelope,
    ) -> Result<AdapterReplyEnvelope, TransportError> {
        if self.failed {
            return Err(TransportError::SessionFailed);
        }
        if request.package_id != self.package_id {
            return Err(TransportError::PackageMismatch);
        }
        if request.protocol != self.protocol {
            return Err(TransportError::UnsupportedNegotiatedVersion(
                request.protocol,
            ));
        }
        if request.request_id <= self.last_request_id {
            return Err(TransportError::InvalidRequestId);
        }
        let frame = request.encode().map_err(TransportError::Protocol)?;
        self.last_request_id = request.request_id;
        let result = (|| {
            let deadline = deadline_after(self.timeout)?;
            let mut io = DeadlineIo::new(&self.stream, deadline);
            write_sdk_frame(&mut io, &frame)?;
            let frame = read_sdk_frame(&mut io)?.ok_or(TransportError::PeerClosed)?;
            let reply = AdapterReplyEnvelope::decode(&frame).map_err(TransportError::Protocol)?;
            if reply.package_id != self.package_id {
                return Err(TransportError::PackageMismatch);
            }
            if reply.protocol != self.protocol {
                return Err(TransportError::UnsupportedNegotiatedVersion(reply.protocol));
            }
            if reply.request_id != request.request_id {
                return Err(TransportError::Protocol(AdapterError::Invalid(
                    "adapter reply request ID mismatch",
                )));
            }
            Ok(reply)
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}

fn read_sdk_frame<R: Read>(reader: &mut R) -> Result<Option<Vec<u8>>, TransportError> {
    let mut header = [0_u8; 4];
    loop {
        match reader.read(&mut header[..1]) {
            Ok(0) => return Ok(None),
            Ok(1) => break,
            Ok(_) => unreachable!("single-byte read returned more than one byte"),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(TransportError::Frame(FrameError::Io(error))),
        }
    }
    reader.read_exact(&mut header[1..]).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            TransportError::Frame(FrameError::TruncatedHeader)
        } else {
            TransportError::Frame(FrameError::Io(error))
        }
    })?;
    let length = u32::from_be_bytes(header) as usize;
    if length == 0 {
        return Err(TransportError::Frame(FrameError::EmptyFrame));
    }
    if length > MAX_WIRE_BYTES {
        return Err(TransportError::Protocol(AdapterError::Invalid(
            "adapter frame exceeds SDK wire limit",
        )));
    }
    let mut frame = vec![0; length];
    reader.read_exact(&mut frame).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            TransportError::Frame(FrameError::TruncatedPayload)
        } else {
            TransportError::Frame(FrameError::Io(error))
        }
    })?;
    Ok(Some(frame))
}

fn write_sdk_frame<W: Write>(writer: &mut W, frame: &[u8]) -> Result<(), TransportError> {
    if frame.is_empty() {
        return Err(TransportError::Frame(FrameError::EmptyFrame));
    }
    if frame.len() > MAX_WIRE_BYTES {
        return Err(TransportError::Protocol(AdapterError::Invalid(
            "adapter frame exceeds SDK wire limit",
        )));
    }
    writer
        .write_all(&(frame.len() as u32).to_be_bytes())
        .and_then(|()| writer.write_all(frame))
        .map_err(|error| TransportError::Frame(FrameError::Io(error)))
}

/// Shares an absolute deadline across partial reads/writes so a peer cannot
/// extend the timeout by trickling bytes.
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
            .map(|remaining| remaining.max(Duration::from_millis(1)))
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "adapter deadline elapsed"))
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
            "adapter timeout is too large",
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package_listener::PrivateUnixListener;
    use rust_boy_adapter_sdk::{AdapterCommand, AdapterReply, Viewport};
    use std::os::unix::net::UnixStream;
    use std::thread;

    const PACKAGE: &str = "org.rustboy.radio";
    const TIMEOUT: Duration = Duration::from_millis(300);

    fn send_frame(stream: &mut UnixStream, frame: &[u8]) {
        write_sdk_frame(stream, frame).unwrap();
    }

    fn hello_frame(package_id: &str) -> Vec<u8> {
        AdapterHello {
            package_id: package_id.into(),
            supported_protocols: vec![1],
        }
        .encode()
        .unwrap()
    }

    fn request(id: u64) -> AdapterEnvelope {
        AdapterEnvelope {
            protocol: ADAPTER_PROTOCOL,
            package_id: PACKAGE.into(),
            request_id: id,
            body: AdapterCommand::Activate {
                viewport: Viewport::new(480, 320).unwrap(),
            },
        }
    }

    #[test]
    fn sdk_session_negotiates_and_correlates_exchange() {
        let (host, mut adapter) = UnixStream::pair().unwrap();
        let peer = thread::spawn(move || {
            send_frame(&mut adapter, &hello_frame(PACKAGE));
            let frame = read_sdk_frame(&mut adapter).unwrap().unwrap();
            let request = AdapterEnvelope::decode(&frame).unwrap();
            let reply = AdapterReplyEnvelope {
                protocol: 1,
                package_id: request.package_id,
                request_id: request.request_id,
                body: AdapterReply::Ack,
            }
            .encode()
            .unwrap();
            send_frame(&mut adapter, &reply);
        });
        let mut session = UnixAdapterSession::from_stream(host, PACKAGE, &[1], TIMEOUT).unwrap();
        assert_eq!(session.protocol_version(), 1);
        assert_eq!(
            session.exchange(&request(1)).unwrap().body,
            AdapterReply::Ack
        );
        peer.join().unwrap();
    }

    #[test]
    fn hello_identity_mismatch_fails_before_requests() {
        let (host, mut adapter) = UnixStream::pair().unwrap();
        let peer =
            thread::spawn(move || send_frame(&mut adapter, &hello_frame("org.other.module")));
        assert!(matches!(
            UnixAdapterSession::from_stream(host, PACKAGE, &[1], TIMEOUT),
            Err(TransportError::Protocol(AdapterError::Denied(
                "adapter package ID mismatch"
            )))
        ));
        peer.join().unwrap();
    }

    #[test]
    fn request_ids_must_increase_and_failed_exchange_poison_session() {
        let (host, mut adapter) = UnixStream::pair().unwrap();
        let peer = thread::spawn(move || {
            send_frame(&mut adapter, &hello_frame(PACKAGE));
            let frame = read_sdk_frame(&mut adapter).unwrap().unwrap();
            let request = AdapterEnvelope::decode(&frame).unwrap();
            let reply = AdapterReplyEnvelope {
                protocol: 1,
                package_id: request.package_id,
                request_id: request.request_id + 1,
                body: AdapterReply::Ack,
            }
            .encode()
            .unwrap();
            send_frame(&mut adapter, &reply);
        });
        let mut session = UnixAdapterSession::from_stream(host, PACKAGE, &[1], TIMEOUT).unwrap();
        assert!(matches!(
            session.exchange(&request(2)),
            Err(TransportError::Protocol(_))
        ));
        assert!(matches!(
            session.exchange(&request(3)),
            Err(TransportError::SessionFailed)
        ));
        peer.join().unwrap();
    }

    #[test]
    fn missing_hello_expires_at_deadline() {
        let (host, _peer) = UnixStream::pair().unwrap();
        let start = Instant::now();
        let error = UnixAdapterSession::from_stream(host, PACKAGE, &[1], Duration::from_millis(25))
            .err()
            .unwrap();
        assert!(
            matches!(error, TransportError::Frame(FrameError::Io(ref error))
            if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut))
        );
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn trickled_hello_cannot_extend_absolute_deadline() {
        let (host, mut adapter) = UnixStream::pair().unwrap();
        let peer = thread::spawn(move || {
            for byte in [0_u8, 0, 0, 10] {
                if adapter.write_all(&[byte]).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(40));
            }
        });
        let start = Instant::now();
        let result =
            UnixAdapterSession::from_stream(host, PACKAGE, &[1], Duration::from_millis(60));
        assert!(matches!(
            result,
            Err(TransportError::Frame(FrameError::Io(ref error)))
                if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut)
        ));
        assert!(start.elapsed() < Duration::from_millis(250));
        peer.join().unwrap();
    }

    #[test]
    fn duplicate_and_reversed_request_ids_never_reach_peer() {
        let (host, mut adapter) = UnixStream::pair().unwrap();
        let peer = thread::spawn(move || {
            send_frame(&mut adapter, &hello_frame(PACKAGE));
            let first = read_sdk_frame(&mut adapter).unwrap().unwrap();
            let request = AdapterEnvelope::decode(&first).unwrap();
            assert_eq!(request.request_id, 2);
            let reply = AdapterReplyEnvelope {
                protocol: 1,
                package_id: request.package_id,
                request_id: request.request_id,
                body: AdapterReply::Ack,
            }
            .encode()
            .unwrap();
            send_frame(&mut adapter, &reply);
            let mut extra = [0_u8; 1];
            assert_eq!(adapter.read(&mut extra).unwrap(), 0);
        });
        let mut session = UnixAdapterSession::from_stream(host, PACKAGE, &[1], TIMEOUT).unwrap();
        session.exchange(&request(2)).unwrap();
        assert!(matches!(
            session.exchange(&request(2)),
            Err(TransportError::InvalidRequestId)
        ));
        assert!(matches!(
            session.exchange(&request(1)),
            Err(TransportError::InvalidRequestId)
        ));
        drop(session);
        peer.join().unwrap();
    }

    #[test]
    fn oversized_sdk_frame_is_rejected_from_header_alone() {
        let header = ((MAX_WIRE_BYTES as u32) + 1).to_be_bytes();
        let error = read_sdk_frame(&mut &header[..]).err().unwrap();
        assert!(matches!(
            error,
            TransportError::Protocol(AdapterError::Invalid(
                "adapter frame exceeds SDK wire limit"
            ))
        ));
    }

    #[test]
    fn public_session_path_requires_listener_verified_peer_credentials() {
        let directory = tempfile::tempdir().unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let listener = PrivateUnixListener::bind(directory.path(), "adapter.sock").unwrap();
        let socket_path = listener.socket_path().to_owned();
        let peer = thread::spawn(move || {
            let mut adapter = UnixStream::connect(socket_path).unwrap();
            send_frame(&mut adapter, &hello_frame(PACKAGE));
            let frame = read_sdk_frame(&mut adapter).unwrap().unwrap();
            let request = AdapterEnvelope::decode(&frame).unwrap();
            let reply = AdapterReplyEnvelope {
                protocol: 1,
                package_id: request.package_id,
                request_id: request.request_id,
                body: AdapterReply::Ack,
            }
            .encode()
            .unwrap();
            send_frame(&mut adapter, &reply);
        });
        let verified = listener
            .accept_expected_process(std::process::id(), TIMEOUT)
            .unwrap();
        assert_eq!(verified.credentials().pid, std::process::id());
        let mut session =
            UnixAdapterSession::from_verified_peer(verified, PACKAGE, &[1], TIMEOUT).unwrap();
        assert_eq!(
            session.exchange(&request(1)).unwrap().body,
            AdapterReply::Ack
        );
        peer.join().unwrap();
    }
}

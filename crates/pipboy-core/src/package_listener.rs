//! Private Unix-domain listener for local adapter sessions.
//!
//! This establishes filesystem and OS-account provenance only. A same-user
//! peer is not proof of package identity, signature, authorization, or
//! isolation; callers must still verify the approved installation and policy
//! before creating an adapter session.

use std::fs;
use std::io;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdapterPeerCredentials {
    /// PID reported by the OS when the connection was established.
    pub pid: u32,
    pub uid: u32,
    pub gid: u32,
}

/// A Unix stream admitted by this listener after checking OS-reported peer
/// credentials against the host's expected process. Its fields are private so
/// callers cannot forge admission by constructing a token from a raw stream.
pub struct VerifiedUnixStream {
    stream: UnixStream,
    credentials: AdapterPeerCredentials,
}

impl VerifiedUnixStream {
    pub(crate) fn into_stream(self) -> UnixStream {
        self.stream
    }

    pub fn credentials(&self) -> AdapterPeerCredentials {
        self.credentials
    }
}

/// A socket bound inside an existing, current-user-only runtime directory.
/// Existing paths are never removed or replaced during bind.
pub struct PrivateUnixListener {
    listener: UnixListener,
    path: PathBuf,
    device: u64,
    inode: u64,
}

impl PrivateUnixListener {
    pub fn bind(directory: &Path, socket_name: &str) -> io::Result<Self> {
        if socket_name.is_empty()
            || socket_name.len() > 32
            || socket_name == "."
            || socket_name == ".."
            || !socket_name.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'.' | b'-' | b'_')
            })
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid adapter socket name",
            ));
        }

        let metadata = fs::symlink_metadata(directory)?;
        if !metadata.file_type().is_dir()
            || metadata.uid() != current_uid()
            || metadata.mode() & 0o077 != 0
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "adapter runtime directory must be a current-user-owned private directory",
            ));
        }

        let path = directory.join(socket_name);
        if path.as_os_str().as_encoded_bytes().len() > 100 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "adapter socket path is too long",
            ));
        }
        if fs::symlink_metadata(&path).is_ok() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "adapter socket path already exists",
            ));
        }

        let listener = UnixListener::bind(&path)?;
        let socket_metadata = fs::symlink_metadata(&path)?;
        let device = socket_metadata.dev();
        let inode = socket_metadata.ino();
        if let Err(error) = fs::set_permissions(&path, fs::Permissions::from_mode(0o600)) {
            remove_if_same_socket(&path, device, inode);
            return Err(error);
        }

        Ok(Self {
            listener,
            path,
            device,
            inode,
        })
    }

    pub fn socket_path(&self) -> &Path {
        &self.path
    }

    /// Accept the expected host-launched child within a bounded interval. An
    /// unrelated same-user connection is dropped and cannot preempt the
    /// expected peer. OS-reported PID and UID bind the stream to that process
    /// at connection time; callers must still verify its approved binary.
    pub fn accept_expected_process(
        &self,
        expected_pid: u32,
        timeout: Duration,
    ) -> io::Result<VerifiedUnixStream> {
        if expected_pid == 0 || timeout.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "expected adapter PID and timeout must be nonzero",
            ));
        }
        let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "adapter timeout is too large")
        })?;
        self.listener.set_nonblocking(true)?;
        loop {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    let credentials = peer_credentials(&stream)?;
                    if credentials.uid == current_uid() && credentials.pid == expected_pid {
                        return Ok(VerifiedUnixStream {
                            stream,
                            credentials,
                        });
                    }
                    // Drop unrelated peers and continue waiting within the
                    // same absolute deadline. Never trust their hello claim.
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "adapter peer did not connect before the deadline",
                ));
            };
            if remaining.is_zero() {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "adapter peer did not connect before the deadline",
                ));
            }
            std::thread::sleep(remaining.min(Duration::from_millis(1)));
        }
    }
}

impl Drop for PrivateUnixListener {
    fn drop(&mut self) {
        remove_if_same_socket(&self.path, self.device, self.inode);
    }
}

fn remove_if_same_socket(path: &Path, device: u64, inode: u64) {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_socket() && metadata.dev() == device && metadata.ino() == inode {
            let _ = fs::remove_file(path);
        }
    }
}

#[cfg(target_os = "linux")]
fn current_uid() -> u32 {
    unsafe { libc::geteuid() }
}

#[cfg(target_os = "macos")]
fn current_uid() -> u32 {
    unsafe { libc::geteuid() }
}

#[cfg(target_os = "linux")]
fn peer_credentials(stream: &UnixStream) -> io::Result<AdapterPeerCredentials> {
    let peer = rustix::net::sockopt::socket_peercred(stream)?;
    Ok(AdapterPeerCredentials {
        pid: peer.pid.as_raw_pid() as u32,
        uid: peer.uid.as_raw(),
        gid: peer.gid.as_raw(),
    })
}

#[cfg(target_os = "macos")]
fn peer_credentials(stream: &UnixStream) -> io::Result<AdapterPeerCredentials> {
    use std::os::fd::AsRawFd;

    let mut uid = 0;
    let mut gid = 0;
    // SAFETY: getpeereid writes one uid_t and gid_t to valid stack pointers.
    let result = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    let mut pid: libc::pid_t = 0;
    let mut pid_len = std::mem::size_of_val(&pid) as libc::socklen_t;
    // SAFETY: getsockopt writes one pid_t to a valid stack pointer and length.
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_LOCAL,
            libc::LOCAL_PEERPID,
            (&mut pid as *mut libc::pid_t).cast(),
            &mut pid_len,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    if pid <= 0 || pid_len as usize != std::mem::size_of_val(&pid) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "OS returned invalid adapter peer PID",
        ));
    }
    Ok(AdapterPeerCredentials {
        pid: pid as u32,
        uid,
        gid,
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn current_uid() -> u32 {
    0
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn peer_credentials(_stream: &UnixStream) -> io::Result<AdapterPeerCredentials> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "adapter peer credentials are unsupported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::thread;
    use tempfile::tempdir;

    fn private_dir() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
        dir
    }

    #[test]
    fn binds_private_socket_accepts_same_user_peer_and_removes_only_its_socket() {
        let dir = private_dir();
        let listener = PrivateUnixListener::bind(dir.path(), "radio.sock").unwrap();
        let metadata = fs::symlink_metadata(listener.socket_path()).unwrap();
        assert!(metadata.file_type().is_socket());
        assert_eq!(metadata.mode() & 0o777, 0o600);

        let path = listener.socket_path().to_owned();
        let peer = thread::spawn(move || UnixStream::connect(path).unwrap());
        let verified = listener
            .accept_expected_process(std::process::id(), Duration::from_secs(1))
            .unwrap();
        let credentials = verified.credentials();
        assert_eq!(credentials.uid, current_uid());
        assert_eq!(credentials.pid, std::process::id());
        drop(peer.join().unwrap());

        let socket_path = listener.socket_path().to_owned();
        drop(listener);
        assert!(!socket_path.exists());
    }

    #[test]
    fn rejects_a_peer_pid_that_does_not_match_the_launched_process() {
        let dir = private_dir();
        let listener = PrivateUnixListener::bind(dir.path(), "radio.sock").unwrap();
        let path = listener.socket_path().to_owned();
        let peer = thread::spawn(move || UnixStream::connect(path).unwrap());
        let expected_pid = std::process::id().wrapping_add(1);
        assert_eq!(
            listener
                .accept_expected_process(expected_pid, Duration::from_secs(1))
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::TimedOut
        );
        drop(peer.join().unwrap());
    }

    #[test]
    fn expected_peer_timeout_is_bounded_and_invalid_timeout_is_rejected() {
        let dir = private_dir();
        let listener = PrivateUnixListener::bind(dir.path(), "radio.sock").unwrap();
        assert_eq!(
            listener
                .accept_expected_process(std::process::id(), Duration::ZERO)
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        let start = Instant::now();
        assert_eq!(
            listener
                .accept_expected_process(std::process::id(), Duration::from_millis(20))
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::TimedOut
        );
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn refuses_non_private_directories_existing_paths_and_invalid_names() {
        let dir = private_dir();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            PrivateUnixListener::bind(dir.path(), "radio.sock")
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();

        let occupied = dir.path().join("occupied.sock");
        fs::write(&occupied, b"preserve").unwrap();
        assert_eq!(
            PrivateUnixListener::bind(dir.path(), "occupied.sock")
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(occupied).unwrap(), b"preserve");

        for name in ["", ".", "..", "../escape", "Upper.sock", "a/b"] {
            assert_eq!(
                PrivateUnixListener::bind(dir.path(), name)
                    .err()
                    .unwrap()
                    .kind(),
                io::ErrorKind::InvalidInput,
                "{name:?}"
            );
        }
    }

    #[test]
    fn listener_drop_does_not_remove_a_replacement_path() {
        let dir = private_dir();
        let listener = PrivateUnixListener::bind(dir.path(), "radio.sock").unwrap();
        let path = listener.socket_path().to_owned();
        fs::remove_file(&path).unwrap();
        fs::write(&path, b"replacement").unwrap();
        drop(listener);
        assert_eq!(fs::read(&path).unwrap(), b"replacement");
        fs::remove_file(path).unwrap();
    }
}

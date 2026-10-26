//! age X25519 queue identities held by OS credential storage, never the journal.
//! This is storage encryption, not an E2EE protocol or independent security audit.
use age::secrecy::ExposeSecret;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use zeroize::Zeroizing;

const SERVICE: &str = "rust-boy.message-queue.v1";
const MARKER: &str = "messages.recipient";
fn locked() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Messenger key unavailable; encrypted queue remains locked",
    )
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
fn entry(recipient: &str) -> io::Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, recipient).map_err(|_| locked())
}

/// Enrollment is intentionally refused for existing journals. Migration needs a
/// separately reviewed export/backup policy; deleting a file cannot erase backups.
pub fn enroll(root: &Path) -> io::Result<String> {
    fs::create_dir_all(root)?;
    let mut options = fs::OpenOptions::new();
    options.create(true).truncate(false).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
        options.mode(0o600);
    }
    let guard = options.open(root.join("messages.lock"))?;
    fs2::FileExt::try_lock_exclusive(&guard)?;
    if root.join("messages.json").exists() || root.join(MARKER).exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Enrollment requires a fresh Messenger journal; existing data preserved",
        ));
    }
    let identity = age::x25519::Identity::generate();
    let recipient = identity.to_public().to_string();
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    {
        let credential = entry(&recipient)?;
        // Never replace an existing credential silently.
        match credential.get_password() {
            Err(keyring::Error::NoEntry) => (),
            _ => return Err(locked()),
        }
        credential
            .set_password(identity.to_string().expose_secret())
            .map_err(|_| locked())?;
        // Verify round-trip before committing the public enrollment marker.
        let stored = Zeroizing::new(credential.get_password().map_err(|_| locked())?);
        let check: age::x25519::Identity = stored.parse().map_err(|_| locked())?;
        if check.to_public().to_string() != recipient {
            return Err(locked());
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    return Err(locked());
    let mut marker = tempfile::NamedTempFile::new_in(root)?;
    marker.write_all(recipient.as_bytes())?;
    marker.as_file().sync_all()?;
    marker
        .persist_noclobber(root.join(MARKER))
        .map_err(|e| e.error)?;
    #[cfg(unix)]
    File::open(root)?.sync_all()?;
    Ok(recipient)
}

pub fn unlock(root: &Path) -> io::Result<Option<age::x25519::Identity>> {
    let file = match File::open(root.join(MARKER)) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let mut recipient = String::new();
    file.take(129).read_to_string(&mut recipient)?;
    if recipient.len() > 128 || recipient.parse::<age::x25519::Recipient>().is_err() {
        return Err(locked());
    }
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    {
        let secret = Zeroizing::new(entry(&recipient)?.get_password().map_err(|_| locked())?);
        let identity: age::x25519::Identity = secret.parse().map_err(|_| locked())?;
        if identity.to_public().to_string() != recipient {
            return Err(locked());
        }
        Ok(Some(identity))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    Err(locked())
}

pub fn encrypt(identity: &age::x25519::Identity, plaintext: &[u8]) -> io::Result<Vec<u8>> {
    let recipient = identity.to_public();
    let encryptor =
        age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
            .map_err(|_| locked())?;
    let mut bytes = Vec::new();
    let mut writer = encryptor.wrap_output(&mut bytes)?;
    writer.write_all(plaintext)?;
    writer.finish()?;
    Ok(bytes)
}

pub fn decrypt(
    identity: &age::x25519::Identity,
    bytes: &[u8],
    limit: u64,
) -> io::Result<Zeroizing<Vec<u8>>> {
    let decryptor = age::Decryptor::new(bytes).map_err(|_| locked())?;
    let reader = decryptor
        .decrypt(std::iter::once(identity as &dyn age::Identity))
        .map_err(|_| locked())?;
    let mut plaintext = Zeroizing::new(Vec::new());
    reader
        .take(limit + 1)
        .read_to_end(&mut plaintext)
        .map_err(|_| locked())?;
    if plaintext.len() as u64 > limit {
        return Err(locked());
    }
    Ok(plaintext)
}

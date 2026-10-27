//! ORG-3.2: operating-system credential custody for organizational identity keys.
//!
//! The protected store is a required dependency. There is NO plaintext-file,
//! environment-variable, or automatic password fallback. Keyring-backed keys
//! remain accessible to the authorized OS account (not hardware attested).
//! This module produces signing evidence only, never API authorization.
use crate::organization_identity::{proof_transcript, SessionProof, SignedCredential};
use crate::organization_policy::ProposedExchange;
use ring::digest::{digest, SHA256};
use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroizing;

const SERVICE: &str = "rust-boy.org-custody.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CustodyError {
    Unavailable,
    AlreadyPresent,
    Missing,
    InvalidInput,
    Corrupt,
    NotApproved,
}

impl fmt::Display for CustodyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "organizational credential custody unavailable or denied: {self:?}"
        )
    }
}

impl std::error::Error for CustodyError {}

/// Implementations must protect confidentiality, integrity, and persistence.
/// Production uses the platform keychain; tests use an isolated memory fake.
/// No caller may substitute a plaintext production implementation.
// Public implementations are deliberately sealed: arbitrary downstream code
// cannot supply a plaintext replacement for production trust-root storage.
mod sealed {
    pub trait Sealed {}
}

pub trait ProtectedSecrets: sealed::Sealed {
    fn read(&self, name: &str) -> Result<Option<String>, CustodyError>;
    fn write(&self, name: &str, value: &str) -> Result<(), CustodyError>;
}

pub struct OsKeyring;

impl sealed::Sealed for OsKeyring {}

impl ProtectedSecrets for OsKeyring {
    fn read(&self, name: &str) -> Result<Option<String>, CustodyError> {
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            let entry =
                keyring::Entry::new(SERVICE, name).map_err(|_| CustodyError::Unavailable)?;
            match entry.get_password() {
                Ok(value) => Ok(Some(value)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(_) => Err(CustodyError::Unavailable),
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            let _ = name;
            Err(CustodyError::Unavailable)
        }
    }

    fn write(&self, name: &str, value: &str) -> Result<(), CustodyError> {
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            keyring::Entry::new(SERVICE, name)
                .map_err(|_| CustodyError::Unavailable)?
                .set_password(value)
                .map_err(|_| CustodyError::Unavailable)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            let _ = (name, value);
            Err(CustodyError::Unavailable)
        }
    }
}

/// A separate, authenticated administrator control plane must implement this
/// gate and compare the SHA-256 fingerprint on an independent channel.
/// Passing an ordinary UI role flag or peer-provided claim is NOT sufficient.
pub trait AdministratorApproval {
    fn approve_root_pin(
        &self,
        organization_id: &str,
        displayed_sha256: &str,
    ) -> Result<(), CustodyError>;
}

pub fn valid_org_id(id: &str) -> bool {
    (2..=64).contains(&id.len())
        && id.bytes().next().is_some_and(|c| c.is_ascii_lowercase())
        && !id.ends_with('-')
        && !id.contains("--")
        && id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 15) as usize] as char);
    }
    output
}

fn unhex(input: &str) -> Result<Zeroizing<Vec<u8>>, CustodyError> {
    if !input.len().is_multiple_of(2) || input.len() > 8192 {
        return Err(CustodyError::Corrupt);
    }
    let mut output = Zeroizing::new(Vec::with_capacity(input.len() / 2));
    for pair in input.as_bytes().as_chunks::<2>().0 {
        let from = |b: u8| match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            _ => None,
        };
        let high = from(pair[0]).ok_or(CustodyError::Corrupt)?;
        let low = from(pair[1]).ok_or(CustodyError::Corrupt)?;
        output.push((high << 4) | low);
    }
    Ok(output)
}

pub fn fingerprint(root_public_key: &[u8]) -> Result<String, CustodyError> {
    if root_public_key.len() != 32 || root_public_key.iter().all(|b| *b == 0) {
        return Err(CustodyError::InvalidInput);
    }
    Ok(hex(digest(&SHA256, root_public_key).as_ref()))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredKey {
    schema_version: u32,
    public_hex: String,
    pkcs8_hex: String,
}

/// Create one independent device/operator Ed25519 identity in OS custody.
/// The private PKCS#8 bytes are never written to the org ledger or returned.
/// Enrollment is deliberately not exposed on a network/HTTP route.
pub fn enroll_identity(
    store: &impl ProtectedSecrets,
    organization_id: &str,
    principal_kind: &str,
    principal_id: &str,
) -> Result<Vec<u8>, CustodyError> {
    let name = identity_name(organization_id, principal_kind, principal_id)?;
    if store.read(&name)?.is_some() {
        return Err(CustodyError::AlreadyPresent);
    }
    let rng = SystemRandom::new();
    let bytes = Ed25519KeyPair::generate_pkcs8(&rng).map_err(|_| CustodyError::Unavailable)?;
    let protected = Zeroizing::new(bytes.as_ref().to_vec());
    let pair = Ed25519KeyPair::from_pkcs8(&protected).map_err(|_| CustodyError::Unavailable)?;
    let public = pair.public_key().as_ref().to_vec();
    let value = serde_json::to_string(&StoredKey {
        schema_version: 1,
        public_hex: hex(&public),
        pkcs8_hex: hex(&protected),
    })
    .map_err(|_| CustodyError::Unavailable)?;
    let value = Zeroizing::new(value);
    store.write(&name, &value)?;
    // Verify that the credential store actually retained the key.
    let check = load_key(store, &name)?;
    if check.public_key().as_ref() != public {
        return Err(CustodyError::Corrupt);
    }
    Ok(public)
}

fn identity_name(org: &str, kind: &str, principal: &str) -> Result<String, CustodyError> {
    if !valid_org_id(org) || !valid_org_id(principal) || !matches!(kind, "operator" | "device") {
        return Err(CustodyError::InvalidInput);
    }
    Ok(format!("identity:{org}:{kind}:{principal}"))
}

fn load_key(store: &impl ProtectedSecrets, name: &str) -> Result<Ed25519KeyPair, CustodyError> {
    let value = Zeroizing::new(store.read(name)?.ok_or(CustodyError::Missing)?);
    let parsed: StoredKey = serde_json::from_str(&value).map_err(|_| CustodyError::Corrupt)?;
    if parsed.schema_version != 1 {
        return Err(CustodyError::Corrupt);
    }
    let protected = unhex(&parsed.pkcs8_hex)?;
    let pair = Ed25519KeyPair::from_pkcs8(&protected).map_err(|_| CustodyError::Corrupt)?;
    if hex(pair.public_key().as_ref()) != parsed.public_hex {
        return Err(CustodyError::Corrupt);
    }
    Ok(pair)
}

/// Sign only the existing domain-separated, scoped, channel-bound proof.
/// This is possession evidence, not permission to use the named action.
pub fn sign_proof(
    store: &impl ProtectedSecrets,
    query: &ProposedExchange<'_>,
    credential: &SignedCredential,
    nonce: &[u8; 32],
    authenticated_channel_binding: &[u8; 32],
) -> Result<SessionProof, CustodyError> {
    let kind = match credential.principal_kind {
        crate::organization_identity::PrincipalKind::Operator => "operator",
        crate::organization_identity::PrincipalKind::Device => "device",
    };
    if credential.organization_id != query.organization_id
        || credential.vault_id != query.vault_id
        || credential.principal_id
            != if kind == "operator" {
                query.operator_id
            } else {
                query.device_id
            }
        || nonce.iter().all(|b| *b == 0)
        || authenticated_channel_binding.iter().all(|b| *b == 0)
    {
        return Err(CustodyError::InvalidInput);
    }
    let name = identity_name(&credential.organization_id, kind, &credential.principal_id)?;
    let pair = load_key(store, &name)?;
    if pair.public_key().as_ref() != credential.public_key {
        return Err(CustodyError::Corrupt);
    }
    Ok(SessionProof {
        signature: pair
            .sign(&proof_transcript(
                query,
                credential,
                nonce,
                authenticated_channel_binding,
            ))
            .as_ref()
            .to_vec(),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct MemorySecrets(pub Mutex<HashMap<String, String>>);

    impl super::sealed::Sealed for MemorySecrets {}

    impl ProtectedSecrets for MemorySecrets {
        fn read(&self, name: &str) -> Result<Option<String>, CustodyError> {
            Ok(self.0.lock().unwrap().get(name).cloned())
        }

        fn write(&self, name: &str, value: &str) -> Result<(), CustodyError> {
            self.0
                .lock()
                .unwrap()
                .insert(name.to_string(), value.to_string());
            Ok(())
        }
    }

    /// Simulates the OS credential write completing but reporting an error,
    /// as can happen when a process crashes or a backend confirmation is lost.
    /// Synthetic only; production store implementations remain sealed.
    #[derive(Default)]
    pub struct UncertainWriteSecrets {
        pub inner: MemorySecrets,
        pub fail_after_write: std::sync::atomic::AtomicBool,
    }

    impl super::sealed::Sealed for UncertainWriteSecrets {}

    impl ProtectedSecrets for UncertainWriteSecrets {
        fn read(&self, name: &str) -> Result<Option<String>, CustodyError> {
            self.inner.read(name)
        }

        fn write(&self, name: &str, value: &str) -> Result<(), CustodyError> {
            self.inner.write(name, value)?;
            if self
                .fail_after_write
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                Err(CustodyError::Unavailable)
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn unique_keys_are_protected_and_not_reenrolled() {
        let store = MemorySecrets::default();
        let a = enroll_identity(&store, "sample-org", "operator", "person-a").unwrap();
        let b = enroll_identity(&store, "sample-org", "device", "device-b").unwrap();
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
        assert_eq!(
            enroll_identity(&store, "sample-org", "operator", "person-a"),
            Err(CustodyError::AlreadyPresent)
        );
        let values = store.0.lock().unwrap();
        assert!(!values.values().any(|s| s.contains("BEGIN PRIVATE KEY")));
    }

    #[test]
    fn bad_principal_and_missing_os_secret_fail_closed() {
        let store = MemorySecrets::default();
        assert_eq!(
            enroll_identity(&store, "sample-org", "device", "../device"),
            Err(CustodyError::InvalidInput)
        );
        assert_eq!(
            load_key(&store, "identity:sample-org:device:missing").err(),
            Some(CustodyError::Missing)
        );
    }
}

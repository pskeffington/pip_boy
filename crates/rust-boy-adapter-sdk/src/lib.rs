//! Minimal, portable protocol SDK for Rust-Boy host packages.
//!
//! Consumers implement UniversalPackageAdapter for trusted in-process tests or
//! serialize AdapterEnvelope/AdapterReply across a separately reviewed IPC
//! boundary. No network, device, filesystem, shell or UI renderer is provided.
//! This crate does not confer installation trust or grant privileges.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt;

pub const ADAPTER_PROTOCOL: u32 = 1;
pub const MAX_WIRE_BYTES: usize = 16 * 1024;
pub const MAX_INTENT_BYTES: usize = 4 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Viewport {
    pub width: u16,
    pub height: u16,
}

impl Viewport {
    pub fn new(width: u16, height: u16) -> Result<Self, AdapterError> {
        if !(320..=4096).contains(&width) || !(240..=4096).contains(&height) {
            return Err(AdapterError::Invalid("unsupported viewport dimensions"));
        }
        Ok(Self { width, height })
    }
}

/// Serializable command vocabulary; no native handles or browser code cross
/// the interface. Intents are data, not automatically authorized operations.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum AdapterCommand {
    Activate { viewport: Viewport },
    Suspend,
    Resume { viewport: Viewport },
    Resize { viewport: Viewport },
    Intent {
        action: String,
        required_scope: Option<String>,
        payload: Value,
    },
    Shutdown,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AdapterEnvelope {
    pub protocol: u32,
    pub package_id: String,
    pub request_id: u64,
    pub body: AdapterCommand,
}

impl AdapterEnvelope {
    /// The encoder can later be used by a bounded IPC transport. Encoding
    /// never executes a command or grants the requested scope.
    pub fn encode(&self) -> Result<Vec<u8>, AdapterError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|_| AdapterError::Invalid("cannot encode adapter request"))?;
        if bytes.len() > MAX_WIRE_BYTES {
            return Err(AdapterError::Invalid("adapter request exceeds wire limit"));
        }
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, AdapterError> {
        if bytes.len() > MAX_WIRE_BYTES {
            return Err(AdapterError::Invalid("adapter request exceeds wire limit"));
        }
        let decoded: Self = serde_json::from_slice(bytes)
            .map_err(|_| AdapterError::Invalid("invalid adapter request"))?;
        decoded.validate()?;
        Ok(decoded)
    }

    fn validate(&self) -> Result<(), AdapterError> {
        if self.protocol != ADAPTER_PROTOCOL || self.request_id == 0 {
            return Err(AdapterError::Invalid("incompatible adapter envelope"));
        }
        // The host's package IDs are already validated by PackageManifest.
        if self.package_id.is_empty() || self.package_id.len() > 96 {
            return Err(AdapterError::Invalid("invalid envelope package ID"));
        }
        match &self.body {
            AdapterCommand::Activate { viewport }
            | AdapterCommand::Resume { viewport }
            | AdapterCommand::Resize { viewport } => {
                Viewport::new(viewport.width, viewport.height)?;
            }
            AdapterCommand::Intent {
                action,
                required_scope,
                payload,
            } => {
                if !valid_action(action) {
                    return Err(AdapterError::Invalid("invalid package intent action"));
                }
                if required_scope
                    .as_deref()
                    .is_some_and(|scope| scope.is_empty() || scope.len() > 80)
                {
                    return Err(AdapterError::Invalid("invalid requested scope"));
                }
                if serde_json::to_vec(payload)
                    .map_err(|_| AdapterError::Invalid("cannot encode package intent"))?
                    .len()
                    > MAX_INTENT_BYTES
                {
                    return Err(AdapterError::Invalid("package intent exceeds 4 KiB"));
                }
            }
            AdapterCommand::Suspend | AdapterCommand::Shutdown => {}
        }
        Ok(())
    }
}

pub fn valid_action(action: &str) -> bool {
    !action.is_empty()
        && action.len() <= 64
        && action
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_lowercase)
        && action.bytes().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'.' | b'-' | b'_')
        })
}

/// Replies contain inert JSON state. The host renderer must never interpret
/// view data as executable script, raw HTML, a Host Settings panel, or a URL.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum AdapterReply {
    Ack,
    View { state: Value },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdapterError {
    Invalid(&'static str),
    Denied(&'static str),
    Unavailable(String),
    Fault(String),
    Registry(String),
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) | Self::Denied(message) => write!(f, "{message}"),
            Self::Unavailable(message) | Self::Fault(message) | Self::Registry(message) => {
                write!(f, "{message}")
            }
        }
    }
}
impl Error for AdapterError {}

/// For trusted *built-in* Rust adapters only. This is not a stable Rust ABI,
/// plugin loader, sandbox, or permission boundary. External modules will use
/// a separately reviewed IPC runner for the same serialized wire vocabulary.
pub trait UniversalPackageAdapter: Send {
    fn package_id(&self) -> &str;
    fn handle(&mut self, request: AdapterEnvelope) -> Result<AdapterReply, AdapterError>;
}

/// Only the host's already-authorized policy authority may approve a scope.
/// The default policy is deny-all. A manifest declaration is never a grant.
pub trait ScopeAuthority {
    fn is_granted(&self, package_id: &str, scope: &str) -> bool;
}

pub struct DenyAllScopes;
impl ScopeAuthority for DenyAllScopes {
    fn is_granted(&self, _package_id: &str, _scope: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stable_versioned_wire_roundtrip() {
        let frame = AdapterEnvelope {
            protocol: ADAPTER_PROTOCOL,
            package_id: "org.rustboy.radio".into(),
            request_id: 11,
            body: AdapterCommand::Activate {
                viewport: Viewport::new(480, 320).unwrap(),
            },
        };
        let bytes = frame.encode().unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["protocol"], 1);
        assert_eq!(json["body"]["command"], "activate");
        assert_eq!(json["body"]["viewport"]["width"], 480);
        assert_eq!(AdapterEnvelope::decode(&bytes).unwrap(), frame);
    }

    #[test]
    fn fail_closed_protocol_validation() {
        let request = AdapterEnvelope {
            protocol: 2,
            package_id: "org.rustboy.gis".into(),
            request_id: 1,
            body: AdapterCommand::Suspend,
        };
        assert!(request.encode().is_err());
        assert!(AdapterEnvelope::decode(b"{}").is_err());
        assert!(AdapterEnvelope::decode(&vec![b'x'; MAX_WIRE_BYTES + 1]).is_err());
        assert!(Viewport::new(1, 240).is_err());
        assert!(!valid_action("../escape"));
        let frame = AdapterEnvelope {
            protocol: ADAPTER_PROTOCOL,
            package_id: "org.rustboy.gis".into(),
            request_id: 2,
            body: AdapterCommand::Intent {
                action: "ui.search".into(),
                required_scope: None,
                payload: json!({"search": "x".repeat(MAX_INTENT_BYTES)}),
            },
        };
        assert!(frame.encode().is_err());
    }
}

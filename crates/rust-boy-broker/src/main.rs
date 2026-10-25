//! One-shot, deny-all physical-device broker protocol precursor.
//!
//! This process is launched ONLY when explicitly invoked with --stdio-once.
//! It opens no device, port, network socket, location service or privileged
//! handle, and is NOT yet an OS-enforced device authorization broker.
//! Only its own inherited stdin/stdout/stderr are used.

use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::io::{self, BufRead, Read, Write};

const SCHEMA_VERSION: u32 = 1;
const MAX_FRAME_BYTES: usize = 1024;
const MAX_DURATION_MS: u64 = 30_000;
const MAX_PURPOSE_BYTES: usize = 120;

/// Closed protocol vocabulary for *unimplemented* external adapter surfaces.
/// The catalog validator checks these keys against the core capability roster.
const CAPABILITY_OPERATIONS: &[(&str, &str)] = &[
    ("device.enumerate", "enumerate"),
    ("gpio.read", "read"),
    ("display.write", "write"),
    ("serial.read", "read"),
    ("gnss.read", "read"),
    ("sdr.receive", "read"),
    ("ble.discover", "discover"),
    ("mesh.transmit", "transmit"),
    ("sensor.read", "read"),
    ("audio.play", "play"),
    ("network.fetch", "fetch"),
];

/// Untrusted caller intent is NOT an OS-backed authorization or grant.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BrokerRequest {
    schema_version: u32,
    request_id: String,
    capability: String,
    device_id: String,
    operation: String,
    duration_ms: u64,
    purpose: String,
    operator_intent: bool,
}

impl BrokerRequest {
    fn valid(&self) -> bool {
        let Some(device_suffix) = self.device_id.strip_prefix("device:") else {
            return false;
        };
        self.schema_version == SCHEMA_VERSION
            && self.request_id.len() == 16
            && self.request_id.bytes().all(|byte| byte.is_ascii_hexdigit())
            && !device_suffix.is_empty()
            && device_suffix.len() <= 48
            && device_suffix
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
            && (1..=MAX_DURATION_MS).contains(&self.duration_ms)
            && !self.purpose.trim().is_empty()
            && self.purpose.len() <= MAX_PURPOSE_BYTES
            && self.purpose.bytes().all(|byte| (32..=126).contains(&byte))
            && CAPABILITY_OPERATIONS
                .iter()
                .any(|&(capability, operation)| {
                    self.capability == capability && self.operation == operation
                })
    }
}

/// There is deliberately no successful authorization variant in schema v1.
#[derive(Debug, Serialize, Eq, PartialEq)]
struct BrokerDecision {
    schema_version: u32,
    allowed: bool,
    status: &'static str,
    reason: &'static str,
    source: &'static str,
}

fn deny_frame(frame: &[u8]) -> BrokerDecision {
    let reason = if frame.is_empty() || frame.len() > MAX_FRAME_BYTES {
        "invalid_request"
    } else {
        match serde_json::from_slice::<BrokerRequest>(frame) {
            Ok(request) if request.valid() && !request.operator_intent => {
                "operator_intent_required"
            }
            Ok(request) if request.valid() => "physical_device_backend_disabled",
            _ => "invalid_request",
        }
    };
    BrokerDecision {
        schema_version: SCHEMA_VERSION,
        allowed: false,
        status: "denied",
        reason,
        source: "rust-boy/deny-all-physical-broker",
    }
}

fn run() -> io::Result<()> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(OsStr::new("--stdio-once")) || args.next().is_some() {
        eprintln!("usage: rust-boy-broker --stdio-once (single bounded JSON request on stdin)");
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "explicit --stdio-once flag required",
        ));
    }

    // Take bounds allocation even for a hostile stream without a newline.
    // The process handles only one request and never starts a long-lived
    // service or opens any OS device handles.
    let stdin = io::stdin();
    let mut reader = stdin.lock().take((MAX_FRAME_BYTES + 1) as u64);
    let mut frame = Vec::new();
    let decision = match reader.read_until(b'\n', &mut frame) {
        Ok(_) => deny_frame(&frame),
        Err(_) => deny_frame(&[]),
    };
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, &decision).map_err(io::Error::other)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

fn main() {
    if run().is_err() {
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn request() -> Value {
        json!({
            "schema_version": 1,
            "request_id": "0123456789abcdef",
            "capability": "serial.read",
            "device_id": "device:demo-serial",
            "operation": "read",
            "duration_ms": 1000,
            "purpose": "test only",
            "operator_intent": true
        })
    }

    fn reason(request: Value) -> &'static str {
        deny_frame(&serde_json::to_vec(&request).unwrap()).reason
    }

    #[test]
    fn explicit_operator_intent_still_never_grants_physical_access() {
        let response = deny_frame(&serde_json::to_vec(&request()).unwrap());
        assert_eq!(response.reason, "physical_device_backend_disabled");
        assert!(!response.allowed);
        assert_eq!(response.status, "denied");
        assert_eq!(response.schema_version, 1);
        assert_eq!(response.source, "rust-boy/deny-all-physical-broker");
    }

    #[test]
    fn missing_operator_intent_has_a_distinct_denial() {
        let mut req = request();
        req["operator_intent"] = json!(false);
        assert_eq!(reason(req), "operator_intent_required");
    }

    #[test]
    fn future_version_and_unknown_keys_are_rejected() {
        let mut req = request();
        req["schema_version"] = json!(2);
        assert_eq!(reason(req), "invalid_request");
        let mut req = request();
        req["unreviewed_permission"] = json!(true);
        assert_eq!(reason(req), "invalid_request");
    }

    #[test]
    fn unknown_and_local_capabilities_cannot_be_requested() {
        for capability in ["host.read", "files.write", "os.root", "serial.write"] {
            let mut req = request();
            req["capability"] = json!(capability);
            assert_eq!(reason(req), "invalid_request");
        }
    }

    #[test]
    fn operations_are_capability_scoped_even_without_grants() {
        let mut req = request();
        req["operation"] = json!("transmit");
        assert_eq!(reason(req), "invalid_request");
        let mut req = request();
        req["capability"] = json!("mesh.transmit");
        req["operation"] = json!("transmit");
        assert_eq!(reason(req), "physical_device_backend_disabled");
    }

    #[test]
    fn reject_device_paths_traversal_and_unknown_identity() {
        for device in [
            "",
            "sim:serial",
            "device:",
            "device:../../COM1",
            "device:/dev/tty0",
            "device:COM1\\evil",
            "device:serial port",
            "device:á",
        ] {
            let mut req = request();
            req["device_id"] = json!(device);
            assert_eq!(reason(req), "invalid_request", "{device}");
        }
    }

    #[test]
    fn request_id_is_opaque_and_never_grants_or_is_echoed() {
        let long_id = "f".repeat(32);
        for id in ["", "0", long_id.as_str(), "not-hex-identifier"] {
            let mut req = request();
            req["request_id"] = json!(id);
            assert_eq!(reason(req), "invalid_request");
        }
        let encoded =
            serde_json::to_string(&deny_frame(&serde_json::to_vec(&request()).unwrap())).unwrap();
        assert!(!encoded.contains("0123456789abcdef"));
        assert!(!encoded.contains("device:demo-serial"));
    }

    #[test]
    fn duration_is_bounded_and_cannot_be_permanent() {
        for duration in [0, 30_001, u64::MAX] {
            let mut req = request();
            req["duration_ms"] = json!(duration);
            assert_eq!(reason(req), "invalid_request");
        }
        let mut req = request();
        req["duration_ms"] = json!(30_000);
        assert_eq!(reason(req), "physical_device_backend_disabled");
    }

    #[test]
    fn purpose_is_bounded_printable_and_not_logged_back() {
        let long_purpose = "x".repeat(121);
        for purpose in ["", " ", "line\nbreak", "á", long_purpose.as_str()] {
            let mut req = request();
            req["purpose"] = json!(purpose);
            assert_eq!(reason(req), "invalid_request");
        }
        let encoded =
            serde_json::to_string(&deny_frame(&serde_json::to_vec(&request()).unwrap())).unwrap();
        assert!(!encoded.contains("test only"));
    }

    #[test]
    fn oversized_empty_and_malformed_frames_deny_without_allocation_spikes() {
        for bytes in [&b""[..], &b"null"[..], &b"{}"[..], &b"{invalid"[..]] {
            assert_eq!(deny_frame(bytes).reason, "invalid_request");
        }
        let large = vec![b'A'; MAX_FRAME_BYTES + 1];
        assert_eq!(deny_frame(&large).reason, "invalid_request");
    }

    #[test]
    fn always_denies_all_11_future_capability_surfaces() {
        for (capability, operation) in CAPABILITY_OPERATIONS {
            let mut req = request();
            req["capability"] = json!(capability);
            req["operation"] = json!(operation);
            assert_eq!(reason(req), "physical_device_backend_disabled");
        }
        assert_eq!(CAPABILITY_OPERATIONS.len(), 11);
    }

    #[test]
    fn response_has_no_session_handle_or_credential_fields() {
        let response =
            serde_json::to_value(deny_frame(&serde_json::to_vec(&request()).unwrap())).unwrap();
        let keys = response.as_object().unwrap();
        assert_eq!(keys.len(), 5);
        for forbidden in ["token", "session", "handle", "path", "request_id"] {
            assert!(!keys.contains_key(forbidden));
        }
        assert_eq!(response["allowed"], false);
    }
}

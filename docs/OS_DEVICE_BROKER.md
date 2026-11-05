# Rust-Boy | OS device authorization boundary

**Status:** an isolated, **deny-all** one-shot stdio process with a bounded protocol is [cross-platform CI-verified](https://github.com/pskeffington/pip_boy/actions/runs/40164717116) (see [BROKER_PROTOCOL.md](BROKER_PROTOCOL.md)); there is still **no authenticated OS-enforced broker**, live device driver, trusted consent service, privileged helper or hardware probe. The in-process simulator and separate deny-only process do not supply real authorization.

## Principle: saved preference is never an active grant

The existing `permissions.json` can contain `requested` or `denied` intent.
Neither `requested` nor the `pipboy simulate --confirm` switch is a grant for
real serial ports, GPIO, display, GNSS, SDR, mesh, BLE or network interfaces.
`denied` must continue to block the mock as well as any future real adapter.

The real broker must be a separate, privilege-minimized OS process or other
independently enforced resource boundary. Rust traits, in-process state, a
filename, and a cryptographically unprotected JSON flag do **not** constrain
untrusted linked code. There is no universal "device permission" API across
Linux, macOS and Windows.

## Proposed real authorization design (not implemented)

```text
Keyboard / user request
    |
Explicit device identity, capability, operation, duration, purpose
    |
Trusted confirmation UI (not persisted as authority)
    |
OS-enforced broker IPC and caller identity check
    |
Ephemeral broker-held resource handle
    |
Explicit start -> bounded operation -> revoke/close/expire
    |
Observable audit/status: denied, active, revoked, error
```

1. **No discovery without opt-in.** Resolve a user-selected, stable device
   identity from an OS-provided source. No automatic scanning, pairing,
   listening, transmitting or network discovery.
2. **Least privilege.** The broker exposes only enumerated operations for
   specific device IDs and allowed directions. Receive-only cannot imply
   transmission; display write cannot imply arbitrary filesystem writes.
3. **Trust the OS, not an in-memory token.** Authenticate the caller, restrict
   IPC endpoints, and keep authoritative OS handles within the broker or
   transfer only narrowly scoped handles with independently enforced ACLs.
4. **Fresh and narrow authorization.** Each real operation must have a
   narrowly scoped one-time approval, a deadline enforced by a real clock,
   an explicit stop control, and a unique non-replayable session identity.
   `permissions.json` records intent only, never session credentials.
5. **Fail closed.** On broker disconnect, malformed IPC request, denial,
   hot-unplug, timeout, process crash or suspend/recovery, close the OS handle,
   clear buffers and report unavailable. Do not silently replace the target
   device or fall back to broader permissions.
6. **Provenance.** Real observations need source/device identity, units,
   quality, freshness, and a distinction between recorded, synthetic and
   measured data. Test fixtures may not be represented as hardware evidence.
7. **No embedded secrets.** Long-lived credentials or authentication keys
   must not be stored in Rust-Boy notes or plain adapter preferences.

## Platform-specific broker design questions

| Platform | Design work before implementation | Required test evidence |
| --- | --- | --- |
| Windows | Enumerated device identity, restricted named-pipe/IPC ACL, signed helper/update path where applicable, OS handle lifetime | Reject unauthorized callers, impossible cross-device escalation, close on termination/hot-unplug |
| macOS | User-approved device selection, applicable privacy/TCC restrictions, scoped IOKit/serial access and sandbox boundary | Denial/consent flow, permission revocation, sleep/wake recovery, zero silent device probes |
| Linux | udev identity and permissions, restricted helper/Unix socket and ownership, safe hot-unplug handling | Unauthorized caller denied, device symlink substitution denied, no root daemon by default |

A cross-platform interface may report `Unavailable`, but it must not mask
platform security differences. No new privileged helper should be installed
or started as a side effect of TUI launch, `doctor`, `status`, or library reads.

## Required gates before real adapters

- **G2-A — design review:** deny-all stdio grammar and source-level no-I/O gates are authored; selected device models, OS-specific handle boundary, authenticated IPC, documented minimal permissions and threat model review remain open. The stand-alone parser does not meet the full gate.
- **G2-B — isolated authorization:** dedicated trusted consent UI,
  per-device session scope, hard OS permission check, structured denial and
  revocation tests, secure update/install route. This gate is not met.
- **G2-C — simulator parity:** mock and physical interfaces have equivalent
  bounded cancel/timeout/disconnect semantics, but physical deadlines must use
  a trustworthy clock and real handle close. Deterministic fixtures stay mock.
- **G2-D — platform evidence:** selected physical hardware on each supported
  OS, hot-unplug recovery, failure injection, accessibility review and
  device-I/O traces showing **no unapproved operations**. This gate is not met.

## ConPTY is a terminal test, not a device broker

`tools/smoke_windows_conpty.py` uses Microsoft's native pseudoconsole API
to test Windows terminal interaction, resize behavior and terminal restoration
inside GitHub Actions; the new interactive regression passed in [run 40162936734](https://github.com/pskeffington/pip_boy/actions/runs/40162936734). It does **not** grant Rust-Boy access to any external
device, and it does not implement or validate the proposed broker.

## Separate LAN receive scope (planned)

Meshtastic over a LAN node is a network resource, not `serial.read`, and a
network receive scope must not be inferred from `mesh.transmit` or the existing
`network.fetch` capability. MESH-05 accepts the `OS_NETWORK_RECEIVE_SCOPE`
design, but neither the scope nor a LAN broker is implemented. Before any live
LAN socket, define a broker-enforced grant bound to an operator-selected
endpoint, protocol, port
and receive-only data operation, with a deadline, bounded buffers,
cancellation, revoke behavior and safe credential handling.

The endpoint must be a literal IPv4 or IPv6 address, with an explicit port; do
not perform DNS, `.local` resolution, mDNS, SRV lookup, subnet scanning or
automatic fallback. Reject unspecified, multicast and broadcast addresses;
reject loopback in production policy (test-only fixtures cannot mint runtime
grants), normalize IPv4-mapped IPv6 before address-class checks, and require an
explicit IPv6 link-local interface scope. TLS is mandatory. For a literal IP,
validate the certificate IP subjectAltName; alternatively accept an out-of-band
verified certificate/public-key pin bound to that exact IP and port. A
self-signed certificate is not trusted by its first successful handshake, and
pin rotation requires a new explicit enrollment. Do not disable certificate
validation or downgrade to plaintext. Credentials may come only from an
operator-approved OS credential store and must not enter logs or capture data.

The first design candidate is MQTT 3.1.1 over TLS at one explicit endpoint,
with one exact topic filter, QoS 0 and a broker ACL limited to that topic. The
ordinary Meshtastic TCP client startup can send `want_config_id` and
heartbeats, so it is excluded from the receive profile. MQTT client frames are
limited to CONNECT, SUBSCRIBE, PINGREQ and DISCONNECT; PUBLISH, UNSUBSCRIBE and
PUBACK are forbidden. The one exact topic must be valid MQTT UTF-8, with no NUL
or wildcard characters. Parse fixed-header flags and Remaining Length strictly
with no overflow, overlong encoding or trailing-frame ambiguity. If the broker
offers QoS above 0, or an unexpected frame, wildcard, retained message or topic,
close the connection and return no accepted message. The actual station service, topic and payload framing must
be identified before MESH-06 implementation; this candidate does not assert
that any is enabled on the user's station.

The broker-to-client allowlist is CONNACK, a SUBACK granting QoS 0 for the
single requested filter, PINGRESP and non-retained PUBLISH at QoS 0 for that
exact topic. Reject a SUBACK granting higher QoS, any mismatched topic or
retained flag, duplicate SUBACK, unexpected control packet, malformed UTF-8 or
invalid Remaining Length; then close and mark the capture incomplete.

Initial per-session maxima for the design are: 300 seconds, one topic of at
most 128 valid UTF-8 bytes, 10 incoming messages/second with burst 20, 4,096 bytes
per MQTT packet, 512 bytes per extracted Meshtastic frame, 16 queued messages
and 65,536 aggregate queued bytes, plus 1 MiB total received payload. Exceeding
any cap closes the connection and reports an incomplete unverified capture;
limits may be lowered per grant but never raised by the caller. QoS 0 and
rejecting retained messages avoid broker acknowledgement/replay writes. Keep
duplicate/reorder metadata; do not deduplicate into authenticated identity.

The host-owned confirmation surface must show the exact endpoint, protocol,
topic and duration and require a direct user action; saved preferences and
module selection cannot create a grant. Each confirmation creates a fresh,
nonserializable session generation bound to the signed caller, endpoint,
protocol, exact topic, caps and deadline.
On revoke, expiry, sleep, disconnect, malformed/over-limit input or broker
failure, immediately cancel reads and close the socket, clear queued bytes,
invalidate the generation, and reject callbacks/replies from old generations.
An interrupted capture is incomplete/UNKNOWN; no message receipt is inferred.
Every resulting observation stays unverified.

On macOS, App Sandbox entitlements for serial and outgoing network access can
limit a packaged helper's capabilities, but the TCP client entitlement does
not restrict TCP data direction. A separately sandboxed, on-demand XPC service
is the candidate owner for the endpoint/socket handle; authenticate the peer
with a code-signing requirement and expose only the narrow receive RPC. Direct
distribution of the app and helper must use the Developer ID/hardened-runtime
path; an ad-hoc local build is not release trust evidence. Local
Network privacy acceptance must use the signed packaged app identity: Apple
documents that command-line tools launched from Terminal or SSH are
automatically allowed, so a CLI probe does not demonstrate the packaged app's
consent flow. Do not install or start a privileged helper. These are design
constraints, not implemented protections. See [Apple's serial entitlement](https://developer.apple.com/library/archive/documentation/Miscellaneous/Reference/EntitlementKeyReference/Chapters/EnablingAppSandbox.html),
[TCP network client entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.network.client),
[XPC peer signing requirement](https://developer.apple.com/documentation/foundation/nsxpcconnection/setcodesigningrequirement%28_%3A%29),
[local network privacy note TN3179](https://developer.apple.com/documentation/technotes/tn3179-understanding-local-network-privacy),
and [notarization requirements](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution).

Denial must happen before socket creation; browsing a module or selecting the
station must not connect. The scope must not imply Meshtastic configuration
writes or RF transmission. See MESH-05 and MESH-06 in the
[Meshtastic task queue](../catalog/meshtastic_tasks.json).

Primary platform references:
- [Microsoft: Creating a Pseudoconsole Session](https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session)
- [Microsoft: UpdateProcThreadAttribute](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute)

Related: [security](../SECURITY.md), [adapter preferences](PERMISSIONS.md),
[virtual transport](VIRTUAL_TRANSPORT.md), [typed mock adapters](MOCK_ADAPTERS.md),
and [G2 roadmap](ROADMAP.md).

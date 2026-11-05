# Messenger interoperability acceptance record

**Status: NOT EXECUTED ON PHYSICAL HARDWARE.** Blank rows are pending, not passes.
Software unit tests, capture fixtures and fake SSH relay results are separate
from field evidence. Record immutable commit, OS/architecture, firmware, device
model/USB VID:PID (redact serial numbers), connection topology, UTC timestamps,
expected/observed result and reviewer for every run. Never commit private keys,
raw private conversations, live node locations or credential-store exports.

| Test | Required observation | Status |
| --- | --- | --- |
| Mac → Pi, normal SSH | One message persists on receiver; returned stored receipt matches sender/receiver/ID | Pending |
| Mac → Pi, local private IP | Unresolvable enrolled hostname routed via explicit IP; correct enrolled host key succeeds | Pending |
| Wrong server at fallback IP | Valid unrelated SSH host key is refused; sender stays QUEUED | Pending |
| Unknown/changed/revoked host key | Strict verification rejects; no known_hosts auto-update | Pending |
| Wrong sender key / forced-command label | No unauthorized message import or receipt | Pending |
| Link drops before receiver stores | Sender stays QUEUED, later explicit retry stores once | Pending |
| Link drops after receiver stores | Sender stays QUEUED; retry returns same remote ID, no duplicate | Pending |
| Host reboot after queue write | Pending/stored state and monotonic IDs survive; no reset or plaintext artifact | Pending |
| Native macOS / Windows / Linux key store | Enroll fresh journal; restart/unlock loads it; locked/missing/wrong credential refuses; original retained | Pending per OS |
| Credential recovery/rotation/revocation | Reviewed lifecycle plan and native test evidence; no unreadable-history surprise | Pending design |
| Read-only USB authorization | OS broker denies unauthorized callers, wrong device, stale grant and saved denial | Pending broker |
| USB passive framing | Actual firmware FromRadio capture decodes without wake/config/heartbeat/ToRadio writes | Pending hardware |
| USB reset / unplug / suspend | No unwanted DTR reset where supported; bounded cancellation, cleanup and stale status | Pending hardware |
| Meshtastic protobuf/model/version | Independently decode an actual FromRadio capture using pinned upstream schema; compare selected fields with Rust-Boy's unverified text observation | Pending hardware/schema interoperability |
| Independent reviewer | Named reviewer signs off on crypto/key boundary and real local-IP evidence | Pending |

## Software capture-decoder slice

The offline `pipboy-core::meshtastic_decode` parser now recognizes only
`FromRadio.packet → MeshPacket.decoded → Data(TEXT_MESSAGE_APP)`. It returns
bounded text and optional node/channel numbers explicitly marked unverified;
it does not authenticate peers, record Messenger messages, or produce delivery
receipts. Its synthetic field-mapping fixture is not a hardware capture and has
has been compared byte-for-byte with `protoc` output using a minimal schema
matching the pinned v2.8.0 field types; no generated upstream bindings or actual
radio capture were used. Parser tests cover malformed/truncated fields,
field-number overflow aliasing, fixed32 node IDs, uint32 bounds, duplicate
interpreted fields/oneofs, well-formed unknown fields, unsupported application
ports, text size/UTF-8 and control/bidi rejection. Live USB, actual firmware
decoding, and independent-device schema comparison above remain Pending.

Minimum hardware: the intended Mac, the intended Pi, and two explicitly owned
Meshtastic USB radios with recorded model/firmware/region. This read-only stage
must not transmit application radio packets. Any later RF send requires a
separate protocol/authentication design and region/airtime configuration. Raw
FromRadio reception must never be promoted to authenticated Rust-Boy peer state.

Record results beneath each row or link a sanitized immutable run artifact.
Do not replace Pending with Passed based only on the test definition.

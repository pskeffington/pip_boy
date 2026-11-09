# SCAN camera capture contract — proposal v0
Status: **non-executable design; schema not frozen; no camera transport implemented**.
Module identity: `org.rustboy.scan`, adapter protocol v1. This document defines a prospective **additional control boundary**; it does not alter the installed SDK or declare new scopes currently accepted by the registry.

## Roles
A device may be **source**, **requester**, **processor**, or **viewer**; the phone and Rust-Boy may switch roles in either direction, independently per session. The source-device OS owns camera permission. The requester needs an explicit, independently authorized request. The processor owns inference only if the required local software/model is installed. Only the designated receiving endpoint may consume this session's media.

## Bounded control descriptor (JSON sketch; never includes image bytes)
```json
{
  "schema": "rustboy-scan-capture/v0-proposal",
  "session_id": "synthetic-session-001",
  "request_id": "synthetic-request-001",
  "source_device_ref": "opaque-source-a",
  "requester_device_ref": "opaque-requester-b",
  "processing_device_ref": "opaque-processor-b",
  "source_kind": "paired_phone_camera",
  "operation": "capture_still",
  "camera_ref": "opaque-camera-choice",
  "permission_state": "awaiting_owner",
  "capture_state": "not_started",
  "media_handle": null,
  "media_sha256": null,
  "expires_at": "2026-10-30T12:00:00Z"
}
```
These example IDs are synthetic and confer no authorization. `source_kind` also allows `attached_camera`, `paired_rustboy_camera`, `operator_selected_file` after validation. No raw URL, filesystem path, bearer token, base64 content, credential, OCR text, or full personal/device identifiers may appear in this descriptor. Keep monotonic request sequence, session TTL and schema-version checks.

## Control / media split
1. **Discover locally, without camera activation**: report supported backend, lens/capability metadata only after permission-safe enumeration. A remote peer is not discoverable for SCAN merely from LAN presence.
2. **Pair explicitly**: authenticate **both** device identities using a reviewed enrollment/transport stack with verified binding and channel security; pair does not authorize camera. No public inbound listener or dashboard LAN exposure.
3. **Authorize source-specific capture**: local user on source device (phone or Rust-Boy) selects camera and grants session-purpose scope. Remote requester request = `awaiting_owner`, not permission; owner accepts or denies. A phone can originate capture for Rust-Boy, and can request Rust-Boy's camera in reverse.
4. **Preview** (later network preview gate) only while app is foreground and lease active; no continuous background capture. Still-image MVP must work without network preview.
5. **Capture** by owner action/approved scoped request; validate width, height, byte count, type, frame count, TTL and classification; verify digest before accepting the receiver's content handle.
6. **Transfer** media via separate encrypted, mutually authenticated, receiver-bound binary channel, with bounded chunks and reassembly. `AdapterEnvelope` and `AdapterReply` carry **only metadata**, respecting 4 KiB intent and 16 KiB wire limits.
7. **Admit** opaque handle + digest to `ctl-injest`; Eagle-Eye creates page/region hashes and evidence; TRANS returns bounded status and review receipt; raw media stays inside controlled temporary store. Explicit local preview may show text in a private viewer, never in cross-repo packet metadata.
8. **End/revoke**: any peer loss, OS permission withdrawal, app suspension or timeout stops preview and capture. Purge temporary media by policy with testable retention receipt. A partial/ambiguous transfer stays `unknown_outcome`, not successful.

## Proposed future permission vocabulary
`scan.camera.enumerate`, `scan.camera.preview`, `scan.camera.capture`, `scan.media.receive`, `scan.media.send`, `scan.file.read`, `scan.local.write`, `scan.infer`. Names **must** be reviewed and mapped to real broker/OS capabilities; declaring these strings in a manifest is never a grant. Existing manifest validators do **not** recognize them yet: add, validate and negative-test those exact scopes together in SCAN-H01/H03 rather than bypassing validation.

## Error and state semantics
Control states: `unavailable`, `ready`, `awaiting_owner`, `authorized`, `previewing`, `capturing`, `transferring`, `received_verified`, `admitted`, `review_required`, `denied`, `cancelled`, `disconnected`, `stale`, `faulted`, `unknown_outcome`. UI must distinguish capture complete from transfer-received, admitted and transcription accepted.

## Required deterministic fixtures
- camera absent/driver unavailable, denied OS permission, camera busy, app background, battery pause;
- paired phone -> RB success, reciprocal RB -> phone success, unpaired device and mismatched camera owner;
- stale/rotated key, revoked session, replayed request/receipt, out-of-order chunks, digest mismatch, duplicate and conflicting retry;
- 0-byte, oversized, malformed, symlink/path traversal, unsupported codec, decompression bomb;
- Hebrew/Latin/unknown script, non-paper signage, handwriting, multi-page, low contrast, blank/rotated frame, mixed script;
- offline inference/model unavailable, review-required, release-denied and permitted local draft (not external publication);
- application restart, no background capture, always-available Settings, one foreground package and 480x320 operation.

## Platform-specific acceptance
Local USB/UVC/CSI backend and OS consent: macOS and Linux/Pi (supported devices only). Paired smartphone capture: iOS and Android on tested native or secure browser surfaces; browser camera API availability and secure-context/platform restrictions must be measured rather than assumed. Reciprocal preview/capture requires source-side permission and receiver-side identity verification. Physical camera and two-way pairing tests are **required before production**.

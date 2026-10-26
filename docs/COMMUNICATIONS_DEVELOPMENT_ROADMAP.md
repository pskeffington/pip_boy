# Rust-Boy — Communications Development Roadmap

**Baseline:** 2026-10-25. **Owner:** Rust-Boy maintainers. **Scope:** secure LINK messaging, local IP resilience, Meshtastic/LoRa, MeshCore, Reticulum/LXMF, disconnected operations, and micro-display integration.

**Status legend:** `DONE-IN-CODE` means merged application behavior, not a hardware field acceptance; `RESEARCH` means designs/source evidence only; `NEXT` is the immediate actionable backlog; `BLOCKED` requires a prior security or hardware gate.

## Development principle

Ship a verified narrow vertical slice, merge only after CI, then expand transports. Reuse one canonical message identity, queue, security policy and delivery-state model across bearers. Do not build one chat protocol per radio. No fallback may silently reduce confidentiality or authorize a previously unpaired endpoint. Only the receiving Rust-Boy application can acknowledge **recipient-stored**; a radio or gateway ACK means merely **link-accepted**.

## Milestones and gates

| ID | Order / milestone | Scope and tangible artifact | Depends on | Verification / definition of done | State |
| --- | --- | --- | --- | --- | --- |
| M00 | Foundation inventory | Peer/SSH/Messenger API audit, transport source matrix, failure-domain diagram, supported hardware/OS roster | — | Pinned sources and working-code inventory; no speculative capability labels | DONE-IN-CODE / RESEARCH |
| M01 | Secure envelope and threat model | Versioned binary envelope with authenticated sender/receiver, origin ID, expiry, priority, length and verified crypto key IDs; key enrollment/revocation design | M00 | Negative crypto vectors, identity mismatch rejection, expiry/replay tests, independent review | NEXT |
| M02 | Encrypted local queue | Per-device encrypted-at-rest durable outbox/inbox, bounded TTL/retry/backoff, crash-safe persistence, migrations from plaintext v1 | M01 | Power-failure recovery, no silent data loss or plaintext remnants in new store; platform key management | BLOCKED |
| M03 | Transport interface and simulator | Rust `Transport` trait, typed health/capacity/MTU and fault-domain metadata, zero-I/O mock, deterministic loss/reorder/latency simulator | M01 | Unit property tests, no automatic discovery, identity/receipt separation, reproducible seeds | NEXT (interface only) |
| M04 | Independent local-IP fallback | Pinned SSH on operator-selected independent Ethernet/Wi-Fi interface; no new trust-on-first-use | M03, M02 | Two-node link interruption/recovery; distinct AP/power verified; no silent downgrade | BLOCKED |
| M05 | Meshtastic USB read-only adapter | Pinned protobuf schema, explicit serial port, handshake and passive device/firmware/capabilities parser | M03 | Captured real radio fixtures, no RF transmissions or BLE discovery on mount; device disconnect tests | NEXT after M03 |
| M06 | Meshtastic authorized short-message transport | End-to-end secured application envelopes with MTU budget, throttling, queued fragmented-message policy only if necessary, final-receiver ACK | M01, M02, M05 | Two physical radios, legal frequency profile, 0/5/20/50% loss, no auto GPS/mqtt; correct ACK semantics | BLOCKED |
| M07 | MeshCore independent adapter | Separate companion protocol, user-chosen serial interface and key mapping | M03, M01 | Firmware-pinned fixtures, incompatible Meshtastic frames rejected; no implicit peer trust | RESEARCH |
| M08 | Reticulum/LXMF bridge | Locally sandboxed RNS interface, explicit destination enrollment, bounded handoff and receipt mapping | M03, M01 | Test peer identity isolation and message TTL with no Internet service | RESEARCH |
| M09 | Offline device handoff | Signed/encrypted export/import package with expiry, custody log and no auto import | M01, M02 | Import rejects changed sender/recipient, stale/duplicate/oversized payloads; tabletop drill | BLOCKED |
| M10 | PACE policy engine | Operator-selected priorities, route eligibility and shared-failure register, emergency no-path UX | M04, M06, M09 | Route faults never bypass identity; explicit manual takeover; traceable reason codes | BLOCKED |
| M11 | Micro HUD integration | LINK state chips (LOCAL QUEUED, RADIO LINK, RELAYED, RECIPIENT STORED, NO PATH) and configuration workflow | M03 onward | 480×320 keyboard/touch accessibility, no scrolling in primary status view, long-name handling | PLANNED |
| M12 | Physical acceptance/release | Mac↔Pi, Meshtastic USB hardware, underground/basement RF, battery and power-outage tests, docs/security review | M01–M11 | All evidence artifacts and gate signoffs; versioned releases | BLOCKED |

## Immediate three development passes

### Pass A — M01 specification + M03 mock-only architecture

**Code:** Add `EnvelopeV1` schema definition and parser to core with explicit compatibility rules; do not invent encryption. Add `TransportCapability`, `TransportHealth`, `TransportReceipt`, `TransportError` types and a zero-I/O `MockTransport` implementation. Maintain clear readiness annotations for research-only bearers.

**Tests:** Reject incorrect version, destination, lengths, expiry, invalid identifier and duplicate conflicting IDs. Simulate no link, delayed ACK, duplicate ACK, packet loss and stale health information. Assert that LINK launch never emits packets or scans.

**Exit:** API design reviewed; Rust stable, Rust 1.90, macOS/Windows and manifest checks green; test logs attached to PR. This phase does **not** claim E2EE.

### Pass B — M01 reviewed cryptography + M02 durability

**Code:** Choose supported audited cryptographic implementation (do not hand-roll primitives); design identity and offline key rotation/revocation. Bind authenticated envelope to peer/device identity and intended receiver. Protect the disk journal with platform-managed keys, backup/restore controls and migration that never deletes v1 data on failure.

**Tests:** Known-answer vectors, tamper/replay, wrong-key, clock drift, revoked identity, abrupt power loss and full disk. Test Mac and Pi key storage; Windows ACL review. Keep old plaintext journal accessible only through an explicitly governed migration.

**Exit:** Independent security review, operator recovery workflow, clear encryption/E2EE scope, no false stored receipts.

### Pass C — M04 independent IP + M05 Meshtastic USB passive read

**Code:** First, operator-selectable second IP path using the established pinned SSH peer contract. Second, connect a selected USB serial Meshtastic device in **read-only** mode and parse its firmware capability/protobuf frames without injecting packets. Keep MQTT and automatic BLE scanning disabled.

**Tests:** Link failure-domain independence, offline hostname/host-key behavior; Meshtastic capture fixtures, disconnection, mismatched schema and maximum frame sizes. Capture device firmware and hardware version identifiers.

**Exit:** Successful bench trials, documented field constraints, no unsanctioned radio transmit. Only then schedule M06 on-air application messaging.

## Workstreams and dependency ownership

| Workstream | Code or artifact | Accountable acceptance |
| --- | --- | --- |
| Security | core/envelope, key store, threat model | Signed-off cryptographic design and negative tests |
| Protocol | core/transport, message state machine | No false delivery; stable versioned schema |
| IP | existing peer/SSH stack and route binding | Explicit approval, pinned host identity, separate failure domain |
| LoRa | Meshtastic adapter, later MeshCore | Actual firmware interoperability, radio/legal/power check |
| DTN | Reticulum/LXMF interoperability sandbox | Identity/receipt semantics and TTL |
| UX | LINK micro/desktop | 480×320, accessibly labeled statuses and fallback controls |
| Reliability | simulator and physical test harness | Reproducible outage tests; no silent queue loss |
| Documentation | literature matrix, PACE cards, SOP | Traceable source/version and operator instructions |

## Release-level metrics

Log per route: attempted envelopes, accepted at interface, accepted by relay, stored by final destination, confirmed read, expiration, duplicates suppressed, corrupt frames, last observed health, airtime/bytes, retry counts, battery draw, median/P95 latency for delivered messages, and undelivered fraction. Do **not** reduce these to a single unqualified success percentage. Never log plaintext bodies, secrets, exact GPS by default or private keys.

Suggested tests, **not achieved results**: 0 unauthorized recipients accepted, 0 false recipient-stored receipts, 0 unprompted external discovery/transmission, deterministic bounded memory under 50% loss, and intact queue after fault-injected restart.

## Hardware / decisions to freeze before active on-air work

- Actual Raspberry Pi model, OS, power system, local GPS provider and USB ports.
- Meshtastic hardware/firmware (e.g. RAK4631), legal frequency region, antenna and radio power.
- Mac host and SSH key enrollment; independent access point or peer-to-peer interface.
- Whether location is shared at all; default **OFF**.
- E2EE library/key model, local hardware-backed keystore support, and rotation policy.
- Maximum message size and LoRa airtime budget; no claim of reliable transmission through tunnels/vaults without field evidence.

## Source and change control

- [14-source contingency matrix](research/contingency_transport_matrix.csv)
- [Contingency design core](CONTINGENCY_NETWORK_CORE.md)
- [Messenger security contract](MESSENGER.md)
- [CISA/NIST/DTN PACE literature](research/communications_redundancy_lit_matrix.csv) (branch PR #28; link becomes available after merge).

Every merged adapter must include: protocol and firmware version pin, change log, risk register entry, unit and hardware-test fixtures, full CI, micro UX evidence, and an update to this roadmap. **Research-only features must remain disabled** until all predecessors and acceptance criteria are met.

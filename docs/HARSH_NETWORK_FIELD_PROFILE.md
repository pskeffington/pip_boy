# Rust-Boy harsh-network field deployment profile

**Status:** Design review, not live field capability. Assumption: deployer administers the initial sector LAN/APs and optional command relay, but field devices regularly leave coverage. Network control does not imply trusted endpoints, persistent connectivity, or RF availability.

## Standards and design sources

- [IETF RFC 9171 — Bundle Protocol v7](https://www.rfc-editor.org/rfc/rfc9171.html): application-level store-carry-forward, expiration and delivery across intermittent networks. Inspiration for durable event envelopes; do not claim BPv7 interoperability.
- [IETF RFC 9172 — BPSec](https://www.rfc-editor.org/rfc/rfc9172.html): confidentiality and integrity on intermittently forwarded bundles. Relay nodes are untrusted with respect to plaintext and command authority.
- [IETF RFC 8966 — Babel](https://www.rfc-editor.org/rfc/rfc8966.html) and [RFC 8965 — applicability](https://www.rfc-editor.org/rfc/rfc8965.html): loop-aware mobile/lossy IP routing and cost-aware hysteresis, but periodic advertisements may drain batteries. Do not implement a bespoke routing protocol without need.
- [IETF RFC 9000 — QUIC](https://www.rfc-editor.org/rfc/rfc9000.html): migration between IP paths; does not replace durable storage during full disconnection.

## Operational states

| State | Entry evidence | Behavior |
| --- | --- | --- |
| SECTOR | Authenticated enrolled command route verified recently | Send approved bounded deltas over managed IP |
| PEER | Authenticated authorized direct peer, command unreachable | Exchange only approved recipient-bound envelopes |
| RELAY | Authenticated and approved forwarding service, no direct path | Forward encrypted, scoped, time-limited bundles; relay cannot issue Command grants |
| ISOLATED | No currently authenticated authorized bearer | Journal locally; clearly show unsent status, never claim receiver got data |
| REJOIN | Previously offline session reauthenticates with fresh policy evidence | Validate revocation, reconcile cursor/events, retry idempotently, import durable receipts |

Do not downgrade authentication, disable certificate validation, or accept stale revocation solely to recover an apparently reachable link. If trustworthy current time/revocation freshness is unavailable, hold transfers whose policy requires it.

## Traffic handling

- Lane 0: emergency alerts **only after separately accepted bearer and receipt security**, never inferred from a generic queued message. Local presentation must distinguish *generated*, *queued*, *transmitted*, *stored*, *human acknowledged*.
- Lane 1: command assignments, cancellation/revocation, acknowledgment and status transitions; bounded small payloads.
- Lane 2: ordinary messages, notes and site observations (only after classification/authorization gates).
- Lane 3: bulk public-demo data, catalog and map metadata.
- Lane 4: live STREAM media on suitable IP links only, paused/deprioritized when constrained.

Priority must not permit unbounded starvation; enforce per-origin quotas, maximum payload sizes, queue space reservations, TTL and eviction policies. Never silently drop durable task updates. Never put PHI, secrets or sensitive organizational records in the initial public-demo queue.

## Durable sync contract

1. Journal immutable events **before** network send (atomic commit/fsync requirements to be tested).
2. Allocate per-source sequence and unique event ID, bind org/device/dataset/recipient and authenticate origin.
3. Prior to transfer, perform independent enrollment and policy checks with revocation freshness and replay ledger. A local policy hint is not authority.
4. Perform bounded manifest/cursor exchange; send missing deltas in bounded chunks.
5. Receiver verifies authorization and message integrity, deduplicates and durably records before returning a receipt.
6. Sender marks *stored at peer* only on a verified durable receipt; human acknowledgment is a separate event.
7. On crash, partition or retry, repeat with same event IDs; do not overwrite conflicting assignment ownership.
8. Use exponential backoff with jitter and route hysteresis to avoid synchronized reconnect storms.

Initial v1 field deployments should preserve the existing manually enrolled/pinned OpenSSH route. A persistent opt-in field listener and relay remain implementation/security gates, not capabilities already provided by the current web dashboard. Never bind the localhost dashboard to a public or sector LAN to bypass this gate.

## Field acceptance matrix

| Test | Required proof |
| --- | --- |
| Pull power on field device during durable queue commit | No partial accepted record; recover safely |
| Device moves beyond sector Wi-Fi | Application remains usable; no false online or delivered state |
| Direct peer encounters wrong SSH host key | Refuse and retain queue; do not downgrade |
| Relay exits mid-transfer | No lost acknowledged event; resume direct if permitted or hold |
| Partition creates duplicate/reordered packets | One applied event per verified identity; idempotent receipt |
| Multiple units reconnect together | Bounded backoff, no unbounded CPU/RAM/socket storm |
| Device clock unreliable or revocation record stale | Fail closed for freshness-dependent grants |
| Pi low battery, radio airtime constrained | Suppress large payloads/periodic chatter; do not starve UI |
| STREAM active while important COM traffic arrives | COM processing not blocked by video worker or bulk transfer |
| One link flaps rapidly | Hold-down/hysteresis prevents route oscillation |
| Unauthorized nearby device advertises itself as command | No trust escalation or data release |
| 2 units with no mini-server | Valid enrolled direct path operates when physically available |

**Initial target:** functional Mac desktop plus two Raspberry Pi units, explicit verified wired/Wi-Fi OpenSSH links, controlled disconnection/rejoin, and synthetic public-demo assignments only. Mesh routing, emergency bearer, E2EE, protected institutional records and autonomous pairing require separate release gates.

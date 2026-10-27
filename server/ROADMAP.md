# Rust-Boy server and field synchronization — phased implementation

**Baseline:** `pipboy` supports manual enrolled-peer pinned-OpenSSH task/message exchange. The desktop connection check, hybrid route simulator and harsh-network guidance are separate pending pull requests (#66, #67, #68), not assumed merged. This roadmap does not mark those as delivered.

[Research and source-to-test crosswalk](LITERATURE_MATRIX.md) documents the NIST/healthcare/tunnel/DTN evidence base and phase-specific gates.

[Mining-grade network research and failure matrix](MINING_COMMUNICATIONS_MATRIX.md) is the primary harsh-network design benchmark for S5–S7.

[Ranked build priorities and engineering increments](BUILD_PRIORITIES.md) map research into implementation order, dependencies and exit criteria.

## Milestone gates

| Phase | Objective | Implementation deliverable | Exit evidence |
| --- | --- | --- | --- |
| S0 (this PR) | Structure and ownership | Dedicated `server/` workspace area, inert `rust-boy-mini-server` binary, indexes, traceable roadmap | Cargo build/test + navigation checks in CI |
| S1 | Host/field identity foundation | Per-device protected keys, administered trust roots, replay ledger, revocation freshness, explicit pairing | negative/revocation/key rotation tests on Mac/Pi; fail closed |
| S2 | Native field agent | Separate opt-in service and minimal bound authenticated API; local durable queue and backpressure | crashes, power cuts, resource ceilings; dashboard remains localhost-only |
| S3 | Direct desktop-to-field | Pinned OpenSSH compatibility, authorized send/receipts, queued delta sync, connection UI | 1 desktop + 2 Pi, disconnect/rejoin, verified durable receipts |
| S4 | Optional mini-server MVP | Authenticated listener, scoped per-recipient encrypted store/forward, quotas/TTL, audit | relay restart/partition, no cross-org access, no relay command authority |
| S5 | Harsh-link PACE routing | Manual then bounded automatic direct/relay switching, hysteresis, offline holds, retry jitter | loss/delay/duplicate/reorder/route flapping simulations and field tests |
| S6 | True mesh transports | Explicit trusted LAN/hotspot peer bridging; radio only narrow low-bandwidth metadata after hardware validation | multi-hop route test, airtime/battery budgets, absence of privilege escalation |
| S7 | Deployment hardening | signed releases, secret provisioning, backup/restore, revocation ops, accessibility and Pi target | independent security assessment, rollback, operator runbooks |

Every phase uses synthetic/public-demo payloads until separately approved classifications. No autonomous peer discovery, organizational command authorization, regulated data, or life-safety reliance follows automatically from completing a software phase.

## Execution order and module boundaries

- `pipboy-core`: versioned envelope, deterministic policy/replay/queue primitives; no sockets.
- `crates/pipboy`: existing CLI and loopback-only dashboard; UI reads state through reviewed APIs.
- `server/mini-server`: optional coordinator/relay daemon; cannot issue Command assignments or act as a root CA.
- `field-agent` (future dedicated crate): bounded authenticated listener on a field device; do not recycle dashboard HTTP exposure.
- `rust_radio`, `rust_stream`, `rusti_rc`: separate native modules with explicit transport isolation, not implicit privileged network routes.
- `rust-boy-broker`: existing deny-all physical device broker, never reused as a networking server.

## Implementation tasks for S1–S4

1. Define a frozen versioned wire schema with event identity, destination binding, content hash, max size, and compatible v1 bridge.
2. Document attacker model, allowlist, key custodians and trustworthy time/revocation decisions before introducing listeners.
3. Use audited authenticated transport; reject IP/hostname/discovery information as an identity substitute.
4. Append to an fsync-verified local journal before receipt; implement replay-safe idempotent retries and reconciled cursors.
5. Require operator/device grants for each dataset and task mutation. Keep IRC names and role UI labels non-authoritative.
6. Add explicit lifecycle and health states: unavailable, paired-not-checked, verified-now, disconnected, rejoining.
7. Provide deterministic chaos tests with dropped/reordered/duplicated packets and offline peer recovery.
8. Introduce the optional relay only after direct authenticated exchange and revocation tests pass.

## Index and change discipline

New server endpoints must be reflected in [server/ARCHITECTURE.md](ARCHITECTURE.md), in the root [docs index](../docs/README.md), and in threat/security test coverage. Avoid adding roots outside `server/` for server-only materials; avoid renaming existing `PIPBOY_HOME` storage keys. Any new Cargo member must be represented in root `Cargo.toml` and CI. Merge independently of prior open PRs after checks pass.

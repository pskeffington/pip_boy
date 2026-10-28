# RB sync v2 | integration map and acceptance contract

**Status: experimental offline core, not deployed fleet synchronization.** This guide is the technical entrypoint for developers integrating the Vault Command desktop with field Rust-Boy devices. The current Rust code does not launch discovery, expose a sync HTTP API, authenticate a receipt, or replicate a task. Never treat a role label, vicinity, IP address or a simulated receipt as authority.

## Source-of-truth inventory

| Responsibility | Source | Available behavior | Not yet implemented |
| --- | --- | --- | --- |
| Delta planner | [sync_delta.rs](../crates/pipboy-core/src/sync_delta.rs) | Deterministic event header selection, origin cursors, bounds, gap and duplicate checks | Signed v2 envelopes, receipt authentication |
| Local ledger | [sync_ledger.rs](../crates/pipboy-core/src/sync_ledger.rs) | Versioned synthetic headers, atomic snapshots, advisory lock for cooperating writers, reject malformed history | Encrypted journal, hostile-filesystem hardening, durable receipt database |
| Organizational model | [organization.rs](../crates/pipboy-core/src/organization.rs) | Tenant/site/unit/operator/device graph | Live authenticated tenant enrollment |
| Policy preflight | [organization_policy.rs](../crates/pipboy-core/src/organization_policy.rs) | Structural grant checks; candidate only | Server-side policy enforcement |
| Identity evidence | [organization_identity.rs](../crates/pipboy-core/src/organization_identity.rs) | Credential and possession proof validation against caller-supplied evidence | Trusted root provisioning, persistent anti-replay, fresh revocation service |
| Existing delivery | [peer.rs](../crates/pipboy/src/peer.rs) | Explicit pinned-SSH task and Messenger compatibility paths | Automatic sync v2 transport |
| Task HTTP surface | [task_routes.rs](../crates/pipboy/src/dashboard/task_routes.rs) | Existing operator-invoked task endpoints | Sync v2 routes, authenticated receipt processing |
| Desktop/mobile shells | [dashboard.rs](../crates/pipboy/src/dashboard.rs) | Local-only HTTP server, desktop and micro presentation | Verified fleet live status and background reconciliation |

## Integration invariants

1. Preserve v1 task and Messenger packet contracts and existing CLI, API paths and persistent formats. Introduce a separate version-negotiated v2 namespace.
2. Rust core owns data validation and dataset policy. UI may display state but cannot grant a device or operator access.
3. A sender cursor must advance **only after** an authenticated, recipient-bound durable receipt is verified and persisted. A transport success or socket write is insufficient.
4. Peer discovery, shared LAN, Bluetooth range or cached fingerprint cannot imply organizational membership. All transports must preserve the same authorization decision.
5. The current snapshot is synthetic-only and has no task payloads. It must not contain secrets, ePHI, field location trails or real mission details.
6. Keep exactly one HTTP authority: the loopback dashboard. Do not expose it to the LAN to achieve desktop-to-handheld links.
7. Lost, duplicated, delayed and reordered frames must be modeled offline before any opt-in real-device test. Unknown receipt state stays unknown.

## Local module use (Rust-only; not an operator feature)

Construct a `sync_delta::CursorSet`, pass synthetic `EventHeader` objects to `plan_batch(events, &cursor, limit)`, and treat the result strictly as a **candidate send plan**. `advance_verified` requires independent trusted receipt evidence as a caller precondition; it does not verify signatures itself.

For synthetic local snapshot tests, create a private test directory, then call `sync_ledger::append_locked(&path, event, true)` for first creation and `append_locked(&path, event, false)` for subsequent updates. The directory must already exist and be controlled by the test operator. Both `.json` and adjacent `.json.lock` must remain in the same trusted filesystem area. Locking is advisory, and a hostile process with write permissions can still race or modify files. Do not connect this snapshot to production task records.

## Acceptance and validation

```bash
cargo fmt --all -- --check
cargo test --locked -p pipboy-core sync_delta
cargo test --locked -p pipboy-core sync_ledger
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 tools/validate_component_index.py
python3 tools/test_component_index.py
python3 tools/validate_repo.py
python3 tools/test_validate_repo.py
```

Test definitions exist in the code, but this document does not claim those commands or cross-platform CI passed at the latest commit. Check [validation evidence](VALIDATION.md) and exact-commit Actions results. Multi-host, Pi and hospital deployment acceptance are independent gates.

## Ordered integration backlog

- **I0 — Local hardening:** enforce trusted directory and lockfile ownership, test concurrent writers and power-loss recovery; define retention and corruption quarantine.
- **I1 — Authenticated receipt ledger:** bind organization, target, dataset, origin sequence, event digest and receiving device; reject forged, replayed, stale and conflicting receipts; persist before cursor updates.
- **I2 — Signed v2 envelopes:** unique key epoch, signed origin, bounded payload and explicit data classification; verify grants independently for both operator and device.
- **I3 — Synthetic desktop-to-field delivery:** explicit pinned-SSH transfer of `public_demo` missions, fault injection, restart recovery, and audit trail. No background peer probing.
- **I4 — UI:** show queued/attempted/stored/acknowledged/reconciled as distinct states. Do not label pending attempts delivered.
- **I5 — Physical and regulated gates:** two-host Mac/Pi trials, OS-protected credentials, revocation, operator accessibility, and independent clinical security assessment before broader data classes.

See [fleet roadmap](RB_SYNC_ROADMAP.md), [delta and ledger contract](RB_SYNC_DELTA_PLANNER.md), [repository ownership](REPO_STRUCTURE.md), and [security policy](../SECURITY.md).

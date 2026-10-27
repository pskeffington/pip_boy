# Build priorities — mine-grade disruption-tolerant Rust-Boy COM

**Decision:** Build resilient authenticated message delivery before new mesh radio hardware or a network-facing relay. Priority means dependency order, not a claim that a gate has passed. Keep existing OpenSSH compatibility and synthetic/public-demo data only.

| Order | Work package | Owner | Dependency | Deliverable | Release blocker / evidence |
| --- | --- | --- | --- | --- | --- |
| P0 | Repo/CI and literature integrity | tooling, server | S0 | Matrix validation, indexed paths, Cargo/CI green | Exact-head tests; source claims distinguish verified from unchecked |
| P1 | Fault-injection harness | pipboy-core/tests | P0 | Deterministic loss, duplicate, reorder, delay, asymmetric link, restart, cable cut and mass rejoin scenarios | Reproducible seeds, zero unauthenticated record acceptance |
| P2 | Durable local COM journal | pipboy-core | P1 | Bounded persisted outbound queue, events, receipts and replay-safe recovery | Crash before/after fsync, duplicate delivery, disk full, truncation |
| P3 | Identity and trust | organization identity / security | P0 | Protected device keys, provisioned roots, independent operator grants, persistent anti-replay and revocation | Cross-org, expired, revoked, wrong-key, stale clock refusal on Mac/Pi |
| P4 | Field agent | server/field-agent (new crate) | P2 + P3 | Opt-in minimal authenticated service, no dashboard LAN exposure | Unconfigured means no listener; quotas and graceful stop verified |
| P5 | Direct command-field sync | pipboy + field-agent | P4 | One desktop/two field devices; assignment and receipt delta exchange | Offline edits persist; rejoin idempotent; receipt is not human read |
| P6 | Optional mini-server | server/mini-server | P5 | Bounded opaque store-forward, per-recipient TTL/quotas, audit | Relay down or compromised cannot authorize tasks or decrypt data |
| P7 | PACE path selection | pipboy-core + transport adapters | P5/P6 | Direct LAN then approved relay then offline hold; route hysteresis, jitter | No downgrade on spoofed Wi-Fi; asymmetry/flapping/mass-rejoin passes |
| P8 | Mine transport gateway evaluation | adapter research | P7 | Leaky-feeder/wired-node/wireless-node/MF capability roster and vendor API assessment | Evidence from site survey; no assumed RF range, hardware certification |
| P9 | Field/industry acceptance | operations and security | P8 | Tunnel, hospital, disconnected field runbooks; signed/rollbackable release | Intrinsic-safety/MSHA and AHJ review where applicable; emergency use independently accepted |

## Next three development increments

### Increment A — software-only proof (P0–P2)

- Normalize navigation, validate literature/Mining IDs and source attribution.
- Define a deterministic transport simulator with named fixtures: severed cable, silent relay, one-way radio, tunnel blackspot, duplicate packet, power loss and 20-node mass reconnection.
- Add immutable event ID, queue age/bytes/capacity stats and distinct queued/transmitted/stored/acknowledged states.
- Do not implement new sockets; baseline tests must run offline in CI.

**Exit:** repeatable failures and recovery without false success or dropped durable task updates.

### Increment B — authenticated two-device link (P3–P5)

- Finish protected key custody, trust-root provisioning, persistent replay prevention and revocation before enabling network-facing services.
- Add `server/field-agent/` with explicit disabled startup and reviewed authenticated transport.
- Integrate desktop Commands with field queues while preserving the pinned-OpenSSH compatibility mode.
- Test Mac + two Pi nodes with on/off coverage and mismatched credentials.

**Exit:** a real authenticated, bounded, audited and independently revocable two-device exchange.

### Increment C — optional relay and harsh link switching (P6–P7)

- Promote the mini-server from inert scaffold to separately configured authenticated service only after Increment B passes.
- Introduce opaque forwarding with per-organization scoping, quotas and expiry; verify the relay cannot mint command authority.
- Implement jittered retry, path hysteresis, queuing on partition and idempotent reconcile.

**Exit:** verified delivery under tested partitions, never relying on relay uptime.

## Mining first: acceptance stress roster

- **CABLE-CUT**: no false connected state, alternate path only if authenticated.
- **POWER-CUT**: durable journal survives abrupt loss and reboot.
- **TUNNEL-BLIND-SPOT**: local assignments persist, reconnect after genuine link restoration.
- **ONE-WAY-RF**: no durable receipt inferred from send success.
- **BRIDGE-FAIL**: retry without double-applying commands.
- **MASS-REJOIN**: backpressure and jitter preserve UI responsiveness.
- **COMMAND-OUTAGE**: field operations continue without server.
- **FAKE-RELAY**: refuse unauthorized forwarding or identity spoofing.

## Recommended first coding task

Create the fault-injection harness and versioned journal contract under `pipboy-core`, with source-independent tests. This has the highest leverage and lowest exposure: it informs direct sync, relay, Wi-Fi and future mining transport adapters without prematurely opening network services.

References: [mining source matrix](MINING_COMMUNICATIONS_MATRIX.md), [general literature](LITERATURE_MATRIX.md), [phase roadmap](ROADMAP.md), [architecture](ARCHITECTURE.md).

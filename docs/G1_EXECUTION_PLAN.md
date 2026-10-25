# Rust-Boy | G1 implementation and acceptance plan

**Status:** early implementation, not a completed development gate.  
**Review:** October 24, 2026.  
**Objective:** establish local data provenance, bounded telemetry, consent-aware lifecycle behavior, reliable storage and accessible operation before connecting external hardware.

## Present implementation versus remaining work

| Slice | Existing code | Status | Verification still needed |
| --- | --- | --- | --- |
| Host measurements | metrics.rs | Linux smoke tested | macOS/Windows fixtures and platform timing |
| Typed observation | observation.rs | Integrated with TUI and finite NDJSON; locally tested | Schema migration and field-level units |
| Bounded sample buffer | buffer.rs | Integrated with TUI and finite NDJSON; locally tested | Runtime integration for other adapters |
| Lifecycle state | lifecycle.rs | Integrated into ModuleHost; locally tested | External authorization/device broker |
| Capability prechecks | capability.rs and runtime.rs | Explicit start/revoke/cleanup; TUI P control tested | Persistent per-adapter consent and device broker |
| Finite NDJSON export | sampling.rs | Locally tested, including writer failures | Signal cancellation and other OS clock checks |
| Local notes | notes.rs | Existing, unverified | Crash recovery, migration and Windows overwrite |
| Research consistency | tools/validate_catalog.py | Passing locally | Keep catalog and Rust registry aligned |
| Quality checks | rust.yml | Python + stable + Rust 1.81.0 matrix | Local quality checks pass; remote Actions still unverified |

See [VALIDATION.md](VALIDATION.md) for executed Linux checks. Both TUI and NDJSON now use `ModuleHost` with a lazy telemetry adapter. TUI P controls local host-read grant/revocation. Persistent settings, general adapter consent, and OS brokering remain planned.

G1 module-plan statuses remain **planned** until integration and acceptance tests are verified. File presence is not proof of a functioning runtime.

## Ordered acceptance packages

### G1-A: Quality and reproducibility (inherits G0 blocker)

- Actions jobs must start. Rust stable and 1.81.0 compile/test successfully.
- Record formatter, Clippy, compiler, test and catalog-validator evidence.
- Generate, review and commit Cargo.lock after a real dependency resolution.
- Test small-terminal navigation, exit cleanup, note persistence and failure handling.

**Exit:** reproducible success for the exact commit; no claims based on workflow startup_failure.

### G1-B: Observation and provenance

- A versioned observation carries a source ID, provenance, optional wall clock, process-relative monotonic time, value, quality and expiry.
- Unknown and invalid readings have no value; measured zero is distinct from absent data.
- Add explicit field-level units and schema migration policy before external ingestion.
- Do not compare process-relative timestamps from separate launches.

**Exit:** round-trip, invalid-schema and stale-at-expiry tests pass, with per-platform clock checks.

### G1-C: Bounded local sampling

- A fixed-capacity queue drops oldest records and counts overflow; it never writes files itself.
- An explicit finite command streams local host snapshots as NDJSON to stdout.
- Command count and interval are validated; no unattended collection process is started.
- Add fake clocks/snapshot fixtures, cancellation tests and output failure handling.

**Exit:** bounded memory, stable JSON schema, no default network/device access, observed sample timing.

### G1-D: Lifecycle and permission integration

- State machine transitions must reject start-before-grant, denied modules, and planned-only modules.
- GrantConfirmed is a fact supplied by a separately reviewed authorization service.
- Combine the lifecycle, capability check and built-in ModuleHost in one orchestration flow.
- Revoke access promptly, cancel resources and show degraded/unavailable state.

**Exit:** deny-before-activity and revoked-before-next-poll integration tests. In-process traits do not isolate malicious native code.

### G1-E: Storage, accessibility and reliability

- Move notes to a versioned migration policy, safe atomic write/recovery and explicit import/export.
- Provide searchable notes, retention controls, and user-visible unencrypted storage warnings.
- Verify keyboard-only controls and state labels readable without color; add reduced motion and small-screen profiles.
- Record platform and CPU/memory/battery baselines on actual supported devices.

**Exit:** crash/error fixtures, access-path checks, and documented platform smoke tests.

## Next-pass priority queue

| Priority | Work item | Blocking dependency |
| --- | --- | --- |
| P0 | Investigate Actions startup failure and capture passing CI | Runner/entitlement investigation |
| P0 | Extend lifecycle host integration to other built-ins | G1-D |
| P0 | Build persistent per-adapter consent UI and OS device broker | G1-D/G2 |
| P0 | Fix notes overwrite/recovery and design schema migrations | G1-A |
| P1 | Build testable host telemetry timing and source-quality dashboard | G1-C |
| P1 | Accessibility/low-motion terminal profiles | G1-E |
| P2 | Specify GPIO/device simulator fixtures; no hardware enablement yet | G1-D, G1-E |

## Acceptance evidence format

For each change capture: PR/commit, module IDs, literature IDs, sample fixtures, exact test command, observed result, permissions used, supported OS/hardware, and outstanding risks. Do not replace missing metrics with estimates or mark an unrun test as passed.

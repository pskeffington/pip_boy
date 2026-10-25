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
| Capability prechecks | capability.rs and runtime.rs | TUI pause/revoke locally tested; one-shot STATUS and scoped LIBRARY/NOTES routing authored, newer tests pending | Per-adapter active-session authorization and device broker |
| Finite NDJSON export | sampling.rs | Locally tested, including writer failures | Signal cancellation and other OS clock checks |
| Local notes | notes.rs | Locked staged writes, validated limits, search and explicit export/import; locally tested | Native Windows/macOS runtime and general schema migrations |
| Adapter preferences | permissions.rs; CLI permissions | Versioned requested/denied preferences, bounded locked persistence and fail-closed parsing authored (tests pending) | Actual consent flow, OS broker, native platforms and tamper resistance |
| Virtual broker prototype | device_simulator.rs, simulator_replay.rs; CLI simulate | 11 virtual fixtures and five replay scenarios passed Ubuntu stable/1.81.0 CI | Real OS broker, trusted UX, hardware and wall-clock watchdogs |
| Virtual transport prototype | virtual_transport.rs; CLI simulate transport | Bounded fixture-only FIFO, drop accounting and revoked-read prevention passed Linux/macOS/Windows CI at 46cc627 | Real device brokers and transport isolation |
| Typed adapter precursor | mock_adapters.rs; CLI simulate adapter | InputSource/DisplayBackend/SerialReader traits, fixed in-memory GPIO/display/serial mocks, 10 Rust test fixtures and CLI smoke suite authored | Execute new checks on stable/MSRV, native macOS/Windows and macOS PTY, then review OS-specific backends |
| Console navigation | console.rs / tui.rs | Compact layout, scrolling, explicit note reload and Linux resize smoke checks | Native platforms, assistive-tech and contrast acceptance |
| Research consistency | tools/validate_catalog.py | Passing locally | Keep catalog and Rust registry aligned |
| Quality checks | rust.yml | Cross-platform Ubuntu/macOS/Windows five-job matrix green at 46cc627; new adapter/terminal checks authored | New exact-commit CI, native Windows ConPTY, accessibility/hardware acceptance |

See [VALIDATION.md](VALIDATION.md) for executed Linux checks. TUI and NDJSON use `ModuleHost` with a lazy telemetry adapter; the status CLI has now been routed through the same host (validated in Ubuntu CI at `f445814`). TUI P controls local host-read grant/revocation. Preference persistence is authored but is not an active grant; session authorization, general adapter consent and OS brokering remain planned.

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
- GrantConfirmed must be supplied by a separately reviewed authorization service; persisted preferences and the *simulator-only* `--confirm` do not provide it.
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
| P0 | Done on Ubuntu: removed problematic third-party setup/cache actions and captured passing full CI | Native platform release acceptance remains |
| P0 | Run stable/MSRV and real CLI checks for STATUS, scoped LIBRARY/NOTES and preference commands | G1-A/G1-D |
| P0 | Validate preference store and virtual broker fixtures; build trusted real-device authorization UI and OS handle broker | G1-D/G2 |
| P0 | Extend tested notes portability to general collection migrations and backup identities | G1-E |
| P1 | Build testable host telemetry timing and source-quality dashboard | G1-C |
| P1 | Native platform, assistive-tech and contrast acceptance | G1-E |
| P2 | Validate new bounded mock transport and native macOS/Windows CI; follow with adapter traits and injection/cancellation harness, no physical I/O | G1-D/G2 |

## Acceptance evidence format

For each change capture: PR/commit, module IDs, literature IDs, sample fixtures, exact test command, observed result, permissions used, supported OS/hardware, and outstanding risks. Do not replace missing metrics with estimates or mark an unrun test as passed.

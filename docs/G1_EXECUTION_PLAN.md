# Rust-Boy | G1 implementation and acceptance plan

**Status:** local native foundation exercised in CI; G1 acceptance and all physical-device gates remain open.  
**Review:** October 25, 2026.  
**Objective:** establish local data provenance, bounded telemetry, consent-aware lifecycle behavior, reliable storage and accessible operation before connecting external hardware.

## Present implementation versus remaining work

| Slice | Existing code | Status | Verification still needed |
| --- | --- | --- | --- |
| Host measurements | metrics.rs | CPU/RAM-only refresh, nonblocking console warm-up and monotonic CPU cadence; local Linux checked | Native delta validation and measured hardware baselines |
| Typed observation | observation.rs, host_report.rs | Field units/quality/reasons shared by console and opt-in detailed status/NDJSON; deterministic fixtures | Stored-observation migrations and external ingestion |
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
| Quality checks | rust.yml | Five-job Linux stable/MSRV, macOS PTY and Windows ConPTY matrix green at `4d27208` in run 40162936734 | Release/accessibility acceptance and selected actual hardware |
| OS broker trust model | docs/OS_DEVICE_BROKER.md | Independent broker IPC, per-device identity/consent, real deadline and revocation acceptance specified | No real OS-enforced broker code, user authorization UX or physical adapter yet |
| Physical broker deny-all precursor | crates/rust-boy-broker; tools/smoke_broker.py | Separate executable; 1 KiB strict request frame, 11 closed capability/operation pairs, 12 Rust fixtures and OS-native subprocess tests passed [run 40164717116](https://github.com/pskeffington/pip_boy/actions/runs/40164717116) | Authenticated OS IPC, caller identity, trusted consent and physical handle isolation (all still absent) |

See [VALIDATION.md](VALIDATION.md) for executed Linux checks. TUI and NDJSON use `ModuleHost` with a lazy telemetry adapter; the status CLI has now been routed through the same host (validated in Ubuntu CI at `f445814`). TUI P controls local host-read grant/revocation. Preference persistence is authored but is not an active grant; session authorization, general adapter consent and OS brokering remain planned.

G1 module-plan statuses remain **planned** until integration and acceptance tests are verified. File presence is not proof of a functioning runtime.

## Ordered acceptance packages

### G1-A: Quality and reproducibility (inherits G0 blocker)

- Actions jobs must start. Rust stable and 1.90.0 compile/test successfully.
- Record formatter, Clippy, compiler, test and catalog-validator evidence.
- Generate, review and commit Cargo.lock after a real dependency resolution.
- Test small-terminal navigation, exit cleanup, note persistence and failure handling.

**Exit:** reproducible success for the exact commit; no claims based on workflow startup_failure.

### G1-B: Observation and provenance

- A versioned observation carries a source ID, provenance, optional wall clock, process-relative monotonic time, value, quality and expiry.
- Unknown and invalid readings have no value; measured zero is distinct from absent data.
- Field-level units/quality now exist for local host reports; complete schema migration and ingestion validation before external ingestion. See [host telemetry](HOST_TELEMETRY.md).
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
| P0 | Validate new deny-all physical broker protocol on all three OSs; design independently authenticated OS IPC and trusted real-device consent (no hardware enablement) | G2-A/G2-B |
| P0 | Extend tested notes portability to general collection migrations and backup identities | G1-E |
| P1 | Host field-quality dashboard and deterministic CPU cadence fixtures implemented; complete native CI and measured hardware timing/overhead baselines | G1-B/G1-C |
| P1 | Windows ConPTY terminal journey completed on hosted runner (40162936734); next native accessibility, signal/interruption and forced-write-error acceptance | G0/G1-E |
| P2 | Validate new bounded mock transport and native macOS/Windows CI; follow with adapter traits and injection/cancellation harness, no physical I/O | G1-D/G2 |

## Acceptance evidence format

For each change capture: PR/commit, module IDs, literature IDs, sample fixtures, exact test command, observed result, permissions used, supported OS/hardware, and outstanding risks. Do not replace missing metrics with estimates or mark an unrun test as passed.

# Rust-Boy | Validation evidence

Updated October 25, 2026 (America/New_York). **Current verified source baseline:** [PR CI 40164915215](https://github.com/pskeffington/pip_boy/actions/runs/40164915215) passed catalog, Linux stable and Rust 1.81.0, native macOS PTY, native Windows ConPTY and the deny-all physical-broker tests at `e7d1090`. This is not physical-hardware, packaging or accessibility certification. **Sections farther below preserve historical development snapshots**; any older “pending”, “unexecuted” or “startup failure” wording describes the state *at the cited historical revision*, not the current status.

## G2-A deny-all broker protocol — cross-platform CI verified

`crates/rust-boy-broker` is a **separate**, explicitly invoked, one-shot Rust executable whose only exposed channel is inherited stdin/stdout; it never starts on the normal TUI/status/doctor/notes paths. Its v1 input is a bounded 1,024-byte JSON record with strict capability/operation, opaque `device:` selector, request identifier, duration and purpose rules. It always returns a fixed five-field `allowed:false` response; even `operator_intent:true` cannot authorize a physical interface. It does not read a real port, device, radio, BLE, network or filesystem. No OS-enforced caller identity, broker privilege boundary, trusted user confirmation or physical handle exists.

Twelve new Rust unit tests and `tools/smoke_broker.py` cover strict deny-all behavior, unknown/future fields, malformed/oversize requests, capability-direction restrictions, invalid selectors, redaction, one-shot argument handling and no unintended data-directory writes. The quality workflow now builds **both** CLI and broker with `cargo build --workspace --locked` and runs the broker smoke on Ubuntu, native macOS and native Windows. The offline catalog validator checks the broker roster against declared adapter capabilities and rejects direct physical I/O imports in the virtual-only modules. **Verified:** [GitHub Actions run 40164717116](https://github.com/pskeffington/pip_boy/actions/runs/40164717116) passed all five jobs at implementation head `6713885`. Ubuntu stable passed 13 CLI + 102 core + 12 broker = **127 Rust unit tests**, formatting, compile, Clippy, binary build and nine black-box smoke suites. Linux Rust 1.81.0 passed workspace compile/tests. Native macOS passed 12 broker tests, the new subprocess smoke and its PTY navigation/recovery journey. Native Windows passed 13 CLI + 101 core + 12 broker = **126 Rust tests**, the offline smoke suites including the broker process, and interactive ConPTY restoration. The catalog audit passed independently. The test suite proves bounded strict denial and cross-platform startup, **not** authenticated callers or OS-enforced physical device authorization. See [broker protocol](BROKER_PROTOCOL.md) and [OS broker requirements](OS_DEVICE_BROKER.md).

## Native Windows ConPTY acceptance — verified on hosted Windows runner

Previous [push run 40162057807](https://github.com/pskeffington/pip_boy/actions/runs/40162057807) and [PR run 40162059963](https://github.com/pskeffington/pip_boy/actions/runs/40162059963) passed all five jobs at `8a4f7c5`, including Linux stable/MSRV, macOS native PTY and native Windows unit/CLI tests (**115 Rust tests per toolchain**, plus eight Linux smoke scripts). That green baseline did **not** exercise Windows interactive TUI ConPTY.

The new `tools/smoke_windows_conpty.py` is a Python-standard-library-only ConPTY harness using the OS `CreatePseudoConsole`, process-startup attribute, resize and process-wait APIs. It launches the **real** Rust-Boy TUI under a temporary `PIPBOY_HOME`, checks the initial STAT render, simulated P pause/resume, 20x5 fallback and resize recovery, Q exit, and alternate-screen restoration. A separate malformed-notes journey expects a nonzero exit and restored alternate screen without rewriting corrupt notes. The test has bounded output and deadlines, reads asynchronously to avoid blocked ConPTY pipes, and cleans up child processes/handles on error. The Windows runner executes it **after** native build and offline smoke suites. **Verified:** [run 40162936734](https://github.com/pskeffington/pip_boy/actions/runs/40162936734) completed with all five jobs successful at commit `4d27208`. The Windows job ran 13 CLI and 101 core unit tests plus its offline adapter/transport/permission smoke suite. Its ConPTY test reported a passing Windows command-shell output-pipe control, actual Rust-Boy TUI launch, P pause/resume, 20x5 shrink and resize recovery, Q normal exit, malformed-notes error exit, alternate-screen restoration and no hidden writes. Ubuntu stable ran formatting, Clippy, 13 CLI + 102 core tests and eight smoke suites; Rust 1.81.0 tests and native macOS PTY passed too. Windows ConPTY is **not** a hardware permission or device broker. Accessibility and physical-device gates remain open. No physical device or real authorization is involved. [Microsoft ConPTY reference](https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session); [OS device broker contract](OS_DEVICE_BROKER.md).

## Typed adapter and native console follow-on — execution pending

The core now exports `InputSource`, `DisplayBackend` and `SerialReader` traits with bounded `MockInputSource`, `MockDisplayBackend` and `MockSerialReader` implementations in `mock_adapters.rs`. Ten new Rust unit tests exercise trait-object dispatch, explicit simulated provenance, printable 1–80 character display frames, injected 1–32 byte serial data, saved denials, no implicit confirmation, revocation/cancellation and repeatable JSON. The CLI exposes `pipboy simulate adapter input|display|serial [--confirm]` using only source-defined fixtures. A new `tools/smoke_mock_adapters.py` checks JSON and policy refusal paths. No device or network I/O was introduced.

The native CI suite now includes `tools/smoke_native_cli.py` on Linux, macOS and Windows to verify noninteractive terminal-output behavior, no ANSI escape sequences, strict status JSON, no silent filesystem writes and CLI errors. On macOS, the job additionally runs the existing POSIX PTY `tools/smoke_runtime.py` for TUI keyboard and restoration behavior. **Windows ConPTY now has a passing native test; accessibility acceptance remains open**, as do physical adapters and real device authorization. These additional checks are authored but are not yet verified for the current code revision. See [MOCK_ADAPTERS.md](MOCK_ADAPTERS.md).

## New cross-platform and virtual transport pass — execution pending

The development branch now contains `virtual_transport.rs` with a **1–64 element FIFO**, `pump(1..=128)`, oldest-event drop accounting, pre-read active lease checks and cancellation/revocation/disconnect/timeout/policy-change queue invalidation. New CLI `pipboy simulate transport ID --count N --capacity N [--confirm]` emits a synthetic-only JSON envelope and leaves no files behind. Eight new core fixtures and `tools/smoke_transport.py` cover bounds, order, drop counts, explicit confirmation, denied/corrupt policy, and no stale-event reads. No network/socket/device driver is included.

The existing first-party-only CI now also defines `native` matrix jobs for GitHub-hosted `macos-latest` and `windows-latest`, using runner-native `rustup` for stable `cargo check --workspace --all-targets --locked`, `cargo test --workspace --locked`, and `cargo build -p pipboy --locked`; offline preference, simulator, replay and transport smoke scripts run on those OSs. The Linux stable/MSRV and catalog jobs remain intact. The prior full green Ubuntu result at `8047a24` is *not* proof that the new transport or platform jobs pass. See [virtual transport](VIRTUAL_TRANSPORT.md). Real TUI/PTY, native recovery, screen-reader/contrast tests and actual hardware are not covered by the new matrix.

## Verified Ubuntu baseline and follow-on tests

[Actions run 40160220866](https://github.com/pskeffington/pip_boy/actions/runs/40160220866) passed the complete Linux quality matrix at `e36fd59`: catalog validation, formatting, stable Clippy and compile, 13 CLI + 79 core unit tests on stable and Rust 1.81.0 (**92 each**), and five CLI/TUI smoke suites. GitHub Actions were enabled throughout; the earlier zero-job failure reproduced when an external Rust setup action was used, and resolved when setup/cache used runner-native `rustup`. The account-level action policy was not directly inspected.

**Latest verified delta:** [push run 40160762403](https://github.com/pskeffington/pip_boy/actions/runs/40160762403) and [PR run 40160765839](https://github.com/pskeffington/pip_boy/actions/runs/40160765839) both passed at `f445814`. Both toolchains completed **13 CLI + 84 core = 97 unit tests** with zero failures. Stable Rust 1.99.1 passed formatting, compile, Clippy with `-D warnings`, binary build and all five black-box scripts (`smoke_runtime`, `smoke_notes`, `smoke_permissions`, `smoke_simulator`, `smoke_replay`). Rust 1.81.0 passed workspace compile and unit tests; the offline catalog job passed. New behaviors: cancel-safe mock leases, 1–60,000 ms manually injected virtual TTL, exact-deadline expiry, fail-closed backward clock, and two new deterministic replay scenarios. This is simulated, in-process policy—not a real-time OS watchdog, network interface, or physical device capability. macOS/Windows and selected real-hardware acceptance remain outstanding.

## Historical initial deterministic replay work (before verified CI)

New `device_simulator.rs` behavior models virtual disconnect/reconnect without reviving a stale broker lease. An explicit `Unavailable` error distinguishes a disconnected device from a revoked token. The new `simulator_replay.rs` implements bounded, fixed `nominal`, `disconnect-recover`, and `revoke` logical-time traces; the CLI exposes `pipboy simulate replay ID SCENARIO [--confirm]`. Each trace is labeled `simulated: true`, uses `synthetic_logical_ms` (never wall time), emits one mock-only event before simulated failures, records subsequent rejection reasons, and does not automatically restart a lease. A requested preference is insufficient, while a saved denial remains blocking.

The new source contains eight replay unit fixtures and two disconnect/reconnect broker fixtures. A separate `tools/smoke_replay.py` is wired into the stable Actions workflow; it defines golden action/time/state sequences, checks byte-identical output for repeated runs, enforces deny-before-success and malformed-schema preservation, and verifies no data directory is created by default. Real device access and OS brokering remain absent. These source and test changes are **not compiled or executed**; static source consistency cannot substitute for Rustfmt, Clippy, cargo tests, or native platform checks.

Acceptance commands include `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo build -p pipboy --locked`, and `python3 tools/smoke_replay.py` plus the three prior smoke scripts. Both stable and Rust 1.81.0 must pass. GitHub Actions has previously produced zero-job `startup_failure`; new run-level status must be checked separately. See [SIMULATION.md](SIMULATION.md).

## G1-D simulated broker follow-on (authored, unexecuted)

The latest inspected [Actions run 40159278156](https://github.com/pskeffington/pip_boy/actions/runs/40159278156) reports `startup_failure` and zero jobs at head `ecbb861b`. A static review confirms 11 distinct `sim:*` fixtures, their one-to-one mapping to the 11 external capability enums, explicit confirmation/denial checks, broker-identity lease validation, no `std::fs`, `std::net`, or sysinfo backend in the simulator source, and CI smoke-script wiring. **Static checks are not compiler or functional test results.**

The development branch now includes `crates/pipboy-core/src/device_simulator.rs` and `pipboy simulate list|run ID [--confirm]`. It contains 11 fixed, virtual fixture IDs and no OS, RF, serial, BLE, location or network I/O backend. A simulation requires a fresh explicit `--confirm`; a stored requested preference never authorizes it, while a denied preference blocks it. A per-broker, generation-scoped lease is invalidated on explicit revoke, close or denied policy reload. Events use `simulated:true` and carry no fabricated device measurements. **This prototype is not the G2 hardware broker or active real-device authorization.**

Eight new Rust tests verify inventory coverage, request-vs-confirmation, saved denials, truthful fixture JSON, lease revocation and replay protection, policy reload/fail-closed behavior and cross-broker identity. A new `tools/smoke_simulator.py` script covers CLI parsing, requested/denied preferences, no default filesystem writes, schema corruption and strict simulated JSON output; the stable workflow invokes it. The static offline catalog validator now also checks the simulator fixture roster against all 11 external capability keys.

**Executable validation has not been performed** on these changes. This environment lacks Rust/Cargo, and GitHub Actions has been returning `startup_failure` before any jobs spawn. Run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo build -p pipboy --locked` followed by `python3 tools/smoke_simulator.py`, plus existing `smoke_permissions.py`, `smoke_notes.py` and `smoke_runtime.py`. Confirm stable and Rust 1.81, then native OS acceptance. The historical passing tests are not evidence for this commit. See [SIMULATION.md](SIMULATION.md).

## Current follow-on G1-D implementation: adapter preferences (unverified)

The development branch now contains `crates/pipboy-core/src/permissions.rs`, exposed by `lib.rs`, and a CLI `pipboy permissions list|request|deny|clear`. This is a **preference-only** record for 11 named future adapter capabilities, not actual consent. It never calls `ModuleHost::grant`; stored `requested` does not permit a radio, mesh, location, BLE or network adapter to run. Preferences are in a bounded 16 KiB schema-1 `permissions.json` under `PIPBOY_HOME`, protected by a stable advisory lock and flushed same-directory replacement. Read/list creates no files, invalid or future schemas are preserved, and the explicit commands update or clear decisions.

Five new unit tests and `tools/smoke_permissions.py` were authored to cover empty defaults, no implicit grants, repeat-safe set/deny/clear, unsupported capability refusal, writer contention, corrupt/future/oversized file preservation and CLI behavior. The CI stable job now includes this smoke script. **None of these new tests have been executed.** The current execution environment has no Rust/Cargo binary or network access to clone/install the toolchain; remote GitHub Actions has been failing at startup before any jobs are created. No new build, compilation, native platform, accessibility, or runtime success is claimed. This change also does not make previously authored STATUS or scoped LIBRARY/NOTES tests pass retroactively.

Before accepting, run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo test --workspace --locked` on stable and 1.81.0, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and both existing smoke suites plus `python3 tools/smoke_permissions.py`. Verify Linux, macOS and Windows native file replacement and advisory locking. Source/static checks only are recorded separately and do not prove compilation. See [PERMISSIONS.md](PERMISSIONS.md).

## Change under test

G1-B/G1-C/G1-D/G1-E: shared `TelemetrySession<SnapshotSource>` for TUI and finite NDJSON. The session bounds history, records source/provenance, rejects backwards collection timestamps, and calculates freshness at render time. TUI history holds 60 readings; NDJSON history holds at most the requested 1–30. No telemetry files are created. The legacy `status --json` shape is preserved.

Module IDs: `stat`, `telemetry`. Existing evidence IDs: L03, L19, L31, L32, L33. TUI and finite NDJSON now use a lazy HostTelemetryModule through the lifecycle host. TUI P revokes/resumes local host-read permission. No external capability grants or device adapters were introduced; persistent per-adapter consent and OS brokering remain planned.

## New follow-on change: status lifecycle integration (not yet executed)

On the development branch, `pipboy status` and `status --json` now call a lazy `HostTelemetryModule` through `ModuleHost` instead of directly creating the collector in `main.rs`. Running this command still explicitly requests read-only local host metrics; it does not grant device enumeration, location, network or radio access. The raw eight-field Snapshot JSON contract and human-readable status output are retained.

Three additional deterministic tests have been authored for deny-before-factory activity, one successful sample with the legacy JSON shape, and collector-start failure without fabricated measurements. These tests and the following checks **have not been run for the new commit**: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `python3 tools/smoke_runtime.py`. The 63-test evidence below belongs to the preceding development head, not this follow-on change. GitHub Actions must execute for the new commit before claiming G0/G1 acceptance.

## Executed quality checks

| Command | Observed result |
| --- | --- |
| `python3 tools/validate_catalog.py` | PASS: 27 modules, 37 evidence records, 14 external sources, 8 runtime entries |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --all-targets --locked` | PASS on stable |
| `cargo test --workspace --locked` | PASS: 10 executable + 53 core tests |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo +1.81.0 test --workspace --locked` | PASS: same 63 tests |
| `cargo build --workspace --locked` | PASS |
| `cargo check --workspace --all-targets --target x86_64-pc-windows-gnu --locked` | PASS on stable: cross-compilation check only, no native execution |
| `python3 tools/smoke_notes.py` | PASS: 24 concurrent CLI appends with retry, unique IDs, no lost notes, search, export/import/re-import and schema preservation |
| `python3 tools/smoke_runtime.py` | PASS: timed NDJSON, closed pipe, no telemetry files, TUI pause/resume, reload/oldest-note selection, resize fallback/recovery and exit/error restoration |

Stable toolchain: rustc 1.99.1 (58cf8f9c3, 2026-10-13). Minimum toolchain: rustc 1.81.0 (eeb90cda1, 2024-09-04).

The generated lockfile uses format 3 for Cargo 1.81 compatibility. Modern Cargo's fallback resolver avoids transitive dependencies requiring Rust above 1.81; the tested lock selects instability 0.3.10 and unicode-segmentation 1.12.0. Cargo 1.81 emits a harmless warning for the newer resolver configuration key. CI now uses `--locked`.

Deterministic fixtures cover bounded shared history, stale-at-expiry, no collection during render, invalid retention/expiry, clock rollback before collector activity, finite NDJSON schema round-trip, injected waits, closed output pipes, and propagation of other writer errors.

## Lifecycle integration checks

New fixtures verify no start/sample before grants and explicit start, immediate collector release on revoke, regrant without implicit restart, scoped revocation, cleanup after partial startup failure, explicit reset after unavailable, repeated poll degradation/recovery, stop/drop cleanup, lazy factory invocation, history reset after restart, and pause text without host values.

`tools/smoke_runtime.py` repeats black-box checks against the built binary: two timed NDJSON records, early completion on a closed pipe despite a 30-second requested interval, no telemetry data files, TUI pause held longer than a refresh period, resume with `1 retained / 0 dropped`, all-panel navigation, and normal/malformed-notes terminal restoration. This smoke script is added to stable CI but remote execution is unverified.

## Notes storage pass

The schema-1 filename and fields remain compatible. Writes now hold a stable fs2 advisory lock over read/modify/replace, flush a same-directory tempfile and persist by replacement without unlinking the original first. Unix additionally flushes the parent directory. Bounded reads (8 MiB), note count (10,000), IDs, text, schema versions and unexpected fields are validated. Search is read-only.

Nine new deterministic fixtures cover no-write reads, search, invalid/future schema preservation, lock contention/release, interrupted staging, failed replacement, count/size/ID limits, private Unix modes and orphaned staging. The real CLI smoke test coordinates 24 subprocess appends through up to eight simultaneous subprocesses, retries only reported lock contention, and verifies every expected entry and unique sequential ID. No actual power-failure test was performed.

New dependencies: fs2 0.4.3 and tempfile 3.27.0, reviewed against primary API/source documentation (L35/L36), compiled/tested on Linux stable/MSRV and cross-checked for the Windows GNU target. Native Windows/macOS runtime and filesystem acceptance remain open. See [storage contract](STORAGE.md).

## Console accessibility pass

Four new memory-backed tests cover navigation bounds/preserved selection, empty/planned views, every panel at seven sizes (42 combinations including 0x0, 1x1, 20x5, 39x11, 40x12, 60x16 and 100x24), and access to an older note's wrapped Unicode text tail followed by scroll clamping after enlargement.

The runtime PTY smoke test also modifies the temporary notes file externally, verifies explicit R reload, reaches the oldest note with End, shrinks to 20x5 for fallback guidance, grows back to 80x24 with selection intact, and exits without modifying the notes file. Notes are no longer re-read on each redraw, and no fixed 12-note display limit remains. Ratatui's locked 0.29.0 rendered-line-info feature is used for matching scroll bounds to the actual word wrapper.

The console pass passed 56 tests on stable and Rust 1.81.0. The updated workspace also passes the Windows GNU target compile check; native execution is not established. See [console contract](CONSOLE.md).

## Notes portability pass

Seven additional core tests verify source-preserving export, ID remapping into an existing collection, timestamp/text preservation, repeat-safe re-import, repeated-identical-note multiplicity, invalid/oversized/empty input with no write side effects, writer exclusion, corrupted canonical-file preservation, capacity/ID overflow and combined encoded-size failure without partial commit, and output failure without source changes.

The CLI smoke suite exports all 25 notes created by its append/concurrency checks, imports them into a second collection with an existing note, verifies 26 unique sequential local IDs, re-imports without rewriting, and rejects malformed/future backup input and extra/missing arguments while preserving data. The temporary input file is separate from live notes.json. No network, backup scheduler or automatic migration was added.

Current local verification: 63 tests on stable and Rust 1.81.0, Clippy, format, Windows GNU all-target compile check and both CLI/PTY smoke suites. Details: [storage contract](STORAGE.md).

## Earlier Linux smoke checks (commit 04f4f0f)

A Python subprocess/PTY harness used the built `target/debug/pipboy` with a temporary `PIPBOY_HOME` and `TERM=xterm-256color`:

- `doctor`: validated bundled catalogs successfully.
- `status --json`: parsed real host output and confirmed a positive memory total.
- `sample --count 2 --interval-ms 500`: parsed exactly two versioned JSON records; monotonic capture timestamps differed by at least 500 ms.
- Before adding notes, the temporary data directory remained empty after telemetry commands.
- Added two notes and read back ID 2, verifying repeated local persistence on Linux.
- In a 100×24 pseudo-terminal, navigated all six panels with Tab and exited with Q (exit 0).
- With deliberately malformed notes JSON, navigated to NOTES and observed an error exit (exit 1).
- Both TUI journeys restored the original terminal attributes and emitted the leave-alternate-screen sequence.

## Remaining acceptance work

Remote GitHub Actions previously failed before jobs started; its current success is not established by these local checks. Storage is published as 2223c2c. Console and portability work remain local while the automatic approval reviewer rejects publication; no approval setting was changed. Verify any eventual pushed revisions separately in Actions. macOS/Windows smoke checks, assistive-technology/contrast acceptance, panic/signal paths, native Windows/macOS note replacement, locking and durability, observation schema migrations/field-level units, persistent per-adapter consent, and device-broker enforcement remain open. G0/G1 are not declared complete, and no hardware readiness is claimed.

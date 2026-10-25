# Rust-Boy | Local validation evidence

Review date: October 24, 2026 (America/New_York). Environment: Linux x86_64 container. These are local results; they do not establish remote Actions success or macOS/Windows support.

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

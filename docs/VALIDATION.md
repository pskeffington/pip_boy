# Rust-Boy | Local validation evidence

Review date: October 24, 2026 (America/New_York). Environment: Linux x86_64 container. These are local results; they do not establish remote Actions success or macOS/Windows support.

## Change under test

G1-B/G1-C/G1-D: shared `TelemetrySession<SnapshotSource>` for TUI and finite NDJSON. The session bounds history, records source/provenance, rejects backwards collection timestamps, and calculates freshness at render time. TUI history holds 60 readings; NDJSON history holds at most the requested 1–30. No telemetry files are created. The legacy `status --json` shape is preserved.

Module IDs: `stat`, `telemetry`. Existing evidence IDs: L03, L19, L31, L32, L33. TUI and finite NDJSON now use a lazy HostTelemetryModule through the lifecycle host. TUI P revokes/resumes local host-read permission. No external capability grants or device adapters were introduced; persistent per-adapter consent and OS brokering remain planned.

## Executed quality checks

| Command | Observed result |
| --- | --- |
| `python3 tools/validate_catalog.py` | PASS: 27 modules, 34 evidence records, 14 external sources, 8 runtime entries |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --all-targets --locked` | PASS on stable |
| `cargo test --workspace --locked` | PASS: 6 executable + 37 core tests |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo +1.81.0 test --workspace --locked` | PASS: same 43 tests |
| `cargo build --workspace --locked` | PASS |
| `python3 tools/smoke_runtime.py` | PASS: timed NDJSON, closed pipe, no telemetry files, TUI pause/resume and exit/error restoration |

Stable toolchain: rustc 1.99.1 (58cf8f9c3, 2026-10-13). Minimum toolchain: rustc 1.81.0 (eeb90cda1, 2024-09-04).

The generated lockfile uses format 3 for Cargo 1.81 compatibility. Modern Cargo's fallback resolver avoids transitive dependencies requiring Rust above 1.81; the tested lock selects instability 0.3.10 and unicode-segmentation 1.12.0. Cargo 1.81 emits a harmless warning for the newer resolver configuration key. CI now uses `--locked`.

Deterministic fixtures cover bounded shared history, stale-at-expiry, no collection during render, invalid retention/expiry, clock rollback before collector activity, finite NDJSON schema round-trip, injected waits, closed output pipes, and propagation of other writer errors.

## Lifecycle integration checks

New fixtures verify no start/sample before grants and explicit start, immediate collector release on revoke, regrant without implicit restart, scoped revocation, cleanup after partial startup failure, explicit reset after unavailable, repeated poll degradation/recovery, stop/drop cleanup, lazy factory invocation, history reset after restart, and pause text without host values.

`tools/smoke_runtime.py` repeats black-box checks against the built binary: two timed NDJSON records, early completion on a closed pipe despite a 30-second requested interval, no telemetry data files, TUI pause held longer than a refresh period, resume with `1 retained / 0 dropped`, all-panel navigation, and normal/malformed-notes terminal restoration. This smoke script is added to stable CI but remote execution is unverified.

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

Remote GitHub Actions previously failed before jobs started; its current success is not established by these local checks. Publication of these local commits remains pending; verify the eventual pushed revision in Actions. macOS/Windows smoke checks, small-screen rendering, panic/signal paths, Windows note replacement and concurrent writes, observation schema migrations/field-level units, persistent per-adapter consent, and device-broker enforcement remain open. G0/G1 are not declared complete, and no hardware readiness is claimed.

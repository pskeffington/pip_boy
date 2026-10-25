# Rust-Boy | Local validation evidence

Review date: October 24, 2026 (America/New_York). Environment: Linux x86_64 container. These are local results; they do not establish remote Actions success or macOS/Windows support.

## Change under test

G1-B/G1-C: shared `TelemetrySession<SnapshotSource>` for TUI and finite NDJSON. The session bounds history, records source/provenance, rejects backwards collection timestamps, and calculates freshness at render time. TUI history holds 60 readings; NDJSON history holds at most the requested 1–30. No telemetry files are created. The legacy `status --json` shape is preserved.

Module IDs: `stat`, `telemetry`. Existing evidence IDs: L03, L19, L31, L32, L33. Lifecycle/consent/ModuleHost integration remains open; no new capability grants or device adapters were introduced.

## Executed quality checks

| Command | Observed result |
| --- | --- |
| `python3 tools/validate_catalog.py` | PASS: 27 modules, 34 evidence records, 14 external sources, 8 runtime entries |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --all-targets --locked` | PASS on stable |
| `cargo test --workspace --locked` | PASS: 5 executable + 31 core tests |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo +1.81.0 test --workspace --locked` | PASS: same 36 tests |
| `cargo build --workspace --locked` | PASS |

Stable toolchain: rustc 1.99.1 (58cf8f9c3, 2026-10-13). Minimum toolchain: rustc 1.81.0 (eeb90cda1, 2024-09-04).

The generated lockfile uses format 3 for Cargo 1.81 compatibility. Modern Cargo's fallback resolver avoids transitive dependencies requiring Rust above 1.81; the tested lock selects instability 0.3.10 and unicode-segmentation 1.12.0. Cargo 1.81 emits a harmless warning for the newer resolver configuration key. CI now uses `--locked`.

New deterministic fixtures cover bounded shared history, stale-at-expiry, no collection during render, invalid retention/expiry, clock rollback before collector activity, finite NDJSON schema round-trip, injected waits, closed output pipes, and propagation of other writer errors.

## Executed Linux smoke checks

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

Remote GitHub Actions previously failed before jobs started; its current success is not established by these local checks. Verify the pushed revision in Actions. macOS/Windows smoke checks, small-screen rendering, panic/signal paths, Windows note replacement and concurrent writes, observation schema migrations/field-level units, and lifecycle/permission-host orchestration remain open. G0/G1 are not declared complete, and no hardware readiness is claimed.

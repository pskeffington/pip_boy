# Rust-Boy | Host telemetry and field quality

STAT reads local CPU counters and RAM through the locked `sysinfo` 0.33.1 dependency (literature L31). It also reports OS, build architecture and host uptime. Collection follows the `host.read` lifecycle precheck. Creating the collector loads CPU/RAM data only; no process inventory, process command line or swap refresh is requested.

## Commands and console

```bash
cargo run --locked -p pipboy -- status
cargo run --locked -p pipboy -- status --json
cargo run --locked -p pipboy -- status --details
cargo run --locked -p pipboy -- status --details --json
cargo run --locked -p pipboy -- sample --details --count 3 --interval-ms 1000
```

`status --json` retains the eight original snapshot keys. `sample` retains its version-1 observation envelope containing a raw snapshot. The CPU value is a JSON number when available and **null** during warm-up, an early refresh, or an unavailable/invalid backend result. Existing numeric CPU JSON still deserializes. Consumers must handle null instead of interpreting missing CPU data as zero.

`--details` opts into a different payload. Status can render it as text or JSON; sampling emits detailed NDJSON. Both accept flags in any order and reject duplicates/unknown arguments before collection. Bounds remain 1–30 samples and 500–30,000 ms, with the platform CPU minimum also enforced. These commands write only stdout and create no user-data files.

The TUI uses the same detailed report. Its first CPU delta may be `Unknown (warming up or unavailable)`; later scheduled polls can produce a measured value. Starting/resuming the console does not sleep to prime CPU. P immediately revokes collection, drops the collector/history and clears cached fields. Resume starts a new collector and a new first-poll interval; prior readings are never reused.

## Report contract

Detailed output retains the outer `Observation` version 1: source `host.stats`, provenance `local:sysinfo`, optional wall clock, process-session capture time, expiry and envelope quality. Its value has `payload_schema_version: 1`, `poll_interval_ms` and seven field readings:

| Field | Unit | Availability / validation |
| --- | --- | --- |
| `os`, `arch` | `text` | Blank or `unknown` becomes unknown with no value |
| `cpu_percent` | `percent` | Finite 0–100 inclusive, after a full CPU refresh window; logical cores must be available |
| `logical_cores` | `count` | Zero means unavailable |
| `memory_used_mib` | `MiB` | Valid only with a positive total and used ≤ total |
| `memory_total_mib` | `MiB` | Zero means unavailable |
| `uptime_s` | `s` | Zero is a valid measured uptime |

Each field has exactly `value`, `unit`, `quality` and `reason`. Qualities are `valid`, `stale`, `unknown` or `invalid`. Unknown/invalid fields carry `value: null`; valid measured zero remains a numeric zero. Reason codes are `warming_up_or_unavailable`, `unavailable`, `non_finite`, `out_of_range`, and `inconsistent_memory`. Valid/stale fields have no reason. RAM is reported in whole mebibytes, rounded down by integer division; it is not byte-precision storage.

Envelope `valid` means a snapshot was collected; inspect **each field** for its measurement quality. At expiry, all valid fields and the envelope become stale. Unknown/invalid fields retain their original quality and missing value. Render-time refresh never polls the host and never revives stale fields, even if passed an earlier time.

`poll_interval_ms` is null for the first poll and resets after resume. Later values measure the difference between process-monotonic poll-start timestamps. They are **not** CPU averaging windows, collector latency, scheduler guarantees or comparable timestamps across app launches. CPU refresh gating uses its own monotonic clock and the platform-dependent `MINIMUM_CPU_UPDATE_INTERVAL`; an early poll does not reset the deadline or relabel cached CPU usage as fresh.

## Compatibility and acceptance

This is output from trusted local collectors, not a general external-ingestion API. The detailed payload has its own schema version; no stored-observation migration is implemented, and observations remain memory-only. The outer envelope version and default JSON keys are unchanged. The Rust core's CPU field is now `Option<f32>`; callers must handle unavailable values.

Deterministic fixtures cover warm-up, exact CPU cadence boundaries, repeated/early/backward clock ticks, measured zero, NaN/infinities, range violations, missing OS/core/memory data, inconsistent memory, stale-at-expiry propagation, provenance and JSON round-trip. CLI fixtures check finite detailed output without double collection; TUI fixtures check unknown/stale text and clearing on pause. `tools/smoke_native_cli.py` checks actual detailed CLI output, flag errors and no file/ANSI side effects on all three native CI platforms. Existing PTY/ConPTY tests cover console navigation and pause/resume.

See [validation evidence](VALIDATION.md) for executed checks. These tests establish report semantics and platform operation; CPU accuracy calibration, timing/overhead baselines on specific hardware, accessibility acceptance, durable observation storage and migrations remain open.

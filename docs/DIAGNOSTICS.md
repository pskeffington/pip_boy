# Rust-Boy | DIAG offline evidence intake (initial implementation)

DIAG is a bounded, read-only diagnostic analysis engine. It does not execute uploaded content, discover devices, start network probes, install packages, repair systems, or verify legal authority. The operator must confirm scope for offline imports, and a matching `asset_id` is enforced per record. Permission to inspect a physical object does not necessarily grant permission to access protected accounts, third-party systems, networks, or credentials.

## Quick start

```bash
cargo run --locked -p pipboy -- diag host
cargo run --locked -p pipboy -- diag host --json
cargo run --locked -p pipboy -- diag ingest examples/diag/network-example.jsonl --asset lab-router --confirm-authorized
cargo run --locked -p pipboy -- diag ingest examples/diag/network-example.jsonl --asset lab-router --confirm-authorized --json
```

On the desktop/micro dashboard, activate the Head Engineer or Security Chief profile and select the DIAG tile for an explicit local-only host check. File import is CLI-only in this pass; the UI cannot upload files, invoke arbitrary commands or scan remote targets. Existing role presets still merely pin tools and grant no capabilities.

## Evidence schema v1

One JSON object per line (NDJSON), containing:

| Field | Type | Contract |
| --- | --- | --- |
| `schema_version` | integer | Must be 1 |
| `asset_id` | string | Same as scoped CLI `--asset` |
| `source_id` | string | Provenance identifier, operator supplied |
| `domain` | enum | `hardware`, `software`, `network`, `physical` |
| `metric` | string | One of the metrics below; unknown metrics are unclassified |
| `value` | number, boolean or short string | Scalars only; no executable payload |
| `unit` | optional string | Must match the known metric rule |
| `captured_unix_ms` | optional integer | Source-supplied timestamp (not independently verified) |

Known metric/unit pairs: `hardware.cpu_usage_percent` and `hardware.memory_used_percent` (`percent`); `hardware.disk_smart_passed` (boolean, no unit); `hardware.device_error_count` (`count`); `software.process_exit_code` (`code`); `software.service_running` (boolean); `software.error_count` (`count`); `network.packet_loss_percent` (`percent`), `network.latency_ms`/`network.jitter_ms` (`ms`), `network.interface_error_count` (`count`); `physical.sensor_fault` (boolean).

The engine rejects invalid known metrics, cross-asset records, unsupported versions, malformed values and oversized files. Unknown metric names are counted as **unclassified**, not automatically treated as healthy or erroneous. No result establishes a root cause or guarantees that every subsystem is healthy.

Boundaries: 64 KiB per import, 2,048 bytes per nonempty line, 128 records, 64-byte identifier fields. Reports include evidence line, source, measured value, fault indicator, severity and a suggested *manual* verification step. Results print to stdout and are not persisted by DIAG.

## Relationship to the existing architecture

- `pipboy-core::diagnostics` performs pure analysis and stores no device handles.
- `pipboy diag host` reuses the existing `status::collect()` `host.read` lifecycle.
- Imported JSONL is treated as untrusted operator-provided evidence; its provenance and time are not verified.
- The loopback dashboard offers only one-shot local host analysis and retains existing request protections.
- The separate [NET ENGINEER](NETWORK_ENGINEERING.md) adapter now performs explicitly confirmed private LAN probes. These are **not** DIAG offline imports or authoritative managed-client inventories. Storage SMART, USB hardware, firmware, serial, sensors remain unimplemented. A separate read-only UniFi connected-client adapter is now available on the NET ENGINEER development branch. Follow-up work needs separate scoped permissions, per-device identity and safe acquisition strategies.

## Validation

Unit tests in `crates/pipboy-core/src/diagnostics.rs` cover scope mismatch/denial, signal classification, unknown metrics, invalid data, report JSON round trips, source evidence line numbers and missing host CPU data.

Run:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
node --check crates/pipboy/assets/micro.js
node --check crates/pipboy/assets/dashboard.js
```

**Status:** code authored on a feature branch. Compilation and integration acceptance require a Rust toolchain/CI and are not claimed here.

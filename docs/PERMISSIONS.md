# Rust-Boy | Adapter preferences and authorization boundary

**Gate:** G1-D settings foundation. **Status:** CLI/storage schema and smoke tests verified in the [cross-platform quality matrix](https://github.com/pskeffington/pip_boy/actions/runs/40164915215); **not** an active real-device authorization system.

## Purpose

Rust-Boy now records a user's *preference* for a future external adapter without granting the underlying capability. A preference is **not consent to activate a device**, a permission token, or an OS security boundary. Actual device authorization will require a separate, explicit current-session user confirmation and a reviewed device broker in a later gate.

The `pipboy` executable has these commands:

```bash
pipboy permissions list
pipboy permissions request sdr.receive
pipboy permissions deny ble.discover
pipboy permissions clear sdr.receive
```

`list` enumerates the 11 external adapter capability keys and reports one of `unset`, `requested`, or `denied`. Requested means "consider enabling this later"; it **does not turn on reception or transmission**. Denied means a user preference not to enable the capability; the future broker must honor it before any device access. Clearing returns the key to unset. Commands never discover nearby devices, contact a service, capture location, or start a collector.

The key set is: `device.enumerate`, `gpio.read`, `display.write`, `serial.read`, `gnss.read`, `sdr.receive`, `ble.discover`, `mesh.transmit`, `sensor.read`, `audio.play`, and `network.fetch`. Core `host.read`, `files.read` and `files.write` remain tied to currently implemented CLI/module operations and cannot be saved as adapter preferences.

## Simulator-only confirmation (no active real-device grant)

`pipboy simulate list` lists 11 *hard-coded* virtual fixtures, and `pipboy simulate run sim:sdr --confirm` demonstrates a short-lived confirmation and lease. A saved `requested` preference alone cannot start even a simulation; a saved `denied` preference blocks it despite `--confirm`. The simulator never opens an OS interface and cannot supply a real device authorization token. See [SIMULATION.md](SIMULATION.md). Actual per-device consent and a privileged OS broker remain unimplemented.

## Host adapter intent checks (RB-CX-02 candidate)

`AdapterAuthorization` is a host-owned, in-memory policy for the trusted in-process adapter seam. It binds each explicit approval to the active Rust role, package ID and exact manifest scope. `UniversalAdapterHost::intent` first requires the foreground package and its declared scope, then checks the current approval immediately before invoking the adapter. Revocation and role changes therefore deny subsequent requests before the adapter can observe them. UI selection and manifest declarations never create approvals.

Audio scope is arbitrated separately. The host authority reserves one package as media owner on `play`/`resume`; only that owner may `pause` or `stop`. A competing package is denied until the current owner successfully stops. Role changes return `WouldBlock` while ownership remains, so changing roles cannot orphan an active owner. Other authorities deny media actions unless they implement this explicit arbitration contract.

This is application-level policy for trusted adapters, not a boundary against native code. Approvals are ephemeral and are not loaded from the preferences file. No consent UI, durable approval flow, OS permission check, media device handle or physical access is provided. The separate OS broker continues to deny every physical request.

## Local schema and limits

- Path: `PIPBOY_HOME/permissions.json` (otherwise the same default Rust-Boy data directory as notes).
- Schema 1: `{"schema_version":1,"entries":[{"capability":"sdr.receive","preference":"requested"}]}`.
- Maximum encoded/read size: 16 KiB; at most one decision per recognized adapter key; all unknown fields, future schemas, invalid enum variants and duplicate entries are rejected.
- Reads and `permissions list` do **not** create any files or folders. Writes use a stable `permissions.lock` advisory lock plus a same-directory, flushed temporary replacement. Newly created Unix directories/files are private (0700 / 0600). Directory fsync is attempted on Unix.
- Repeating an unchanged request/denial, or clearing an absent entry, does not rewrite the JSON document. Removing or corrupting the file never authorizes devices.
- Unsupported or malformed preference data is preserved on disk, not guessed or automatically migrated. Recover from a user-owned backup after examining the file.

Filesystem atomic replacement, locks and directory syncing have different platform and power-failure guarantees. Native Windows/macOS CLI and test coverage is in place, but hostile-filesystem/path handling and power-failure acceptance remain open. Do not treat this store as tamper-resistant.

## Threat boundary and remaining work

`PreferenceStore` is a local storage service; it does not call `ModuleHost::grant` or connect to any device. `ModuleHost` still enforces separate in-process runtime capability checks. The current adapter registry marks radio, mesh, GNSS and physical devices as **planned**. Even a stored `requested` entry remains unable to start them.

Pending: settings migration policy, command/TUI approval UX for actual session grants, broker-issued OS handles, revocation while operating, per-device identity binding, and threat-model/platform acceptance. A Rust trait or preference file cannot enforce OS permissions against untrusted native code.

## Acceptance evidence required

Run from the development branch:

```bash
python3 tools/validate_catalog.py
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build -p pipboy --locked
python3 tools/smoke_permissions.py
python3 tools/smoke_simulator.py
python3 tools/smoke_notes.py
python3 tools/smoke_runtime.py
```

The new unit fixtures cover no-write defaults, request/deny/clear round trips, no implicit runtime grants, future schema/duplicate/unknown preservation, writer lock contention, and oversized-document rejection. The black-box script tests CLI visibility and invalid-input handling. **These tests execute in CI**, including native macOS/Windows and Linux, as recorded in [validation](VALIDATION.md). Passing preference tests still cannot grant an external device; see [G1 plan](G1_EXECUTION_PLAN.md) and [OS broker requirements](OS_DEVICE_BROKER.md).

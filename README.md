# Rust-Boy

**A Rust-native, offline-first field systems console.**

Rust-Boy is an independent, modular toolkit for desktop terminals and future wearable field computers. It combines local host diagnostics, a curated research library, and a user-controlled offline journal under one minimal native interface. Planned adapters extend it to Raspberry Pi hardware, maps, receive-only radios, and authenticated local communications.

> **Project status — Linux, macOS and Windows CI baseline verified on the feature branch.** [Push run 40161263824](https://github.com/pskeffington/pip_boy/actions/runs/40161263824) and [PR run 40161267203](https://github.com/pskeffington/pip_boy/actions/runs/40161267203) passed all five jobs at `46cc627`: catalog validation, stable/MSRV compilation and tests, formatting, Clippy, six Linux smoke suites, and native macOS/Windows build/tests plus offline virtual-device smoke. A new set of typed GPIO-input, display and serial fixture interfaces and native console/PTTY checks is undergoing validation on the current branch. External physical devices and an OS-enforced authorization broker remain **unimplemented**. Saved adapter requests never turn on a device. See [validation evidence](docs/VALIDATION.md).

## Rust-Boy identity

| Role | Name |
| --- | --- |
| **Public product name** | **Rust-Boy** |
| Product descriptor | Offline-first field systems console |
| GitHub repository | [pskeffington/pip_boy](https://github.com/pskeffington/pip_boy) |
| Existing command / executable | `pipboy` (retained for compatibility) |
| Rust workspace packages | `pipboy`, `pipboy-core` (internal names retained) |
| Local configuration variable | `PIPBOY_HOME` (unchanged) |

**Rust-Boy** is the name used in the interface, documentation, and future releases. Technical identifiers remain unchanged during this branding pass so existing commands and local data continue to work. The naming, visual system, and attribution rules are defined in **[BRANDING.md](BRANDING.md)**.

Rust-Boy is an **unofficial, independent project**. It is not affiliated with or endorsed by Bethesda, the Fallout franchise, The Wand Company, or their licensors. External Pip-Boy projects in the research catalog are references only; no game artwork, protected logos, proprietary firmware, or third-party code is bundled.

## Design principles

- **Native first:** a Rust core and lightweight Ratatui terminal shell.
- **Offline first:** no accounts, analytics, implicit network access, or background scanning.
- **Observable:** measured local data is separated from unknown, simulated, or planned features.
- **Modular:** physical controls, location, radio, and mesh tools use opt-in adapters.
- **Traceable:** external sources carry license and reuse classifications.
- **Accessible:** keyboard-operated navigation, legible contrast, and future low-motion display profiles.

## Available modules

| Module | G0 state | Purpose |
| --- | --- | --- |
| STAT | Implemented | Local CPU, memory, OS, architecture, and uptime |
| LIBRARY | Implemented | Read-only GitHub reference catalog with license indicators |
| NOTES | Implemented | Offline, user-initiated plaintext notes |
| MAP | Planned | Optional GNSS/GPX and licensed offline maps |
| RADIO | Planned | Opt-in, legal receive-only radio integration |
| MESH | Planned | User-authorized local transport and peer authentication |
| GPIO / Devices | Planned | Physical encoders, buttons, display, and serial/BLE adapters |

## Build and run

Requires a stable Rust toolchain and a modern terminal on macOS, Linux, or Windows.

```bash
git clone https://github.com/pskeffington/pip_boy.git
cd pip_boy
git switch feat/rust-native-foundation  # PR #1; development code not yet in main
cargo run -p pipboy -- tui
cargo run -p pipboy -- status --json
cargo run -p pipboy -- sample --count 3 --interval-ms 1000
cargo run -p pipboy -- sources
cargo run -p pipboy -- sources rust
cargo run -p pipboy -- modules
cargo run -p pipboy -- plan G3
cargo run -p pipboy -- evidence Rust
cargo run -p pipboy -- doctor
cargo run -p pipboy -- permissions list
cargo run -p pipboy -- permissions request sdr.receive
cargo run -p pipboy -- permissions deny sdr.receive
cargo run -p pipboy -- permissions clear sdr.receive
cargo run -p pipboy -- simulate list
cargo run -p pipboy -- simulate run sim:gpio --confirm
cargo run -p pipboy -- simulate replay sim:gpio nominal --confirm
cargo run -p pipboy -- simulate replay sim:serial disconnect-recover --confirm
cargo run -p pipboy -- simulate replay sim:sdr revoke --confirm
cargo run -p pipboy -- simulate replay sim:ble cancel --confirm
cargo run -p pipboy -- simulate replay sim:network timeout --confirm
cargo run -p pipboy -- simulate transport sim:sdr --count 5 --capacity 2 --confirm
cargo run -p pipboy -- simulate adapter input --confirm
cargo run -p pipboy -- simulate adapter display --confirm
cargo run -p pipboy -- simulate adapter serial --confirm
cargo run -p pipboy -- notes add "Field observation"
cargo run -p pipboy -- notes list
cargo run -p pipboy -- notes search "battery"
cargo run -p pipboy -- notes export > notes-backup.json
cargo run -p pipboy -- notes import notes-backup.json

python3 tools/validate_catalog.py
cargo fmt --all -- --check
cargo test --workspace
python3 tools/smoke_permissions.py # run after cargo build -p pipboy
python3 tools/smoke_simulator.py   # fixed in-memory fixtures only
python3 tools/smoke_replay.py      # synthetic trace fixtures only
python3 tools/smoke_transport.py   # bounded fake event FIFO only
python3 tools/smoke_mock_adapters.py # typed offline input/display/serial fixtures
python3 tools/smoke_native_cli.py    # portable console/JSON/no-ANSI output checks
cargo clippy --workspace --all-targets -- -D warnings
```

The `sample` command is intentionally bounded to 1–30 samples at 500–30000 ms intervals, writing NDJSON only to stdout. It does not create a recording or start a background service. Actual CPU sampling is subject to the sysinfo platform-dependent minimum refresh interval. The TUI retains at most 60 observations in memory and displays source, freshness, reading age, and dropped-record counts. Closing an NDJSON output pipe stops collection cleanly.

Use **Left/Right** or **Tab** to change panels, **Up/Down** to select entries or scroll STAT, **Page Up/Down** to scroll detail text, **Home/End** for first/last, **R** to reload notes, **P** to pause/resume host collection, and **Q** to exit. The compact layout supports 40×12; smaller terminals show resize/exit guidance. See [console controls](docs/CONSOLE.md). Note files are plaintext and stored under `PIPBOY_HOME` or the platform's default Rust-Boy data directory. Do not store passwords, credentials, or sensitive records in notes.

## Saved adapter preferences

The new `permissions` CLI records `unset`, `requested`, or `denied` intent in a bounded, versioned local JSON file. A `requested` value is only a preference for future functionality: Rust-Boy still does not authorize radio, mesh, GNSS, Bluetooth, GPIO, device enumeration or network access. `permissions list` does not create files. See the [permission storage and threat boundary](docs/PERMISSIONS.md). The implementation requires compilation and runtime acceptance before use is considered verified.

## G1 virtual-device simulator

`simulate list` reports 11 **hard-coded virtual fixtures**, never real connected devices. `simulate run sim:gpio --confirm` requires explicit confirmation for that one in-memory event and emits JSON with `simulated: true`, no hardware readings. Saved `requested` preferences do not satisfy confirmation; saved `denied` preferences block the fixture. Revocation invalidates old simulated sessions in the core API. This precursor does **not** implement any G2 device adapter, OS broker, active real-device grant or physical I/O. See [simulation contract](docs/SIMULATION.md). The `simulate replay` command now provides three fixed, bounded, logical-time traces: `nominal`, `disconnect-recover`, and `revoke`. A disconnected fixture refuses further opens, reconnecting does not revive its old lease, and a revoked fixture requires new explicit confirmation. Traces are simulated evidence of the state-machine behavior, not measurements. New Rust and CLI tests await execution.

Two new fixed mock replay scenarios exercise cancellation and bounded logical-time expiration. They invalidate virtual leases without activating real hardware or using wall-clock timers. [Simulator documentation](docs/SIMULATION.md) describes the test-only contract.

## Virtual transport precursor

The new `simulate transport` CLI generates bounded *fixture-only* events through a 1–64 event in-memory FIFO. It accepts a finite `--count` of 1–128 and records every oldest-event drop. A stored preference cannot authorize a virtual session: `--confirm` is required, and an explicit saved denial takes precedence. A cancelled, disconnected, expired or revoked virtual session cannot drain old buffered data. There is no real serial, BLE, SDR, mesh or network transport. See [virtual transport contract](docs/VIRTUAL_TRANSPORT.md). The new code is undergoing CI validation on Linux, macOS and Windows.

## Typed mock adapters

Three new Rust core traits (`InputSource`, `DisplayBackend`, and `SerialReader`) now have explicit, bounded *virtual-only* implementations. CLI demonstration reports a mock GPIO input status (no sampled pin value), a non-rendered display receipt, or explicitly injected serial bytes. The examples never connect to a physical display, GPIO device, or serial port. A new cross-platform CLI smoke suite and macOS POSIX terminal restoration checks are included in CI; see [typed mock adapter contract](docs/MOCK_ADAPTERS.md).

## Repository navigation

| Resource | Purpose |
| --- | --- |
| [BRANDING.md](BRANDING.md) | Name, product language, palette, identifiers, and asset boundaries |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Development, attribution, and documentation conventions |
| [SECURITY.md](SECURITY.md) | Local data, permission gates, and security reporting |
| [docs/README.md](docs/README.md) | Technical documentation index |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Core boundaries, adapters, and data guarantees |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Phased implementation and acceptance gates |
| [docs/G1_EXECUTION_PLAN.md](docs/G1_EXECUTION_PLAN.md) | Next G1 engineering slices and testable acceptance criteria |
| [docs/STORAGE.md](docs/STORAGE.md) | Notes schema, limits, write locks and recovery rules |
| [docs/PERMISSIONS.md](docs/PERMISSIONS.md) | External adapter preferences, storage and consent boundary |
| [docs/SIMULATION.md](docs/SIMULATION.md) | Fixed virtual device fixtures, session revocation and mock-only outputs |
| [docs/VIRTUAL_TRANSPORT.md](docs/VIRTUAL_TRANSPORT.md) | Bounded fake FIFO, drop accounting, per-lease read checks and cross-platform CI |
| [docs/MOCK_ADAPTERS.md](docs/MOCK_ADAPTERS.md) | Typed input/display/serial fixture interfaces and no-hardware contract |
| [docs/CI_TRIAGE.md](docs/CI_TRIAGE.md) | GitHub Actions startup investigation and local verification |
| [docs/LITERATURE_MATRIX.md](docs/LITERATURE_MATRIX.md) | Evidence matrix connecting standards and studies to modules |
| [docs/MODULE_CATALOG.md](docs/MODULE_CATALOG.md) | Planned modules, features, dependency gates and acceptance criteria |
| [docs/OBJECT_MODEL.md](docs/OBJECT_MODEL.md) | Rust traits, composition, host policy, and proposed adapters |
| [catalog/module_plan.json](catalog/module_plan.json) | Machine-readable planned features and module dependencies |
| [catalog/literature.json](catalog/literature.json) | Machine-readable engineering evidence roster |
| [docs/RESEARCH_LIBRARY.md](docs/RESEARCH_LIBRARY.md) | Evaluated third-party references and licensing policy |
| [catalog/sources.json](catalog/sources.json) | Structured source index, not vendored code |
| [crates/pipboy-core](crates/pipboy-core) | Shared data, telemetry, modules, and notes |
| [crates/pipboy](crates/pipboy) | Terminal UI and command-line entry point |

## Licensing and provenance

Original Rust-Boy code is MIT-licensed; see [LICENSE](LICENSE). Upstream projects, graphics, hardware designs, sound recordings, and trademarks retain their own rights. A repository listed in the research catalog is **not** automatically approved for source or asset reuse.

**Maintained by Paul A. Skeffington and contributors.**

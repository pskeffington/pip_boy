# Rust-Boy

[![Rust-Boy quality gates](https://github.com/pskeffington/pip_boy/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/pskeffington/pip_boy/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-informational)](LICENSE)
[![Rust: 1.81+](https://img.shields.io/badge/Rust-1.81%2B-informational)](Cargo.toml)

**Offline first. Native by design.**

Rust-Boy is an independent, Rust-native, role-configurable developer workbench for desktop terminals, with a modular foundation for future 4-inch wearable and handheld applications. It brings local host diagnostics, a curated research library and user-controlled field notes together in a keyboard-driven interface.

**Current stage:** development preview. The terminal console and offline workflows are implemented and tested on Linux, macOS and Windows. Physical GPIO, serial, SDR, Bluetooth, mesh, GNSS, mapping and physical device integrations are **not enabled**. An explicitly requested Fallout 4 TCP companion browser is available for validation. No packaged production release is claimed.

## Vault-Tec occupational workbenches

**Seven ready-made roles + one empty Custom mode.** On a fresh workbench, **Overseer Operations** is the default; users can switch between complete, pre-pinned stations. Each preloaded tool must still be invoked explicitly. No saved role opens cameras or devices by itself. This development branch has **unresolved CI gates**; a profile present in source is not a certified release.

| Role | Real-world station focus | Readiness |
| --- | --- | --- |
| Overseer | Local status, notes, research, explicit game companion | Built-in palette; task mission board planned |
| Head Engineer | Hex/CRC32, IPv4 subnet math, simulated device rigs | Workbench commands present; physical serial/GPIO planned |
| Security Chief | Security diagnostics, **four-camera 2×2 wall with focus view** | Camera video planned, not connected |
| Vault Doctor | Research library, environmental observation and safety checklists | Library/notes exist; environmental sensing planned |
| University Researcher | Data inspection, reproducible experiments and statistics | Fixture/palette tools exist; data lab planned |
| Reclamation Scout | Offline field notes, coordinates, maps and game companion | Map/GNSS planned; PS4 acceptance pending |
| Records Custodian | Archive provenance, search, SHA-256 integrity and export | Notes/library exist; secure hashing/index planned |
| Custom | Truly empty expert workspace with user-selected modules | Tool pinning code present; plugin sandbox planned |

```bash
# On the feat/rust-boy-workbench development branch:
cargo run --locked -p pipboy -- workbench profiles
cargo run --locked -p pipboy -- workbench list
cargo run --locked -p pipboy -- workbench use security-chief --replace
cargo run --locked -p pipboy -- workbench use custom --replace
cargo run --locked -p pipboy -- workbench net cidr 192.168.1.50/24
```

Inside the desktop TUI, navigate to WORKBENCH and press **P** to choose a complete role (Enter, then Y to replace), or **A** to browse individual tools. Role replacement modifies pinned layout only, not notes or saved permission rules.

**Design and evidence:** [role-by-role roadmap](docs/ROLE_ROADMAP.md) · [Vault-Tec lore matrix](docs/VAULT_TEC_LORE_MATRIX.md) · [Security Chief four-camera wall](docs/SECURITY_CAMERA_WALL.md). Profiles and hardware readiness are separately gated.

## Local PS4 testing on a Mac

The [Mac → PS4 field guide](docs/MAC_PLAYSTATION.md) describes a direct local-network connection, opt-in discovery, and a finite observation report. The client does not modify PlayStation firmware or perform gameplay actions. Actual PS4 acceptance remains pending.

```bash
./scripts/launch-playstation.command --discover-ps4
./scripts/launch-playstation.command 192.168.1.50 --diagnose
./scripts/launch-playstation.command 192.168.1.50 --observe-seconds 30
./scripts/launch-playstation.command 192.168.1.50
```

## Quick start

Requires [Rust and Cargo](https://www.rust-lang.org/tools/install) (Rust 1.81 or newer), Git and a supported terminal.

```bash
git clone https://github.com/pskeffington/pip_boy.git
cd pip_boy
cargo run --locked -p pipboy -- tui
```

Once running, use **Tab / Shift+Tab** or **Left / Right** to switch panels, **Up / Down** to navigate, **P** to pause/resume host sampling, **R** to reload notes in NOTES, and **Q**, **Esc** or **Ctrl+C** to exit. A terminal of at least 40×12 is recommended. See [all keyboard controls](docs/CONSOLE.md).

Try the command-line interface:

```bash
cargo run --locked -p pipboy -- status --json
cargo run --locked -p pipboy -- status --details --json
cargo run --locked -p pipboy -- sample --count 3 --interval-ms 1000
cargo run --locked -p pipboy -- sources rust
cargo run --locked -p pipboy -- modules
cargo run --locked -p pipboy -- doctor
cargo run --locked -p pipboy -- notes add "Field observation"
cargo run --locked -p pipboy -- notes list
```

The `sample` command emits a finite stream of NDJSON to standard output; it does not record or upload data. Notes are saved locally as **plaintext**, in the application data directory or the location specified by `PIPBOY_HOME`. Do not store passwords, credentials or sensitive records in notes.

Use `status --details` or `sample --details` for per-field units, quality, missing-value reasons and poll timing. The console shows these same quality labels, including CPU warm-up and stale readings. Raw CPU JSON can be null when unavailable. See the [host telemetry contract](docs/HOST_TELEMETRY.md).

## Fallout 4 game mode

```bash
./scripts/launch-playstation.command 192.168.1.50
# Or prompt for the console IP and check the companion protocol:
./scripts/launch-playstation.command --diagnose
```

Current priority: **Mac → PlayStation**. Double-click `scripts/launch-playstation.command` or substitute your console IP above. See [Mac/PlayStation setup and diagnostics](docs/MAC_PLAYSTATION.md). Enable the game's Pip-Boy App setting on a build that provides it and load a save first. Use the host LAN IP when running Rust-Boy on another computer. Game mode browses live game objects and updates with keyboard navigation; it is currently read-only. **Actual game-build compatibility remains unverified.** See [setup, controls and limits](docs/GAME_MODE.md).

## Current capabilities

| Area | Status | What it does |
| --- | --- | --- |
| **Fallout 4 companion** | Implemented; live game acceptance pending | Opt-in TCP game-data browser, heartbeat, incremental state and snapshot diagnostics |
| **STAT** | Available | Local CPU, memory, OS, architecture, uptime, field quality and bounded telemetry |
| **LIBRARY** | Available | Browse bundled, license-aware research/source references without contacting GitHub |
| **NOTES** | Available | Create, search, list, export and import offline field notes |
| **Permission preferences** | Available, intent only | Save `requested` / `denied` settings; never activates devices |
| **Simulation** | Test-only | Replay fixed synthetic fixtures, cancellation, timeouts and bounded event queues |
| **Typed mock adapters** | Test-only | Simulated input, display and serial events, with no physical I/O |
| **Physical broker** | Deny-only prototype | Separate `rust-boy-broker` executable rejects **every** real-device request |
| **MAP, RADIO, MESH, physical GPIO / serial / BLE** | Planned | No live hardware, network acquisition, location or transmission backend |

**A simulator confirmation is not device authorization.** The standalone broker parses at most one bounded JSON request and always responds with `allowed: false`. Real device support requires an independently enforced OS permission boundary, trusted consent, scoped handles and physical-platform acceptance. See [security](SECURITY.md) and the [broker contract](docs/OS_DEVICE_BROKER.md).

Try the virtual fixtures (never physical devices):

```bash
cargo run --locked -p pipboy -- simulate list
cargo run --locked -p pipboy -- simulate run sim:gpio --confirm
cargo run --locked -p pipboy -- simulate replay sim:serial disconnect-recover --confirm
cargo run --locked -p pipboy -- simulate transport sim:sdr --count 5 --capacity 2 --confirm
cargo run --locked -p pipboy -- simulate adapter display --confirm
```

## Project layout

| Location | Purpose |
| --- | --- |
| [`crates/pipboy`](crates/pipboy) | `pipboy` CLI and Ratatui terminal interface |
| [`crates/pipboy-core`](crates/pipboy-core) | Local telemetry, module lifecycle, notes, permissions and simulation |
| [`crates/rust-boy-broker`](crates/rust-boy-broker) | Separate deny-only request-protocol executable; no device grants |
| [`catalog/`](catalog) | License-reviewed source references, research evidence and module plan |
| [`tools/`](tools) | Offline validation and cross-platform smoke tests |
| [`docs/`](docs/README.md) | Architecture, developer contracts, roadmap and executed test evidence |

The public brand is **Rust-Boy**. The repository remains `pip_boy`; the primary command and Rust package remain `pipboy`; `PIPBOY_HOME` is retained for compatibility. The separate denial-only binary is named `rust-boy-broker`. See [brand guidelines](BRANDING.md).

## Build and verify

```bash
cargo build --workspace --locked
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 tools/validate_catalog.py
python3 tools/smoke_broker.py
```

The quality workflow runs catalog checks, stable Rust, the Rust 1.81 minimum-supported version, native macOS terminal tests and native Windows ConPTY tests. For the exact evidence and remaining platform limits, see [validation records](docs/VALIDATION.md) and [GitHub Actions](https://github.com/pskeffington/pip_boy/actions/workflows/rust.yml). A green CI result is not certification for hardware control or safety-critical use.

## Documentation

**Start with:** [documentation index](docs/README.md) · [architecture](docs/ARCHITECTURE.md) · [roadmap](docs/ROADMAP.md) · [validation](docs/VALIDATION.md).

**Develop and contribute:** [contributing](CONTRIBUTING.md) · [changelog](CHANGELOG.md) · [security](SECURITY.md) · [branding](BRANDING.md).

**Device boundaries:** [simulator](docs/SIMULATION.md) · [mock adapters](docs/MOCK_ADAPTERS.md) · [deny-all broker protocol](docs/BROKER_PROTOCOL.md) · [OS-broker requirements](docs/OS_DEVICE_BROKER.md).

## License and independence

Original Rust-Boy software is [MIT-licensed](LICENSE). The curated upstream reference catalog is not a source or asset bundle; third-party software, hardware designs, graphics and trademarks retain their own terms. Rust-Boy is **unofficial and independent**, not affiliated with or endorsed by Bethesda Softworks, the Fallout franchise, The Wand Company, or their licensors. No game assets, proprietary firmware or licensed trade dress are bundled.

Maintained by Paul A. Skeffington and contributors.

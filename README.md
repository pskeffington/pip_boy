# Rust-Boy

[![Rust-Boy quality gates](https://github.com/pskeffington/pip_boy/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/pskeffington/pip_boy/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-informational)](LICENSE)
[![Rust: 1.90+](https://img.shields.io/badge/Rust-1.90%2B-informational)](Cargo.toml)

**Offline first. Native by design.**

Rust-Boy is an independent, Rust-native developer workbench with separate local desktop and micro-handheld graphical shells plus a terminal console, built toward a future 4-inch device. It brings local host diagnostics, a curated research library and user-controlled field notes together in a keyboard-driven interface.

**Current stage:** development preview. The terminal console and offline workflows are implemented and tested on Linux, macOS and Windows. Physical GPIO, serial, SDR, Bluetooth, mesh and hardware GNSS are **not enabled**. The browser MAP viewport can display local RustGIS vector maps and opt-in browser location; native RustGIS rendering and raster basemaps remain unavailable. Offline WGS84 GeoJSON Point tools are provided for the Reclamation Scout. An explicitly requested Fallout 4 TCP companion browser is available for validation. No packaged production release is claimed.

## RustGIS-native field maps and GPS (feature branch)

Rust-Boy's MAP screen now imports actual data in RustGIS `.rgismap`, `.rgis`, `.rgislyr`, mobile `.rgismpk`, project `.rgispkg`, and GeoJSON files. Its independent WGS84 canvas renderer preserves embedded vector geography, source names, visible layers, and basic styling, and aligns an explicitly authorized browser location fix to that geometry. **No OSM proxy or alternative street tiles replace your RustGIS source data**; the viewer reports missing raster/streamed layers instead of inventing a basemap. Select `RUSTGIS` from MAP to load a local export, and expand **DATA SOURCES** to inspect its provenance.

```bash
git clone --branch feat/scout-gps-rustgis-map https://github.com/pskeffington/pip_boy.git
cd pip_boy
cargo run --locked -p pipboy -- dashboard --micro --open
```

See the [RustGIS native map/GPS field guide](docs/SCOUT_GPS_MAP.md). This branch remains a preview: upstream RustGIS's rendering engine, terrain/raster layer integration, native serial GNSS and physical Raspberry Pi tests are separate work.

## Peer-to-peer assignments — Mac ↔ handheld

Overseer now sends tasks **directly to a paired Rust-Boy device**, not via email. The new offline-first peer backend uses an **explicit OpenSSH connection**: known-host fingerprint checks, a dedicated SSH key, a restricted receiver command, a bounded assignment envelope, receiver-side durable storage and a stored-delivery receipt. The receiving job HUD has its **own** task copy and can acknowledge and update work; the Overseer explicitly refreshes peer-reported progress. No auto-discovery, task broadcasting, notification daemon, new TCP listener or background sync is enabled.

```bash
git clone --branch feat/peer-task-transport https://github.com/pskeffington/pip_boy.git
cd pip_boy
./scripts/launch-micro.command
# Device enrollment uses the peer CLI; see the complete setup guide.
```

[Device-to-device installation, SSH pairing and security](docs/PEER_TASK_TRANSPORT.md) · [Overseer Task Control](docs/TASK_CONTROL.md). **Linux/macOS/Windows CI checks are not a substitute for testing two physical devices and out-of-band host keys.** The SSH relay test exercises the Rust-Boy protocol without a real SSH server.

## Overseer Task Control — delegate and track

The **Overseer** is now a manager, not a generic tool collector. Its first preloaded module is **TASK CONTROL**, a persistent offline assignment board. Create tasks with assignee, target job, due date, priority and instructions; filter overdue/blocked/completed work; reassign and record status and manual acknowledgments with revision-safe audit history. On the handheld, **STAT** provides actual task counts and a task queue, and each operational job sees its own assigned-duty list.

**Device dispatch:** A manager explicitly selects a previously paired SSH device, confirms **SEND**, and receives an acknowledgement only after the remote Rust-Boy stores the assignment. Recipient status/acknowledgment is retrieved only when the manager requests a sync. SSH server setup and trust enrollment are manual; no automatic peer discovery, broadcasts or background task polling are enabled.

Test the developer branch:

```bash
git clone --branch feat/overseer-task-ops https://github.com/pskeffington/pip_boy.git
cd pip_boy
./scripts/launch-micro.command
```

See the [Task Control operator and security contract](docs/TASK_CONTROL.md). The regular desktop command `cargo run --locked -p pipboy -- dashboard --open` uses the same board file.

## Micro handheld shell — test on Mac

The **micro mode** is a separate phosphor-style 4-inch-inspired interface, not a collapsed desktop sidebar. It has a top status strip, a single scrolling viewport, six bottom tabs (**STAT / GEAR / MAP / DATA / LINK / CONF**) and a touch/keyboard-friendly occupational profile picker. Seven preloaded jobs and **Custom last** all use the existing Rust data store. Offline Scout waypoint/distance, CIDR and byte inspection work from the micro controls. The Scout viewport can load RustGIS maps on the GPS feature branch; the earlier micro-only branch still shows a placeholder; the Security Chief 2×2 camera focus view says **NO VIDEO** until real drivers exist.

```bash
git clone --branch feat/micro-handheld-shell https://github.com/pskeffington/pip_boy.git
cd pip_boy
./scripts/launch-micro.command
```

On Mac, the launcher starts Rust-Boy and opens **http://127.0.0.1:8765/micro**. Or run `cargo run --locked -p pipboy -- dashboard --micro --open`. Press Ctrl+C in Terminal to stop. The **DESK** link switches to the existing desktop UI at `/`; desktop's **MICRO** link returns here. The same profile changes are visible in both UIs. The micro layout is intended for **800×480** and **480×320** testing, but *physical Pi touchscreen acceptance remains outstanding*. See [micro operator guide](docs/MICRO_UI.md).

## Try the graphical dashboard

<small>**Developer preview branch: `feat/dashboard-ui-shell`**. Local browser window; Rust executable handles the service on 127.0.0.1. No cloud deployment or remote port.</small>

```bash
git clone --branch feat/dashboard-ui-shell https://github.com/pskeffington/pip_boy.git
cd pip_boy
./scripts/launch-dashboard.command
```

Alternatively run `cargo run --locked -p pipboy -- dashboard --open` on macOS. The launcher opens your browser to **http://127.0.0.1:8765/**. Press **Ctrl+C** in the launching Terminal to stop it. Other operating systems can use `cargo run --locked -p pipboy -- dashboard` and open the displayed local URL.

The working UI uses the authoritative Rust job templates: **Overseer, Head Engineer, Security Chief, Vault Doctor, University Researcher, Reclamation Scout, Records Custodian, then Custom**. Click a role to preview it; explicitly confirm loading the profile. Custom starts empty and offers a tool palette. Interactive local labs calculate **GeoJSON waypoints/distance, IPv4/CIDR and hex inspection**; other modules show accurate permission/availability labels. Security Chief has a four-camera **layout fixture only** and Reclamation Scout an **empty RustGIS viewport**, both plainly marked as unconnected. No real video, game link, GNSS or TAK activity starts from the dashboard.

See the [graphical dashboard guide](docs/DASHBOARD_UI.md) for keyboard/touch layout, Mac instructions, API/privacy boundaries and acceptance tests.

## Vault-Tec occupational workbenches

**Seven ready-made roles + one empty Custom mode.** On a fresh workbench, **Overseer Operations** is the default; users can switch between complete, pre-pinned stations. Each preloaded tool must still be invoked explicitly. No saved role opens cameras or devices by itself. The desktop dashboard baseline has passed its cross-platform checks; the dedicated micro branch requires its own exact-SHA CI acceptance. A rendered module is not a certified physical-device integration.

| Role | Real-world station focus | Readiness |
| --- | --- | --- |
| Overseer | **Task Control: assign, track, reassign and manually hand off work** | Local board and role queues implemented; remote delivery not connected |
| Head Engineer | Hex/CRC32, IPv4 subnet math, simulated device rigs | Workbench commands present; physical serial/GPIO planned |
| Security Chief | Security diagnostics, **four-camera 2×2 wall with focus view** | Camera video planned, not connected |
| Vault Doctor | Research library, environmental observation and safety checklists | Library/notes exist; environmental sensing planned |
| University Researcher | Data inspection, reproducible experiments and statistics | Fixture/palette tools exist; data lab planned |
| Reclamation Scout | Native GIS Lab, GeoJSON waypoints, field tools and future TAK-on-Pi | Offline Point tools implemented; maps/GNSS/TAK transport planned |
| Records Custodian | Archive provenance, search, SHA-256 integrity and export | Notes/library exist; secure hashing/index planned |
| Custom | Truly empty expert workspace with user-selected modules | Tool pinning code present; plugin sandbox planned |

```bash
# On the feat/dashboard-ui-shell development branch:
cargo run --locked -p pipboy -- workbench profiles
cargo run --locked -p pipboy -- workbench list
cargo run --locked -p pipboy -- workbench use security-chief --replace
cargo run --locked -p pipboy -- workbench use custom --replace
cargo run --locked -p pipboy -- workbench net cidr 192.168.1.50/24
```

Inside the desktop TUI, navigate to WORKBENCH and press **P** to choose a complete role (Enter, then Y to replace), or **A** to browse individual tools. Role replacement modifies pinned layout only, not notes or saved permission rules.

**Scout GIS development:** [RustGIS/TAK on Raspberry Pi architecture](docs/SCOUT_GIS_TAK.md) · [source](crates/pipboy-core/src/scout_gis.rs). Offline GIS operations can be run independently: `pipboy scout geo waypoint "Trailhead" 42.0 -70.75`, `pipboy scout geo inspect trailhead.geojson`, and `pipboy scout geo distance 42.0 -70.75 42.01 -70.76`. No ATAK Android app, TAK server, GNSS integration, live map renderer or external RustGIS source is bundled.

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

**Compiler policy:** Rust **1.90** is now the minimum supported compiler. Local development uses `stable` from `rust-toolchain.toml`; CI independently tests **1.90.0** and current stable. Rust edition **2021** is retained; 2024-edition crates can be used as dependencies without migrating Rust-Boy's own source edition. This compiler upgrade removes the earlier MSRV mismatch with RustGIS; direct RustGIS crate integration remains an independent engineering step.

For an existing Rust installation, use `rustup update stable` or `rustup toolchain install 1.90.0` to obtain a supported compiler.

Requires [Rust and Cargo](https://www.rust-lang.org/tools/install) (Rust 1.90 or newer), Git and a supported terminal.

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

The quality workflow runs catalog checks, stable Rust, the Rust 1.90 minimum-supported version, native macOS terminal tests and native Windows ConPTY tests. For the exact evidence and remaining platform limits, see [validation records](docs/VALIDATION.md) and [GitHub Actions](https://github.com/pskeffington/pip_boy/actions/workflows/rust.yml). A green CI result is not certification for hardware control or safety-critical use.

## GPS / GeoJSON map preview

The handheld MAP tab and desktop Scout viewport support an opt-in browser position fix, local WGS84 GeoJSON layer rendering, accuracy circle, follow, pan and zoom. This is an independent coordinate viewer, **not** native RustGIS or a street/terrain basemap. See [GPS map setup, privacy and acceptance](docs/SCOUT_GPS_MAP.md).

## Documentation

**Start with:** [documentation index](docs/README.md) · [architecture](docs/ARCHITECTURE.md) · [roadmap](docs/ROADMAP.md) · [validation](docs/VALIDATION.md).

**Develop and contribute:** [contributing](CONTRIBUTING.md) · [changelog](CHANGELOG.md) · [security](SECURITY.md) · [branding](BRANDING.md).

**Device boundaries:** [simulator](docs/SIMULATION.md) · [mock adapters](docs/MOCK_ADAPTERS.md) · [deny-all broker protocol](docs/BROKER_PROTOCOL.md) · [OS-broker requirements](docs/OS_DEVICE_BROKER.md).

## License and independence

Original Rust-Boy software is [MIT-licensed](LICENSE). The curated upstream reference catalog is not a source or asset bundle; third-party software, hardware designs, graphics and trademarks retain their own terms. Rust-Boy is **unofficial and independent**, not affiliated with or endorsed by Bethesda Softworks, the Fallout franchise, The Wand Company, or their licensors. No game assets, proprietary firmware or licensed trade dress are bundled.

Maintained by Paul A. Skeffington and contributors.

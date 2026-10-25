# Rust-Boy

Micro navigation uses STAT / GEAR / MAP / DATA / LINK / SETUP, a consistent header Back button and separate local-tool input/results screens. Back returns through inputs before leaving a tool; temporary local drafts survive navigation until reload.

[![Rust-Boy quality gates](https://github.com/pskeffington/pip_boy/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/pskeffington/pip_boy/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-informational)](LICENSE)
[![Rust: 1.90+](https://img.shields.io/badge/Rust-1.90%2B-informational)](Cargo.toml)

**Offline first. Native by design.**

Rust-Boy is a Rust workbench for local diagnostics, field maps, network engineering and device-to-device task assignments. It has a desktop dashboard, a phosphor-style micro HUD and a terminal console. Seven occupational profiles come preloaded; Custom starts empty.

**Development preview on `main`.** Implemented modules are merged after software checks pass. Physical Raspberry Pi touchscreen, actual UniFi controller, Mac-to-Pi SSH and PlayStation game acceptance remain separate field tests. There is no packaged production release.

## Quick start

Install [Rust and Cargo](https://www.rust-lang.org/tools/install) — **Rust 1.90 or newer** — and Git. Daily development uses stable Rust; CI also checks the exact 1.90.0 minimum. The workspace retains Rust edition 2021.

```bash
git clone https://github.com/pskeffington/pip_boy.git
cd pip_boy
./scripts/launch-micro.command
```

On Mac, the launcher opens **http://127.0.0.1:8765/micro**. Press Ctrl+C in Terminal to stop. Use **DESK** to switch to the desktop dashboard and **MICRO** to return. Other platforms can run:

```bash
cargo run --locked -p pipboy -- dashboard --micro
# Desktop browser UI:
cargo run --locked -p pipboy -- dashboard
# Terminal UI:
cargo run --locked -p pipboy -- tui
```

Open the printed loopback URL in a browser. The dashboard binds only to 127.0.0.1. Keep it local.

Update an existing checkout from `main`; the features below no longer require switching among older preview branches:

```bash
git switch main
git pull --ff-only origin main
./scripts/launch-micro.command
```

## Available modules

| Module | Implemented behavior | Field acceptance still open |
| --- | --- | --- |
| Network Engineer | Private-LAN scan/watch; responder/cache counts; UniFi connected-client inventory and comparison; bounded ICMP/TCP health; nine offline calculators | Actual controller TLS/API access and authorized Mac/Pi LAN measurements |
| DIAG | Local CPU/RAM indicators and scoped, versioned NDJSON fault-evidence ingestion | Device-specific collectors and validated diagnostic rules |
| Overseer Task Control | Persistent assignments, role queues, status, due dates and revision-safe history | Human/organizational identity beyond local role labels |
| Peer task transport | Explicit paired-device OpenSSH send and receipt/status retrieval | Real Mac-to-Pi SSH pairing, interruption and key-rotation tests |
| Reclamation Field Kit | Persistent manual-position survey sites, statuses, selectable offline map markers and GeoJSON export | Field GNSS, RustGIS full renderer, Pi touch acceptance and operational verification |
| Scout maps | Default offline central-Boston real City SAM street geometry; imported RustGIS/GeoJSON layers, opt-in GPS and separately enabled web reference | Upstream RustGIS renderer, raster/terrain, native GNSS and Pi display |
| Host / Library / Notes | Local telemetry with quality labels, bundled references and offline notes | Production packaging and accessibility acceptance |
| Fallout 4 companion | Read-only TCP protocol browser, diagnostics and bounded observation | Actual game-build/PlayStation interoperability |
| Device simulation | Fixed fixtures, bounded queues and mock adapters | Physical GPIO, serial, SDR, BLE, mesh and camera integrations |

The separate `rust-boy-broker` prototype denies every physical-device request. Simulated success does not grant peripheral access.

## Network Engineer

Load **Head Engineer → NET ENGINEER** on desktop or micro. Both views use the same Rust operation catalog and shared tool component. The handheld separates tool selection, inputs, authorization and results; large inventories have pages.

| Tools | Behavior |
| --- | --- |
| SCAN LAN | Explicit RFC1918 IPv4 /24–/32 sweep, at most 254 targets; ICMP/TCP evidence and separate neighbor-cache sightings |
| UNIFI SITES / CLIENTS / LAN | Verified-HTTPS local Integration API reads; connected clients and comparison against active probes |
| PING / JITTER | 3–8 native ICMP attempts; RTT distribution, response loss and successive-reply RTT variation |
| NETWORK HEALTH / TCP CHECK | Fixed literal-IP endpoint; connect timings/failures or one connection check |
| IPv4/IPv6 CIDR, SUBNET SPLIT, ROUTE MATCH, CIDR MERGE, OVERLAP | Offline address and routing calculations |
| MAC / EUI-64, 802.1Q VLAN, MTU / MSS | Offline encoding and packet-budget calculations |

Only explicit Run actions start network requests. Optional inventory refresh runs every 60 seconds, up to 60 runs, and stops on navigation, hidden tabs, cancellation or errors. Returning to the page does not restart it.

```bash
# Offline:
cargo run --locked -p pipboy -- workbench net list
cargo run --locked -p pipboy -- workbench net cidr6 2001:db8::1/64
cargo run --locked -p pipboy -- workbench net split4 10.40.0.0/24 26

# Explicitly requested measurements of an authorized network:
cargo run --locked -p pipboy -- net scan local --confirm
cargo run --locked -p pipboy -- net watch 192.168.1.0/24 --confirm --interval-sec 60 --count 5
cargo run --locked -p pipboy -- workbench net icmp 192.168.1.1 5 --confirm
cargo run --locked -p pipboy -- workbench net health 192.168.1.1 443 5 --confirm

# Controller configuration and certificate trust are required first:
cargo run --locked -p pipboy -- net unifi sites --confirm
```

Responders, cache sightings and controller-connected clients are different counts. A sweep can miss sleeping or filtered devices. TCP connection failures are not packet loss; reported RTT variation is not an RTP jitter measurement. See [Network Engineer](docs/NETWORK_ENGINEERING.md) and [health/UniFi setup](docs/NETWORK_HEALTH_UNIFI.md).

## Occupational stations

| Role | Primary workspace |
| --- | --- |
| Overseer | Assign, delegate and track work in Task Control |
| Head Engineer | Network Engineer, byte tools and host diagnostics |
| Security Chief | Network/host diagnostics and disconnected four-camera layout |
| Vault Doctor | Research references, observations and notes |
| University Researcher | Reproducible local inspection and research tools |
| Reclamation Scout | Site register, field maps, waypoints and WGS84 distance |
| Records Custodian | Local notes and archive references |
| Custom | Empty workspace with explicit tool pinning |

Switch profiles through the UI or `pipboy workbench use head-engineer --replace`. Profile replacement changes pinned layout; each active operation still has its own Run/authorization step. [Role roadmap](docs/ROLE_ROADMAP.md) · [Lore/source matrix](docs/VAULT_TEC_LORE_MATRIX.md) · [Lore-aligned names](docs/LORE_NAMING.md) · [Camera-wall contract](docs/SECURITY_CAMERA_WALL.md).

## Maps, tasks and diagnostics

**Reclamation:** load Reclamation Scout and select **SITES** on micro. Add site observations with manual WGS84 coordinates, condition, priority, notes and position uncertainty; update status and export RFC 7946 GeoJSON. Records persist at `PIPBOY_HOME/reclamation.json` (default `~/.pipboy/reclamation.json`). **MAP → SITES** loads saved markers explicitly; selecting **MAP** on a site opens its location on the grid. WEB is blocked while saved sites are displayed, so site coordinates are not sent to Overpass. No background GNSS or uploads. [Field kit guide](docs/RECLAMATION.md).

**County MAP:** select GPS ON or manual CENTER, then COUNTY. The map resolves the containing Census county GEOID and pulls approved TIGER 2025 county-wide roads, areal water and streams with bounded downloads. A prior county package restores from the local cache without remote requests. [County source guide](docs/COUNTY_SOURCES.md).

**Scout MAP:** defaults to an **offline real-geometry central Boston street basemap** (10,634 source-derived segments, no fictional roads or landmarks), rendered in Rust-Boy's original phosphor style. **BOS OFF / BOS ON** hides or restores the reference; the source manifest identifies its dated municipal data. Coverage is partial, not live. [Boston basemap provenance](docs/BOSTON_BASEMAP.md) · [Lore naming](docs/LORE_NAMING.md).

**RustGIS import:** choose **RUSTGIS** to load `.rgismap`, `.rgis`, `.rgislyr`, `.rgismpk`, `.rgispkg` or GeoJSON. Native vector data remains primary; missing raster/streamed sources are reported. Browser GPS needs permission. **WEB ON** enables a separately labeled OpenStreetMap/Overpass reference overlay; provider reachability checks contain no coordinates. [Map/GPS guide](docs/SCOUT_GPS_MAP.md).

**Overseer TASKS:** create assignments by person/job, priority and due date; acknowledge, update and inspect history. Paired-device Send uses OpenSSH with a dedicated key, pinned host key and restricted receiver. Remote progress is retrieved explicitly. [Task Control](docs/TASK_CONTROL.md) · [Peer enrollment](docs/PEER_TASK_TRANSPORT.md).

**DIAG:** evaluate local host indicators or import bounded evidence associated with an authorized asset. Findings preserve provenance and unknown states. [Evidence schema and examples](docs/DIAGNOSTICS.md).

```bash
cargo run --locked -p pipboy -- diag host --json
cargo run --locked -p pipboy -- diag ingest examples/diag/network-example.jsonl --asset lab-router --confirm-authorized --json
cargo run --locked -p pipboy -- status --details --json
cargo run --locked -p pipboy -- notes add "Field observation"
```

Notes, task files and imported evidence may contain private data. Local stores are plaintext; avoid putting credentials in them. `PIPBOY_HOME` selects the data directory. [Storage](docs/STORAGE.md).

## Mac → PlayStation

```bash
./scripts/launch-playstation.command --discover-ps4
./scripts/launch-playstation.command 192.168.1.50 --diagnose
./scripts/launch-playstation.command 192.168.1.50
```

The installed game must expose the companion service. The client browses game data and sends protocol heartbeats; gameplay controls are not implemented. [Mac/PlayStation guide](docs/MAC_PLAYSTATION.md) · [Game mode](docs/GAME_MODE.md).

## Build and verify

```bash
cargo build --workspace --locked
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
python3 tools/validate_catalog.py
python3 tools/validate_roles.py
python3 tools/validate_rust_version.py
python3 tools/smoke_network.py
python3 tools/smoke_network_health.py
node tools/test_network_ui.cjs
```

CI runs Linux stable, exact Rust 1.90.0, native macOS, native Windows and manifest checks, with the broader runtime/API/terminal smoke suites. New network UI journeys cover consent, inventory pagination, refresh cancellation, hidden tabs, stale responses and safe labels. Test fixtures do not certify physical integrations. [Validation evidence](docs/VALIDATION.md) · [Contributing](CONTRIBUTING.md).

## Project layout and guides

| Location | Purpose |
| --- | --- |
| [crates/pipboy](crates/pipboy) | CLI, terminal UI and loopback dashboard with embedded assets |
| [crates/pipboy-core](crates/pipboy-core) | Telemetry, workbenches, tasks, diagnostics, bounded network adapters and simulation |
| [crates/rust-boy-broker](crates/rust-boy-broker) | Deny-only physical-device request prototype |
| [catalog](catalog) | Source references, literature matrix and module/role plans |
| [tools](tools) | Offline validators and controlled smoke/journey tests |
| [docs](docs/README.md) | Operator guides, architecture, roadmap and evidence |

[Micro HUD](docs/MICRO_UI.md) · [Desktop dashboard](docs/DASHBOARD_UI.md) · [Roadmap](docs/ROADMAP.md) · [Architecture](docs/ARCHITECTURE.md) · [Changelog](CHANGELOG.md) · [Security](SECURITY.md) · [Branding](BRANDING.md) · [MIT license](LICENSE).

Public brand: **Rust-Boy**. Compatibility names remain `pip_boy` (repository), `pipboy` (package/CLI) and `PIPBOY_HOME` (storage). No third-party game artwork or source is bundled.

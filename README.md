# Rust-Boy

**First-party CI:** the [Rust workflow](.github/workflows/rust.yml) uses zero external GitHub Action `uses:` dependencies, checks out the exact triggering SHA, and validates stable Rust, the 1.90 MSRV, both optional SDR features, and native macOS/Windows builds. The status badge tracks this development branch's PR checks. PR head `9d21447dd1b5c0dfcf646a70824fd6d9f023e18c` passed all seven checks in [run 40840855437](https://github.com/pskeffington/pip_boy/actions/runs/40840855437). Default `main` remains a separate failing badge until its workflow repair is eligible to merge; PR #71 stays draft while RB-CX-05 parity and operator/field gates remain open. This is not a fully offline build: Rustup, system packages and pinned Cargo dependencies still require their respective sources.


[![Rust-Boy quality gates](https://github.com/pskeffington/pip_boy/actions/workflows/rust.yml/badge.svg?branch=codex%2Fpackage-adapter-protocol)](https://github.com/pskeffington/pip_boy/actions/workflows/rust.yml?query=branch%3Acodex%2Fpackage-adapter-protocol)
[![License: MIT](https://img.shields.io/badge/license-MIT-informational)](LICENSE)
[![Rust: 1.90+](https://img.shields.io/badge/Rust-1.90%2B-informational)](Cargo.toml)

**Offline first. Native by design.**

Rust-Boy is a Rust workbench for local diagnostics, field maps, network engineering and device-to-device task assignments. It has a desktop dashboard, a phosphor-style micro HUD and a terminal console. Seven occupational profiles come preloaded; Custom starts empty.

**Development preview on `main`.** Implemented modules are merged after software checks pass. Physical Raspberry Pi touchscreen, actual UniFi controller, Mac-to-Pi SSH and PlayStation game acceptance remain separate field tests. There is no packaged production release.

Roles use short operator titles: **Overseer, Maintenance, Security, Doctor, Scientist, Reclamations, Recon, Archivist, Custom**. Security's unconfigured camera view offers labeled public publisher views for **DC, NYC, Boston and SF**; private IP-camera enrollment/decoding remains a separate gate. See [camera sources and lifecycle](docs/SECURITY_CAMERA_WALL.md).

**Planned host redesign:** [Single-package viewport and phased package gates](docs/HOST_PACKAGE_ROADMAP.md). The target shell displays one package at a time with permanent Settings; the existing desktop/micro interface remains in place until the acceptance gates pass.

**Universal adapter precursor:** `crates/rust-boy-adapter-sdk` defines a minimal shared module protocol; `pipboy-core::universal_adapter` hosts trusted built-in adapters, manages one foreground package and keeps Settings host-owned. Four synthetic adapters remain in tests; the development branch additionally mounts one explicitly compiled, read-only RADIO catalogue sourced from `rust_radio`. Its selection starts no playback, network, background task or device access. Real interactive package integration and external execution remain open. [Universal adapter guide](docs/UNIVERSAL_ADAPTER.md).

**Socket registry and transport precursor:** Four independent designs (MAP / TV / RADIO / COMMS) can be inspected and validated without loading packages. `pipboy packages list --json` shows checked-in reference designs; `pipboy packages check PATH` validates an explicit file; `pipboy packages scan DIR --json` discovers JSON manifests in an operator-chosen local directory. A private Unix listener helper checks directory and socket permissions and requires the OS-reported peer PID and UID to match caller-supplied expectations; it is not connected to a package launcher or binary-signature verifier. Registry results remain **reference/discovered metadata only**, not installed or runnable packages. See [socket contract](docs/PACKAGE_SOCKET.md).

## Project navigation

**Codex development:** [Root instructions](AGENTS.md) · [phased passes](docs/CODEX_PASSES.md) · [machine-readable queue](catalog/codex_passes.json) · [index ownership](docs/INDEXING.md). These development-branch sources do not certify production acceptance.


[Documentation index](docs/README.md) · [Repository structure](docs/REPO_STRUCTURE.md) · [Architecture](docs/ARCHITECTURE.md) · [Organizational research](docs/ORGANIZATIONAL_OPTIMIZATION.md) · [Validation](docs/VALIDATION.md) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md).

**Desktop focus:** The Vault Command Center separates **COMMAND / MODULES / VIEWPORT / MESSAGES / SETTINGS**, plus RADIO. Command manages mission/task assignment and explicit paired-device dispatch; the handheld `/micro` remains the field terminal. [Desktop guide](docs/DASHBOARD_UI.md).

**Common Vault organization:** [Default hierarchy and responsibility matrix](docs/VAULT_ORG_CHART.md) defines Overseer → Security/Recon, Engineering, Medical/Medic, Science, Reclamations and Administration. The [machine-readable positions](catalog/vault_org_chart.json) are design-only; actual users, permissions and keys require separate institutional enrollment.

**RB-to-RB fleet synchronization:** [Phased organizational/proximity/IP/keyed-pairing roadmap](docs/RB_SYNC_ROADMAP.md). Existing pinned OpenSSH task and Messenger links are manual; automatic discovery, organizational enrollment and background synchronization are **not implemented**. Regulated clinical data is outside the present release boundary. The [sync integration guide](docs/RB_SYNC_INTEGRATION.md) maps implemented source modules, interfaces, safe integration order, and verification commands; the [delta and ledger contract](docs/RB_SYNC_DELTA_PLANNER.md) covers local-only batching and locked snapshot writes.

## Quick start

Install [Rust and Cargo](https://www.rust-lang.org/tools/install) — **Rust 1.90 or newer** — and Git. Daily development uses stable Rust; CI also checks the exact 1.90.0 minimum. The workspace retains Rust edition 2021. Linux builds need a C toolchain, `pkg-config` and ALSA development headers. On Debian/Ubuntu:

```bash
sudo apt-get update
sudo apt-get install --no-install-recommends build-essential pkg-config libasound2-dev
```

Build the complete workspace from the locked dependency set, then verify the CLI:

```bash
cargo +stable build --workspace --locked
cargo +stable run --locked -p pipboy -- --help
```

The remote `main` branch at `629f34f0ce21e37c6d2bc6567919ccc5c862c80d` was extracted into a clean directory and both commands passed on macOS ARM64. The resulting binary also passed `python3 tools/smoke_micro.py`, which exercises the real loopback UI/API with temporary state and no WAN/device access. This verifies that source snapshot's build and local startup path; it does not certify hardware or field acceptance.

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

Micro navigation uses STATUS / [active role] / DATA / MAP / RADIO with one compact title/control header and Settings, a consistent Back button and separate local-tool input/results screens. Back returns through inputs before leaving a tool; temporary drafts survive navigation until reload.

Handheld tool pages adapt to available height: compact GEAR shows four tools, while DATA and the Custom palette reserve their action row and show two. Tool paging keeps keyboard focus; profile selection stays in SETUP.

Open the printed loopback URL in a browser. The dashboard binds only to 127.0.0.1. Keep it local.

Update an existing checkout from `main`; the features below no longer require switching among older preview branches:

```bash
git switch main
git pull --ff-only origin main
./scripts/launch-micro.command
```

## Native Mac MAP test bench

**Mac-first:** develop and test the single Rust workspace on macOS. The built Rust binary hosts the localhost desktop/micro MAP; the Fallout-style vector renderer remains browser JavaScript. No Raspberry Pi is required for this phase.

```bash
# In your cloned pip_boy directory, with Rust 1.90+, Python 3 and Node 22+:
bash scripts/test-map-macos.command --verify
bash scripts/test-map-macos.command --launch
```

`--verify` builds and checks real native Rust plus offline map/GPS/ingestion tests against a temporary loopback service; no live Census or OSM requests. `--launch` opens the native Rust micro HUD on a free localhost port with a **separate persistent Mac test home**, so cached counties can be verified after restarting offline without modifying `~/.pipboy`. County download remains an explicit Settings → Location / Map action. Optional Chromium device-emulation verification is available via `--verify --require-chromium`; Safari is supported for manual UI inspection. Settings → Location / Map → DATA SOURCES includes local-only source load and drawing timings. [Mac MAP operator checklist](docs/MAC_MAP_TESTING.md). Actual operator-Mac acceptance is distinct from GitHub's macOS CI runner.

After explicitly downloading a county in location settings, run `bash scripts/test-map-macos.command --audit-cache` to independently check the real saved boundary and ZIP contents through the **local Rust cache-only API**. It makes no Census, OSM or GPS requests and reports complete/partial/empty/corrupt status; missing optional sources are not treated as fabricated features.

## Available modules

| Module | Implemented behavior | Field acceptance still open |
| --- | --- | --- |
| Network Engineer | Private-LAN scan/watch; responder/cache counts; UniFi connected-client inventory and comparison; bounded ICMP/TCP health; nine offline calculators | Actual controller TLS/API access and authorized Mac/Pi LAN measurements |
| DIAG | Local CPU/RAM indicators and scoped, versioned NDJSON fault-evidence ingestion | Device-specific collectors and validated diagnostic rules |
| Overseer Task Control | Persistent assignments, role queues, status, due dates and revision-safe history | Human/organizational identity beyond local role labels |
| Peer task transport | Explicit paired-device OpenSSH send and receipt/status retrieval | Real Mac-to-Pi SSH pairing, interruption and key-rotation tests |
| Reclamations | Persistent manual-position survey sites, statuses, selectable offline map markers and GeoJSON export | Field GNSS, RustGIS full renderer, Pi touch acceptance and operational verification |
| Scout maps | Mac-local native Rust service + location-driven county map; Census TIGER roads, waters, verified-county landmarks, RustGIS/GeoJSON imports, explicit GPS/OSM | Mac operator/browser/GPS acceptance, upstream RustGIS renderer, raster/terrain and later Pi hardware |
| Host / Library / Notes | Local telemetry with quality labels, bundled references and offline notes | Production packaging and accessibility acceptance |
| Fallout 4 companion | Read-only TCP protocol browser, diagnostics and bounded observation | Actual game-build/PlayStation interoperability |
| Radio / SDR (in `main`; hardware acceptance pending) | Native in-process IP playback, 10 curated stations, favorites/order/last selection, Bluetooth-ready OS output chooser and a receive-only simulated SDR contract | Real SDR receiver/DSP, individual stream checks, Pi speaker/touch and physical Bluetooth failure acceptance |\n| Device simulation | Fixed fixtures, bounded queues and mock adapters | Physical GPIO, serial, SDR, BLE, mesh and camera integrations |

The separate `rust-boy-broker` prototype denies every physical-device request. Simulated success does not grant peripheral access.

## Radio / SDR development

**Status:** Native IP radio is integrated into `main` through [PR #35](https://github.com/pskeffington/pip_boy/pull/35), after all five pre-merge CI checks passed. The operator confirmed audible Skylab playback, Play/Stop, volume and OS-visible Bluetooth output selection on Mac. Rust-Boy uses Symphonia and CPAL inside the same native process; **no browser audio or external player** is launched. The selected [nRSP-ST uses the optional SDRconnect network adapter](docs/RADIO_NRSP_ST.md) for native PCM playback in RADIO as well as bounded ingress checks. The operator also confirmed radio audio on MacBook Air speakers through the system default output. USB receivers use the separate SoapySDR adapter; raw I/Q and Pi acceptance remain separate gates.

From the micro HUD open **RADIO**. Choose one of 10 publisher-referenced stations, press **PLAY** to begin an explicit HTTPS stream, and use **STOP** or volume controls. Tap **☆/★** to manage favorites, **ALL / ★ ONLY** to filter, and **SETTINGS** to select an OS-connected output or reorder favorites. A small local `radio-presets.json` file stores approved station IDs and the last selection—never audio, arbitrary URLs or an autoplay setting. Station listing and browsing do not start streaming.

```bash
cargo run --locked -p pipboy -- radio stations
cargo run --locked -p pipboy -- radio check skylab --confirm
cargo run --locked -p pipboy -- radio probe skylab --confirm
cargo run --locked -p pipboy -- radio validate skylab --confirm
cargo run --locked -p pipboy -- radio listen skylab --confirm --seconds 30
./scripts/launch-micro.command
node tools/test_radio_ui.cjs
node tools/test_radio_viewport.cjs  # Requires Chromium; synthetic local fixture
```

The radio implementation is consumed from the standalone `rust-radio` library at a pinned Git revision, with no GUI dependency or copied playback engine. The optional `soapy-sdr` feature adds explicit `radio sdr-devices --confirm` and bounded `radio sdr-capture` commands; the default build needs no SDR SDK. Host compatibility modules retain the existing Rust import paths, HUD and preference format. `radio validate STATION --confirm` silently verifies decoded PCM; a successful byte `probe` alone only establishes that response bytes were received. Stream endpoints and applicable listening terms still need release review; publisher inclusion does not grant commercial redistribution permission. The 480×320 real-Chromium regression verifies synthetic layout and no autoplay, **not** audible sound, real Bluetooth recovery or Raspberry Pi hardware. See [RADIO engineering contract](docs/RADIO_SDR.md) and [native-radio roadmap](docs/RADIO_NATIVE_ROADMAP.md).

## Network Engineer\n
Load **Head Engineer → NET ENGINEER** on desktop or micro. Both views use the same Rust operation catalog and shared tool component. The handheld separates tool selection, inputs, authorization and results; large inventories have pages.

**Handheld controls:** Network Engineer uses four tool tiles per page. Touch PREV/NEXT or use physical D-pad left/right (also PageUp/PageDown) to turn pages; up/down focuses controls in reading order. The browser's normal Enter/Space activates a focused control. Escape/Back returns to the prior screen. The focus ring remains visible without hover. Left/right arrow keys preserve caret editing in one-line inputs; up/down moves between fields and actions, while multi-line inputs and native selects keep their own arrow behavior. Opening a page or moving focus never starts a scan; active operations still require separate authorization. This is keyboard-emulated hardware support, not physical Raspberry Pi button acceptance. A Chromium layout acceptance script (`node tools/test_network_viewport.cjs`) now verifies the 480×320 screen and neighboring viewport sizes without ever probing a real network.

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

**Messenger / LINK:** [PACE routing](docs/PACE_MESSAGING.md) provides manual P/A/C/E selection, a saved private-IP alternate and persistent local holds. LINK shows tracked attempt counts, the last route and whether an exchange is unconfirmed, a receipt was rejected or storage was confirmed; interrupted attempts retain an unknown outcome. Paired OpenSSH authenticates the link, and only a validated durable receipt marks a message STORED. P/A share the IP network; C cannot send and E does not deliver. Default journals remain plaintext; fresh journals can opt into **age encryption at rest with an OS-stored queue key**. This is not E2EE. Live USB, physical interoperability and independent security review remain pending. [Build passes A–C](docs/MESSENGER_BUILD_PASSES.md). [Setup and limitations](docs/MESSENGER.md).

**Reclamation:** load Reclamation Scout and select **SITES** on micro. Add site observations with manual WGS84 coordinates, condition, priority, notes and position uncertainty; update status and export RFC 7946 GeoJSON. Records persist at `PIPBOY_HOME/reclamation.json` (default `~/.pipboy/reclamation.json`). **Settings → Location / Map → SITES** loads saved markers explicitly; selecting **MAP** on a site opens its location on the grid. WEB is blocked while saved sites are displayed, so site coordinates are not sent to Overpass. No background GNSS or uploads. [Field kit guide](docs/RECLAMATION.md).

**GPS troubleshooting:** a browser geolocation refusal now shows **GPS HELP** with site permission status and OS/browser guidance, while **CENTER** remains a manual fallback. [GPS troubleshooting](docs/SCOUT_GPS_MAP.md#gps-denied-despite-system-location-services-being-on).

**County MAP:** open **Settings → Location / Map → ZIP** (enter five digits → Census 2020 ZCTA → matching county → 2025 county GIS), or use GPS ON or manual CENTER → LOAD AREA in the same settings screen. Multiple intersecting counties require explicit choice; a postal ZIP/town is not guaranteed to correspond to one county. Reverse **COUNTY ZIPs** lists intersecting 2020 ZCTAs. [ZIP/County source guide](docs/ZIP_COUNTY_LOOKUP.md). The map resolves the containing Census county GEOID and pulls approved TIGER 2025 county-wide roads, areal water and streams with bounded downloads. A prior county package restores from the local cache without remote requests. [County source guide](docs/COUNTY_SOURCES.md).

**Scout MAP:** begins with an unlocated reference grid. The operator configures GPS, ZIP or manual coordinates under **Settings → Location / Map** to resolve a Census GEOID and acquire verified 2025 TIGER/Line road, water and county-clipped point-landmark geometry from a fixed source registry. Cached county geometry and county-clipped landmarks reload without internet. In location settings, expand LAYERS to independently show/hide official roads, waterways, landmarks and border; the renderer uses a bounded viewport index while preserving source geometries. No demonstration city or fictitious features are bundled. [County ingestion and provenance](docs/COUNTY_SOURCES.md) · [Lore naming](docs/LORE_NAMING.md).

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

### RADIO from the Rust-Boy HUD

Launch the micro HUD with `./scripts/launch-micro.command` or `cargo run --locked -p pipboy -- dashboard --micro`. Under **RADIO**, select a station and explicitly press **PLAY**; Play/Stop and volume control native Rust audio. **☆/★** saves favorites, **★ ONLY** shows user-ordered presets, and **SETTINGS** manages output devices and ordering. Selection survives restart without autoplay. On desktop, select **VIEWPORT → RADIO / IP** to open the same native session and local preferences; it is not shown as an extra scrolling page. Mac speaker/Bluetooth playback has been demonstrated; expanded station availability and Pi hardware acceptance remain open.

# Rust-Boy documentation

Technical contracts, architecture decisions, research provenance and validation evidence for the [Rust-Boy](../README.md) offline-first field console.

**Current scope:** Rust 1.90+ desktop CLI/TUI, a separate micro-handheld shell and a loopback-only graphical dashboard, bundled reference library, notes and synthetic-only device workflows. NET ENGINEER performs an explicitly authorized, bounded LAN scan; OS-enforced peripheral permissions and physical device permissions remain separate; a TLS-only, explicitly configured UniFi connected-client reader is implemented. Physical controller acceptance remains open.

## Start here

| Guide | Focus |
| --- | --- |
| [Network health and UniFi setup](NETWORK_HEALTH_UNIFI.md) | Unified 16-tool catalog, bounded ICMP/TCP tests, controller configuration and view-owned refresh |
| [NET ENGINEER LAN diagnostics](NETWORK_ENGINEERING.md) | Explicit private-subnet scan, ICMP/TCP RTT, bounded monitor and device-count limitations |
| [DIAG offline ingestion](DIAGNOSTICS.md) | Scoped fault-evidence JSONL intake and local host heuristics |
| [Peer task transport](PEER_TASK_TRANSPORT.md) | Explicit OpenSSH Mac/Pi enrollment, restricted bridge, delivery and receipt sync |
| [Overseer Task Control](TASK_CONTROL.md) | Persistent assignments, status, manual handoff and role queues |
| [Micro handheld shell](MICRO_UI.md) | Minimalist paged six-tab HUD, 480×320 test and compact Task Control |
| [Graphical dashboard](DASHBOARD_UI.md) | Run local browser UI on Mac, load role templates, test offline tools and Custom |
| [Quick start](../README.md#quick-start) | Clone, build, run the terminal UI and inspect local status |
| [Console controls](CONSOLE.md) | Keyboard navigation, resize behavior and terminal restoration |
| [RustGIS-native map and GPS](SCOUT_GPS_MAP.md) | Native RustGIS map import, GPS, optional Overpass road reference, privacy and provenance |
| [Scout GIS + TAK on Pi](SCOUT_GIS_TAK.md) | Offline GeoJSON, RustGIS ARM64 roadmap, Pi TAK server and CoT integration gates |
| [Role roadmap](ROLE_ROADMAP.md) | Eight occupational workbenches, operational needs and R0-R3 acceptance gates |
| [Lore/source matrix](VAULT_TEC_LORE_MATRIX.md) | Publisher-source evidence, role name fidelity and technical standards |
| [Security camera wall](SECURITY_CAMERA_WALL.md) | Four-feed 2×2 view, one-key focus, camera authorization and media acceptance |
| [Architecture](ARCHITECTURE.md) | Workspace, lifecycle, local boundaries and adapter layering |
| [Roadmap](ROADMAP.md) | G0–G4 priorities, planned modules and acceptance gates |
| [Validation](VALIDATION.md) | Executed tests, exact CI runs, limitations and historic evidence |
| [Changelog](../CHANGELOG.md) | Current implemented scope and remaining work |

## Subsystems and contracts

| Topic | Documents |
| --- | --- |
| Mac → PlayStation | [Launcher, setup and protocol diagnostics](MAC_PLAYSTATION.md) |
| Reclamation Scout GIS | [RustGIS and Raspberry Pi / TAK architecture](SCOUT_GIS_TAK.md) |
| Fallout 4 | [Game mode setup and protocol](GAME_MODE.md) |
| Task management | [Task Control, local persistence and manual dispatch](TASK_CONTROL.md) |
| Micro handheld | [Launch, controls, themes, safety and acceptance](MICRO_UI.md) |
| Graphical UI | [Dashboard launcher, API, responsive viewport and test plan](DASHBOARD_UI.md) |
| Local host and UI | [Host telemetry and field quality](HOST_TELEMETRY.md) · [Object model](OBJECT_MODEL.md) · [Scoped local operations](LOCAL_OPERATIONS.md) · [Console](CONSOLE.md) |
| Data and persistence | [Storage](STORAGE.md) · [Permission preferences](PERMISSIONS.md) |
| Synthetic devices | [Simulator](SIMULATION.md) · [Virtual transport](VIRTUAL_TRANSPORT.md) · [Typed mock adapters](MOCK_ADAPTERS.md) |
| Physical-device safety | [Deny-only broker protocol](BROKER_PROTOCOL.md) · [Future OS-enforced broker requirements](OS_DEVICE_BROKER.md) |
| Implementation planning | [G1 execution plan](G1_EXECUTION_PLAN.md) · [Module catalog](MODULE_CATALOG.md) · [Research literature matrix](LITERATURE_MATRIX.md) |
| External references | [Research library](RESEARCH_LIBRARY.md) · [Source catalog](../catalog/README.md) |
| Build diagnostics | [CI historical investigation](CI_TRIAGE.md) · [GitHub Actions workflow](../.github/workflows/rust.yml) |

## Reading status labels

- **Available** means a source implementation exists and has applicable CI tests.
- **Test-only** means fixed, labeled synthetic data and in-memory/mock behavior, not real input or measurement.
- **Deny-only** means the separate broker rejects every physical-device request; it has no real authorization ability.
- **Planned** means acceptance/design work remains and no deployable physical capability is claimed.

The machine-readable `implemented_unverified` catalog label is a **module acceptance state**, not an assertion that CI did not execute. A passing test run does not automatically complete accessibility, release or hardware gates.

## Policies

[Security](../SECURITY.md) · [Contributing](../CONTRIBUTING.md) · [Branding](../BRANDING.md) · [License](../LICENSE).

The product name is **Rust-Boy**; the existing `pip_boy` repository, `pipboy` CLI and `PIPBOY_HOME` storage variable remain compatibility interfaces. Keep implementation status explicit, preserve data provenance, and obtain separate rights clearance before copying third-party software or art.

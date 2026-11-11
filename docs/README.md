# Rust-Boy documentation

- [Mac release path](MAC_RELEASE.md) — menu-bar app bundle, local validation, Developer ID signing/notarization, and current release blockers.
- [Mac release task queue](../catalog/mac_release_tasks.json) — app packaging, signing, installation, update, and acceptance gates.

Technical contracts, architecture decisions, research provenance and validation evidence for the [Rust-Boy](../README.md) offline-first field console.

**Current scope:** Rust 1.90+ desktop CLI/TUI, a separate micro-handheld shell and a loopback-only graphical dashboard, bundled reference library, notes and synthetic-only device workflows. NET ENGINEER performs an explicitly authorized, bounded LAN scan; OS-enforced peripheral permissions and physical device permissions remain separate; a TLS-only, explicitly configured UniFi connected-client reader is implemented. Physical controller acceptance remains open.

## GitHub-first agents and task queue

**Current RB-G5 agent packet:** [2026-11-09 post-consolidation assignments](RBG5_AGENT_TASKING_2026-11-09.md). This supersedes earlier work-order priority without retroactively changing earlier CI or hardware acceptance evidence.

[Agent instructions](../AGENTS.md) · [Execution work packets and release gates](TERMINAL_AGENT_EXECUTION.md) · [Machine-readable task queue](../catalog/terminal_agent_tasks.json) · [Core dispatcher issue](https://github.com/pskeffington/pip_boy/issues/73) · [Security/CI issue](https://github.com/pskeffington/pip_boy/issues/74). Software implementation is developed and validated through GitHub; operator-local side-by-side testing is not a prerequisite.

[November 3 execution packet and post-packet reconciliation](CODEX_PASS_2026-11-03.md) records the #88/#92/#93/#96 merges, current CI evidence, unresolved drafts, and Pi 5 / Meshtastic physical acceptance boundaries.

## Terminal module command integration

[Terminal-to-module command surface](TERMINAL_MODULE_COMMANDS.md) documents TC-A01 through TC-A05. PR #88 merged the CLI/TUI bridge; PR #92 merged the shared desktop/micro palette at `22c43f8`. Pi hardware evidence remains pending.

## Meshtastic communications

[Meshtastic interoperability record](MESSENGER_INTEROPERABILITY.md) and [machine-readable queue](../catalog/meshtastic_tasks.json) distinguish the merged recorded-capture decoder from live USB/LAN reception. PR #96 merged at `ee04da5`; its exact head passed 7/7 jobs in [Actions run 41958732527](https://github.com/pskeffington/pip_boy/actions/runs/41958732527). PR #99 accepted the LAN receive design only (7/7 in [Actions run 41963337334](https://github.com/pskeffington/pip_boy/actions/runs/41963337334)). The operator-verified web UI root does not identify an enabled receive service. MESH-07 now has an accepted offline scope/freshness contract (7/7 in [Actions run 41965952404](https://github.com/pskeffington/pip_boy/actions/runs/41965952404)); live broker, socket, and hardware acceptance remain pending.

## Universal adapter SDK

[Universal adapter implementation and four-module integration](UNIVERSAL_ADAPTER.md) documents the portable SDK, trusted host lifecycle manager, one-package viewport contract and open acceptance gates.

## Package socket and GIS extraction

[Pop-in package socket and GIS extraction](PACKAGE_SOCKET.md) defines the host-owned viewport boundary, package manifest, and independent rb_gis migration gates.

## Host package roadmap

[Single-package viewport and phased host implementation gates](HOST_PACKAGE_ROADMAP.md) is the canonical target design; the existing desktop and micro interfaces remain active until their replacement gates pass.

## Raspberry Pi 5 field evidence

[Pi 5 hardware test baseline](https://github.com/pskeffington/pip_boy/blob/docs/longterm-codex-roadmap-20261031/docs/PI5_REFERENCE_TEST_BASELINE.md) defines PI5-00–PI5-08 in pending PR [#89](https://github.com/pskeffington/pip_boy/pull/89). The [Codex smoke checklist and evidence template](PI5_CODEX_SMOKE.md) prepares repeatable HDMI/keyboard/mouse runs; it does not record a physical result. All Pi field acceptance remains pending until actual device evidence is attached.

## Start here

[Security review: data leakage and credential boundaries](SECURITY_REVIEW_2026-10-26.md) documents the current privacy audit, confirmed controls and residual risks.


| Guide | Focus |
| --- | --- |
| [Repository structure and maintenance](REPO_STRUCTURE.md) | Canonical file ownership, code boundaries, documentation navigation and cleanup gates |
| [Organizational optimization research](ORGANIZATIONAL_OPTIMIZATION.md) | Measured code concentration, evidence matrix, component boundaries and ranked improvements |
| [Mac-first / Pi-later architecture decision](adr/0001-mac-first-portable-core.md) | Accepted single-workspace strategy and separate hardware acceptance gates |
| [Mac-local MAP testing](MAC_MAP_TESTING.md) | Native Cargo launcher, offline tests, real county acquisition, cache restart and local render timing |
| [Network health and UniFi setup](NETWORK_HEALTH_UNIFI.md) | Unified 16-tool catalog, bounded ICMP/TCP tests, controller configuration and view-owned refresh |
| [NET ENGINEER LAN diagnostics](NETWORK_ENGINEERING.md) | Explicit private-subnet scan, ICMP/TCP RTT, bounded monitor and device-count limitations |
| [DIAG offline ingestion](DIAGNOSTICS.md) | Scoped fault-evidence JSONL intake and local host heuristics |
| [Common Vault organizational chart](VAULT_ORG_CHART.md) | Overseer hierarchy, Security/Recon, Engineering, Medical/Medic, Science, Reclamations, Administration, and authority boundaries |
| [RB <-> RB organizational synchronization roadmap](RB_SYNC_ROADMAP.md) | Trusted proximity discovery, IP/key pairing, scoped fleet enrollment, delta sync and healthcare release gates (design only) |
| [RB sync integration and boundaries](RB_SYNC_INTEGRATION.md) | Source entrypoints, compatibility, testing, and authorization/transport integration gates |
| [RB sync v2 offline delta planner](RB_SYNC_DELTA_PLANNER.md) | Bounded cursor selection, source regression cases, and security gates (not connected to live sync) |
| [Peer task transport](PEER_TASK_TRANSPORT.md) | Explicit OpenSSH Mac/Pi enrollment, restricted bridge, delivery and receipt sync |
| [Overseer Task Control](TASK_CONTROL.md) | Persistent assignments, status, manual handoff and role queues |
| [Approved Rust-Boy character animation](AVATAR.md) | Source-derived phosphor CRT sprite atlas, native Rust frame sampler, and isolated local preview |
| [Micro handheld shell](MICRO_UI.md) | Universal location settings, single-row header, five universal sections, Rust-owned module categories and height-aware tool pages, touch pagers and compact Task Control |
| [Graphical dashboard](DASHBOARD_UI.md) | Run local browser UI on Mac, load role templates, test offline tools and Custom |
| [Native RADIO and station presets](RADIO_NATIVE_ROADMAP.md) | In-process IP playback, favorites, output selection, 480×320 screen checks and hardware release gates |
| [Quick start](../README.md#quick-start) | Clone, build, run the terminal UI and inspect local status |
| [Console controls](CONSOLE.md) | Keyboard navigation, resize behavior and terminal restoration |
| [Lore naming and tool crosswalk](LORE_NAMING.md) | Six-tab HUD names, role/tool alignment and capability boundaries |
| [County source acquisition](COUNTY_SOURCES.md) | Allowlisted TIGER county roads/water plus polygon-clipped state landmarks and offline cache |
| [ZIP/ZCTA ↔ county GIS lookup](ZIP_COUNTY_LOOKUP.md) | Five-digit ZIP lookup, ambiguous counties, reverse county ZIP index, offline and privacy boundaries |
| [RustGIS-native map and GPS](SCOUT_GPS_MAP.md) | Native RustGIS map import, GPS, optional Overpass road reference, privacy and provenance |
| [PACE messaging and route setup](PACE_MESSAGING.md) | Manual P/A routes, persistent wait/receipt evidence, blocked C transmission, local E hold and shared failure domains |
| [Messenger build passes A–C](MESSENGER_BUILD_PASSES.md) | Wire contract, fault simulation, opt-in encrypted queue, local-IP retry and passive USB capture |
| [Messenger hardware acceptance](MESSENGER_INTEROPERABILITY.md) | Pending native key-store, two-host and USB field evidence |
| [Messenger: authenticated device-to-device text](MESSENGER.md) | LINK conversation UI, SSH pairing, delivery receipts, and explicit security limitations |
| [Reclamations](RECLAMATION.md) | Local manual site register, offline markers, GeoJSON export and privacy controls |
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
| Vault organizational model | [Common reporting hierarchy and position IDs](VAULT_ORG_CHART.md) · [Machine-readable chart](../catalog/vault_org_chart.json) |
| Fleet and peer synchronization | [Org-bound RB synchronization (planned)](RB_SYNC_ROADMAP.md) · [Existing peer transport (implemented)](PEER_TASK_TRANSPORT.md) · [Offline sync core (experimental)](RB_SYNC_INTEGRATION.md) |
| RADIO / SDR | [Receiver and curated stations](RADIO_SDR.md) · [Native player and acceptance passes](RADIO_NATIVE_ROADMAP.md) |
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


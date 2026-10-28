# Rust-Boy host and single-package viewport roadmap

Status (2026-10-28): **architecture accepted; read-only manifest registry source committed; activation/UI gates open**. This document describes target behavior, not a claim that dynamic package activation, approved installation, or isolated containers exist.

### Current gate evidence

- **HOST-REG-1 / SOCKET-0, implementation candidate:** strict manifest parser in `crates/pipboy-core/src/package_socket.rs`; read-only CLI `pipboy packages list|check|scan`; loopback-only `GET /api/packages`; four indexed design fixtures, Rust unit tests, and offline Python validation.
- **Not yet accepted:** A GitHub Actions validation run returned `startup_failure` before any jobs ran; Rust compilation, `cargo fmt`, Clippy, tests, and Mac/Pi integration remain unverified. The read-only registry is intentionally not the executable package loader.
- **HOST-UI-1, pure-state precursor only:** `ForegroundSelection` models exactly one selected package, host Settings, and rejection of packages absent from a separately approved installed roster; Rust tests use synthetic approval only. The desktop/micro shell is not yet bound to this controller.
- **ADAPTER-SDK-1 / ADAPTER-HOST-1, implementation candidate:** `crates/rust-boy-adapter-sdk` is a lightweight portable module protocol crate; `pipboy_core::universal_adapter` implements trusted adapter mounting, lifecycle, scoped UI intents, bounded views, one foreground, Settings recovery and explicit unmount. Rust tests emulate all four module identities. [Adapter integration guide](UNIVERSAL_ADAPTER.md). CI and physical integration remain open.
- **Next priority:** bind host-owned foreground selection and always-available Settings to desktop/micro rendering, then real adapters for MAP / TV / RADIO / COMMS. No module is activated merely because it appears in the reference registry.


## Non-negotiable host contract

Rust-Boy is the host shell, identity/policy boundary, capability broker, and networking coordinator. Radio, rusti_rc, GIS, stream/video, and later packages retain independent ownership and release lifecycles.

- Exactly **one foreground package** occupies the workspace at any moment, on desktop and handheld alike. No simultaneous split view, persistent inspector, second package overlay, or module-specific masthead.
- A **fixed compact package switcher** and **always-available Settings control** are host-owned and outside the package canvas. Settings is a host view, not a package. In Settings the active package can be suspended visually without necessarily stopping its explicitly authorized background service.
- Keep the navigation minimal: compact single row, with bounded overflow/selection for installed packages rather than squeezing more than the available width. On short displays preserve usable touch targets and do not introduce document-level scroll bars.
- Each package fills the remaining safe viewport and adapts to size. Host does not impose package-specific control panels or map overlays. Package-owned internal scrolling is allowed only where necessary; no host document scroll.
- Role and organizational policy filter visibility and capabilities but do not alter the shell contract. Role selection and configuration remain in Settings.
- Foreground selection never implicitly grants a capability, connects hardware, sends messages, performs active discovery, downloads data, or changes role. External access requires explicit policy and, where appropriate, user confirmation.
- Background messaging, audio and emergency monitoring may continue only through declared background capabilities, lifecycle and authorization rules. Emergency services must not depend on which package is visible.
- Third-party packages are not assumed trustworthy. In-process Rust crates are not security containers: deploy an IPC process/sandbox boundary before permitting untrusted code.

## Socket extraction decision (2026-10-28)

The reusable **pop-in package socket** is specified in [PACKAGE_SOCKET.md](PACKAGE_SOCKET.md), with a non-executable [GIS manifest fixture](../catalog/host_packages/rb_gis.example.json). The MAP implementation is being extracted into [rb_gis](https://github.com/pskeffington/rb_gis). Gate ordering: SOCKET-0 -> GIS-EXTRACT-1/2 -> trusted GIS-EXTRACT-3 -> process/socket security and eventual legacy removal. Do not delete existing map logic before behavioral parity and rollback evidence.

## Lifecycle and package contract

Each registered package has an immutable identifier, display title, semantic package version, host API compatibility range, entrypoint type, capability requests, background-behavior declaration, supported viewport sizes, and human-readable source/license metadata. A signed distribution/verification policy is a separate release gate.

State machine: `discovered -> validated -> installed -> disabled|enabled -> foreground|background -> suspended -> stopped`. Invalid/unsupported modules are never activated. Crash -> fail-closed state, visible diagnostics, controlled restart, never a silent permission escalation. Maintain one authoritative active-package ID; atomically switch foreground ownership; preserve background service policies independently.

Suggested interface boundaries (design, not committed code): `PackageManifest`, `PackageRegistry`, `PackageLifecycle`, `HostCapabilityBroker`, `HostViewport`, `PackageEvents`. Prefer a typed versioned IPC protocol for external packages. Keep HTML/JS renderer details and network credentials outside portable package manifests.

## Phased implementation gates

| Gate | Work package | Acceptance evidence | Dependency |
| --- | --- | --- | --- |
| HOST-0 | Inventory existing UI routes, module catalog, core host and broker; document existing behavior | Mapping of legacy COMMAND/MODULES/VIEWPORT/MESSAGES/RADIO and micro tabs to package IDs; no feature loss list | None |
| HOST-UI-1 | Single foreground state machine and host-owned compact switcher plus gear Settings | One canvas, one active package, persistent Settings at desktop/micro resolutions; role selection only in Settings; no root scrolling | HOST-0 |
| HOST-UI-2 | Package-focused layout and responsive touch controls | Automated 480x320, 800x480, 1024x768, 1440x900 checks; map/stream use available canvas; controls accessible by keyboard/touch | HOST-UI-1 |
| HOST-REG-1 | Stable machine-readable package manifests and registry | Missing/bad/incompatible/duplicate packages rejected deterministically; ordered list drives switcher; schema fixtures validated offline | HOST-0 |
| HOST-LIFE-1 | Load/suspend/resume/stop adapter contract | Atomic switching; fault injection; no cross-package UI leak; explicit background continuation tests | HOST-UI-1, HOST-REG-1 |
| HOST-SEC-1 | Deny-by-default capability requests and persisted approvals | Network/peripheral/storage requests denied by default; backend checks independently of UI; audit evidence; revocation enforced | HOST-REG-1 |
| HOST-ADAPT-1 | Adopt independently maintained `rust_radio` | Host opens Radio as one package; audio continues or stops per declared lifecycle; output switching remains package-owned | HOST-LIFE-1, HOST-SEC-1 |
| HOST-ADAPT-2 | Adopt `rusti_rc` as Comms package | No implicit connect/send; authenticated message transport tested; background receipt explicitly authorized | HOST-LIFE-1, HOST-SEC-1 |
| HOST-NET-1 | Brokered module event transport and RB-to-RB integration | Offline/local IP/paired-device behavior with retries, bounded queues, replay controls; no unauthorized discovery; desktop-field interoperability | HOST-SEC-1 and existing sync security gates |
| HOST-DIST-1 | Versioning, signed installation and rollback | Authenticity, integrity, ABI/API compatibility and downgrade tests; compromised package revocation test | HOST-REG-1, HOST-SEC-1 |
| HOST-ISO-1 | Separate-process / OS sandbox or WASM isolation for third-party modules | Crash isolation, privilege isolation, resource quotas, platform-specific enforcement tests | HOST-DIST-1 |
| HOST-REL-1 | Mac desktop / Raspberry Pi hardware acceptance | Real-device display, disconnect, recovery, restart, input and performance evidence; security review and operator signoff | All relevant prior gates |

## Build order and safety

Ship **HOST-0 -> HOST-UI-1 -> HOST-UI-2** first to remove clutter without introducing an untrusted module loader. In parallel, draft registry fixtures for HOST-REG-1. Then implement lifecycle/policy contracts before external adapters. Add RB-to-RB transport only after existing organizational trust-root, anti-replay, revocation and physical-device security gates pass.

Preserve current CLI/TUI, loopback-only dashboard, operational data paths, explicit SSH task transport, map cache workflow, and existing Rust radio functionality while refactoring. No new public listener, automatic hardware connection, network scan, or package download as a side effect of navigation.

## Validation matrix

- **UI:** exactly one package visible; Settings reachable from every package and error state; background service status exposed without overlays; viewport has no document scroll; keyboard and touch navigation; state restoration after Settings and reload.
- **Registry:** duplicate IDs, malformed JSON, incompatible host version, absent entrypoint, missing capability, disabled package, corrupted manifest.
- **Lifecycle:** rapid switching, crash during activation, background audio/IRC behavior, host restart, shutdown and suspend.
- **Security:** deny-all defaults, scoped grants, revocation, audit logging, cross-role access, credentials never exposed to package UI.
- **Networking:** offline start, intermittent links, bounded queues, replay/reorder, authenticated peer, revocation while offline and eventual reconciliation.
- **Target systems:** macOS desktop first; Raspberry Pi touchscreen and actual connected radio/peripheral separately accepted.

## Documentation and indexing contract

This is the canonical implementation sequencing document. Existing [architecture](ARCHITECTURE.md), [module catalog](MODULE_CATALOG.md), [object model](OBJECT_MODEL.md), [dashboard UI](DASHBOARD_UI.md), [micro UI](MICRO_UI.md), [sync roadmap](RB_SYNC_ROADMAP.md) and [security](../SECURITY.md) continue to describe currently implemented and separately gated work. When code lands, update these documents, the root README, machine-readable catalog and CI acceptance tests in the same change. Do not mark a gate complete merely because documentation exists.

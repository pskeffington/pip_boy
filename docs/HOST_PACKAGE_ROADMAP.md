# Rust-Boy host and single-package viewport roadmap

Status (2026-10-30): **RM-018 remains in progress; desktop and handheld switching/input routes are wired, and the supervised launcher passes exact-head hosted CI, but production integration is blocked on a reviewed release artifact**. The source-controlled executable and manifest roster is empty and startup mounts no external modules. This document describes target behavior, not a claim that dynamic package installation or OS isolation exists.

### Current gate evidence

- **HOST-REG-1 / SOCKET-0:** strict manifest parser in `crates/pipboy-core/src/package_socket.rs`; read-only CLI and loopback API; five indexed design fixtures. Catalog validator and Rust tests pass locally. The registry remains metadata; executable trust comes only from the separate compiled approval roster.
- **ADAPTER-SDK-1 / ADAPTER-HOST-1:** trusted adapter manager and five synthetic package identities pass local Rust tests. `/api/packages/select` and `/api/packages/intent` connect the desktop and micro shells to the manager. Selection can resolve only a package in the compiled host roster; the route rejects non-`ui.*` intents and the package view is rendered as inert text and buttons.
- **RM-018 supervisor, locally implemented / acceptance open:** `approved_package_executables.json` is the only launch roster; each entry pins an executable and manifest beneath `$PIPBOY_HOME/packages/<package-id>` (default `~/.pipboy/packages/<package-id>`), outside signed application bundles. The reviewed staging command copies only the pinned files and cannot alter approval. Selection resolves those fixed relative paths, verifies both SHA-256 values and manifest identity, then directly spawns the reviewed binary. Temporary-install tests cover success, changed manifest bytes and symlinks. The runner applies 2-second control and 30-second scan deadlines, kills and reaps the timed-out direct child, and releases its audio lease only after child exit/reaping. Failure-path child fixtures exercise timeout, unexpected exit, shutdown and manager drop. This is fault containment, not an OS sandbox or process-tree boundary.
- **HOST-AUDIO-1, partial:** `audio_focus.rs` provides one non-preemptive host lease. The native Radio path acquires it before opening/replacing output; `UniversalAdapterHost` reserves the same broker for RADIO, MUSIC and TV before activation/resume. A successful suspend or shutdown releases the reservation. Failed lifecycle cleanup retains it; supervised-process exit releases it after reaping. The broker is in-process only; actual MUSIC/TV engines remain synthetic.
- **Local verification (2026-10-30):** Rust stable and 1.90.0 each passed 293 core tests with the sandbox-blocked loopback TCP test excluded, and 75 `pipboy` tests with three socket-binding tests excluded. Both SDR feature suites passed 75 tests per toolchain with those same three tests excluded. Warnings-denied Clippy, formatting, host-package/component/CI validators and the micro UI test passed. The unfiltered socket tests require loopback permissions unavailable in this sandbox; hosted CI must verify them.
- **Exact-head hosted validation:** source commit [`d15247c0ee95663dd3b731db6716a42e6dc4e04f`](https://github.com/pskeffington/pip_boy/commit/d15247c0ee95663dd3b731db6716a42e6dc4e04f) passed **7/7 jobs** in [Actions run 41124747110](https://github.com/pskeffington/pip_boy/actions/runs/41124747110): catalog/security, stable Rust, Rust 1.90, macOS, Windows, SoapySDR and SDRConnect. Stable CI includes desktop and handheld UI regressions and the unfiltered workspace tests. This verifies source behavior, not production artifact approval or physical-device operation.
- **HOST-UI-1, partial:** the handheld and desktop MODULES stages show host mount state and can select mounted adapters. Selecting a non-mounted identity may resolve and mount it only through the trusted host roster and fixed per-user install directory; the UI cannot provide paths, arguments or approval. Both shells render bounded view data and route UI selection through the host intent endpoint. No package currently mounts because the approved roster is empty.
- **Production blockers:** review and pin actual release executables and manifests; run staging and live select/input through those real artifacts; integrate real module builds (including rust_tv/rusti_rc) without absorbing their repositories; complete target-platform package acceptance and remaining viewport/input review; separately assess process-tree and OS isolation.
- **Next priority:** add the first independently built module as a reviewed pinned release artifact, then prove live startup, switch, input, timeout recovery and audio cleanup on target CI/devices. Reference catalog presence alone never activates a module.


## Non-negotiable host contract

Rust-Boy is the host shell, identity/policy boundary, capability broker, and networking coordinator. Radio, rusti_rc, GIS, stream/video, and later packages retain independent ownership and release lifecycles.

- Exactly **one foreground package** occupies the workspace at any moment, on desktop and handheld alike. No simultaneous split view, persistent inspector, second package overlay, or module-specific masthead.
- All audio-producing packages use the same host-owned, non-preemptive output lease. A package must stop and release it before another module can open output; selection never steals an active stream.
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
| HOST-ADAPTER-API-1 | Portable SDK and [machine-readable protocol](../catalog/host_adapter_contract.json) | v1 bounded wire roundtrip, five reference IDs, no hardware/OS dependencies; local tests pass | HOST-REG-1 |
| HOST-ADAPTER-CORE-1 | Trusted first-party universal adapter host | Five synthetic module identities, one foreground, persistent Settings, unmount, fail-closed replies; handheld and desktop select/input routes wired. Live process bootstrap remains open. | HOST-ADAPTER-API-1 |
| HOST-AUDIO-1 | Shared Radio/TV/Music output ownership | One RAII broker; host reserves audio identities before activation, releases only after acknowledged suspend/shutdown, and retains claims after failed cleanup; native Music/TV output integration still required | HOST-ADAPTER-CORE-1 |
| HOST-LIFE-1 | Bind adapter lifecycle to actual desktop and handheld views | Atomic switching, fault injection, no cross-package UI leak, explicit background continuation | HOST-UI-1, HOST-ADAPTER-CORE-1 |
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

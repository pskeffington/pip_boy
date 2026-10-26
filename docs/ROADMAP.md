# Rust-Boy | Evidence-based development roadmap

## Mainline integration status — 2026-10-25

- LAN/UniFi connected-client inventory and DIAG ingestion are merged in PR #15; Linux stable, Rust 1.90, native macOS/Windows and catalog validation passed at `badabc5`.
- The consolidation pass brings nine offline network calculators and bounded ICMP/TCP health into the existing desktop/micro Network Engineer, with a shared paged interface and view-owned refresh. All five code-quality jobs passed at `f77df8c` in run 40217917079. Mainline docs replace obsolete per-feature checkout instructions.
- Next field gates: real UniFi certificate/key access and inventory comparison, authorized Mac LAN measurements, physical 480×320 Pi touch layout, paired-device SSH acceptance and actual PlayStation/game protocol compatibility. These remain independent of software integration.

**Current milestone:** G0 console verified; G1 local core in progress; G2 deny-only broker precursor · **Review:** 2026-10-25  
**Release condition:** No phase is complete solely because its files or test definitions exist. CI execution, platform checks, and documented acceptance results must be verified.

Rust-Boy is the public product name. The existing `pip_boy` repository, `pipboy` executable, `pipboy-core` crate and `PIPBOY_HOME` data interface remain stable.

> **Verified integration baseline:** [Five-job Linux/macOS/Windows CI](https://github.com/pskeffington/pip_boy/actions/runs/40164915215) passed at `e7d1090`, including macOS PTY, Windows ConPTY, and the strict deny-all broker suite. **This does not complete a packaged release or enable physical devices.** The catalog's `implemented_unverified` status denotes a pending module acceptance review, not an absence of CI execution.

## PACE message routing

[PACE routing](PACE_MESSAGING.md) now exposes per-peer plans and an explicit LINK
selector. Primary uses paired SSH, Alternate uses a saved private-IP route with
the same pinned identity, Contingency remains receive-only and Emergency is a
local queue hold. There is no automatic failover or claim of four independent
bearers. Local route changes/holds are verified without transport calls; actual
hardware, independent review and key-lifecycle gates remain open. Queued delivery
now persists tracked attempt counts, last P/A route, local holds and unconfirmed
exchange/rejected receipt evidence. Interrupted attempts remain unknown until an
explicit retry; local status inspection never resumes delivery automatically.

## Messenger immediate build passes A–C

The [canonical build contract](MESSENGER_BUILD_PASSES.md) now governs message
envelopes, transport receipts and deterministic loss/delay/duplication/reordering
simulations. The existing SSH path uses the shared interface. Fresh journals can
opt into age encryption with an OS-stored queue key. Explicit local-IP retry
preserves the enrolled SSH host identity. Passive Meshtastic framing operates on
recorded captures; the physical serial broker remains deny-only.

**Open gates:** independent crypto/integration review, native key lifecycle and
plaintext migration, actual two-host local-IP validation, OS-enforced serial
access and [physical interoperability evidence](MESSENGER_INTEROPERABILITY.md).
These implementations do not complete E2EE or physical mesh messaging.

## G1 compact micro HUD — four-inch acceptance

The handheld primary dashboard, role selection and tool lists now use concise, paginated screens rather than scrollable long-form panels. Task Control has staged entry, two-task pages, and separate detail actions. Local lab input/results are distinct. Peer send/sync remain explicitly confirmed actions; camera and RustGIS renderers remain disconnected placeholders. [Micro UI guide](MICRO_UI.md).

**Pending acceptance:** manually verify Safari/Firefox at 480×320 and 800×480, including text truncation, touch targets, keyboard/ESC behavior, modal paging, focus at zoom and 4-inch Pi hardware. At extreme sizes, allow detail scrolling rather than clipping required controls.

## G1 explicit Rust-Boy device-to-device task link

The new [peer task transport](PEER_TASK_TRANSPORT.md) introduces enrolled device labels, validated peer destinations and **explicit** host-key-pinned OpenSSH task delivery between two Rust-Boy installations. A receiver durably imports the job in its own task file; on-demand SSH reports convey operator acknowledgment and progress back without silently overwriting the manager's status. The task UI replaces the old email draft with a peer-device picker. No auto-connect, native mesh/radio, background sync or PS4 integration is implied.

The test's local SSH substitute covers typed sender/receiver behavior, but real Mac-to-Pi host verification, restricted authorized_keys, network loss/retry and low-resolution UI acceptance are still external release gates. Future G2 extensions: native authenticated peer identity, durable offline outbox/retries, revocation, secure BLE/mesh transports and push notifications.

## G1 Overseer management and job-specific work queues

[Task Control](TASK_CONTROL.md) is now an offline real module for Overseer with persistent work assignments, role and person fields, priorities, due dates, statuses, operator-marked sends/acknowledgments, bounded history and expected-revision edit conflict rejection. Both micro and desktop views share the same Rust task store, and non-management jobs show their assigned role queue while retaining their specialized tools. **Sharing is manual via copy/email draft**; no background notification, authenticated delivery or live cross-device synchronization is claimed.

**Next gates:** test all R1 task interactions across macOS/Windows, manually review the 4-inch ergonomics, then separately design a secure roster, authenticated receiving inbox/outbox, replay-safe acknowledgment protocol and optional reminders. Do not describe a handoff preview as an actual transmission.

## G1 micro display shell — implementation and acceptance

[Micro handheld mode](MICRO_UI.md) now has a distinct `/micro` shell served by the same local Rust dashboard. It uses original phosphor-style scanlines and restrained glow, a dedicated top status ribbon, one scrolling content viewport, and six fixed thumb-sized tabs (**STAT / GEAR / MAP / DATA / LINK / CONF**). Occupational presets are loaded from the real Rust registry; Custom remains the final empty template. Implemented local labs and explicit profile persistence are shared with desktop. The micro `--micro` launcher, route, asset and end-to-end loopback smoke checks are CI-gated.

**Current limitations:** no real RustGIS navigation renderer, no GNSS or TAK transport, no live IP camera feed, no device battery telemetry, and no Pi touchscreen performance claim. The Scout and Security Chief fixture panels label these absences. The next G1 screen milestone is manual visual/keyboard/touch review at **480×320** and **800×480**, then independent Raspberry Pi 5 hardware testing before GNSS, camera or native RustGIS integration.

## G1 graphical dashboard / job-template experiment station

The [local graphical dashboard](DASHBOARD_UI.md) is the first experimental UI shell. A Rust-only loopback HTTP service hosts original responsive assets; job templates and availability labels come from `PRESETS`/`PALETTE`, with seven ready-made profiles and Custom last. The UI saves explicitly confirmed template changes via the existing WorkbenchStore and exposes allowlisted **offline** GIS, CIDR, hex and user-invoked host experiments. Test coverage includes same-origin denial, local-only bind, persistence and responsive assets; final viewport readability still requires real browser and Pi-screen checks.

**Integration order:** (1) accept Rust dashboard CI on macOS, Windows and 1.90; (2) manually validate Safari/Firefox at desktop, 800×480 and 480×320; (3) improve job-specific widgets and safe input; (4) prototype RustGIS process/renderer integration in Scout; (5) pursue OS-enforced camera/GNSS/TAK access gates. The current Scout viewport is a labeled **reference grid**, not functioning RustGIS navigation; Security Chief cameras are **focus-layout fixtures**, not streams. No embedded runtime module executes on profile load.

## Role-specific roadmap — R0 through R3

The [eight-role workbench plan](ROLE_ROADMAP.md) and [machine-readable role matrix](../catalog/role_roadmap.json) define per-job needs, preloaded tools, new modules, dependencies and objective acceptance tests. The [Vault-Tec lore matrix](VAULT_TEC_LORE_MATRIX.md) distinguishes canon from themed adaptations. **Security Chief** prioritizes a user-requested [2×2 camera wall with focused single-feed view](SECURITY_CAMERA_WALL.md) after deterministic offline mocks; real streams and ONVIF compatibility require separately accepted device and privacy gates.

**Verification policy:** The predecessor workbench and Scout branches passed the full native CI matrix before the compiler upgrade. The Rust 1.90 transition is a new compatibility gate: only an exact-SHA run on the upgraded branch proves it passed. The R0 labels remain provisional for any untested hardware, and real device permissions are not silently enabled.

## Reclamation Scout: GIS + Raspberry Pi + TAK

[Scout architecture and acceptance gates](SCOUT_GIS_TAK.md) define the current independent RFC 7946 Point geometry tools and the future separately gated ARM64 RustGIS adapter and OpenTAKServer/CoT connector. **ATAK itself remains Android-only, and no Pi-native ATAK app or TAK connection is shipped.** Rust-Boy now requires Rust 1.90+, matching RustGIS's compiler minimum; direct RustGIS integration remains deferred for API, licensing, ARM64 and performance verification. This work does not alter the external RustGIS repository.

## Navigate the build

- [Literature matrix](LITERATURE_MATRIX.md) — 37 traceable primary/official references, limitations and corresponding tests.
- [Module and feature catalog](MODULE_CATALOG.md) — 27 modules, dependency gates, capability needs, acceptance criteria.
- [Rust object model](OBJECT_MODEL.md) — object boundaries, traits, composition and runtime access policy.
- [G1 execution plan](G1_EXECUTION_PLAN.md) — ordered engineering acceptance packages and next implementation priority.
- [CI triage](CI_TRIAGE.md) — zero-job startup investigation and required quality evidence.
- [Adapter permissions](PERMISSIONS.md) — persisted preference state, explicit CLI and authorization boundary.
- [Device simulator precursor](SIMULATION.md) — in-memory fixture broker, active simulated leases, confirmation and revocation.
- [Typed adapter interfaces](MOCK_ADAPTERS.md) — input/display/serial mock contracts and OS limitations.
- [Architecture](ARCHITECTURE.md) — host / UI / core / adapter boundaries.
- [OS device broker contract](OS_DEVICE_BROKER.md) — independent privilege boundary and real-device acceptance requirements.
- [Deny-all process protocol](BROKER_PROTOCOL.md) — standalone strict request parser; no active grant or OS-bound device handle.
- [Research library](RESEARCH_LIBRARY.md) — external open-source Pip-Boy implementation references.

## Sequencing rules

The build is a directed acyclic dependency plan, **not** a schedule. All module IDs and dependencies live in [catalog/module_plan.json](../catalog/module_plan.json). Each PR must identify: affected module IDs; evidence IDs; data contracts; permission changes; deterministic test fixtures; user-visible status language; and its exit gate. A module cannot advance from **planned** until its testable implementation has been reviewed.

### G0: Native foundation and quality recovery — **Linux/macOS/Windows terminal CI verified; release gate open**

- [x] Curated public GitHub reference catalog with license status.
- [x] Rust-Boy product identity and documented independent-project IP boundaries.
- [x] Rust workspace, Ratatui interface, system status, local notes and CLI.
- [x] CI quality workflow and offline catalog validator authored, compiled and executed on Ubuntu feature-branch runners.
- [x] Add workflow triggers and stable/1.81.0 compatibility matrix, executed successfully on feature branch at `f445814` (97 tests each).
- [x] GitHub Actions admitted jobs and [passed the full Ubuntu Rust quality matrix](https://github.com/pskeffington/pip_boy/actions/runs/40160220866) after replacing unsupported third-party setup/cache actions with runner-native `rustup`.
- [x] Validate dependency lockfile and pinned toolchain compatibility. **Local Linux: passed; lockfile committed.**
- [x] Native macOS/Windows build, unit test and offline fixture smoke jobs passed in [run 40161263824](https://github.com/pskeffington/pip_boy/actions/runs/40161263824). Native console and terminal restoration checks subsequently passed as part of the later cross-platform CI matrix.
- [x] Windows ConPTY TUI pause/resume, resize, clean/error exit and restoration verified by [run 40162936734](https://github.com/pskeffington/pip_boy/actions/runs/40162936734), using `tools/smoke_windows_conpty.py`.
- [ ] Complete screen-reader/contrast checks, interrupt/write durability and other native accessibility/persistence acceptance.
- [ ] Verify terminal restoration on panic/error and note persistence during interrupted writes.

**G0 exit:** green checks and reproducible build logs; tested CLI and TUI with no implicit external IO. Do not call CI startup failure a code test failure or a passing check.

### G1: Local foundation, provenance, and permission policy — **implemented foundation; acceptance in progress**

**P0: telemetry, storage, settings, accessibility, diagnostics**; **P1: power, network link**.

- [x] Draft versioned `Observation<T>` envelope with source, provenance, optional wall clock, process-session time, unit, quality and expiry (unit-tested; field-level integration remains open).
- [x] Add local host field units/quality/reasons and connect a shared report to console and opt-in detailed status/NDJSON; deterministic fixtures and Linux checks executed. See [host telemetry](HOST_TELEMETRY.md).
- [ ] Complete stored-observation schema migrations and external ingestion validation; native field-report CI and hardware accuracy acceptance remain separate.
- [ ] Separate local read-only metrics from user-initiated notes/storage writes.
- [x] Author a bounded in-memory sample buffer with drop accounting and finite NDJSON host sampling (executed in CI; storage/migration acceptance remains open).
- [ ] Integrate sample buffer with actual UI/runtime, add property-based invariants and output/interruption fixtures.
- [ ] Establish explicit retention/erase policies and schema migration for stored observations. Notes now support explicit schema-1 export/import; observation storage/migration remains planned.
- [x] Author a module lifecycle state machine and deny-before-poll host prechecks (**in-process only; unit-tested**, not an OS sandbox).
- [x] Author a bounded version-1 **preference-only** adapter request/deny/clear store and CLI, with no implicit runtime grants (permission/denial smoke tested in CI).
- [x] Scaffold a separate `rust-boy-broker` executable with **deny-all** schema-1 physical-device request protocol, strict cap/operation/selector checks and bounded stdio; 12 broker unit tests and the black-box CLI subprocess suite passed at [run 40164717116](https://github.com/pskeffington/pip_boy/actions/runs/40164717116). It opens no OS device handle and is not an authorization broker. See [protocol](BROKER_PROTOCOL.md).
- [ ] Implement an independently OS-enforced, per-device authorization broker with authenticated caller identity, trusted consent, scoped handles and real revocation. The [broker threat model](OS_DEVICE_BROKER.md) is defined but **not** enforced. A saved preference and virtual `--confirm` are **not** grants.
- [ ] Add battery, thermal, disk and network-interface status only where supported, with `Unknown` rather than guessed values.
- [ ] Complete accessibility acceptance across platforms. Keyboard collection/detail navigation, compact 40x12 rendering and smaller-screen fallback are locally tested; motion remains static, with no decorative animation.
- [x] Author and execute state/quality/overflow and sample-argument unit tests in CI.
- [ ] Add deterministic UI render fixtures, inject mock clocks/collectors, execute all lifecycle and permission tests.

**G1 exit:** no silent network/socket access, deterministic fixtures, authenticated UI statuses, explicit capability denial before adapter execution, accessible keyboard journeys, no unreviewed writable side effects.

### G2: Physical device and wearable compatibility — **planned**

**P0: trusted OS device-broker boundary (only the deny-all stdio protocol precursor exists) and simulator**; **P1: GPIO, display, serial after broker acceptance**.

- [ ] Select concrete Raspberry Pi and display board revisions with documented wiring, connector limits and operating supply.
- [x] Introduce typed `InputSource`, `DisplayBackend`, `SerialReader` traits and bounded simulated implementations (new source/tests undergoing CI); no platform-specific OS dependency.
- [ ] Connect a real implementation of each trait only after OS-broker and device identity reviews.
- [ ] Support rotary encoder, buttons, and terminal input through one action enum, with debounce/long-press tests.
- [ ] Set adapter timeouts, request cancellations, hot-unplug recovery and reversible settings.
- [ ] Make native dependencies optional by compile-time feature / target gating.
- [ ] Add cross-build for arm64 Linux and hardware-in-loop acceptance tests.

**G2 precursor (cross-platform CI verified):** the in-memory `SimulatedBroker` provides 11 fixed virtual fixtures, simulation-only confirmation, preference denial and lease revocation. Five deterministic traces (`nominal`, `disconnect-recover`, `revoke`, `cancel`, `timeout`) exercise synthetic logical time and denial paths. Typed mock adapters and a finite virtual FIFO are tested; the separate `rust-boy-broker` executable rejects **every** physical-device request. No trusted OS broker, physical measurement, receiver, transmitter or actual device handle is implemented.

**New G2 precursor:** `VirtualTransport` now provides a bounded mock-event FIFO, finite queue pumping, drop accounting and fail-closed buffered-read invalidation; it is not a serial, SDR, mesh or network implementation. This module and native macOS/Windows CI jobs are undergoing acceptance (see [virtual transport](VIRTUAL_TRANSPORT.md)).

**G2 exit:** clean simulator replay on CI and verified physical input on selected hardware; no automatic port opening, scanning, or unsafe fallback.

### G3: Location, sensors, receivers, and peer tools — **planned**

**P1: GNSS and offline map**, then **P2: sensors, SDR, ADS-B, BLE, mesh, weather and audio**.

- [ ] GNSS: explicit fix quality and stale timestamps using official gpsd/receiver input; history disabled by default.
- [ ] Maps: GPX/GeoJSON import and user-provided offline map bundles with attribution and license manifest.
- [ ] SDR and ADS-B: receive-only by default; sample source and decode confidence; legal-purpose/consent review; deterministic recorded fixtures.
- [ ] BLE: user-initiated discover/pair/stop and adapter cancellation behavior.
- [ ] Mesh: choose transport after threat modeling (BLE/serial/other); no assumption that LoRaWAN, Meshtastic and generic BLE are interchangeable; authenticate peers, protect messages, require explicit send.
- [ ] Environment: sensor units, calibration, supported/unsupported indicators; local or explicitly imported weather.
- [ ] Accessibility outputs: optional licensed audio, keyboard alternatives and low-power display profiles.

**G3 exit:** per-adapter integration reports, no unauthorized transmission, data provenance and user consent verified, attack-surface and hardware-failure review completed.

### G4: Release platform and optional ecosystem — **planned**

**P0: release assurance**; **P1: versioned module interface**; **P2: external feeds**.

- [ ] Stable typed module manifest and schema migrations; static linking first, no untrusted native `dlopen`.
- [ ] Optional scoped endpoints with offline-first cache and explicit network consent.
- [ ] Linux/macOS/Windows and selected ARM64 release builds, signed where applicable.
- [ ] SBOM, license scan, vulnerability audit, traceable build provenance (SLSA guidance), reproducible artifacts when feasible.
- [ ] Publish operations and privacy notes, disable-by-default connectivity profile and release regression matrix.

**G4 exit:** release checklist and provenance attestations independently verified. Do not claim security certification, SLSA level, or production readiness without evidence.

## G0 and G1 status summary

**G0:** GitHub Actions is operational. [Run 40162059963](https://github.com/pskeffington/pip_boy/actions/runs/40162059963) passed Linux stable/MSRV, macOS native PTY and Windows native CLI/fixture checks at `8a4f7c5` (115 Rust tests per toolchain). Windows native ConPTY is now verified at [run 40162936734](https://github.com/pskeffington/pip_boy/actions/runs/40162936734); release hardening and accessibility gates remain open.

**G1:** the bounded observation buffer, lifecycle checks, finite NDJSON command, one-shot STAT, local NOTES/LIBRARY and versioned `PreferenceStore` are implemented and exercised in CI. Host telemetry shares lifecycle checks across CLI/TUI. Preferences record `requested`/`denied` intent only and **cannot enable** radio, GNSS, mesh, BLE, GPIO, serial or network access. Field-level observation integration, accessibility and independent OS-enforced consent/brokering remain open. See [permissions contract](PERMISSIONS.md) and [executed validation](VALIDATION.md).

**G2–G4:** still planned, with no external hardware, location, scanning or transmission enabled. The G1 virtual broker cannot be used as a hardware permission service.

The next engineering handoff is [G1_EXECUTION_PLAN.md](G1_EXECUTION_PLAN.md). Do not merge or release based on static files alone.

## Quality and threat gates (all phases)

| Area | Required evidence |
| --- | --- |
| Correctness | Cargo tests, format, Clippy, panic/recovery and malformed-input fixtures |
| Reliability | Disconnect, retries, bounded queues, resource exhaustion and restart conditions |
| Privacy | No background upload/location collection; retention and purge validated |
| Security | Device broker, explicit grants, no implicit peer trust, denial-path tests |
| Interoperability | Concrete platform matrix, capability detection, protocol version testing |
| Research | Module-to-literature IDs, limitations, license and source provenance |
| UX/accessibility | Keyboard navigation, no color-only meaning, reduced motion, small screen |
| IP / licensing | No Fallout protected media; map tile/driver/data notices checked |

## Risk register

1. **CI regressions:** require passing checks for each new source revision. The historical startup blocker was resolved; the current five-job baseline is linked above.
2. **Platform-specific APIs:** GPIO/serial/BLE/SDR can require native libraries or user privileges; postpone runtime wiring until target selection.
3. **Legal datasets:** OSM data licensing and OSM Foundation raster tile policy are different; do not scrape community tiles for offline caching.
4. **Messaging security:** pairing is not authentication of a human peer, and HPKE is not a complete messaging app; threat model and protocol review are mandatory.
5. **Power/resource budget:** wearable operation needs measured idle/active battery and CPU consumption; no guessed figures.
6. **Object-model drift:** runtime module registry must agree with the roadmap manifest; automated cross-validation is required.

**Explicit non-goals:** proprietary game assets; unauthorized scans, surveillance, jamming or infrastructure control; silent device access; arbitrary native plugin loading; unsupported claims of live GNSS/radio/mesh capabilities.

## Fallout 4 companion acceptance (requested: all versions)

An opt-in read-only direct TCP browser and incremental decoder are implemented. See [game mode and edition matrix](GAME_MODE.md). Separate gates remain: actual legacy/Anniversary PC service tests; AE game-side bridge/enabler validation; console-build service availability; gameplay command validation; local-map image rendering; and a recorded real-game end-to-end session for each supported build. Protocol mocks cannot complete these gates. No all-version compatibility claim is approved.

**Priority change:** Mac → PlayStation launch/setup/diagnostics now leads game-mode work. PC/AE and other console expansion are deferred; see [Mac/PlayStation acceptance](MAC_PLAYSTATION.md).

The source launcher has a verified native macOS CI baseline. Follow-on readiness checks and prompted-launcher regressions remain controlled-peer evidence; a real Mac/PS4 or Mac/PS5 session and supported gameplay commands are still required before full compatibility can be claimed.

The game browser now has separate navigation/value panes, long-list selection visibility, parent-context restoration, bounded text scrolling and recovery from resize or replaced/removed object references. These are native-client usability improvements; they do not substitute for the live console or gameplay-command gates.

# Rust-Boy | Evidence-based development roadmap

**Working version:** 0.3 G1 foundation pass · **Review:** 2026-10-24  
**Release condition:** No phase is complete solely because its files or test definitions exist. CI execution, platform checks, and documented acceptance results must be verified.

Rust-Boy is the public product name. The existing `pip_boy` repository, `pipboy` executable, `pipboy-core` crate and `PIPBOY_HOME` data interface remain stable.

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
- [x] Native macOS/Windows build, unit test and offline fixture smoke jobs passed in [run 40161263824](https://github.com/pskeffington/pip_boy/actions/runs/40161263824). Native portable output and POSIX/macOS TUI restoration checks are expanded in the next pass (new runs pending).
- [x] Windows ConPTY TUI pause/resume, resize, clean/error exit and restoration verified by [run 40162936734](https://github.com/pskeffington/pip_boy/actions/runs/40162936734), using `tools/smoke_windows_conpty.py`.
- [ ] Complete screen-reader/contrast checks, interrupt/write durability and other native accessibility/persistence acceptance.
- [ ] Verify terminal restoration on panic/error and note persistence during interrupted writes.

**G0 exit:** green checks and reproducible build logs; tested CLI and TUI with no implicit external IO. Do not call CI startup failure a code test failure or a passing check.

### G1: Local foundation, provenance, and permission policy — **planned**

**P0: telemetry, storage, settings, accessibility, diagnostics**; **P1: power, network link**.

- [x] Draft versioned `Observation<T>` envelope with source, provenance, optional wall clock, process-session time, unit, quality and expiry (**unverified prototype**).
- [ ] Verify schema compatibility, implement field-level units and connect observations to each frontend.
- [ ] Separate local read-only metrics from user-initiated notes/storage writes.
- [x] Author a bounded in-memory sample buffer with drop accounting and a user-triggered finite NDJSON host sampling command (**unverified prototype**).
- [ ] Integrate sample buffer with actual UI/runtime, add property-based invariants and output/interruption fixtures.
- [ ] Establish explicit retention/erase policies and schema migration for stored observations. Notes now support explicit schema-1 export/import; observation storage/migration remains planned.
- [x] Author a pure module lifecycle state machine and existing deny-before-poll host prechecks (**in-process only; unverified**).
- [x] Author a bounded version-1 **preference-only** adapter request/deny/clear store and CLI, with no implicit runtime grants (**new tests unexecuted**).
- [x] Scaffold a separate `rust-boy-broker` executable with **deny-all** schema-1 physical-device request protocol, strict cap/operation/selector checks and bounded stdio; authored unit/CLI test fixtures are pending exact-commit CI. It opens no OS device handle and is not an authorization broker. See [protocol](BROKER_PROTOCOL.md).
- [ ] Implement an independently OS-enforced, per-device authorization broker with authenticated caller identity, trusted consent, scoped handles and real revocation. The [broker threat model](OS_DEVICE_BROKER.md) is defined but **not** enforced. A saved preference and virtual `--confirm` are **not** grants.
- [ ] Add battery, thermal, disk and network-interface status only where supported, with `Unknown` rather than guessed values.
- [ ] Complete accessibility acceptance across platforms. Keyboard collection/detail navigation, compact 40x12 rendering and smaller-screen fallback are locally tested; motion remains static, with no decorative animation.
- [x] Author state/quality/overflow and sample-argument unit tests (**not yet executed**).
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

**G2 precursor (G1-D source authored, unverified):** an in-memory `SimulatedBroker` now has 11 fixed virtual fixtures, explicit simulation confirmation, preference denial, generation-based lease revocation and truthful fake-only events. The G1 precursor now also has fixed `nominal`, `disconnect-recover`, and `revoke` logical-time trace scripts (tests authored, unexecuted). Two additional source-defined `cancel` and `timeout` scenarios exercise virtual session invalidation with injected logical clock bounds (**Ubuntu CI verified at `f445814`**). This is not the full G2 hardware simulator, OS broker, driver event replay system or physical adapter.

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

**G1:** typed observation, bounded sample buffer, lifecycle transitions, finite NDJSON command and offline catalog validator have initial source implementations with telemetry now shared by TUI and finite NDJSON through a bounded session. The lazy host telemetry adapter now connects ModuleHost, lifecycle and capabilities to the TUI and finite sampling command; the one-shot status CLI has also been wired through it (new tests awaiting execution). The new `PreferenceStore` and CLI record requested/denied adapter intent only, with authored schema/locking/smoke tests not yet executed. No preference enables radio, GNSS, mesh, BLE, GPIO or network access; actual consent and brokering remain planned. See [permissions contract](PERMISSIONS.md) and [validation evidence](VALIDATION.md).

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

1. **CI startup failure (active):** blocks code-level verification. Repair platform access/runner state before claiming G0 complete.
2. **Platform-specific APIs:** GPIO/serial/BLE/SDR can require native libraries or user privileges; postpone runtime wiring until target selection.
3. **Legal datasets:** OSM data licensing and OSM Foundation raster tile policy are different; do not scrape community tiles for offline caching.
4. **Messaging security:** pairing is not authentication of a human peer, and HPKE is not a complete messaging app; threat model and protocol review are mandatory.
5. **Power/resource budget:** wearable operation needs measured idle/active battery and CPU consumption; no guessed figures.
6. **Object-model drift:** runtime module registry must agree with the roadmap manifest; automated cross-validation is required.

**Explicit non-goals:** proprietary game assets; unauthorized scans, surveillance, jamming or infrastructure control; silent device access; arbitrary native plugin loading; unsupported claims of live GNSS/radio/mesh capabilities.

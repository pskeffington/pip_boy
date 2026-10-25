# Rust-Boy | Architecture

Rust-Boy is the public name of the modular console. The repository, executable, crate names, and data paths retain their existing `pip_boy`/`pipboy` identifiers for compatibility; see [BRANDING.md](../BRANDING.md).

## Engineering navigation

- [Object model](OBJECT_MODEL.md): traits, runtime registry, permission policy, typed data proposals and lifecycle.
- [Module catalog](MODULE_CATALOG.md): 27 scoped modules and their dependency/acceptance gates.
- [Literature matrix](LITERATURE_MATRIX.md): 37 references tied to engineering decisions and test criteria.
- [Roadmap](ROADMAP.md): phase sequencing, release gates and risks.

The current `ModuleHost` is an **in-process testable policy precheck** and does not isolate untrusted code. Physical device access, OS-enforced permissions and transport isolation remain planned, not implemented.

## Dual graphical presentation: desktop and micro

The Rust `pipboy dashboard` process serves two intentionally distinct local views: `/` for the desktop workstation and `/micro` for the dedicated handheld frame (or use `dashboard --micro --open` on Mac). Both send requests to the **same** bounded JSON API and authoritative `pipboy-core::workbench::PRESETS` / `PALETTE` / WorkbenchStore. A role change in one view is reflected in the other after reload. Neither view may acquire new permissions merely by displaying a tile.

The micro renderer has a fixed status header, six-tab rail and one scrollable active viewport rather than responsive desktop columns. It uses original CSS effects, no game artwork, no external scripts/CDNs, and only allowlisted offline tool operations. Scout's map and the Security Chief camera grid are clearly marked **unconnected UI fixtures**. The API still runs only on IPv4 loopback with strict request checks. See [micro controls](MICRO_UI.md).

## Local graphical UI and template state

The new `pipboy dashboard [--port N] [--open]` command runs a loopback-only HTTP service and serves static, embedded original UI assets. The browser is a **presentation client**, not a device broker. `GET /api/state` projects `pipboy-core::workbench::{PRESETS, PALETTE, Workbench}` to the UI; profile activation and Custom pin/unpin go through the same versioned, atomic WorkbenchStore used by the TUI and CLI. Explicitly invoked GIS/NET/HEX calculations call the existing Rust core functions and **never** shell out arbitrary commands. The service neither exposes note contents nor automatically opens live devices.

```text
Mac/Pi local browser (keyboard, mouse, touch)
              |
         127.0.0.1 only
              |
       pipboy dashboard (Rust)
         /              \
  GET projection      approved JSON POST
         |              |
       PRESETS      WorkbenchStore + offline tools
         \              /
           pipboy-core
                  |
        no device broker grants
```

The 4-inch reference layouts and mock camera focus are **visual UX fixtures**. A future RustGIS process/renderer integration in Reclamation Scout is a separate gate and must not be substituted with a simulated map. See [dashboard contract](DASHBOARD_UI.md) and [Scout GIS interoperability](SCOUT_GIS_TAK.md).

## Boundaries

    User keyboard / physical controls (future)
                   |
          Rust-Boy console
        (pipboy executable)
             CLI + Ratatui
                   |
            pipboy-core
       /         |        \
    Snapshot   Catalog   Local notes
       |
    sysinfo (read-only)

Future feature-gated adapters attach beneath core capability interfaces:
- location: GNSS/NMEA and user-imported GPX; opt-in and explicit provenance.
- map: locally licensed offline tiles and track overlays; do not ship map tile data.
- radio: legal receive-only hardware discovery before any stream reader.
- mesh: separate serial/BLE adapters, authenticated transports, no automatic pairing.
- hardware: Raspberry Pi GPIO rotary encoders, buttons, and display classes.
- remote data: disabled unless user configures a specific endpoint.

## Design rules

1. Every datum records its source and freshness. Unknown is never converted into zero.
2. Every module reports implemented / planned / unavailable, not simulated success.
3. Side effects are isolated: collecting CPU metrics and displaying a read-only catalog never opens sockets or devices.
4. Read-only and offline are the defaults. Any network, recording, location, or transmission capability needs opt-in.
5. UI and command-line frontends share core structs; adapters must remain independently testable.
6. The catalog holds links and reviews, not third-party copied sources, game assets, or trademarks.
7. Cross-platform first: Linux, macOS, Windows; add arm64 Raspberry Pi build checks in a later gate.

## Shared telemetry session

`host_report.rs` derives field-level units, quality and bounded reason codes from the same retained snapshot, without additional collection. `ModuleSample` carries both the raw observation and this detailed report. The TUI and opt-in CLI `--details` output share it; stale propagation is applied at render time. `Collector` requests only CPU/RAM refreshes and gates CPU deltas with a monotonic cadence; the TUI does not prime by sleeping. See [host telemetry](HOST_TELEMETRY.md) for schemas and CPU null semantics.

`TelemetrySession<SnapshotSource>` owns a bounded in-memory observation history. The TUI retains 60 readings; finite NDJSON retains at most the requested 1–30 readings. Both use the same source/provenance and monotonic expiry logic. Tests inject fixture collectors and timestamps. Reading a freshness view never samples the host. `status --json` keeps its existing raw Snapshot format. Both frontends now use `HostTelemetryModule` through `ModuleHost`; collector initialization follows an explicit permitted start. TUI P immediately revokes host reads and clears cached/buffered data; resume grants local reads and explicitly starts a fresh buffer. The one-shot `status` CLI now also uses the lazy `HostTelemetryModule` through `ModuleHost`, with an explicit local read grant, while retaining the original raw `status --json` Snapshot format. LIBRARY and NOTES CLI/TUI paths now use scoped `ModuleHost::run_local` read/write prechecks (execution testing pending). These synchronous operations do not represent running device lifecycle modules; the OS sandbox and device broker are still unimplemented.

## Adapter preference state (unverified)

`PreferenceStore` uses a separate version-1 `permissions.json` document to record only `requested` or `denied` choices for 11 future external adapters. All unset permissions remain disabled. Its API never calls `ModuleHost::grant`, and the current runtime receives no permissions from this file. Reads are side-effect-free; writes take a stable advisory lock and perform flushed, bounded same-directory replacement. A corrupted or future document is rejected without rewrite. The CLI supports list/request/deny/clear, but physical or network access still requires a distinct session authorization service and future OS device broker. See [adapter preference contract](PERMISSIONS.md). New tests and native platform checks are unverified.

## Simulated device broker (G1-D, new tests pending)

`device_simulator.rs` contains an in-memory-only broker with 11 hard-coded `sim:*` fixture device IDs. It reads validated `PreferenceEntry` values supplied by a caller but never converts `requested` into an active grant. Each virtual session needs a new explicit `SessionConfirmation::Confirmed`, and `denied` blocks it. A lease uses per-broker identity and generation to prevent cross-instance replay and is invalidated on close, revocation and a denied policy reload. Events carry `simulated: true` and **no fabricated measurements**. The CLI `simulate list|run ID --confirm` only creates virtual events; no OS device interface is reachable. See [simulation contract](SIMULATION.md). This is not an OS device broker or user consent service for real adapters, and the G2 milestones remain planned.

## Deterministic virtual replay (new tests pending)

`simulator_replay.rs` creates fixed `nominal`, `disconnect-recover`, and `revoke` traces with synthetic logical timestamps (0–700 ms), independent of real clocks, hardware and background sampling. The broker's virtual disconnect immediately invalidates leases; reconnect does not restart anything. Every rejection is explicitly labeled, and source-defined replay cannot authorize real adapters. See [simulation](SIMULATION.md). Both the compiler and CLI acceptance suites remain pending.

## Bounded virtual transport (G2 precursor)

`virtual_transport.rs` wraps the in-memory `SimulatedBroker` with a bounded `VecDeque` retaining at most 64 labeled mock events. `pump` is finite (1–128), FIFO overflow discards oldest entries and records the number dropped. `receive` checks active simulated session validity; cancel, disconnect, revoke, timeout, clock inversion or denied/corrupt policy clear the queue before any stale data can be consumed. The CLI `simulate transport ID --count N --capacity N --confirm` reports only synthetic events and drop counts, without measuring or touching external interfaces. These are trusted process-local checks, **not** OS-enforced isolation, real device drivers or authenticated communications. See [virtual transport](VIRTUAL_TRANSPORT.md); CI validation on the new implementation is pending.

## Typed simulated adapter interfaces (G2 precursor)

The core now defines three object-safe traits: `InputSource`, `DisplayBackend` and `SerialReader`. `mock_adapters.rs` implements each using the existing confirmation-gated, in-memory `VirtualTransport`, with fixed `sim:gpio`, `sim:display` or `sim:serial` fixtures. GPIO mock events explicitly contain **no physical input or pin value**; display accepts a bounded printable frame but returns `rendered_to_hardware: false`; serial returns **injected** fixture bytes only. Inputs are never polled from a physical interface, no serial port is opened, and no display is driven. See [typed mock adapters](MOCK_ADAPTERS.md). These traits are not OS security boundaries, and any real backend must use a separate trusted consent service and scoped OS broker.

## User data

PIPBOY_HOME overrides the data directory. Otherwise, use $HOME/.pipboy on Unix and %USERPROFILE%/.pipboy on Windows. Notes use a stable advisory writer lock, a flushed staging file in the data directory, and replacement of notes.json without deleting it first. Unix also syncs the containing directory; native Windows/macOS durability acceptance remains open. Reads/search/export create no files. Explicit notes import validates its named input, locks the merge transaction, assigns fresh local IDs and commits only a complete bounded result. No background synchronization. See [storage contract](STORAGE.md). Delete the folder to remove locally stored notes. Content is not encrypted.

## Release gates

No release marked production-ready until CI, hardware simulation, terminal restoration, failure-path tests, dependency audit, accessibility review, and platform smoke tests pass. See ROADMAP.md.

## Console rendering

`console.rs` owns pure navigation and frame rendering; it opens no files or collectors. `tui.rs` maps terminal events to actions and manages telemetry and explicit note reloads. Collections use an item selector and a separately scrollable detail pane. Notes load on entering NOTES or pressing R; ordinary redraws reuse the in-memory list. See [console controls and size behavior](CONSOLE.md).

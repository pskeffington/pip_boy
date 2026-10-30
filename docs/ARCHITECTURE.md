# Rust-Boy | Architecture

## Target host and package architecture

The [host package roadmap](HOST_PACKAGE_ROADMAP.md) defines a **one-foreground-package-at-a-time** shell with an always-available host Settings control. A **read-only manifest registry precursor** is implemented as `pipboy_core::package_socket`; `pipboy packages` CLI and loopback `GET /api/packages` inspect schema-1 package references but **cannot install or launch them**. The handheld `MODULES` control displays the registry, with activation unavailable until installed/trusted adapter state and a guarded foreground route exist. Background lifecycle, capability brokering and independent adapter release gates remain planned; the current in-process ModuleHost is not a security sandbox. Desktop foreground switching remains pending HOST-UI gates.

## Current Network Engineer integration

`pipboy_core::network_suite` provides the shared 16-operation catalog and nine deterministic offline tools. `network_health` computes statistics without I/O. The binary's `endpoint_probe` performs finite ICMP/TCP tests; the existing core `network_probe` and `unifi` adapters provide private-LAN evidence and controller-connected clients.

Both browser shells mount `assets/network.js` with the catalog from `/api/state`. The component owns its request generation, AbortController, pagination and single 60-second refresh timer. Destroy/hidden/error paths clear refresh and suppress stale replies; requests already executing on the Rust server retain their own bounds. Original `/api/net/scan` and `/api/net/unifi` contracts remain intact; `/api/experiment` adds typed network modes, extra input, samples and explicit confirmation. See [health/UniFi setup](NETWORK_HEALTH_UNIFI.md).

Rust-Boy is the public name of the modular console. The repository, executable, crate names, and data paths retain their existing `pip_boy`/`pipboy` identifiers for compatibility; see [BRANDING.md](../BRANDING.md).

## Network Engineer HTTP adapter boundary

`crates/pipboy/src/dashboard/network_routes.rs` owns the existing explicit HTTP handlers `POST /api/net/scan` and `POST /api/net/unifi`. The single `dashboard.rs` loopback listener continues to enforce exact Host, same-origin POST and JSON body bounds **before** either handler. `pipboy-core::network_probe` still validates private CIDR and performs bounded, confirmed LAN scanning; `pipboy-core::unifi` retains read-only HTTPS/controller policy, and the OS subprocess credential-isolation layer remains unchanged. Role selection alone or opening NET ENGINEER does not initiate probes, fetch controller data or grant network permissions.

The extraction changes source organization only. It does not create another listener, duplicate credentials, expand the allowed scan scope, or certify a real UniFi controller or Raspberry Pi hardware. Dashboard and existing network/UniFi smoke tests enforce the contract.

## Engineering navigation

- [Organizational optimization research](ORGANIZATIONAL_OPTIMIZATION.md): evidence matrix, code concentration, component accountability and Mac-to-Pi sequencing.
- [Architecture decision 0001](adr/0001-mac-first-portable-core.md): single portable Rust workspace with separately accepted hardware adapters.

- [Object model](OBJECT_MODEL.md): traits, runtime registry, permission policy, typed data proposals and lifecycle.
- [Module catalog](MODULE_CATALOG.md): 27 scoped modules and their dependency/acceptance gates.
- [Literature matrix](LITERATURE_MATRIX.md): 37 references tied to engineering decisions and test criteria.
- [Roadmap](ROADMAP.md): phase sequencing, release gates and risks.

The current `ModuleHost` is an **in-process testable policy precheck** and does not isolate untrusted code. Physical device access, OS-enforced permissions and transport isolation remain planned, not implemented.

## Explicit peer task transport (no Rust-Boy network listener)

A separate `pipboy-core::peers` private config holds local device labels, trusted sender labels and explicitly approved OpenSSH destinations. `pipboy peer send ID ALIAS --confirm` and the protected dashboard route invoke a fixed SSH command with strict host-key and key authentication. A receiving SSH account executes a **restricted `pipboy peer bridge SOURCE` forced command**: it only permits receive or source-specific receipt reporting, validates trusted source and stores the typed assignment through the task store. The sender checks the receiver's stored-task ID before persisting delivery; recipient acknowledgments/status are retrieved only on explicit `peer sync`. Host fingerprints must be verified out-of-band when enrolling a peer. This backend does not expose its own network service or run a discovery scan.

The sender's status, authenticated SSH delivery receipt, receiving device's manual acknowledgment and receiver-reported progress remain distinct. Job template selection grants no SSH privilege and triggers no networking. See [peer transport guide](PEER_TASK_TRANSPORT.md).

## Task Control data and operational boundaries

The dashboard's `crates/pipboy/src/dashboard/task_routes.rs` contains the operator-facing Task Control and explicit peer-delivery route handlers. `dashboard.rs` retains the single loopback-host / same-origin request gate and dispatch table; the extracted module uses those shared request/response contracts, not a second server, storage schema or permission mechanism. The native SSH bridge remains `crates/pipboy/src/peer.rs`. Existing endpoint paths and error/status behavior are preserved by dashboard unit and smoke tests.

The Rust `pipboy-core::tasks` module persists a distinct, versioned `tasks.json` board under `PIPBOY_HOME`; the `workbench.json` file still stores role/pin configuration only. Task create/update requests use the existing loopback API but **never** start a communication channel. A reusable browser component `assets/tasks.js` powers both desktop and handheld mission boards. Overseer/Custom with `mission-board` pinned sees the full local manager board; other job profiles see only their locally assigned queue. These UI scopes are **not authenticated role-based access control**.

Task records include expected-revision optimistic concurrency, bounded events and explicit manual `sent_at` / `acknowledged_at` tracking. Clipboard handoffs and email drafts are only prepared after operator action; no automatic sending or receipt inference is performed. The local store rejects invalid/future schemas without replacing them. See [Task Control contract](TASK_CONTROL.md).

## Compact handheld renderer

`assets/micro.{html,css,js}` owns the 480×320 page and tab layout. `RustBoyTasks.mount(..., {compact:true})` selects an independently paginated micro task presentation; desktop continues using the full component. The shared Rust REST API, task store, peer SSH transport, role catalog and capability checks are not duplicated. The normal micro root view suppresses vertical scrolling; long technical outputs and detail accessibility fallbacks remain reachable. No hardware action occurs simply by displaying a screen.

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

## Operator-authorized LAN inspection

NET ENGINEER is the first live network tool. Only an explicit CLI command or same-origin, confirmed dashboard POST to /api/net/scan starts a bounded ICMP/TCP sweep of a selected RFC1918 /24..../32. The dashboard remains loopback-only, and long scans use limited request workers. Its report distinguishes verified responders from stale-prone neighbor-cache entries; it is not a total-client count. UI opt-in can refresh every 60 seconds while NET ENGINEER stays open. See [network diagnostics contract](NETWORK_ENGINEERING.md). An optional read-only UniFi API adapter uses server-environment credentials over verified TLS to an explicitly configured RFC1918 controller; no credentials are exposed through dashboard routes. No controller mutations, vulnerability scanning, or Internet probing are enabled.

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

## Messenger/PACE HTTP adapter boundary

`crates/pipboy/src/dashboard/messenger_routes.rs` now owns the three Messenger HTTP handlers: read `GET /api/messages`, explicitly confirmed `POST /api/messages/send`, and explicitly confirmed `POST /api/messages/retry`. The parent `dashboard.rs` remains the sole loopback server, HTTP request parser and Host/Origin security gate; the extraction adds no new listener, permissions or transport. `pipboy-core::messenger` and `pipboy-core::message_pace` continue to own the journal, envelope and PACE policy. Pinned-identity OpenSSH actions remain in `crates/pipboy/src/peer.rs`, invoked only after validated operator actions. Reading LINK never initiates network delivery. These boundaries are covered by dashboard route tests and existing Messenger/peer smoke tests; independent physical bearer verification remains open.

## Messenger canonical transport and encrypted queue

`message_protocol` defines the v1 envelope, typed storage receipt and one-attempt
`MessageTransport`; the existing explicit OpenSSH path implements that interface.
The deterministic virtual-clock simulator runs without real transports.
`message_keys` delegates queue encryption to age and private identity persistence
to native OS credential storage. Fresh journals enroll explicitly; plaintext
legacy journals are preserved. Local-IP fallback changes SSH routing while
retaining the enrolled host identity. `meshtastic_passive` takes only a borrowed
`Read` stream/recorded capture and cannot acquire a physical port or send ToRadio.
The deny-only OS broker remains unchanged. See the [canonical contract and
remaining gates](MESSENGER_BUILD_PASSES.md).

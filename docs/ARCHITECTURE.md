# Rust-Boy | Architecture

Rust-Boy is the public name of the modular console. The repository, executable, crate names, and data paths retain their existing `pip_boy`/`pipboy` identifiers for compatibility; see [BRANDING.md](../BRANDING.md).

## Engineering navigation

- [Object model](OBJECT_MODEL.md): traits, runtime registry, permission policy, typed data proposals and lifecycle.
- [Module catalog](MODULE_CATALOG.md): 27 scoped modules and their dependency/acceptance gates.
- [Literature matrix](LITERATURE_MATRIX.md): 37 references tied to engineering decisions and test criteria.
- [Roadmap](ROADMAP.md): phase sequencing, release gates and risks.

The current `ModuleHost` is an **in-process testable policy precheck** and does not isolate untrusted code. Physical device access, OS-enforced permissions and transport isolation remain planned, not implemented.

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

`TelemetrySession<SnapshotSource>` owns a bounded in-memory observation history. The TUI retains 60 readings; finite NDJSON retains at most the requested 1–30 readings. Both use the same source/provenance and monotonic expiry logic. Tests inject fixture collectors and timestamps. Reading a freshness view never samples the host. `status --json` keeps its existing raw Snapshot format. Both frontends now use `HostTelemetryModule` through `ModuleHost`; collector initialization follows an explicit permitted start. TUI P immediately revokes host reads and clears cached/buffered data; resume grants local reads and explicitly starts a fresh buffer. Legacy `status`, library and notes remain direct service calls.

## User data

PIPBOY_HOME overrides the data directory. Otherwise, use $HOME/.pipboy on Unix and %USERPROFILE%/.pipboy on Windows. Notes use a stable advisory writer lock, a flushed staging file in the data directory, and replacement of notes.json without deleting it first. Unix also syncs the containing directory; native Windows/macOS durability acceptance remains open. Reads/search create no files. No background synchronization. See [storage contract](STORAGE.md). Delete the folder to remove locally stored notes. Content is not encrypted.

## Release gates

No release marked production-ready until CI, hardware simulation, terminal restoration, failure-path tests, dependency audit, accessibility review, and platform smoke tests pass. See ROADMAP.md.

## Console rendering

`console.rs` owns pure navigation and frame rendering; it opens no files or collectors. `tui.rs` maps terminal events to actions and manages telemetry and explicit note reloads. Collections use an item selector and a separately scrollable detail pane. Notes load on entering NOTES or pressing R; ordinary redraws reuse the in-memory list. See [console controls and size behavior](CONSOLE.md).

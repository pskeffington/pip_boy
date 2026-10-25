# Rust-Boy | Architecture

Rust-Boy is the public name of the modular console. The repository, executable, crate names, and data paths retain their existing `pip_boy`/`pipboy` identifiers for compatibility; see [BRANDING.md](../BRANDING.md).

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

## User data

PIPBOY_HOME overrides the data directory. Otherwise, use $HOME/.pipboy on Unix and %USERPROFILE%/.pipboy on Windows. Notes are written atomically to notes.json; no background synchronization. Delete the folder to remove locally stored notes. Content is not encrypted.

## Release gates

No release marked production-ready until CI, hardware simulation, terminal restoration, failure-path tests, dependency audit, accessibility review, and platform smoke tests pass. See ROADMAP.md.

# PIP_BOY

An independent, offline-first Rust systems console inspired by the utility of wearable field computers. This repository is **not** an official Fallout, Bethesda, or Wand Company product. It includes no game assets, proprietary firmware, or copied third-party code.

## Objective

Build one reusable Rust-native platform that can run as a desktop terminal interface and later support Raspberry Pi-class wearable computers, physical input controls, location services, offline mapping, radio receivers, and local mesh transports.

## Current foundation

- Native terminal interface: STAT, LIBRARY, NOTES, MAP, RADIO, MESH.
- Live local CPU, RAM, OS, architecture, and uptime telemetry.
- Read-only, machine-readable GitHub research catalog with explicit license status.
- Local notes: a user-initiated, offline journal with JSON persistence.
- Module registry that distinguishes live implementations from planned adapters.
- Diagnostics command, unit tests, and GitHub Actions quality checks.

MAP, RADIO, and MESH are intentionally labeled **planned**. No imaginary hardware, location fix, transmission, or internet data is presented as real.

## Requirements

Rust stable toolchain (cargo and rustc); a modern terminal on macOS, Linux (including Raspberry Pi OS), or Windows.

## Build and run

    cargo run -p pipboy -- tui
    cargo run -p pipboy -- status
    cargo run -p pipboy -- status --json
    cargo run -p pipboy -- sources
    cargo run -p pipboy -- sources pipboy
    cargo run -p pipboy -- modules
    cargo run -p pipboy -- doctor
    cargo run -p pipboy -- notes add "Field observation"
    cargo run -p pipboy -- notes list
    cargo test --workspace
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings

The installed executable is named pipboy. In the TUI, use Left/Right or Tab to change panels, and Q to quit. The interface never needs an account or a hosted server.

## Repository map

- crates/pipboy-core: domain models, host telemetry, catalog, notes.
- crates/pipboy: native CLI and Ratatui terminal UI.
- catalog/sources.json: indexed external project references, **not vendored source**.
- docs/RESEARCH_LIBRARY.md: comparative research and license gates.
- docs/ARCHITECTURE.md: boundaries, adapters, data contracts.
- docs/ROADMAP.md: progressive hardware and field-computing gates.
- SECURITY.md: local data, permissions, and ingestion rules.

## Important boundaries

External project links are references, not an invitation to copy code. Missing or unclear licenses are blocked from ingestion; GPL and noncommercial licenses require separate compatibility review. Radio and mesh integrations will be opt-in, legal-use, and receive-only/read-only by default. The local notes file is not encrypted; do not store secrets.

## Attribution

Original implementation: Paul A. Skeffington and contributors. MIT license for original code in this repository only. Third-party copyright and trademarks remain with their respective owners.

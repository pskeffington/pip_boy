# Rust-Boy

**A Rust-native, offline-first field systems console.**

Rust-Boy is an independent, modular toolkit for desktop terminals and future wearable field computers. It combines local host diagnostics, a curated research library, and a user-controlled offline journal under one minimal native interface. Planned adapters extend it to Raspberry Pi hardware, maps, receive-only radios, and authenticated local communications.

> **Project status — G0 verification blocked; G1 prototypes in progress.** STAT, LIBRARY, and NOTES have initial implementations. Typed observations, bounded sampling, a pure lifecycle state machine, and an explicit finite NDJSON host command have been added but remain uncompiled and unverified. MAP, RADIO, MESH, GPIO, and external device access remain planned. No placeholder implies a real device connection.

## Rust-Boy identity

| Role | Name |
| --- | --- |
| **Public product name** | **Rust-Boy** |
| Product descriptor | Offline-first field systems console |
| GitHub repository | [pskeffington/pip_boy](https://github.com/pskeffington/pip_boy) |
| Existing command / executable | `pipboy` (retained for compatibility) |
| Rust workspace packages | `pipboy`, `pipboy-core` (internal names retained) |
| Local configuration variable | `PIPBOY_HOME` (unchanged) |

**Rust-Boy** is the name used in the interface, documentation, and future releases. Technical identifiers remain unchanged during this branding pass so existing commands and local data continue to work. The naming, visual system, and attribution rules are defined in **[BRANDING.md](BRANDING.md)**.

Rust-Boy is an **unofficial, independent project**. It is not affiliated with or endorsed by Bethesda, the Fallout franchise, The Wand Company, or their licensors. External Pip-Boy projects in the research catalog are references only; no game artwork, protected logos, proprietary firmware, or third-party code is bundled.

## Design principles

- **Native first:** a Rust core and lightweight Ratatui terminal shell.
- **Offline first:** no accounts, analytics, implicit network access, or background scanning.
- **Observable:** measured local data is separated from unknown, simulated, or planned features.
- **Modular:** physical controls, location, radio, and mesh tools use opt-in adapters.
- **Traceable:** external sources carry license and reuse classifications.
- **Accessible:** keyboard-operated navigation, legible contrast, and future low-motion display profiles.

## Available modules

| Module | G0 state | Purpose |
| --- | --- | --- |
| STAT | Implemented | Local CPU, memory, OS, architecture, and uptime |
| LIBRARY | Implemented | Read-only GitHub reference catalog with license indicators |
| NOTES | Implemented | Offline, user-initiated plaintext notes |
| MAP | Planned | Optional GNSS/GPX and licensed offline maps |
| RADIO | Planned | Opt-in, legal receive-only radio integration |
| MESH | Planned | User-authorized local transport and peer authentication |
| GPIO / Devices | Planned | Physical encoders, buttons, display, and serial/BLE adapters |

## Build and run

Requires a stable Rust toolchain and a modern terminal on macOS, Linux, or Windows.

```bash
git clone https://github.com/pskeffington/pip_boy.git
cd pip_boy
cargo run -p pipboy -- tui
cargo run -p pipboy -- status --json
cargo run -p pipboy -- sample --count 3 --interval-ms 1000
cargo run -p pipboy -- sources
cargo run -p pipboy -- sources rust
cargo run -p pipboy -- modules
cargo run -p pipboy -- plan G3
cargo run -p pipboy -- evidence Rust
cargo run -p pipboy -- doctor
cargo run -p pipboy -- notes add "Field observation"
cargo run -p pipboy -- notes list

python3 tools/validate_catalog.py
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The `sample` command is intentionally bounded to 1–30 samples at 500–30000 ms intervals, writing NDJSON only to stdout. It does not create a recording or start a background service. Actual CPU sampling is subject to the sysinfo platform-dependent minimum refresh interval.

Use **Left/Right** or **Tab** to change console panels and **Q** to exit. Note files are plaintext and stored under `PIPBOY_HOME` or the platform's default Rust-Boy data directory. Do not store passwords, credentials, or sensitive records in notes.

## Repository navigation

| Resource | Purpose |
| --- | --- |
| [BRANDING.md](BRANDING.md) | Name, product language, palette, identifiers, and asset boundaries |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Development, attribution, and documentation conventions |
| [SECURITY.md](SECURITY.md) | Local data, permission gates, and security reporting |
| [docs/README.md](docs/README.md) | Technical documentation index |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Core boundaries, adapters, and data guarantees |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Phased implementation and acceptance gates |
| [docs/G1_EXECUTION_PLAN.md](docs/G1_EXECUTION_PLAN.md) | Next G1 engineering slices and testable acceptance criteria |
| [docs/CI_TRIAGE.md](docs/CI_TRIAGE.md) | GitHub Actions startup investigation and local verification |
| [docs/LITERATURE_MATRIX.md](docs/LITERATURE_MATRIX.md) | Evidence matrix connecting standards and studies to modules |
| [docs/MODULE_CATALOG.md](docs/MODULE_CATALOG.md) | Planned modules, features, dependency gates and acceptance criteria |
| [docs/OBJECT_MODEL.md](docs/OBJECT_MODEL.md) | Rust traits, composition, host policy, and proposed adapters |
| [catalog/module_plan.json](catalog/module_plan.json) | Machine-readable planned features and module dependencies |
| [catalog/literature.json](catalog/literature.json) | Machine-readable engineering evidence roster |
| [docs/RESEARCH_LIBRARY.md](docs/RESEARCH_LIBRARY.md) | Evaluated third-party references and licensing policy |
| [catalog/sources.json](catalog/sources.json) | Structured source index, not vendored code |
| [crates/pipboy-core](crates/pipboy-core) | Shared data, telemetry, modules, and notes |
| [crates/pipboy](crates/pipboy) | Terminal UI and command-line entry point |

## Licensing and provenance

Original Rust-Boy code is MIT-licensed; see [LICENSE](LICENSE). Upstream projects, graphics, hardware designs, sound recordings, and trademarks retain their own rights. A repository listed in the research catalog is **not** automatically approved for source or asset reuse.

**Maintained by Paul A. Skeffington and contributors.**

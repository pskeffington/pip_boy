# Rust-Boy documentation

Technical contracts, architecture decisions, research provenance and validation evidence for the [Rust-Boy](../README.md) offline-first field console.

**Current scope:** local desktop CLI/TUI, bundled reference library, notes and synthetic-only device workflows. Real external adapters and trusted OS authorization are not available.

## Start here

| Guide | Focus |
| --- | --- |
| [Quick start](../README.md#quick-start) | Clone, build, run the terminal UI and inspect local status |
| [Console controls](CONSOLE.md) | Keyboard navigation, resize behavior and terminal restoration |
| [Architecture](ARCHITECTURE.md) | Workspace, lifecycle, local boundaries and adapter layering |
| [Roadmap](ROADMAP.md) | G0–G4 priorities, planned modules and acceptance gates |
| [Validation](VALIDATION.md) | Executed tests, exact CI runs, limitations and historic evidence |
| [Changelog](../CHANGELOG.md) | Current implemented scope and remaining work |

## Subsystems and contracts

| Topic | Documents |
| --- | --- |
| Mac → PlayStation | [Launcher, setup and protocol diagnostics](MAC_PLAYSTATION.md) |
| Fallout 4 | [Game mode setup and protocol](GAME_MODE.md) |
| Local host and UI | [Host telemetry and field quality](HOST_TELEMETRY.md) · [Object model](OBJECT_MODEL.md) · [Scoped local operations](LOCAL_OPERATIONS.md) · [Console](CONSOLE.md) |
| Data and persistence | [Storage](STORAGE.md) · [Permission preferences](PERMISSIONS.md) |
| Synthetic devices | [Simulator](SIMULATION.md) · [Virtual transport](VIRTUAL_TRANSPORT.md) · [Typed mock adapters](MOCK_ADAPTERS.md) |
| Physical-device safety | [Deny-only broker protocol](BROKER_PROTOCOL.md) · [Future OS-enforced broker requirements](OS_DEVICE_BROKER.md) |
| Implementation planning | [G1 execution plan](G1_EXECUTION_PLAN.md) · [Module catalog](MODULE_CATALOG.md) · [Research literature matrix](LITERATURE_MATRIX.md) |
| External references | [Research library](RESEARCH_LIBRARY.md) · [Source catalog](../catalog/README.md) |
| Build diagnostics | [CI historical investigation](CI_TRIAGE.md) · [GitHub Actions workflow](../.github/workflows/rust.yml) |

## Reading status labels

- **Available** means a source implementation exists and has applicable CI tests.
- **Test-only** means fixed, labeled synthetic data and in-memory/mock behavior, not real input or measurement.
- **Deny-only** means the separate broker rejects every physical-device request; it has no real authorization ability.
- **Planned** means acceptance/design work remains and no deployable physical capability is claimed.

The machine-readable `implemented_unverified` catalog label is a **module acceptance state**, not an assertion that CI did not execute. A passing test run does not automatically complete accessibility, release or hardware gates.

## Policies

[Security](../SECURITY.md) · [Contributing](../CONTRIBUTING.md) · [Branding](../BRANDING.md) · [License](../LICENSE).

The product name is **Rust-Boy**; the existing `pip_boy` repository, `pipboy` CLI and `PIPBOY_HOME` storage variable remain compatibility interfaces. Keep implementation status explicit, preserve data provenance, and obtain separate rights clearance before copying third-party software or art.

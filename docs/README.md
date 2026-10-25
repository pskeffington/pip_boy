# Rust-Boy | Documentation

Rust-Boy is an offline-first, Rust-native field systems console. This directory contains engineering documents; the top-level [README](../README.md) provides the product overview and working commands.

| Document | Focus |
| --- | --- |
| [Architecture](ARCHITECTURE.md) | Native core, interface, persistence, capability boundaries |
| [Roadmap](ROADMAP.md) | G0–G4 work packages, verification and device gates |
| [G1 execution plan](G1_EXECUTION_PLAN.md) | Ordered acceptance slices, current prototypes, remaining integration |
| [CI triage](CI_TRIAGE.md) | Startup failures, manual workflow and Rust verification commands |
| [Adapter preferences](PERMISSIONS.md) | Persistent requested/denied intent, schema limits and non-authorization boundary |
| [Simulated device broker](SIMULATION.md) | Fixed virtual fixtures, explicit mock-only confirmation and revocation checks |
| [Virtual transport](VIRTUAL_TRANSPORT.md) | Bounded fake event queue, revocation and platform validation |
| [Typed mock adapters](MOCK_ADAPTERS.md) | Input/display/serial trait contracts, fixed fixtures and OS boundaries |
| [OS device authorization](OS_DEVICE_BROKER.md) | Required real-device broker trust model and acceptance gates (not implemented) |
| [Scoped local operations](LOCAL_OPERATIONS.md) | LIBRARY/NOTES in-process permission prechecks |
| [Literature matrix](LITERATURE_MATRIX.md) | Primary research and standards mapped to module decisions and tests |
| [Module catalog](MODULE_CATALOG.md) | Features, dependency edges, permission requirements and acceptance criteria |
| [Object model](OBJECT_MODEL.md) | Rust trait architecture, host policy, adapter boundaries and lifecycle |
| [External research library](RESEARCH_LIBRARY.md) | Pip-Boy ecosystem survey, license review, source handling |

**Top-level policies:** [Brand identity](../BRANDING.md) · [Security](../SECURITY.md) · [Contributing](../CONTRIBUTING.md) · [MIT license](../LICENSE)

## Naming rule

The product is **Rust-Boy**. The repository path `pip_boy`, CLI `pipboy`, internal crate `pipboy-core`, and existing `PIPBOY_HOME` setting remain stable technical interfaces. Do not confuse naming changes with device capability or validated runtime behavior.

## Documentation policy

Every declared feature must state whether it is implemented, planned, or unavailable. Measurements require a data source. External repository references do not grant permission to copy their code, media, or product identity. Update this index when adding a new architecture or operating document.

- [Executed local validation](VALIDATION.md) — toolchains, unit checks, CLI and terminal smoke evidence.

- [Storage contract](STORAGE.md) — notes schema compatibility, contention, limits and recovery behavior.

- [Console controls](CONSOLE.md) — keyboard navigation, full note/source details and resize behavior.

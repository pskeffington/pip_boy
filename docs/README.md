# Rust-Boy | Documentation

Rust-Boy is an offline-first, Rust-native field systems console. This directory contains engineering documents; the top-level [README](../README.md) provides the product overview and working commands.

| Document | Focus |
| --- | --- |
| [Architecture](ARCHITECTURE.md) | Native core, interface, persistence, capability boundaries |
| [Roadmap](ROADMAP.md) | G0–G4 work packages, verification and device gates |
| [External research library](RESEARCH_LIBRARY.md) | Pip-Boy ecosystem survey, license review, source handling |

**Top-level policies:** [Brand identity](../BRANDING.md) · [Security](../SECURITY.md) · [Contributing](../CONTRIBUTING.md) · [MIT license](../LICENSE)

## Naming rule

The product is **Rust-Boy**. The repository path `pip_boy`, CLI `pipboy`, internal crate `pipboy-core`, and existing `PIPBOY_HOME` setting remain stable technical interfaces. Do not confuse naming changes with device capability or validated runtime behavior.

## Documentation policy

Every declared feature must state whether it is implemented, planned, or unavailable. Measurements require a data source. External repository references do not grant permission to copy their code, media, or product identity. Update this index when adding a new architecture or operating document.

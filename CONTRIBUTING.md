# Contributing to Rust-Boy

Rust-Boy is an independent Rust-native, offline-first field systems console. Contributions should preserve its modular design, data provenance, opt-in device access, and explicit feature readiness.

## Start here

Read [README.md](README.md), [BRANDING.md](BRANDING.md), [SECURITY.md](SECURITY.md), and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). Work through [docs/ROADMAP.md](docs/ROADMAP.md) acceptance gates.

Use the existing package and executable names while they are still part of the public build interface:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p pipboy -- doctor
```

## Change requirements

1. Distinguish implemented behavior from planned adapters; never insert fabricated device telemetry.
2. Keep operations local and read-only by default. Network, GNSS, serial, GPIO, BLE, SDR, and mesh functionality require explicit permission and security review.
3. Add tests for any data contract, parser, persistence change, or new module.
4. Update relevant documentation and the [brand guide](BRANDING.md) whenever public names or labels change.
5. When proposing upstream integration, include repository, exact revision, SPDX/license text, dependencies, attribution needs, and media restrictions. Missing or incompatible terms mean reference-only.
6. Do not commit API keys, live geolocation logs, credential captures, copyrighted game assets, or proprietary firmware.
7. Keep terminal commands valid and backward compatible unless a breaking change is separately approved and documented.

## Pull requests

Describe scope, checks performed, unverified items, permission impacts, and the relevant roadmap gate. A failing or unavailable CI run is **not** a passing validation result. Use **Rust-Boy** for the product name in titles and prose; keep `pip_boy` in repository URLs and `pipboy` in commands.

## Disclaimer

Rust-Boy is unofficial and independent. References to third-party Pip-Boy projects do not imply sponsorship, endorsement, license compatibility, or code reuse authorization.

# Contributing to Rust-Boy

Rust-Boy is an open-source, offline-first Rust console. Contributions are welcome when they preserve clear provenance, stable commands and the boundary between **tested local features**, **simulators** and **future physical devices**.

## Set up

Install Rust (minimum 1.81), Git and Python 3 for the optional validation scripts. The checked-in `Cargo.lock` is authoritative.

```bash
git clone https://github.com/pskeffington/pip_boy.git
cd pip_boy
cargo build --workspace --locked
cargo run --locked -p pipboy -- doctor
```

Core layout: `crates/pipboy` (CLI/TUI), `crates/pipboy-core` (local core and mock adapters), `crates/rust-boy-broker` (separate **deny-only** process), `catalog/` (evidence and planned modules), `tools/` (validation), and `docs/` (technical contracts).

## Run the checks

```bash
python3 tools/validate_catalog.py
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --locked
python3 tools/smoke_broker.py
```

GitHub Actions also runs the CLI, notes, permission, simulator, transport and mock-adapter smoke scripts; native macOS PTY and Windows ConPTY tests are platform-specific. Check [the workflow](.github/workflows/rust.yml) and [validation records](docs/VALIDATION.md). A test defined but not executed is **not** a passing result.

## Contribution requirements

1. **Be precise about status.** Do not call a planned capability available, a synthetic observation measured, or a developer build production-ready.
2. **Keep devices disabled by default.** Never introduce network acquisition, radio transmission, Bluetooth scanning, serial/GPIO access or privileged helpers through an existing `--confirm` flag. Real-device authorization needs a separately reviewed OS-enforced broker and trusted consent.
3. **Include evidence and tests.** Add deterministic fixtures and failure-path tests for new parsers, policy rules, persistence or module behavior. Update `catalog/module_plan.json` and mapped literature references when module scope changes.
4. **Protect stored data.** Preserve versioned schemas, deny unsupported input, and avoid implicit writes, uploads and credential logging. Notes are plaintext; do not add secrets or personal data to tests.
5. **Respect reuse boundaries.** Include an upstream URL, revision, SPDX/license status and notice requirements before proposing third-party code. Referenced repositories and assets are not automatically approved for inclusion.
6. **Preserve compatibility.** The public brand is Rust-Boy; `pipboy`, `pipboy-core`, `rust-boy-broker` and `PIPBOY_HOME` are current technical interfaces. Document any proposed breaking change.

## Pull-request checklist

- [ ] Explain the problem, scope, affected modules and user-visible behavior.
- [ ] Link the roadmap gate, design evidence and relevant docs.
- [ ] Include platform-specific passing checks **and** explicitly list what has not been validated.
- [ ] Describe changes to permissions, persistence, real-world device access or external dependencies.
- [ ] Update the README, documentation index and [CHANGELOG](CHANGELOG.md) when public behavior changes.

For architecture, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). For security reports, use the private channel described in [SECURITY.md](SECURITY.md), not a public issue with sensitive details.

Rust-Boy is an unofficial, independent project; third-party references do not imply endorsement or permission to copy proprietary artwork or firmware.

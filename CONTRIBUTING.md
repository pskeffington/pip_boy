# Contributing to Rust-Boy

## Mainline module integration

Keep `main` as the current development preview. Build each module on a focused branch, update the README, documentation index, changelog and affected operator contracts, then merge after all applicable quality jobs pass at the proposed head. Software integration does not assert physical controller, radio, touchscreen or game acceptance; record those field gates explicitly instead of leaving verified software permanently on stacked branches.

The shared Network Engineer view uses Rust `network_suite::OPERATIONS`. Keep controller inventory on the existing `pipboy_core::unifi` adapter and endpoint measurements on `endpoint_probe`; do not add another incompatible credential/configuration path. Run `node tools/test_network_ui.cjs`, `python3 tools/smoke_network.py` and `python3 tools/smoke_network_health.py` alongside the workspace checks. NET ENGINEER's opt-in local scan and UniFi handlers belong to `crates/pipboy/src/dashboard/network_routes.rs` behind the existing `dashboard.rs` Host/Origin gate. The extraction must not introduce an additional network listener, automatic probes, an alternate controller credential path, or a change to explicit consent.

Rust-Boy is an open-source, offline-first Rust console. Contributions are welcome when they preserve clear provenance, stable commands and the boundary between **tested local features**, **simulators** and **future physical devices**.

## Security and privacy changes

Treat all operator messages, task boards, peer rosters, GIS location exports, inventory snapshots and device credentials as private, even when a test looks synthetic. Keep public fixtures explicitly invented and license-cleared. Never use actual `PIPBOY_HOME` files in test fixtures or PR attachments. Run `python3 tools/validate_security.py` and `python3 tools/test_validate_security.py` before pushing. This check rejects high-confidence token formats and tracked runtime filenames; it does **not** inspect Git history or replace GitHub secret scanning. Security findings involving real credentials should be reported privately, and the credentials rotated, not copied into public issues.

Any new external subprocess must use `pipboy_core::process_safety::isolated_command` to avoid passing the UniFi credential and other commonly named API tokens through the child environment. Adding or changing external network providers, logging of coordinates, data retention, or local API authentication needs a security review and regression test.

## Repository and documentation hygiene

Place implementation in the owning crate, browser assets in the existing shared dashboard/micro shells, and machine-readable research in `catalog/`. Use the [repository map](docs/REPO_STRUCTURE.md) and avoid moving existing on-disk stores or duplicating role/network catalogs in JavaScript. For new externally visible subsystem entrypoints, update the [component index](catalog/component_index.json), name the related validation tests and retain separate hardware acceptance status. Keep the root README concise; module contracts belong under `docs/`.

Changes to top-level navigation or local Markdown links must pass `python3 tools/validate_repo.py` and `python3 tools/test_validate_repo.py`. The check is offline and covers required navigation, reference-style and inline local links, and nested public Markdown under `docs/`, `catalog/` and optional `data/`. It never traverses `PIPBOY_HOME` and does not certify third-party URLs or hardware.

## Set up

Install Rust (minimum 1.90), Git and Python 3 for the optional validation scripts. The checked-in `Cargo.lock` is authoritative.

```bash
git clone https://github.com/pskeffington/pip_boy.git
cd pip_boy
cargo build --workspace --locked
cargo run --locked -p pipboy -- doctor
```

Core layout: `crates/pipboy` (CLI/TUI), `crates/pipboy-core` (local core and mock adapters), `crates/rust-boy-broker` (separate **deny-only** process), `catalog/` (evidence and planned modules), `tools/` (validation), and `docs/` (technical contracts).

## Peer transport and trusted remote commands

The optional SSH bridge is implemented in `crates/pipboy/src/peer.rs` with validated configuration in `pipboy-core::peers`; typed durable receive/report storage belongs to `pipboy-core::tasks`. Do not add an unauthenticated listener, TOFU host-key enrollment, a shell-based arbitrary remote command or implicit task send on role load. Preserve the restricted `peer bridge SOURCE` command allowlist, idempotent receiver storage, and the distinction between a stored-delivery receipt and manual acknowledgment. Run `python3 tools/smoke_peer.py` and review [peer security](docs/PEER_TASK_TRANSPORT.md). The CI SSH substitute is not a cryptographic verification test.

## Task Control development

The separate `crates/pipboy-core/src/tasks.rs` owns all validation, durable storage, revision-conflict checks and human handoff text. The local browser UI lives in `crates/pipboy/assets/tasks.{js,css}` and is shared by the desktop and micro renderers. Do not duplicate task state inside profile JSON, confuse an email draft with a send, or mark manual acknowledgment as verified. Follow [Task Control security contract](docs/TASK_CONTROL.md); test `python3 tools/smoke_tasks.py` after compiling the Rust binary.

## Micro UI development

The micro renderer is separately maintained at `crates/pipboy/assets/micro.{html,css,js}`; the Rust server serves it at `/micro` without creating a second backend. Source roles, tools, availability and permission settings **must** continue to come from the Rust API, not a duplicate JavaScript role catalog. The same confirmed profile-change API and local-workbench storage are shared with desktop. Keep bottom tabs and touch targets stable at 480×320 and 800×480, preserve keyboard and reduced-motion behavior, and never display fabricated camera frames, battery charge, GPS fixes or RustGIS maps.

To test: `cargo run --locked -p pipboy -- dashboard --micro --open` on macOS; `node --check crates/pipboy/assets/micro.js`; `python3 tools/smoke_micro.py` after a build. Before merging, manually inspect both handheld widths and native Pi hardware when available. See [micro acceptance plan](docs/MICRO_UI.md).

## Dashboard development

The [graphical dashboard](docs/DASHBOARD_UI.md) is a local browser shell served by `crates/pipboy/src/dashboard.rs`. UI assets live under `crates/pipboy/assets/`; do not add external font/CDN/API references or new network routes without review. Task/peer handlers are in `crates/pipboy/src/dashboard/task_routes.rs` and Messenger/PACE handlers in `crates/pipboy/src/dashboard/messenger_routes.rs`; preserve the single parent Host/Origin gate and explicit confirmation on every mutation. The source split does not authorize new sockets or background message delivery. The job-template source of truth is `pipboy-core::workbench::PRESETS`, and physical/simulated availability is `PALETTE`. Do **not** add a hard-coded duplicate role list to JavaScript; keep Custom last.

Start with `cargo run --locked -p pipboy -- dashboard --open` on Mac; run `python3 tools/smoke_dashboard.py` after building the binary. Only allowlisted, bounded, explicitly requested Rust core operations can run from the dashboard. Scope non-loopback access, device transport, authentication, RustGIS/TAK/video integration and any storage of sensitive user information as separate changes.

## Run the checks

```bash
python3 tools/validate_catalog.py
python3 tools/validate_component_index.py
python3 tools/test_component_index.py
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --locked
python3 tools/run_smokes.py --check
python3 tools/test_run_smokes.py
python3 tools/run_smokes.py linux  # Linux; use 'native' on macOS/Windows
```

GitHub Actions uses the [single smoke-test roster](tools/run_smokes.py) for 19 Linux and 17 native-platform CLI/API checks; native macOS PTY and Windows ConPTY tests remain separate. Adding a new `smoke_*.py` script requires explicit roster classification. [CI inventory and historical triage](docs/CI_TRIAGE.md). Check [the workflow](.github/workflows/rust.yml) and [validation records](docs/VALIDATION.md). A test defined but not executed is **not** a passing result.

## Role feature contract

A new role needs an entry in [catalog/role_roadmap.json](catalog/role_roadmap.json), a cited [lore classification](docs/VAULT_TEC_LORE_MATRIX.md), an [operational R0–R3 roadmap](docs/ROLE_ROADMAP.md), explicitly permitted default tools, and real acceptance evidence. The role validator compares the manifest against `PRESETS`; run `python3 tools/validate_roles.py` alongside other offline checks. Security camera integrations additionally require [camera-media acceptance](docs/SECURITY_CAMERA_WALL.md). No new role may silently grant network or hardware permissions.

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

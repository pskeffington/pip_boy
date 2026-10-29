# Rust-Boy repository structure and maintenance

Rust-Boy keeps a deliberately small top level. **Do not relocate operational data paths or rename crates to improve appearance**: the `pipboy` CLI, `pip_boy` repository, and `PIPBOY_HOME` environment variable are compatibility contracts.

## Canonical ownership

| Path | Purpose | Change rule |
| --- | --- | --- |
| [`../crates/pipboy-core`](../crates/pipboy-core) | Shared Rust model, policy, persistence and capability definitions | Keep business logic out of browser JavaScript; add unit tests with changes |
| [`../crates/pipboy`](../crates/pipboy) | Native CLI/TUI, loopback HTTP server, embedded dashboard/micro assets | Keep endpoints bounded and operator gated; share one core backend |
| [`../crates/rust-boy-broker`](../crates/rust-boy-broker) | Deny-only peripheral broker prototype | Do not enable physical I/O without separate threat review |
| [`../crates/rust-boy-adapter-sdk`](../crates/rust-boy-adapter-sdk) | Portable versioned module socket SDK | Keep lean and host-agnostic; no device, networking or UI renderer dependencies |
| [`../catalog`](../catalog) | Machine-readable evidence, sources, modules and role roadmaps | Stable IDs and source provenance; run catalog and role validators |
| [`../AGENTS.md`](../AGENTS.md) | Codex agent instructions and exact-head pass discipline | Read the machine backlog first, keep unmerged PRs separate |
| [`../catalog/codex_passes.json`](../catalog/codex_passes.json) | Codex pass status/dependency/index contract | Exactly one eligible `next`; checked paths, owners and evidence |
| [`README.md`](README.md) | Operator, architecture and acceptance contracts | Add a link to this index for new operator-facing guides |
| [`../tools`](../tools) | Offline validation, smoke tests and fixture helpers | Prefer Python standard library for manifest checks; no network activity in validators |
| [`../scripts`](../scripts) | Explicit local entry-point launchers | Keep launchers portable and free of implicit network actions |
| [`../examples`](../examples) | Clearly labeled deterministic examples | No private telemetry, credentials, or user data |
| [`../.github/workflows/rust.yml`](../.github/workflows/rust.yml) | CI build, Rust/toolchain and navigation gates | New quality checks must execute on pull requests |

Operator state belongs under **`PIPBOY_HOME`**, not in source control. This includes notes, task boards, peer configuration, messaging queues, geospatial caches, diagnostics and other local records. See [storage](STORAGE.md) and [security](../SECURITY.md). Do not add example files taken from real users or field devices.

The [universal adapter contract index](../catalog/host_adapter_contract.json) is the machine-readable socket vocabulary; the [implementation guide](UNIVERSAL_ADAPTER.md) owns the phase and security explanation. `tools/validate_host_packages.py` checks both the manifests and SDK boundary without executing a module.

## Machine pass navigation

The [Codex passes](CODEX_PASSES.md), [index hierarchy](INDEXING.md), [root Codex instructions](../AGENTS.md) and [machine backlog](../catalog/codex_passes.json) are a **branch-specific development index**, complementary to the component owner list. They cannot grant runtime package permissions or certify hardware. Run `python3 tools/validate_codex_passes.py` and `python3 tools/test_validate_codex_passes.py` after every queue edit. The root dashboard HTTP entrypoint, `crates/pipboy/src/dashboard.rs`, is owned by `local-hud`; the universal-adapter component references it via documentation instead of claiming duplicate code ownership.

## Documentation entrypoints

Start at the [project README](../README.md), then the [documentation index](README.md), [architecture](ARCHITECTURE.md), [storage contract](STORAGE.md), and [validation evidence](VALIDATION.md). Keep the root README as a concise operator-oriented landing page; put module internals and research matrices under `docs/` or `catalog/` instead of appending extensive technical content to the repository root.

## Synchronization module ownership

The offline-only sync v2 source boundaries are [`sync_delta.rs`](../crates/pipboy-core/src/sync_delta.rs) for bounded cursor planning and [`sync_ledger.rs`](../crates/pipboy-core/src/sync_ledger.rs) for cooperative locked local snapshots. Neither is a live transport or authorization authority. Keep peer SSH and Messenger v1 compatibility unchanged while integration gates are open. The [sync integration guide](RB_SYNC_INTEGRATION.md) documents module consumers, storage preconditions and regression commands.

## HTTP route ownership

`crates/pipboy/src/dashboard.rs` is the single loopback HTTP entrypoint and remains responsible for request bounds, host/origin validation and route dispatch. Task Control CRUD, manual handoff and explicit peer-send/receipt-sync implementations live in [`../crates/pipboy/src/dashboard/task_routes.rs`](../crates/pipboy/src/dashboard/task_routes.rs). This is a **source-module extraction**, not a new API server: callers still use the same `/api/tasks`, `/api/tasks/*`, `/api/peers` and `/api/peer/*` URLs. Messenger/PACE read/send/retry handlers are independently owned by [`../crates/pipboy/src/dashboard/messenger_routes.rs`](../crates/pipboy/src/dashboard/messenger_routes.rs), with the unchanged `/api/messages`, `/api/messages/send` and `/api/messages/retry` endpoints. The parent retains HTTP parsing and same-origin enforcement; the Messenger adapter delegates core policy to `pipboy-core` and explicit SSH dispatch to `peer.rs`. No route family creates a second HTTP listener. Other route families remain in the parent until separate contract-preserving extraction passes tests.

NET ENGINEER's opt-in private-LAN scan and read-only UniFi HTTP handlers live in [`../crates/pipboy/src/dashboard/network_routes.rs`](../crates/pipboy/src/dashboard/network_routes.rs). The parent server still validates exact Host/Origin and request size before dispatch; `pipboy-core::network_probe` and `pipboy-core::unifi` remain the sole probe/controller implementations. Open workspaces and rejected requests never initiate network activity. No new socket/listener, credentials file or user-facing API path was created by this extraction.

## Machine-readable component responsibility map

The [component ownership and acceptance index](../catalog/component_index.json) tracks mainline subsystem entrypoints, their engineering guides, tests, responsibility boundaries and physical-hardware gates. It is intentionally **not** an exhaustive inventory of every source file or a license/permissions roster. The [organizational optimization research](ORGANIZATIONAL_OPTIMIZATION.md) explains the evidence and proposed sequencing; [ADR 0001](adr/0001-mac-first-portable-core.md) records the Mac-first/Pi-later decision.

Run `python3 tools/validate_component_index.py` and `python3 tools/test_component_index.py` when an externally visible component entrypoint, guide, test or hardware-gate label changes. CI validates both using standard-library-only, offline checks. Keep Raspberry Pi acceptance distinct from portable code compilation; `field_pending` identifies open device acceptance and `deny_only` is not a grant. Avoid unreviewed duplicate ownership of the same source entrypoint.

## Change and cleanup controls

1. **Avoid duplicate authority.** Rust core owns module catalogs and permissions; desktop and micro shells consume the same APIs. Never fork another independent JavaScript roster of roles or network tools.
2. **Keep compatibility.** Do not move existing `catalog/*.json` inputs, change stored JSON schemas or rename public CLI flags without migration and tests.
3. **Separate previews.** Radio, mapping, messaging and indexing work can remain on their own feature branches until their exact-head CI and relevant hardware gates are understood. Do not collapse unrelated open PRs into cleanup work.
4. **Treat tracked fixtures as public.** Clean out caches, temporary outputs and secrets from local working trees; do not delete durable operator records as part of a repository tidy.
5. **Validate on changes.** Run `python3 tools/validate_repo.py` and `python3 tools/test_validate_repo.py` after editing the root or documentation navigation, alongside `python3 tools/validate_catalog.py`, `python3 tools/validate_roles.py`, and the Rust workspace checks.

The repository check validates mandatory navigation, root-level Markdown and nested Markdown under `docs/`, `catalog/`, and an optional public `data/` directory. It checks inline and reference-style local link destinations while ignoring fenced examples, external URLs and footnotes. Missing paths, escaping links and symlinked documentation are rejected. The scope never extends into `PIPBOY_HOME` or other operator data. The [CI smoke roster](../tools/run_smokes.py) is the authoritative list for the shared Linux and native-platform CLI checks. Register every new `smoke_*.py` explicitly and run `python3 tools/run_smokes.py --check`; see [CI troubleshooting and coverage](CI_TRIAGE.md). These checks confirm source-tree link integrity and test coverage registration, **not** external URL validity, physical hardware acceptance or a finished release.

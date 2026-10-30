# Rust-Boy repository agent instructions

Scope: the entire `pip_boy` repository. More-specific nested agent instructions may refine, but must not weaken, the security and acceptance gates here.

## Mission and workflow

Develop the Rust-Boy host into a production-capable, offline-first, one-package-at-a-time shell. The terminal, full-screen TUI, desktop Vault Command Center, and micro HUD must share **one host-owned typed command catalog**, common capability checks, and the approved native Rust handlers. MAP, RADIO, TV and COMMS remain separately owned packages: never pretend reference manifests are installed packages.

**GitHub-first development:** make source, documentation, tasking, reviews and CI evidence authoritative in GitHub. Do not require the operator to run local side-by-side programs, paste terminal sessions or perform manual Mac testing to advance **software-only** gates. Keep physical-device acceptance separate and honestly pending. Never claim a CI pass if no workflow job actually executed.

1. Read `README.md`, `docs/README.md`, `docs/HOST_PACKAGE_ROADMAP.md`, `docs/UNIVERSAL_ADAPTER.md`, `docs/TERMINAL_MODULE_COMMANDS.md`, and `docs/TERMINAL_AGENT_EXECUTION.md` before a terminal/adapter change.
2. Select the earliest unblocked task in `catalog/terminal_agent_tasks.json`. Inspect the repository's current HEAD and existing PRs/issues first; avoid duplicative branches or speculative refactors.
3. Use a short task-specific branch/PR for unverified code, or an authorized direct change to `main` when appropriate. Keep branch count low. Do not bypass protection, alter CI workflow gates to conceal failures, or force-push unrelated changes.
4. Implement the smallest vertical slice with tests and proof; update task status only with links to real code/test evidence. If CI is blocked by startup_failure or has zero jobs, mark `blocked_ci`, not `passed`.
5. On every pass update relevant docs, machine-readable index, command help, compatibility notes and release evidence; explicitly list what is implemented, tested, simulated, not connected, or still requires hardware verification.

## Terminal command and module rules

- `crates/pipboy/src/terminal_commands.rs` is a provisional offline-only command prompt, NOT the final shared dispatcher. Do not copy or embed its verb handling separately in TUI/GUI. Extract a typed parser/catalog/dispatcher into `pipboy-core` first, then migrate callers one at a time with compatibility tests.
- The saved `WorkbenchStore` pins define selectable tools, not authorization. Toolset selection never starts operations or grants privileges.
- Use `UniversalAdapterHost` for host-owned trusted adapter lifecycle. Discovered/reference packages are not executable, installed or implicitly trusted. No arbitrary shell, command-string subprocess launch, dynamic repo download, implicit playback/scan/send, automatic location upload, or fabricated stream/device success.
- Typed commands have explicit handler ID, argument bounds, side-effect class, required scope, confirmation, connectivity state and structured outcomes. Handle interrupted remote operations as UNKNOWN OUTCOME; never call them delivered/succeeded without receipts.
- Maintain one foreground package; Settings is always host-owned. Do not reintroduce stacked headers, map overlays or root-page scrolling.
- Keep radio/TV/GIS/COMMS internals in their own repositories; integration uses reviewed local interfaces, with no side effects from simply viewing a module.
- Do not use external code repositories for runtime installation or build-time tooling unless already pinned and explicitly approved; migration toward first-party/local tool ownership must have a verified replacement plan. Do not rewrite cryptography, network stacks or codecs without independent interoperability and security validation.

## Agent ownership and order

The single coordinator owns the queue, cross-module contracts, GitHub PR merge decisions and status updates. Delegate *non-overlapping* tasks:
- **CORE agent:** TC-A01 typed catalog, validation and dispatcher in `pipboy-core`.
- **TERMINAL agent:** TC-A02 migration of `pipboy terminal` and full-screen TUI after TC-A01.
- **ADAPTER agent:** TC-A03 approved RADIO/MAP native handlers and lifecycle, then TV/COMMS only after actual installed handler contracts; depends on TC-A01.
- **SECURITY/QA agent:** TC-A04 negative tests, permission/revocation and CI evidence, can begin independently; gates every merge.
- **GUI agent:** TC-A05 shared desktop/micro command palette after TC-A01 and TC-A02; no duplicate dispatcher.
- **RELEASE agent:** TC-A06 documentation, regression, CI and release matrix after predecessors.

Assign concrete file ownership per PR; if two agents need the same file, serialize their changes or coordinate a single PR. Always inspect versioned source before changing it.

## Minimum software gate

Run through CI (or reproducible authorized execution where available):
```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo +1.90.0 check --workspace --locked
```

No side effects from select/help/tab; unknown/unpinned/uninstalled commands denied; explicit authorization for network, audio, messaging, hardware and filesystem writes; malformed/oversized input rejected; tested offline/error/restart/timeout paths; stable command compatibility for existing CLI routes. Test Rust + browser surfaces where applicable. Record exact run/job URL, SHA, test totals and exceptions. A `startup_failure` with zero jobs is **not** a successful validation. Do not rewrite workflow gates speculatively to make badges green.

## Communication and completion

Continue through unblocked tasks without demanding operator live-side testing. When blocked, leave an actionable GitHub issue with exact blocker/evidence and move to a different independent task. Physical Mac/Pi, paired-device, clinical or external-source acceptance remains explicitly pending until real evidence exists. Never mark a production release based on unit tests alone.

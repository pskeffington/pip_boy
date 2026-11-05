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

### Current execution checkpoint — PR #100 merged

- PR [#92](https://github.com/pskeffington/pip_boy/pull/92) merged at `22c43f835465a5656b1c907f44eb853b754d0c2b`; desktop/micro shared command palette and typed dispatcher are in `main`.
- PR [#93](https://github.com/pskeffington/pip_boy/pull/93) merged at `00582b9c19743302863a0fac3249e760c3ee69df`; the native Mac app packaging path is implemented, while Developer ID signing/notarization and clean-account Gatekeeper/accessibility acceptance remain open. Credentials are tracked in [issue #94](https://github.com/pskeffington/pip_boy/issues/94). No signed artifact or production readiness is claimed.
- PR [#96](https://github.com/pskeffington/pip_boy/pull/96) merged at `ee04da59f1bdb1a20f097b37271a992bcdae81a3`. Exact head `842eb38dad53a683422c129de3341fa6ad22cb3e` passed 7/7 jobs in [Actions run 41958732527](https://github.com/pskeffington/pip_boy/actions/runs/41958732527). It adds a bounded recorded-capture Meshtastic text decoder only; no serial/network connection, transmission, persistence, authentication, or hardware acceptance is included.
- A read-only Mac OS inventory observed a WisCore RAK4631 board (`239A:8029`) and `/dev/cu.usbmodem1101`; the serial endpoint was not opened. This observation is not device authorization or acceptance evidence.
- The operator also reports a WisMesh Station-family system on the LAN. PR [#99](https://github.com/pskeffington/pip_boy/pull/99) merged at `721c072f719852c7a193449ab93ce9fb0ea41a4b`; exact head `5f9c4c1e6884638e7d6d7a09dabd3f282838afc5` passed 7/7 jobs in [Actions run 41963337334](https://github.com/pskeffington/pip_boy/actions/runs/41963337334), accepting only the LAN receive design. The operator confirmed the observed station web certificate fingerprint out of band; the supplied root returned a generic web UI and the supplied broadcast route returned 404. No other paths or message data were accessed. The enabled service/topic/ACL/payload remain unknown, and no LAN broker or radio operation is implemented. The normal Meshtastic TCP client flow can send configuration/heartbeat traffic; do not call that receive-only.
- PR [#100](https://github.com/pskeffington/pip_boy/pull/100) merged at `0c87af0fea31dbf373a2f3e963ff7d8521aa5134`; exact head `4a280a8edc89a52a0f1741aa077de7fc26f3faa3` passed 7/7 jobs in [Actions run 41964063909](https://github.com/pskeffington/pip_boy/actions/runs/41964063909). It reconciles the verified web-root observation and keeps MESH-06 blocked on service identity and the OS broker.
- PR [#101](https://github.com/pskeffington/pip_boy/pull/101) adds the MESH-07 offline Rust scope/freshness contract; exact head `fdfb2589473c3505026a7dce965b85c3156a027a` passed 7/7 jobs in [Actions run 41965952404](https://github.com/pskeffington/pip_boy/actions/runs/41965952404). The queue marks only this contract `accepted_software`. It has no transport-opening API; do not treat it as an authenticated grant, OS enforcement, live connection, or hardware acceptance.

The [November 3 execution packet](docs/CODEX_PASS_2026-11-03.md) preserves its original PR reconciliation and includes the post-packet integration addendum. Open drafts must be synchronized and revalidated before merge; exact workflow success is not merge or production evidence.

Mac product distribution uses [docs/MAC_RELEASE.md](docs/MAC_RELEASE.md) and [catalog/mac_release_tasks.json](catalog/mac_release_tasks.json). Do not publish an unsigned bundle. Signing/notarization, clean-account Gatekeeper launch, bundle identity, update policy, privacy and accessibility acceptance are independent release gates.

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
- **HARDWARE_PI agent:** Prepare repeatable Pi 5 HDMI evidence capture; no physical result without actual-device records.
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

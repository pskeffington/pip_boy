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

### RB-G5 consolidation checkpoint — 2026-11-09

The current reviewed GitHub `main` baseline is `651b0b07e02ac60132ab2eae9f6cee60ce9ae14f`. PRs #104, #95 and #105–#108 were merged; #105–#108 each had 7/7 successful exact-head Actions jobs. PR #108 additionally passed local formatting, Clippy, 441 workspace tests and Rust 1.90 checking. Superseded PRs #80/#81/#82/#86 and #91 are closed. These are software and integration facts only, not Pi 5 field or production acceptance.

**Execute [RB-G5 agent work packets](docs/RBG5_AGENT_TASKING_2026-11-09.md) before older work packets.** Coordinator must revalidate actual current head and PR status, then assign RBG5-00 (evidence inventory) first; RBG5-01 (#89/#79/#71), RBG5-02 (#66–#68), RBG5-03 (other conflicts), RBG5-04 (#83/#87 SCAN review), RBG5-05 (#70 missing CI), RBG5-06 (Pi 5 runbook), and RBG5-07 (release security/CI evidence) are scoped follow-ons. Preserve the original checkout's uncommitted work. A new head requires its own CI; a missing run is never a pass. Older dated checkpoints below are retained as historical snapshots, not current marching orders.

RBG5-00 coordinator verification is recorded in the task packet and current task catalog. The tasking PR's original head `8337ff6c22195c301f56261da53b652142614101` passed 7/7 jobs in [run 42694024298](https://github.com/pskeffington/pip_boy/actions/runs/42694024298); an independent review found stale #86/#91 labels in the retained November 3 snapshot, so a corrective head is being prepared and must receive its own CI before merge. Current #66/#67 formatting failures, #68's historical Clippy lint already fixed on current `main` but stale PR head, #70 zero-job startup failure, and the #83/#87 review dependency are documented as blockers; none is counted as accepted.

### Current execution checkpoint — 2026-11-05 (main at `017614dc`)

- PR [#92](https://github.com/pskeffington/pip_boy/pull/92) merged at `22c43f835465a5656b1c907f44eb853b754d0c2b`; desktop/micro shared command palette and typed dispatcher are in `main`.
- PR [#93](https://github.com/pskeffington/pip_boy/pull/93) merged at `00582b9c19743302863a0fac3249e760c3ee69df`; the native Mac app packaging path is implemented, while Developer ID signing/notarization and clean-account Gatekeeper/accessibility acceptance remain open. Credentials are tracked in [issue #94](https://github.com/pskeffington/pip_boy/issues/94). No signed artifact or production readiness is claimed.
- PR [#96](https://github.com/pskeffington/pip_boy/pull/96) merged at `ee04da59f1bdb1a20f097b37271a992bcdae81a3`. Exact head `842eb38dad53a683422c129de3341fa6ad22cb3e` passed 7/7 jobs in [Actions run 41958732527](https://github.com/pskeffington/pip_boy/actions/runs/41958732527). It adds a bounded recorded-capture Meshtastic text decoder only; no serial/network connection, transmission, persistence, authentication, or hardware acceptance is included.
- A read-only Mac OS inventory observed a WisCore RAK4631 board (`239A:8029`) and `/dev/cu.usbmodem1101`; the serial endpoint was not opened. This observation is not device authorization or acceptance evidence.
- The operator also reports a WisMesh Station-family system on the LAN. PR [#99](https://github.com/pskeffington/pip_boy/pull/99) merged at `721c072f719852c7a193449ab93ce9fb0ea41a4b`; exact head `5f9c4c1e6884638e7d6d7a09dabd3f282838afc5` passed 7/7 jobs in [Actions run 41963337334](https://github.com/pskeffington/pip_boy/actions/runs/41963337334), accepting only the LAN receive design. The operator confirmed the observed station web certificate fingerprint out of band; the supplied root returned a generic web UI and the supplied broadcast route returned 404. No other paths or message data were accessed. The enabled service/topic/ACL/payload remain unknown, and no LAN broker or radio operation is implemented. The normal Meshtastic TCP client flow can send configuration/heartbeat traffic; do not call that receive-only.
- PR [#100](https://github.com/pskeffington/pip_boy/pull/100) merged at `0c87af0fea31dbf373a2f3e963ff7d8521aa5134`; exact head `4a280a8edc89a52a0f1741aa077de7fc26f3faa3` passed 7/7 jobs in [Actions run 41964063909](https://github.com/pskeffington/pip_boy/actions/runs/41964063909). It reconciles the verified web-root observation and keeps MESH-06 blocked on service identity and the OS broker.
- PR [#101](https://github.com/pskeffington/pip_boy/pull/101) adds the MESH-07 offline Rust scope/freshness contract; exact head `fdfb2589473c3505026a7dce965b85c3156a027a` passed 7/7 jobs in [Actions run 41965952404](https://github.com/pskeffington/pip_boy/actions/runs/41965952404). The queue marks only this contract `accepted_software`. It has no transport-opening API; do not treat it as an authenticated grant, OS enforcement, live connection, or hardware acceptance.
- PR [#102](https://github.com/pskeffington/pip_boy/pull/102) merged the latest Mac packaging evidence; its exact head passed all 7 jobs in [Actions run 41970133790](https://github.com/pskeffington/pip_boy/actions/runs/41970133790), including native macOS universal app validation. Local ARM app smoke exited 0; local universal validation could not write a missing Cargo crate into the restricted user registry cache.
- PR [#103](https://github.com/pskeffington/pip_boy/pull/103) merged at `017614dcf722f6a9547444b76ff2aaf358b6125e`; exact head `51c6a7c12a3a290adf3598df4646994f67dadd66` passed all 7 jobs in [Actions run 41971027779](https://github.com/pskeffington/pip_boy/actions/runs/41971027779). It clarifies that Developer ID signing and notarization credentials are separate and records Apple's supported notarytool credential profiles. No signing/notarization credentials were used; MAC-REL-02 remains blocked on provisioning.

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

## Central agent-skills doctrine (Codex development)

**Mandatory for every Codex development task in this repository.** The canonical reusable agent and skill catalog is [pskeffington/agent_skills](https://github.com/pskeffington/agent_skills). The central registry is the source of reusable procedures; this repository's own AGENTS.md, security controls, ownership boundaries, release gates, and task-specific acceptance criteria remain authoritative and must not be weakened.

1. **Discover before editing.** Identify the repository's active task and read its local AGENTS.md, applicable nested instructions, task tracker, and existing tests. Resolve relevant registered skills/agent roles from `agent_skills` before planning or changing code. Record the registry commit SHA and the exact skill paths/revisions used. Do not assume an unverified skill path exists.
2. **Plan with agents.** Assign appropriate architecture, implementation, testing, and independent-review roles based on task complexity. Use specialized skills (Rust, audio, biomedical, GIS, security, etc.) when applicable; do not substitute a generic coding pass where a registered specialist exists.
3. **Execute with evidence.** Use isolated branches/worktrees, bounded tasks, reproducible commands, and truthful evidence. In PR descriptions record task ID, skills/roles invoked, registry revision, validation commands/results, failures and blockers. Merely mentioning a skill is not proof of executing it.
4. **Validate independently.** Run applicable unit/integration/security/CI checks. Have a separate review role check the implementation against acceptance criteria. A missing tool, unavailable skill or unrun test is a documented BLOCKED state, never a claimed pass.
5. **Learn without weakening controls.** Feed sanitized, reproducible failures and repairs into the FACTORY-002 learning process in `agent_skills`. Proposed skill improvements require independent evaluation and review. Do not transmit private code, credentials, operational secrets, protected research data or sensitive personal information to a public registry.
6. **Fail closed on agent noncompliance.** If required skill resolution or execution is unavailable, record BLOCKED-AGENT and do not claim an agent-compliant release. Once FACTORY-003's trusted `agent-compliance` check and branch protections are deployed, treat that check as a mandatory merge gate. Until then, these instructions are policy, not proof that technical enforcement is enabled.

**Precedence:** immutable safety/authorization boundaries and repository-specific security/acceptance gates take precedence over generic central guidance. Central skills augment, not replace, domain-specific controls. Agents must not relax approvals, branch protection, CI, or deployment policy to finish a task.

References: [FACTORY-002 learning](https://github.com/pskeffington/agent_skills/issues/14) and [FACTORY-003 enforcement](https://github.com/pskeffington/agent_skills/issues/15).

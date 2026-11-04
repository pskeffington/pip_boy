# Codex pass — 2026-11-03: integrate TUI, reconcile host PRs, gate Pi 5

Status: execution instructions and agent tasking, not implementation or acceptance evidence.
Base: inspect current `main`, all open PRs and exact-head CI before execution.
Canonical sources: `AGENTS.md`, `catalog/terminal_agent_tasks.json`, `docs/POLISHING_PASS_2026-10-31.md`, `docs/HOST_PACKAGE_ROADMAP.md`, `docs/PI5_REFERENCE_TEST_BASELINE.md`.

## Codex pickup prompt (execute, do not merely plan)

You are lead coordinator for `pskeffington/pip_boy`. Continue Rust-Boy toward a native, offline-first, single-foreground-package host with persistent Settings and shared typed command dispatch. Work GitHub-first; do not require operator live side-by-side testing for software-only work.

**Phase 0: reconcile reality.** Fetch latest `main`, enumerate open PRs/issues and branch heads; read `AGENTS.md` and canonical task/catalog docs. In particular reconcile PR #88 (supervised package host and terminal routing), #89 (production roadmap and Pi hardware inventory), #86 (adapter recovery), #87 (inert SCAN registry), #83 (SCAN plan), and any newer/merged PRs. PR #88 reports a terminal migration, but `main` queue still lists TC-A02 open; determine whether its scope is partial and do not overwrite or duplicate changes. Repair stale docs only after source comparison. Preserve distinct source SHA, merged SHA, and exact-head CI evidence.

**Phase 1: implement earliest unblocked vertical slice.** Prioritize TC-A02 full-screen `tui` command palette, unless already implemented in a newer merged PR. Reuse `pipboy-core::command_catalog` typed descriptors/parser/dispatcher. Slash opens scoped palette; Esc cancels without dispatch; Enter on a selection never triggers a side-effectful handler. Preserve direct terminal tool/verb compatibility, `diag host` alias, bounded input/output, keyboard focus and small-screen rendering. No duplicate handler logic; add behavioral fixtures for CLI/TUI parity, prompt injection, unknown/unpinned commands, offline restart and zero-I/O navigation. Keep file ownership to TERMINAL; SECURITY_QA owns independent black-box negatives.

**Phase 2: package lifecycle and UI.** After inspecting PR #88 and related fixes, determine missing work for actual installed roster, activation states and error recovery. Never treat reference or scanned manifests (including SCAN) as installed, trusted or executable. Maintain exactly one foreground package, host-owned Settings, no MAP overlays/location controls, no root document scrolling. GUI tests at 480x320, 800x480, 1024x768 and 1440x900. Explicit authorization before audio, network, camera, location, storage or message actions. Keep external RADIO/GIS/TV/COMMS implementations in their own repos.

**Phase 3: Raspberry Pi hardware gate preparation.** Treat Pi 5 plus full-sized HDMI monitor/keyboard/mouse as primary reference bench; Pi 4 and unidentified Pi 3 variant are compatibility-only. Review `catalog/hardware_test_inventory.json` and validator work from #89 after merge/reconciliation. Produce reproducible native Pi 5 smoke commands, logs schema and pass/fail checklist, but mark physical results PENDING until recorded on the actual unit. Touchscreen, SDR/GNSS, real audio, camera, paired devices, emergency signaling and operator security acceptance remain separate gates.

## Agent assignments (one coordinator; nonoverlapping ownership)

| Agent | Next unit of work | Gate |
| --- | --- | --- |
| COORDINATOR | Compare merged/active PR scope to main and task JSON; maintain branch order; publish evidence table | No duplicate PRs or fabricated completions |
| TERMINAL | TC-A02 TUI catalog parity, command UI focus/cancel, compatibility tests | `cargo test`, CLI/TUI behavior fixtures |
| CORE | Review shared dispatcher API and handler boundaries only; no parallel edits to TERMINAL files | Typed requests and bounded results preserved |
| ADAPTER | Reconcile #88/#86, then TC-A03 read-only MAP/RADIO real-native handlers | Explicit capabilities; no inert manifest activation |
| GUI | TC-A05 palette and single-viewport regressions only after TUI/host API stabilizes | Desktop + four viewport checks |
| SECURITY_QA | Independent negative, revoke, crash, unknown-outcome and zero-side-effect tests; inspect CI run/job logs | Fail closed, receipts for remote delivery |
| HARDWARE_PI | Inventory/validator integration, Pi 5 HDMI acceptance script; Pi 4/3 compatibility matrix | Hardware pending until device evidence |
| RELEASE_DOCS | Maintain README, docs index, roadmap, JSON queues and change log after code checks | Exact SHA, executed jobs and residual risks |

## Required software gate

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo +1.90.0 check --workspace --locked
python3 tools/validate_host_packages.py
```

Run additional relevant existing validators and UI checks, not just these commands. Record exact CI SHA, workflow run URL, individual job conclusions and test counts. A workflow with zero jobs or `startup_failure` is `blocked_ci`, not accepted. Do not change workflow gates to conceal failures or pull unapproved external Actions/repositories. No forced main merges or bypassing release controls.

## Reporting / next-pickup artifact

For each task, update the canonical JSON queue and linked PR/issue with: owner, dependency, current branch/head, implemented files, tests executed, exact-head Actions evidence, known failures, and status (`queued`, `in_progress`, `implemented_unverified`, `blocked_dependency`, `blocked_ci`, `accepted_software`, or `accepted_hardware`). Update docs index/README only to describe verified implementation. After TC-A02, continue to the earliest independent unblocked TC-A03 or security task. Never claim full production or Pi acceptance from synthetic test results.

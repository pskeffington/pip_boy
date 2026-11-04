# Codex pass — 2026-11-03: terminal parity, host reconciliation, and Pi 5 evidence

Status: execution instructions plus coordinator progress; physical acceptance remains pending.
Source packet: PR [#91](https://github.com/pskeffington/pip_boy/pull/91), exact head `d9ac51a46cbb7dd7335eb70b2b25e44c9af16c84`, Actions run [41758312559](https://github.com/pskeffington/pip_boy/actions/runs/41758312559) (7/7 jobs passed).
Current main after reconciliation: `1ba2b8f019cbd3b61b36c89171ddbbb5941f829d`.

## Reconciled state

| PR | State against current `main` | Evidence / action |
|---|---|---|
| #88 supervised host + terminal/TUI | Merged at `1ba2b8f`; exact head `457d965` passed 7/7 jobs | TC-A02 is software-verified at this merged scope. GUI palette and production pins remain separate. |
| #89 Pi 5 roadmap and inventory | Open draft, current head `7971879`, GitHub reports non-mergeable and branch is four commits behind | Exact head passed run `41177160241` (7/7). It contains Pi 5 inventory/baseline files not yet in main. Resolve branch conflict/base before merge. |
| #86 adapter recovery | Open draft, current head `d98895a`, non-mergeable | Exact head passed run `41177620852`; reconcile branch base before merge. |
| #83 camera/phone planning | Open draft, current head `1f926de`, non-mergeable | Planning only; run `41023134572` succeeded. No camera runtime or acceptance. |
| #87 inert SCAN registry | Open draft on #83's planning branch, current head `9608dbe` | Run `41025584846` succeeded. Reference metadata only; not a camera package. |
| #91 execution packet | Open draft, current head `d9ac51a`, now behind main after #88 | Packet used for this pass; this checked-in copy records updated reconciliation. |

Do not duplicate the open PRs or describe their branches as mainline. Exact workflow success is source evidence only; a mergeable state, code review, and required checks still govern integration.

## Work completed in this pass

- TC-A02: PR #88 already contained the CLI migration and full-screen TUI palette; its exact-head CI ran seven successful jobs and it was merged. The task queue and docs now identify the merge SHA and separate hardware/production gates.
- TC-A03 partial: `radio stations` is a catalog-backed, pinned, offline/read-only listing of static curated metadata. It does not probe the network, enumerate SDR hardware, or start audio. MAP filesystem operations, TV, and COMMS remain unavailable.
- TC-A05 local slice: desktop and micro read pinned catalog descriptors from the Rust host and dispatch only on explicit RUN through the same typed executor. Browsing does not execute. The follow-up still needs exact-head CI and browser viewport coverage.
- Pi 5 preparation: [PI5_CODEX_SMOKE.md](PI5_CODEX_SMOKE.md) supplies repeatable HDMI/keyboard/mouse steps and a redacted evidence template. It is procedure only; no Pi was probed.

Local checks for the current worktree: formatting passed; warnings-denied workspace Clippy passed; `cargo test --workspace --locked` passed 421 tests; `cargo +1.90.0 check --workspace --locked` passed. Micro UI and desktop focus tests passed. The Chromium viewport test could not run locally because Chromium is unavailable. No GitHub CI result exists yet for the new TC-A03/TC-A05 follow-up.

## Architecture and safety gates

- Keep one foreground package and host-owned Settings always available.
- Toolset pins select discoverable commands; they are not authorization. The host rechecks the saved pin and dispatch metadata.
- Keep MAP, RADIO, TV, and COMMS internals in their owned packages. Only registered native handlers are callable.
- No shell invocation, network scan/probe, audio playback, SDR discovery, location upload, camera access, messaging, package download, or filesystem write from help, browse, selection, or tab changes.
- Every command declares handler ID, bounded typed arguments, side-effect class, required scope, confirmation, connectivity and structured result. Remote interruptions remain UNKNOWN OUTCOME.
- Raspberry Pi 5 with HDMI/keyboard/mouse is the primary physical acceptance platform. Pi 4 and unidentified Pi 3 hardware are compatibility-only. CI and cross-compilation do not count as physical evidence.

## Required next actions

1. Publish the current TC-A03/TC-A05 source as a focused GitHub branch/PR and inspect exact-head Actions jobs, including Rust stable, Rust 1.90, native platforms, catalog/security and Chromium viewport checks.
2. If any workflow has `startup_failure` or zero jobs, record `blocked_ci`; never infer a pass.
3. Reconcile PR #89 and #86 against latest main before merging. Preserve their exact heads and rerun CI after any conflict resolution.
4. Keep TC-A03 partial until its MAP scope decision is implemented and tested; do not activate reference/discovered packages.
5. Record real Pi 5 evidence only after an operator runs the physical checklist on the actual board. Touch, SDR/GNSS, audio, camera, paired devices, and production isolation remain distinct gates.


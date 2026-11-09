# RB-G5 agent execution packet — 2026-11-09

**Authority:** GitHub main at `651b0b07e02ac60132ab2eae9f6cee60ce9ae14f`, following merges #104, #95, #105, #106, #107, #108. This packet is tasking, not evidence that any remaining PR is merge-ready or any hardware is accepted.

## Coordinator startup
1. Fetch latest main and inspect current open PRs, task catalogues, `AGENTS.md`, nested instructions, and the central `agent_skills` registry at a recorded SHA. Recompute all merge states, heads, workflows, changed files and semantic overlap. Do not trust a prior PR snapshot without rechecking.
2. Preserve the original checkout's uncommitted changes; use isolated branches/worktrees from current main. Never overwrite operator work, force-push unrelated history, or merge in the absence of exact-head evidence.
3. Route work through independently reviewed, non-overlapping agent packets. Document paths owned, linked PR/issue, prerequisite, evidence SHA, CI job URL and blocking conditions. Do not silently change functional behavior when resolving conflicts.
4. Software gate requires executed relevant jobs and normal Rust/JS/security checks. No Actions admission or zero-job startup_failure means BLOCKED_CI. Previous green jobs do not validate rebased heads. Keep Pi 5, signing/notarization, live radio/transport, and cross-device acceptance as separate gates.

## Ordered agent assignments

| Work ID | Owner | Task | Acceptance / blocker |
| --- | --- | --- | --- |
| RBG5-00 | COORDINATOR + RELEASE | Verify main SHA, merged #104/#95/#105–#108, superseded closures #80/#81/#82/#86 and #91, enumerate open PRs, classify conflicts/overlap, and correct stale queue and roadmap claims | Exact refs, outcomes, and evidence linked; no speculative state promotion |
| RBG5-01 | CORE + ADAPTER | Reconcile #89, #79 and #71 against current main; extract only missing independently valuable changes, prefer narrow replacement PRs | Diff-level provenance and no regressions to shared dispatcher, package trust, or foreground ownership |
| RBG5-02 | SECURITY_QA + CI | Diagnose #66–#68 unstable merge state / failing checks; inspect run/job logs and code, repair root causes in separate small patches | Reproduced failure or explicit unknown cause, new exact-head passing required jobs; no relaxed gates |
| RBG5-03 | COORDINATOR + MODULE OWNERS | Triage conflicts #22, #27–#29, #53–#55, #60–#61, #65, #69; categorize redundant, salvageable, blocked, or superseded | Each decision supported by comparison to main; close/delete only after evidence of coverage and review |
| RBG5-04 | SCAN + SECURITY | Maintain #83/#87 review dependency; no activation or foreground camera access from inert design fixtures | Contract review and independent approval precede implementation; real camera/phone tests later |
| RBG5-05 | GUI + CI | Inspect #70 missing checks; confirm artwork provenance, required tests, and head-specific CI admission | No acceptance on missing job evidence |
| RBG5-06 | HARDWARE_PI + QA | Prepare Pi 5 HDMI/keyboard/mouse acceptance runbook and evidence capture, including startup, one-package viewport, Settings, terminal dispatch, RADIO read-only behavior, offline fallback, restart/fault recovery | Software scripts may be accepted; physical tests remain PENDING until recorded on Pi 5 |
| RBG5-07 | RELEASE + SECURITY | Consolidate CI matrix, Rust 1.90, workspace tests and negative regressions; audit OS device broker, trust root, offline revocation, installation and Mac release gates | Release candidate only after software, security, platform and physical evidence each independently satisfy their respective criteria |

## Known baseline (historical evidence, not a new run)
- Consolidation report: #105–#108 each passed 7/7 exact-head Actions jobs before merge. #108 passed local formatting, Clippy, 441 workspace tests and Rust 1.90 check.
- GitHub main was checked at the above SHA when this packet was authored. The original checkout's uncommitted changes were reported preserved.
- These are historical software checks only; no Pi 5 physical acceptance, signed/notarized distribution, or production readiness is asserted.

## Next Codex execution
Start with RBG5-00. Rebase/reconcile the oldest unblocked packet only after evidence inventory. Use a short-lived task branch and independent QA; update this packet, relevant machine-readable queues, existing issue/PR discussions and indexes as facts change. If a task is blocked, record the precise blocker and advance a disjoint work packet. Never mark a module connected, installed, delivered, or production-ready on simulation or metadata alone.

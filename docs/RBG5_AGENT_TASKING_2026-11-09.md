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

## RBG5-00 coordinator evidence — 2026-11-09

- GitHub API verified `main` at `651b0b07e02ac60132ab2eae9f6cee60ce9ae14f`. PRs #104, #95, and #105–#108 are merged. PRs #80–#82, #86, and #91 are closed as superseded/reconciled; their obsolete remote branches were removed. The original checkout remains dirty and was not modified by this pass.
- Tasking PR #109 head `8337ff6c22195c301f56261da53b652142614101` had 7/7 actual Actions jobs pass in [run 42694024298](https://github.com/pskeffington/pip_boy/actions/runs/42694024298), but independent review found its inherited November 3 `pr_reconciliation` snapshot still labels #86 and #91 open. That snapshot is retained as history and explicitly renamed; a separate current inventory has been added to `catalog/terminal_agent_tasks.json`. The amended head must receive a new exact-head run before merge.
- Current inventory is based on GitHub refs and PR status as listed in `catalog/terminal_agent_tasks.json`. #83/#87 remain a planning/review dependency; #87 is inert metadata and does not install or activate a camera. #70 had a `startup_failure` with zero jobs in run 40752144382 and remains unvalidated.
- Read-only investigations found #66 and #67 fail the Rust stable formatting job on their changed source files. #68's historical stable Clippy failure was `manual_contains` in `organization_authority.rs:359`; the current `main` source already uses `contains(&0)`, so #68 now needs base reconciliation and a fresh exact-head run rather than a duplicate lint fix. Evidence and corrective direction are in the current machine-readable inventory. These were executed-job findings, not startup failures.
- Current reconciliation of #89/#79/#71 is not yet complete at diff-level. #89 appears to contain unique Pi 5 inventory/runbook material; #79 overlaps newer tasking; #71 is broad and substantially overlaps current host/adapter/UI architecture. No functionality has been transplanted pending commit/file provenance review.
- RBG5-03 comparison found all named candidates substantially behind current `main`: #22 (551), #27 (508), #28/#29 (476), #53/#54 (415), #55 (260), #60/#61 (256), and #65/#69 (192). Their earlier green runs are old-head evidence only. Potential unique salvage includes #22's Boston/Census geometry, #53's county-switch state fix, #54's viewport/follow behavior, and #55's Medic handoff; each needs current-source semantic and security review. #27–#29 likely overlap newer PACE/Messenger integration; #60/#61 require camera-source/right-to-use review and #61 has a stable failure; #65 overlaps merged host/adapter/UI work; #69 is an unvalidated broad server scaffold. No PR was closed based only on age or overlap inference.
- SCAN dependency is intact: #83 remains planning-only; #87 is based on #83 and explicitly awaits SCAN-H00 review. Both are 61 commits behind current main. #83's latest inspected completed run passed 7/7; #87's exact-head run passed 7/7. This does not satisfy the dependency or activate camera access.
- #70's current exact head has a PR-triggered run that ended in `startup_failure` with zero jobs. The workflow's pull-request trigger is not path-filtered; the cause is not yet established. Provenance is asserted in PR documentation but no independent approval artifact has been verified.

### Agent skills resolved for this pass

Central registry: [`pskeffington/agent_skills`](https://github.com/pskeffington/agent_skills/tree/4ba56a83a7cce807d7cb645b4111e6ddd21bd7ce), revision `4ba56a83a7cce807d7cb645b4111e6ddd21bd7ce`. Skills read and applied: `software-delivery-orchestrator`, `engineering-planning`, `documentation-maintenance`, `independent-code-review`, `release-readiness`, `security-assurance`, `test-verification`, and `architecture-design`. Each requires source-bound evidence, explicit PASS/FAIL/BLOCKED states, and independent gates where applicable; release-readiness fails closed on stale evidence and missing approvals.

## Next Codex execution
RBG5-00 inventory is underway; this packet amendment itself requires new exact-head CI before it can be merged. Rebase/reconcile the oldest unblocked packet only after evidence inventory. Use a short-lived task branch and independent QA; update this packet, relevant machine-readable queues, existing issue/PR discussions and indexes as facts change. If a task is blocked, record the precise blocker and advance a disjoint work packet. Never mark a module connected, installed, delivered, or production-ready on simulation or metadata alone.

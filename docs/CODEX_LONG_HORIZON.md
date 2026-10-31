# Codex long-horizon development operating instructions

Read root [AGENTS.md](../AGENTS.md) first. This file supplements but cannot override its security and review rules. Follow the [production evidence roadmap](PRODUCTION_ROADMAP_2026_10_31.md).

## Primary physical target

Raspberry Pi 5 with **full-sized HDMI monitor and keyboard/mouse** is the active physical test bench. Follow [PI5_REFERENCE_TEST_BASELINE](PI5_REFERENCE_TEST_BASELINE.md) and gate PI5-00 through PI5-08; run PI5-02A on current hardware and defer physical touchscreen PI5-02B until one is attached. Continue GitHub-first software implementation when the Pi is unavailable, but never mark hardware passed from Linux ARM64 cross-builds, macOS or synthetic tests. Mac/Windows remain development and portability targets.

## Mission

Develop Rust-Boy into a robust, portable, offline-first host shell for independently versioned MAP, RADIO, TV, COMMS and eventual SCAN packages. Preserve working desktop, micro, terminal, map, radio, tasks and networking during every incremental migration. Production requires demonstrated software, security, interoperability and physical acceptance—not a large PR.

## Startup / autonomous loop

1. Fetch latest default branch metadata and enumerate open PRs/issues. Review `AGENTS.md`, `docs/README.md`, host and terminal contracts, `catalog/terminal_agent_tasks.json`, and relevant source/tests. Compare new work with #88, #86, #80, #78, #87 and active module branches. Do not duplicate it.
2. Build an up-to-date evidence table: gate, implementation HEAD, PR/merge state, exact CI run and job results, real-device status, blocker and owner. Prefer verified GitHub data over stale prose.
3. Choose the highest-priority ready package from the roadmap. If blocked, write actionable blocker/evidence into the issue or PR and select a file-disjoint ready task. Never silently downgrade the gate.
4. Scope a focused branch with explicit paths, API contract, backward compatibility, failure cases, release risk and rollback. One coordinator resolves shared files; workers own disjoint code. Do not arbitrarily edit agents' branches.
5. Implement a first-party Rust vertical slice. Keep modules in their own repositories and use pinned reviewed interfaces. Maintain stable `pipboy` commands and `PIPBOY_HOME` behavior. No remote installer or shell-driven arbitrary code execution.
6. Add deterministic positive and **negative** tests before describing a capability as operational: denied manifest, revoked grant, oversized/bad packet, crash, timeout, offline startup, corrupted state, interruption/unknown delivery and restart. Add UI size/accessibility tests when frontends change.
7. Submit PR and evaluate exact-head executed jobs: stable, Rust 1.90, fmt, Clippy warnings denied, tests, UI/fixtures, native OS and security lanes where relevant. Treat no jobs, startup_failure or missing checks as **blocked**, not pass. Never disable gates to unblock yourself.
8. Merge only after required review and acceptance; update machine-readable queues, docs/index, compatibility notes and evidence links. Continue to the next ready task, respecting dependencies.
9. At each pass, report: changed files, commit/PR, tested SHA, executed jobs, tests, failures, blocked gates, next ready task. Never claim live device success from mocks.

## Dependency and ownership order

- Coordinator/RELEASE: INTEG-00; merge/conflict queue; semantic release rules; update roadmap and evidence.
- HOST: HOST-01 supervised first-party RBPK process, package staging, lifecycle/fault recovery; HOST-02 GUI binding only after accepted contract.
- TERMINAL: TC-A02 CLI/TUI typed integration; preserve `run COMMAND`, tool help and native offline handlers.
- ADAPTER: TC-A03 approved RADIO/MAP handler capability binding; TV/COMMS are unavailable until actual adapter/install acceptance.
- SECURITY/QA: TC-A04 negative matrix, CI admission and grant/revocation review; inspect each security-sensitive PR independently.
- GUI: HOST-02 and TC-A05 desktop/micro command palette without browser-side duplicated parser.
- MODULES: package hash/provenance; GIS, radio, TV, COMMS adapters; SCAN consent/phone bridge only once camera broker exists.
- ORG/COMMS: ORG-3.2 precedes organization-authorized sync and any mini-server trust expansion.
- RELEASE: TC-A06, hardware matrix, accessibility and deliverable signoff.

## Non-negotiable operational safeguards

One foreground package; persistent host-owned Settings; role and ZIP/GPS setup in Settings; map prioritizes viewport. Opening a tab or MODULES dialog must never start playback, scans, network requests, camera streams or messaging. The host must not treat metadata as installed code. Third-party package isolation, capability checks, signing and revocation are independent gates. Keep credential and diagnostic text out of packages and logs. Mark remote send without authenticated receipt as UNKNOWN. Keep peer discovery and background services opt-in, bounded and revocable. No PHI, automated distress calls or unverified clinical claims.

## Stop/recovery policy

On test failure: reproduce, inspect logs, implement narrowly scoped correction, rerun affected checks then entire required gate; never assert fixed based only on a code diff. On ambiguous ownership or overlapping PRs: reconcile before writing. On CI startup failure: preserve branch, log exact run and check admission, then advance independent work rather than editing workflow blindly. On a missing hardware/credential-dependent test: leave it explicitly pending, with test plan and owner; continue software work. Do not force-push other agents' work or bypass protections.

## End-of-pass machine-readable report

Use this schema concept in PR description/issue update (not fabricated release status):
```json
{"gate":"HOST-01","state":"source_submitted","branch":"...","commit":"...","pr":"...","ci":{"run":"...","executed_jobs":[],"results":{}},"software_tests":[],"hardware_evidence":null,"blockers":[],"next_ready_task":"..."}
```

States must distinguish proposed, source_submitted, software_verified, merged, field_verified and release_accepted. A passing unit suite alone does not imply release_accepted.

# Rust-Boy change reconciliation and agent dispatch — 2026-10-30

This checkpoint was prepared against the repository mainline following commits `635a724`, `3a647e9`, `0a326ae`, and `016610e`. It distinguishes observed implementation from unverified integration. Read [AGENTS.md](../AGENTS.md), [terminal task queue](../catalog/terminal_agent_tasks.json), [terminal execution guide](TERMINAL_AGENT_EXECUTION.md), and [host/package roadmap](HOST_PACKAGE_ROADMAP.md) before changing code.

## Recent delta / evidence

| Change | Confirmed source-level milestone | Not yet implied |
| --- | --- | --- |
| TC-A01 typed command catalog | Shared bounded offline command descriptors, parsing and structured outcomes in core; task queue records accepted software and a seven-job Actions run | Existing terminal/TUI/GUI migrated; external module execution |
| TC-A04 command security matrix | Negative regressions and fixed error categories; task queue records exact-head seven-job CI acceptance | Hardware security, credential lifecycle, privileged device authorization |
| Handheld MODULES control | Read-only selection/registry entry point with inert module labels and touch/modal tests | Trusted installation, adapter mount, module activation, runtime socket |
| GitHub-first agent instructions | Owned work packets, dependency graph and CI acceptance language | That queued tasks have executed |
| GIS/module extraction and PR #71 | Host-side GIS package preflight candidate; PR description records seven-job exact-head hosted run for its branch | Independent rb_gis renderer parity or field acceptance |
| Avatar PR #70, mini-server PR #69 and field network PRs | Separate open PR work, inspect individually before integration | Merge to main or production readiness |

## Next agent dispatch

1. **TERMINAL / TC-A02 (unblocked):** migrate `pipboy terminal` and full-screen TUI to the core typed dispatcher, preserve existing CLI verbs; test selected-tool scoping, help, slash search, Enter invocation and Esc dismissal. Never execute on focus/selection.
2. **ADAPTER / TC-A03 (unblocked):** inventory existing native RADIO and offline MAP operations; wire only truly installed trusted read-only handlers through `UniversalAdapterHost`. Keep TV/COMMS and absent adapters NOT INSTALLED/UNAVAILABLE. Explicit network/audio/scan/send require separate scope and confirmation.
3. **HOST/UI (parallel, disjoint ownership):** bridge host-owned foreground selection and Settings to handheld and desktop. The existing MODULES registry remains inert until verified installed adapter roster + lifecycle integration. Exactly one foreground package, no host-level scroll, suitable 480x320 touch targets. Do not silently mount discovered manifests.
4. **SECURITY/QA (continuous):** extend negative cases as terminal/adapter and UI paths connect; enforce identical policy across CLI/TUI/GUI. Redact secret/location/message data; distinguish unknown remote delivery from success. Run both Rust toolchains and responsive browser checks where relevant.
5. **RELEASE / TC-A05 then TC-A06 (dependency-gated):** implement shared GUI palette only after TC-A02, then reconcile README, docs index, help, catalogs and release matrix. Preserve physical Mac/Pi, two-device, external stream and production gates as pending until evidenced.

## Cross-repository and PR constraints

- `pip_boy` owns shell, policy, dispatcher, Settings, and integration contracts; `rb_gis`, `rust_radio`, `rust_tv`, `rusti_rc`/COMMS and `rust_music` remain separately maintained. Never copy whole external modules or add remote repository fetching as a runtime dependency.
- PR #71 (GIS/package work), #70 (avatar), #69 (mini server), and network/sync PRs must be reviewed for conflict and status before starting overlapping changes. An open PR is not merged code.
- Avoid altering `rust_radio/` directly in this integration pass. Preserve existing road/county map functionality until extraction parity is proven.
- Never make model, module, or terminal selection imply organizational authority, connectivity, background execution or successful remote transmission.

## Acceptance / dispatch discipline

For each packet, record owned paths and GitHub issue/PR, exact head SHA, check names and actual job results, test counts, regression cases, coverage gaps and rollback. Require `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, Rust 1.90 compatibility, and applicable GUI/feature tests. A workflow with zero jobs or `startup_failure` is blocked CI, not a pass. Never infer hardware or production acceptance from software CI. Do not change task status without evidence.

**Coordinator loop:** inspect latest main, open PRs/issues, queue and dependencies; choose earliest unblocked non-overlapping slice; implement and test; open/update a focused PR; reconcile documentation and queue with evidence; move to next independent unblocked work. If CI admission is blocked, record diagnostic evidence and work on a nonconflicting packet rather than fabricating acceptance.

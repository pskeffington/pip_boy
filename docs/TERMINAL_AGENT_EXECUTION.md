# Codex execution plan — terminal and module command integration

**Mode:** GitHub-first, agent-driven, no required operator side-by-side/local terminal testing for software gates.
**Source of truth:** [machine-readable queue](../catalog/terminal_agent_tasks.json), [agent instructions](../AGENTS.md), [terminal contract](TERMINAL_MODULE_COMMANDS.md), [host roadmap](HOST_PACKAGE_ROADMAP.md).
**Current implementation:** TC-A01 typed bounded command catalog/parser/dispatcher and structured outcomes are implemented in `pipboy-core`, with exact-head CI pending. The provisional offline `pipboy terminal` and full-screen `tui` have not yet migrated to it. MAP / TV / RADIO / COMMS reference sockets remain uninstalled and unavailable.

## Agent work packets

| ID | Agent | Work | Dependencies | Merge gate |
|---|---|---|---|---|
| TC-A01 | CORE | Define first-party `CommandDescriptor`, `CommandRequest`, structured `CommandResult`, bounded parser and stable help/source registry in `pipboy-core`; map only inventoried offline handlers | None | Local typed validation passes; exact-head CI and formatter gate pending |
| TC-A02 | TERMINAL | Route existing `terminal_commands` plus full-screen `tui` through dispatcher; keyboard prompt + scoped toolset help, no privileged side effects from navigation | A01 | TUI snapshot/interaction and CLI compatibility tests |
| TC-A03 | ADAPTER | Bind actual read-only RADIO and offline MAP handlers, trusted adapter selection/lifecycle. Do not advertise TV/COMMS as runnable until installed and approved | A01 | Real native handler tests; denied manifests/capability gating |
| TC-A04 | SECURITY/QA | Negative permission, source trust, bad input, cancellation, restart and CI-gate evidence tests | None | Reproducible CI job evidence, not green badges without jobs |
| TC-A05 | GUI | One command palette for desktop and micro using shared host dispatcher, not browser-side command emulation | A01+A02 | Automated desktop and 480x320 viewport checks |
| TC-A06 | RELEASE | Validate release matrix and update root docs, help, catalog indices and status report | A02+A03+A04+A05 | Exact CI jobs and security signoff, field-gate separation |

## Next Codex instruction

> Read root AGENTS.md and catalog/terminal_agent_tasks.json. Take the highest-priority unblocked TC-A task; first inventory existing implementation and any open PRs/issues so no duplicate work is started. Build a small first-party Rust vertical slice with typed commands and tests; preserve CLI compatibility. Use GitHub branches/PRs and CI as the development environment, without requesting user-local side-by-side testing. Keep physical/device acceptance pending. When finished, record exact commit, tests, workflow jobs and blockers in the queue and corresponding issue, then take the next unblocked task. Do not bypass safety checks or treat manifest discovery as executable installation.

## Definition of ready

- Assigned agent and owned file paths are named.
- Dependencies are accepted or independent.
- The command maps to a real handler, not placeholder UI or unchecked process string.
- Security/peripheral/transport impact is documented.
- Required test code can be executed in supported CI jobs.

## Definition of done — software

1. `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, and Rust 1.90 workspace checks have real recorded results.
2. No unauthorized network/audio/device I/O on tab switch, toolset select, page open, help or restore.
3. Malformed, absent, unpinned, untrusted, revoked and unsupported handlers fail closed.
4. Existing CLI behavior preserved by tests; no duplicate parser drift between TUI and browser.
5. Evidence cites SHA, workflow URL, job run, test names and test counts; a zero-job `startup_failure` is a blocker, not a pass.
6. Root docs, index and JSON task state match actual code.

## Release boundary

Software acceptance is not hardware or production acceptance. No demand for operator Mac verification during coding. Hardware radio, physical Raspberry Pi, external streams, paired devices, emergency message delivery, regulated data and native isolation remain separate real-device/security release gates.

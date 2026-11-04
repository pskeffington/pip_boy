# Codex execution plan — terminal and module command integration

**Mode:** GitHub-first, agent-driven, no required operator side-by-side/local terminal testing for software gates.
**Source of truth:** [machine-readable queue](../catalog/terminal_agent_tasks.json), [agent instructions](../AGENTS.md), [terminal contract](TERMINAL_MODULE_COMMANDS.md), [host roadmap](HOST_PACKAGE_ROADMAP.md).
**Current implementation:** TC-A01 and TC-A02 are software-verified. PR [#88](https://github.com/pskeffington/pip_boy/pull/88) merged at `1ba2b8f`; its exact head `457d965` passed all seven jobs in [Actions run 41127024688](https://github.com/pskeffington/pip_boy/actions/runs/41127024688). A local follow-up implements read-only curated RADIO metadata and a desktop/micro palette through the same dispatcher; full local Rust gates and browser DOM journeys pass, while exact-head CI and Chromium viewport checks are pending. Live network/audio/SDR operations, MAP filesystem access, and TV/COMMS execution remain unavailable.

## Agent work packets

| ID | Agent | Work | Dependencies | Merge gate |
|---|---|---|---|---|
| TC-A01 | CORE | Define first-party `CommandDescriptor`, `CommandRequest`, structured `CommandResult`, bounded parser and stable help/source registry in `pipboy-core`; map only inventoried offline handlers | None | 7/7 exact-head Actions jobs pass on source commit `7c4b06bc64f89cdb41b4be877b7b8742c130f38e` |
| TC-A02 | TERMINAL | Route CLI and full-screen TUI through shared catalog; preserve aliases and bounded palette | A01 | **Software-verified:** PR #88 merged; exact head and 7/7 Actions jobs linked above |
| TC-A03 | ADAPTER | Expose only static curated RADIO station metadata through the typed catalog; MAP remains scope-gated, TV/COMMS unavailable | A01 | Local tests pass; exact-head CI pending |
| TC-A04 | SECURITY/QA | Negative permission, source trust, bad input, cancellation, restart and CI-gate evidence tests | None | Reproducible CI job evidence, not green badges without jobs |
| TC-A05 | GUI | Desktop and micro palettes resolve pinned descriptors and submit via shared Rust dispatcher | A01+A02 | Local DOM/Rust tests pass; Chromium viewport and exact-head CI pending |
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

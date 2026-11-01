# Rust-Boy polishing pass: command surfaces and host viewport

Status: engineering work packet; **not** evidence of implementation or passing tests.
Date: 2026-10-31
Primary queue: [terminal_agent_tasks.json](../catalog/terminal_agent_tasks.json)
Owner: TERMINAL (TC-A02), then ADAPTER (TC-A03), then GUI (TC-A05).
Related issue: [#75](https://github.com/pskeffington/pip_boy/issues/75)

## Confirmed current-state discrepancy

- `crates/pipboy-core/src/command_catalog.rs` already defines bounded typed handler IDs, arguments, and side-effect metadata.
- `crates/pipboy/src/terminal_commands.rs` still maintains a separate `OFFLINE_TOOLS`, `commands()`, and `dispatch()` implementation. This permits help/dispatch drift from the canonical catalog.
- `crates/pipboy/src/tui.rs` handles workbench navigation but has not yet adopted the shared catalog as its command execution path.
- A package reference is **not** an installed or authorized module. Do not use this cleanup to activate unapproved MAP, TV, RADIO, COMMS, or SCAN adapters.

## Execution order

### POLISH-01 / TC-A02: unify offline command behavior

1. Inventory existing CLI direct verbs, selected-tool flow, aliases and error/output snapshots **before** editing. Preserve `host` as the diagnostic verb under selected `diag`, while standalone `host` remains a selectable pinned tool.
2. Have the terminal parse commands through `command_catalog`. Use a single reviewed native handler executor, not a second string-token matcher. Leave built-in shell controls such as `help`, `select`, `tools` and `quit` as UI routing with no handler side effects.
3. Derive discoverable help/tool availability from canonical descriptors while still requiring the tool to be pinned for invocation. Pinned does not mean authorized.
4. Add TUI command-palette navigation using the same parser/handler path. `/` opens; Escape closes without dispatch; Enter on a selection does not perform a network/audio/write action. Focus restoration, small-terminal clipping, and error feedback get deterministic tests.
5. Compare CLI behavior against frozen compatibility fixtures for every offline handler before removing legacy dispatch code.

### POLISH-02 / TC-A03: native module adapter binding

1. Start with an entirely read-only MAP metadata/status handler and RADIO preset listing using real existing Rust APIs.
2. Require explicit separately checked user action for county download, stream playback, SDR hardware, device scan, remote send, or persistent changes.
3. Keep TV/COMMS/SCAN references unmounted until a reviewed installed-handler contract exists. Discovered manifests never become executable.
4. Test failure, suspend, cancellation, rapid foreground switching, and Settings recovery.

### POLISH-03 / TC-A05 and HOST-UI-1/2: clean UI

1. Exactly one foreground module canvas; no second module overlay. Compact switcher and always-reachable host Settings.
2. The MAP canvas fills remaining space without location controls or stacked page headings. ZIP/GPS, county source acquisition, role and device configuration remain in Settings.
3. Validate usable touch controls, focus-visible keyboard navigation, accessible labels, no accidental action from tab switching, and no document-level scroll at 480x320, 800x480, 1024x768 and 1440x900.
4. Desktop Vault Command retains assignment/receipt visibility without pretending unacknowledged delivery. Micro field view and desktop share dispatcher and permissions.

## Required negative/regression cases

| Area | Required outcome |
| --- | --- |
| Unknown, unpinned, malformed or oversized command | Denied without execution; bounded safe error |
| Prompt-pasted shell command, control characters or injection-looking input | Denied; no process launch |
| Help, list, selection, tab switch, Settings open | No network, scan, stream, send, download or hardware access |
| Revoked, incompatible or discovered-only adapter | Not mounted; visible unavailable state |
| Interrupted remote operation | UNKNOWN outcome until verified receipt |
| Module panic/error, rapid switches | One foreground; Settings and recovery remain available |
| Small touchscreen / keyboard | No hidden controls, involuntary activation or host scrolling |
| Offline restart | No silent external connection or background authorization |

## Acceptance and evidence

Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `cargo +1.90.0 check --workspace --locked`, plus existing browser viewport tests and new command compatibility tests. Record exact branch SHA, Actions run and **executed jobs**. Any zero-job `startup_failure` is `blocked_ci`, not a pass. Keep Mac operator, Pi 5 full-screen, available Pi 4/Pi 3, radio/audio, hardware and paired-device testing in separate field acceptance rows. No production claim from simulated tests.

## Codex pickup instruction

Take **POLISH-01 / TC-A02** as the next implementation slice. Inspect open PRs and current branch/head first; do not duplicate in-progress work. Preserve existing native functionality and CLI compatibility. Commit code plus tests in a scoped PR; update `catalog/terminal_agent_tasks.json`, `docs/TERMINAL_AGENT_EXECUTION.md`, this work packet and issue #75 only after actual evidence. Continue to POLISH-02 and POLISH-03 after their respective gates pass.

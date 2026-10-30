# Terminal-to-module command surface (implementation contract)

Status: **TC-A01 software gate accepted.** Exact source commit `7c4b06bc64f89cdb41b4be877b7b8742c130f38e` passed all 7 jobs in [Actions run 41003353102](https://github.com/pskeffington/pip_boy/actions/runs/41003353102). `pipboy-core::command_catalog` exposes five pinned offline toolsets with bounded typed arguments, stable native handler IDs, explicit read-only/scope/confirmation/connectivity metadata, source provenance, stable JSON and help descriptors, and structured timestamped outcomes. The `pipboy terminal` bridge now dispatches through that catalog; the full-screen TUI and GUI palette migration remain open. This prior run is not CI evidence for the current RM-018 PR head.

## Agent execution

Follow [root AGENTS.md](../AGENTS.md), [GitHub-first execution packets](TERMINAL_AGENT_EXECUTION.md), and [machine-readable task statuses](../catalog/terminal_agent_tasks.json). Software gates should progress through GitHub source and CI, without requiring operator live side-by-side testing. Hardware/field release acceptance remains separate.

## TC-A01 implementation checkpoint

The catalog is descriptive and deterministic. Parsing a command or selecting a tool never invokes a handler. Requests are admitted only for registered handlers and tools pinned in the caller-supplied Workbench set. Every registered command is currently read-only and offline; side-effect, scope, confirmation and connectivity fields are explicit so future commands cannot inherit implicit authority. `CommandResult<T>` records a typed value or error, handler status, source path and caller-supplied observation time. Tests cover bounds, unsupported/unpinned/shell-like inputs, help/source provenance, stable serialization and non-execution on selection.

## Goal

Both `pipboy tui` and the graphical desktop/micro shell use one host-owned command catalog. The operator selects a toolset, sees only that toolset's available verbs, and can run reviewed commands through the same Rust module/service implementations used by the graphical UI. No duplicated module logic, untrusted shell invocation, or implicit permissions.

## Surfaces and source of truth

- **Toolset selection**: `WorkbenchStore` and its profile/slot persistence remain the source of truth for pinned tools. A session-level active toolset must be selected explicitly and must never mutate an organizational role or privilege.
- **Installed package selection**: `UniversalAdapterHost` owns actual trusted installed adapter lifecycle. `PackageRegistry::scan_directory` and the MAP/TV/RADIO/COMMS reference catalog are metadata only. Never treat them as runnable or trusted.
- **Command definitions**: introduce a small first-party `ModuleCommandCatalog` in `pipboy-core`. Each descriptor identifies package/tool ID, stable verb, typed argument schema, discoverability, read-only/side-effect class, required capability and confirmation policy. One shared dispatcher feeds TUI, CLI and browser command palette.
- **Commands**: do not execute arbitrary process strings, `sh -c`, or unchecked `std::process::Command`. Dispatch typed arguments to approved Rust handlers. A module's display string is NOT an executable command.
- **Auth**: every request passes the existing host permission, scope and broker policies, independently of whether it originated in CLI, TUI or graphical interface. Selecting a package or toolset grants nothing.

## TUI interaction

One package/toolset at a time. A slim header indicates active toolset, connectivity and explicit capability state; Settings and Back remain host-owned. Add a **COMMAND** pane (or equivalent modal) to `crates/pipboy/src/console.rs` and key handling to `tui.rs` with these keyboard actions:

- `/` opens command search scoped to the current toolset; `?` lists supported commands and argument help.
- Enter executes a selected **read-only, offline** action or opens a typed argument/confirmation form for anything else.
- Esc closes the command pane without executing; Tab navigates modules; Up/Down navigates results.
- Each action renders distinct states: UNAVAILABLE / NOT INSTALLED / DISCONNECTED / NEEDS AUTH / READY / RUNNING / SUCCEEDED / FAILED / UNKNOWN OUTCOME.
- No CLI command is executed merely from changing a tab, highlighting a verb, restoring a session, or opening a preset.

Expected example commands, subject to actual handler registration:
```text
toolsets
toolset show
toolset select radio
radio stations                 # existing built-in command; read only
radio validate skylab          # requires explicit --confirm and stream permission
map status                     # only when implemented and registered
tv channels                    # only when implemented and registered
comms peers                    # only when implemented and registered
```
Unknown or unimplemented verbs must return an informative **NOT IMPLEMENTED** error; no simulated result may appear as live evidence.

## Dispatcher contracts

1. Parse bounded input to typed command + arguments; reject unknown verbs, excessive input, control characters and ambiguous destinations.
2. Resolve against **installed trusted handlers**, not discovered manifests.
3. Check active toolset membership, handler readiness and declared capability.
4. Evaluate organizational authority and scope at invocation time; require user confirmation for all side effects, live network operations, broadcasts, pairing, messaging and hardware access.
5. Dispatch to existing native Rust operations or `UniversalAdapterHost::intent` with the declared scope and validated payload. Non-UI adapter intents require a scope; all privileged device operations must still pass their own broker/permission check.
6. Surface structured outcome, timestamp, source and error; never downgrade denied/unknown outcomes to success.
7. Store bounded, redacted local command history. Never record authentication secrets, session tokens, live location, message bodies, or protected clinical data in diagnostics/history.
8. Cancellation and timeout must not imply remote cancellation; report UNKNOWN OUTCOME where delivery status cannot be proven.

## Implementation order and gates

| Gate | Implementation | Required evidence |
|---|---|---|
| TC-0 | Inventory existing `workbench`, `tui`, `console`, `radio`, `package_socket`, `universal_adapter`, and dashboard operations; matrix every advertised verb to an actual Rust handler | No imaginary or placeholder live verbs |
| TC-1 | Shared command descriptor/schema, typed parser and bounded argument validation in `pipboy-core` | Unit tests for malformed inputs, injection attempts, arg bounds and stable help |
| TC-2 | Local read-only dispatcher plus TUI palette, toolset selection and consistent results; use existing saved Workbench | TUI+CLI integration tests; selecting a toolset never executes a verb |
| TC-3 | Reuse implemented native RADIO and offline MAP operations only where already validated; keep TV/COMMS unavailable until their real adapters register | Real handler tests and deny-by-default tests; do not import external code or fetch repos during execution |
| TC-4 | Connect trusted installed adapter commands to the universal adapter host, including lifecycle and capability checks | Cross-surface contract tests, rejected discovery fixtures, revocation and fault isolation |
| TC-5 | GUI command palette shares catalog, state and policy with TUI; Settings remains host-owned | Desktop + 480x320 keyboard/touch acceptance; no overflow or forced scroll |
| TC-6 | Offline restart, broken network, lost device, cancellation, concurrency, journal privacy and physical Mac/Pi acceptance | Reproducible evidence and release gate signoff |

## Repository rules

Prefer in-repo first-party Rust code. Do not add runtime dependencies on outside repositories or dynamically download/install modules. Keep `rust_radio/` and other separately maintained modules unchanged unless explicitly in scope; reference local, pinned interfaces without copying their internals. Update `README.md`, documentation indexes, CLI help, `AGENTS.md`/Codex guidance as appropriate with **actual** implementation status. Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`; retain toolchain 1.90 compatibility. Do not mark acceptance passed without CI and field evidence.

**Release rule:** the TUI command toolset must fail closed when a module is not installed, not trusted or unavailable; this is a safer and more useful outcome than pretending the module is mounted.

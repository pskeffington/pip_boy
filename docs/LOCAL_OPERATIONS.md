# Rust-Boy | Scoped local operations (G1-D)

**Implementation authored; Rust compilation, format, Clippy, and runtime acceptance are not yet verified.**

The local-operation gate is separate from the lifecycle-managed telemetry collector. `ModuleHost::run_local` verifies a built-in's readiness and current permission set **before** invoking a trusted, one-shot closure. It does not mark a module running or open hardware.

| Operation | Required grants | Call sites |
| --- | --- | --- |
| `LibraryRead` | `files.read` | CLI `sources`; TUI LIBRARY |
| `NotesRead` | `files.read` | CLI list/search/export; TUI NOTES |
| `NotesWrite` | `files.read`, `files.write` | CLI add/import |

The offline default grants local read access, not write access. The user explicitly calling `notes add` or `notes import` grants local write permission to that command's short-lived host only. File writes remain protected by the note store's existing validation, locking and staged replacement. No background sync, external device access, or network activity was introduced.

This is **cooperative in-process policy**, not OS-enforced isolation; arbitrary native code linked into the process is not restricted by these checks. Persistent per-adapter consent and device brokering remain separate, unfinished work.

## Acceptance commands

```bash
python3 tools/validate_catalog.py
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build -p pipboy --locked
python3 tools/smoke_notes.py
python3 tools/smoke_runtime.py
```

Three new core fixtures cover denial-before-callback, explicit grant/revocation behavior and error propagation. No pass results are claimed for them until a Rust toolchain executes the suite. GitHub Actions was previously failing with `startup_failure` and zero jobs.

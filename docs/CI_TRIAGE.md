# Rust-Boy | CI troubleshooting history

**Current policy (October 25, 2026):** Rust-Boy declares Rust **1.90** as its minimum; CI uses stable and **1.90.0**. The `rust` job now sets `RUSTUP_TOOLCHAIN` and asserts `rustc --version`. This is necessary because `rust-toolchain.toml` selects stable and overrides `rustup default`; for example, the job labeled 1.81 in [run 40172664621](https://github.com/pskeffington/pip_boy/actions/runs/40172664621) actually used rustc **1.99.0**. Treat old matrix-job labels as historical records, not proof of minimum-compiler verification.

**Updated October 25, 2026 — resolved for Rust-Boy's GitHub-hosted Ubuntu workflow.** GitHub Actions was already enabled. A [minimal runner](https://github.com/pskeffington/pip_boy/actions/runs/40159954191) and [first-party checkout](https://github.com/pskeffington/pip_boy/actions/runs/40160030487) both passed. Adding a third-party Rust toolchain action reproduced [startup_failure with zero jobs](https://github.com/pskeffington/pip_boy/actions/runs/40160046958). Replacing third-party toolchain/cache actions with runner-native `rustup` recovered job admission. Following rustfmt cleanup, [run 40160147824](https://github.com/pskeffington/pip_boy/actions/runs/40160147824) passed all three jobs: catalog, Rust stable (92 unit tests and five smoke scripts), and Rust 1.81.0 (92 unit tests). The precise account-level third-party action policy was not retrieved; this is a reproducible workflow-level diagnosis, not proof of a specific account setting.

[Example failed workflow](https://github.com/pskeffington/pip_boy/actions/runs/40149614562)

## Game terminal smoke synchronization

At `09dbf2d`, native Windows compiled and passed 146 Rust tests plus the inventory terminal journey, then the delayed-reference mock peer reported WSAECONNABORTED. The child had exited successfully, but a short raw-output `90` match could accept connection metadata or a terminal sequence before the live HP update. A unique nine-digit value and deliberate `fixture-90` announcement decoy now distinguish actual live rendering from metadata. A mutation check restoring the old marker must fail. Do not treat successful process exit alone as evidence that a requested game update was observed. Windows EOF/reset/aborted handling is limited to the final expected-close check; active protocol errors remain failures. See [validation evidence](VALIDATION.md).

## Historical startup-failure diagnostic procedure

1. Open the failed run in GitHub Actions. Read the run-level banner/annotations; zero-job runs do not contain compiler logs.
2. Review repository Settings > Actions > General for enabled workflows, action permissions, required approvals and runner restrictions.
3. If applicable, check organization policy, GitHub-hosted runner eligibility, billing/spending restrictions and account status. These are possible causes to investigate, **not confirmed diagnoses**.
4. Inspect GitHub Status for wider outages, and check whether a minimal workflow starts.
5. Use the added manual workflow_dispatch trigger after resolving the startup condition. Never disable quality checks merely to obtain a green badge.

[Official GitHub Actions log guidance](https://docs.github.com/en/actions/how-tos/monitor-workflows/use-workflow-run-logs).

## Verified Linux quality gates

The workflow originally checked Rust stable and Rust 1.81.0; the current minimum-version job now tests Rust 1.90.0. The stable job now includes adapter-preference, fixed-device-simulator and deterministic replay CLI smoke suites alongside existing notes/runtime checks, and all were executed in the passing run cited above. Stable must pass format, compile, tests and Clippy; the minimum-supported Rust version must compile and test.

Local commands on a machine with Rust installed:

    python3 tools/validate_catalog.py
    rustc --version
    cargo fmt --all -- --check
    cargo check --workspace --all-targets
    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings
    cargo run -p pipboy -- doctor
    cargo run -p pipboy -- sample --count 2 --interval-ms 1000
    cargo run -p pipboy -- permissions list
    python3 tools/smoke_permissions.py
    cargo run -p pipboy -- simulate list
    python3 tools/smoke_simulator.py
    cargo run -p pipboy -- simulate replay sim:serial disconnect-recover --confirm
    python3 tools/smoke_replay.py

A generated Cargo.lock is now committed and tested locally on stable and Rust 1.81.0. CI uses `--locked`. Modern Cargo uses the MSRV fallback resolver; Cargo 1.81 ignores that newer configuration key but consumes the compatible version-3 lockfile. The lockfile selects instability 0.3.10 and unicode-segmentation 1.12.0 rather than newer versions requiring newer Rust. See [local validation evidence](VALIDATION.md).

## Outstanding platform and release acceptance

Capture successful run URL, exact commit SHA, runner version, catalog validation, stable and minimum-Rust check logs, TUI launch/exit, note persistence and error restoration tests. Confirm macOS, Linux and Windows behavior separately. Once jobs start, failures within cargo are code or dependency findings distinct from the original runner startup failure.

The temporary, feature-branch-only `runner-probe.yml` diagnostic workflow was removed during main-branch documentation cleanup; the full [Rust-Boy quality workflow](../.github/workflows/rust.yml) now runs on `main`, pull requests, feature branches and manual dispatch. [Cross-platform run 40164915215](https://github.com/pskeffington/pip_boy/actions/runs/40164915215) passed catalog checks, Ubuntu stable/MSRV, native macOS PTY, Windows ConPTY and the deny-all broker test. Real hardware, release packaging and accessibility signoff remain outside that CI evidence.

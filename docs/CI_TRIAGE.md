# Rust-Boy | GitHub Actions startup investigation

**Updated October 25, 2026 — resolved for Rust-Boy's GitHub-hosted Ubuntu workflow.** GitHub Actions was already enabled. A [minimal runner](https://github.com/pskeffington/pip_boy/actions/runs/40159954191) and [first-party checkout](https://github.com/pskeffington/pip_boy/actions/runs/40160030487) both passed. Adding a third-party Rust toolchain action reproduced [startup_failure with zero jobs](https://github.com/pskeffington/pip_boy/actions/runs/40160046958). Replacing third-party toolchain/cache actions with runner-native `rustup` recovered job admission. Following rustfmt cleanup, [run 40160147824](https://github.com/pskeffington/pip_boy/actions/runs/40160147824) passed all three jobs: catalog, Rust stable (92 unit tests and five smoke scripts), and Rust 1.81.0 (92 unit tests). The precise account-level third-party action policy was not retrieved; this is a reproducible workflow-level diagnosis, not proof of a specific account setting.

[Example failed workflow](https://github.com/pskeffington/pip_boy/actions/runs/40149614562)

## Historical startup-failure diagnostic procedure

1. Open the failed run in GitHub Actions. Read the run-level banner/annotations; zero-job runs do not contain compiler logs.
2. Review repository Settings > Actions > General for enabled workflows, action permissions, required approvals and runner restrictions.
3. If applicable, check organization policy, GitHub-hosted runner eligibility, billing/spending restrictions and account status. These are possible causes to investigate, **not confirmed diagnoses**.
4. Inspect GitHub Status for wider outages, and check whether a minimal workflow starts.
5. Use the added manual workflow_dispatch trigger after resolving the startup condition. Never disable quality checks merely to obtain a green badge.

[Official GitHub Actions log guidance](https://docs.github.com/en/actions/how-tos/monitor-workflows/use-workflow-run-logs).

## Verified Linux quality gates

The revised workflow checks the offline catalog with Python, plus Rust stable and Rust 1.81.0 toolchains. The stable job now includes adapter-preference, fixed-device-simulator and deterministic replay CLI smoke suites alongside existing notes/runtime checks, and all were executed in the passing run cited above. Stable must pass format, compile, tests and Clippy; the minimum-supported Rust version must compile and test.

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

The temporary path-scoped `runner-probe.yml` is a first-party-only diagnostic; it does not run on ordinary Rust source updates. Later development commits require their own green run before claiming acceptance. macOS/Windows and real hardware remain outside the Ubuntu CI evidence.

# Rust-Boy | GitHub Actions startup investigation

**Status checked October 24, 2026:** Actions reports startup_failure and the inspected runs contain zero jobs. Run [40153691749](https://github.com/pskeffington/pip_boy/actions/runs/40153691749) at development head `642ea67` also reports `startup_failure` with zero jobs; the new status integration has not been run in CI. This is a workflow startup failure, not evidence of Rust compiler or test execution.

[Example failed workflow](https://github.com/pskeffington/pip_boy/actions/runs/40149614562)

## Determine the startup cause

1. Open the failed run in GitHub Actions. Read the run-level banner/annotations; zero-job runs do not contain compiler logs.
2. Review repository Settings > Actions > General for enabled workflows, action permissions, required approvals and runner restrictions.
3. If applicable, check organization policy, GitHub-hosted runner eligibility, billing/spending restrictions and account status. These are possible causes to investigate, **not confirmed diagnoses**.
4. Inspect GitHub Status for wider outages, and check whether a minimal workflow starts.
5. Use the added manual workflow_dispatch trigger after resolving the startup condition. Never disable quality checks merely to obtain a green badge.

[Official GitHub Actions log guidance](https://docs.github.com/en/actions/how-tos/monitor-workflows/use-workflow-run-logs).

## Quality gates after jobs start

The revised workflow checks the offline catalog with Python, plus Rust stable and Rust 1.81.0 toolchains. The stable job now includes the adapter-preference CLI smoke suite alongside existing notes/runtime checks, but this is only authored coverage until runners start. Stable must pass format, compile, tests and Clippy; the minimum-supported Rust version must compile and test.

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

A generated Cargo.lock is now committed and tested locally on stable and Rust 1.81.0. CI uses `--locked`. Modern Cargo uses the MSRV fallback resolver; Cargo 1.81 ignores that newer configuration key but consumes the compatible version-3 lockfile. The lockfile selects instability 0.3.10 and unicode-segmentation 1.12.0 rather than newer versions requiring newer Rust. See [local validation evidence](VALIDATION.md).

## G0 exit evidence

Capture successful run URL, exact commit SHA, runner version, catalog validation, stable and minimum-Rust check logs, TUI launch/exit, note persistence and error restoration tests. Confirm macOS, Linux and Windows behavior separately. Once jobs start, failures within cargo are code or dependency findings distinct from the original runner startup failure.

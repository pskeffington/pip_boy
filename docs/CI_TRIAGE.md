# Rust-Boy | CI troubleshooting history

## First-party CI diagnosis and Node bootstrap repair — 2026-10-29

All `uses:` references have been removed from the workflow in this draft. Exact SHA checkout uses GitHub's own repository via `git fetch`; stable/1.90 Rust use runner-native `rustup`. `scripts/ci_bootstrap.sh` pins Node.js v22.23.4 from the official Node distribution, verifies its published SHA-256 for each supported runner architecture, and adds the verified binary to the job PATH. The shell checkout must remain inline because no repository script exists before the initial fetch.

This removes third-party **GitHub Action repositories**, not upstream toolchain distributions or Cargo dependencies. `rustup`, apt packages, locked Rust dependencies and the pinned Node distribution still require external network sources or a future mirrored/offline supply chain. The prior exact-head run exposed Node 24 on macOS despite the Node 22 requirement; the repository bootstrap now obtains a pinned, digest-checked Node 22 instead of depending on runner image contents. Test with exact-head Actions evidence before closing RB-CX-00.

### Exact admitted runs

- [Run 40823597748](https://github.com/pskeffington/pip_boy/actions/runs/40823597748), head `63093badc2046a59689693f68896fa502dd95234`: manifest validation passed. Stable, Rust 1.90, SDR and native jobs were cancelled when the newer PR run started under the workflow's `cancel-in-progress` concurrency policy. There were no failed steps; cancellation is not a source failure.
- [Run 40823687340](https://github.com/pskeffington/pip_boy/actions/runs/40823687340), exact head `5ad4327d34751cb723eae0ae7de7ef653e3702cf`: all seven jobs were admitted. Catalog validation, stable and Rust 1.90, both SDR feature jobs, and native Windows passed. Stable Linux UI/Chromium regressions and runtime smokes passed. macOS compile, 393-test suite, native smoke roster and PTY passed, then the job failed at Node verification (`v24.22.0` supplied; Node 22 required); the subsequent Mac-first MAP step was skipped. This is the specific failure repaired by the pinned bootstrap.
- RB-CX-00 remains open until the repair passes on a new exact PR head. Do not accept this partial green run or treat job admission alone as acceptance.


## Codex queue/ownership offline gates — 2026-10-29

The catalog job additionally registers `python3 tools/validate_codex_passes.py` and `python3 tools/test_validate_codex_passes.py`. These are filesystem-only schema, dependency, component-ID and path checks; they do not build Rust or certify a release. [Actions run 40821379695](https://github.com/pskeffington/pip_boy/actions/runs/40821379695) at `5b4a66e` ended in `startup_failure` with zero jobs, so the new pass checks did not execute. Investigate run-level annotations and runner admission without asserting a compiler failure. Inspect exact-head workflow evidence before marking a pass accepted. The existing component owner index must remain single-owner even when a module is used by multiple integration layers.

For exact source revision `8a5c9d162af5ede9449dcf924f88fba15e9bdb38`, [Actions run 40822394927](https://github.com/pskeffington/pip_boy/actions/runs/40822394927) also ended `startup_failure`. PR check diagnostics returned zero jobs, checks and statuses. Local stable and Rust 1.90 gates passed, but this run provides no CI execution result or root-cause explanation; the pass remains open.

The next exact-head run, [40822764662](https://github.com/pskeffington/pip_boy/actions/runs/40822764662) for `6f1e8d276595e62930b506d5f878e0b952832777`, failed the same way: REST reports `startup_failure` and its jobs endpoint returns `total_count: 0`. The checked-in workflow has an unfiltered `pull_request` trigger and statically defines the catalog, stable/1.90 Rust matrix, optional SDR and native jobs; no workflow-level skip condition is visible. [GitHub Status](https://www.githubstatus.com/) lists Actions operational and no incident for Oct 29. These observations narrow the evidence to pre-job admission but do not identify whether the repository policy, account settings, or GitHub's run service caused it. Keep workflow gates unchanged and obtain repository Actions settings or GitHub support diagnostics before altering YAML.


## Current smoke-test inventory (2026-10-26)

The CI workflow now uses one explicit [Python smoke-test roster](../tools/run_smokes.py). It replaces duplicated per-platform command lists; it does **not** add hardware operations or reduce test coverage. Child scripts run in their prior order, with the currently selected Python interpreter, and the first failure stops the suite.

| Profile | Command after `cargo build --workspace --locked` | Coverage |
| --- | --- | --- |
| Linux stable | `python3 tools/run_smokes.py linux` | 19 tests: 17 shared CLI/API mocks plus notes and POSIX PTY |
| Native macOS / Windows | `python3 tools/run_smokes.py native` (or `python` on Windows) | The same 17 shared scripts, in the same order |
| Native terminal extras | Dedicated workflow steps, unchanged | `smoke_runtime.py` on macOS; `smoke_windows_conpty.py` on Windows |

Run `python3 tools/run_smokes.py --check` to verify the names, file presence, and classification of all 20 `smoke_*.py` scripts **without running them**. New tests fail CI until explicitly assigned to a profile or a platform-specific step; symlinked/missing tests also fail. `python3 tools/run_smokes.py native --list` displays the exact ordered roster, and `python3 tools/test_run_smokes.py` runs 11 standard-library regressions. Windows ConPTY and macOS PTY retain their dedicated CI steps; notes remain Linux-only pending separate native acceptance.

The roster itself does not build Rust or discover devices. Build the executable before starting an execution profile. Existing child smoke scripts may use loopback mocks; this is not evidence of physical hardware acceptance.


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

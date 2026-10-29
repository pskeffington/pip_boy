# Codex development passes — Rust-Boy

**Branch scope, 2026-10-29:** This sequence targets draft [PR #71](https://github.com/pskeffington/pip_boy/pull/71), branch `codex/package-adapter-protocol`. The canonical machine backlog is [`catalog/codex_passes.json`](../catalog/codex_passes.json). Read the root [`AGENTS.md`](../AGENTS.md) for Codex operating rules and [production checkpoint](PROD_STATUS.md) for what was actually verified.

Rust-Boy is a modular pop-in socket: the shell owns **one package viewport**, compact navigation, host-owned Settings and permission boundaries. MAP, RADIO, TV and COMMS retain independent release lifecycles. The draft already contains a portable adapter SDK, in-process lifecycle manager, host Settings/foreground APIs, an unfinished Unix IPC precursor and a **compiled-in passive RADIO station catalogue**. It does *not* provide authenticated untrusted packages, safe dynamic installation or completed interactive native media adapters.

## Pass sequence

| Pass | Priority | Status | Focus | Prerequisites |
| --- | --- | --- | --- | --- |
| RB-CX-00 | P0 | **Next** | Exact-head build/CI baseline and passive Radio bootstrap verification | None |
| RB-CX-01 | P0 | Queued | Safe universal data-driven viewport, Settings and fault handling | 00 |
| RB-CX-02 | P0 | Queued | Backend capability intent, audio/device/transport arbitration | 00, 01 |
| RB-CX-03 | P1 | Blocked | Real interactive `rust_radio` package, explicit Play/Stop | 01, 02 |
| RB-CX-04 | P0 | Blocked | Unix socket ownership, authenticated peer provenance, bounded IPC | 00, 02 |
| RB-CX-05 | P1 | Queued | `rb_gis` migration with roads, county/ZIP parity and rollback | 01, 02 |
| RB-CX-06 | P1 | Blocked | `rust_tv` native display and exclusive audio handoff | 01, 02 |
| RB-CX-07 | P1 | Blocked | `rusti_rc` session, explicit COMMS consent and offline recovery | 01, 02, 04 |
| RB-CX-08 | P1 | Blocked | Signed package installation, revocation, OS isolation | 02, 04 |
| RB-CX-09 | P2 | Blocked | Trusted RB-to-RB sync and optional mini-server after ORG-3.2 | 04, 07, 08 |
| RB-CX-10 | P2 | Blocked | Mac/Pi, accessibility, network failure and release acceptance | 03, 05, 06, 07, 08, 09 |

Each JSON pass declares its **entrypoints, component IDs, docs, tests, acceptance conditions, blockers and independent upstream repositories**. The table is a reading aid, not a second source of truth. `next` means work can begin, not that it has passed testing. Open PRs #64–#70 contain separate work and cannot be treated as merged prerequisites.

## Read-only next-pass handoff

Run `python3 tools/codex_next.py` for the next pass, source entrypoints, checks and blockers, or `python3 tools/codex_next.py --json` for machine-readable output. It validates the manifest before displaying anything and performs no network, repository mutation or module execution. The `remaining` list reports unsatisfied dependencies; it does not schedule the next task.

## Pass protocol for Codex

1. **Preflight.** Inspect checkout, current HEAD, branch and PR against `main`; preserve local edits. Read the one `next` pass and verify its dependencies. Do not silently import other PRs or independent repos.
2. **Implement one slice.** Read the machine-declared files first and add code/tests only where required. Keep modular algorithms in their independent source packages and preserve existing legacy behavior until parity and rollback are proven.
3. **Fail closed.** Package switching, mounting, page preview or malformed IPC must not start devices, media, network operations or messages; backend permissions and explicit operator consent are separate gates.
4. **Run checks.** Execute the offline index checks, relevant Rust workspace tests, and package-specific browser or physical tests. A GitHub `startup_failure` with zero jobs is neither a compiler failure nor a green run.
5. **Document evidence.** Record current commit SHA, commands and result, failures, not-run gates and exact CI run URL if available. Promote a pass to `accepted` only with a real 40-character commit hash and GitHub Actions run; a `field_pending` pass requires a checked-in field report. The validator checks structure, **not truth** of the run.
6. **Checkpoint.** Update `docs/PROD_STATUS.md`, `docs/VALIDATION.md` if evidence changes, `catalog/codex_passes.json` and component/doc navigation. Advance the next eligible pass atomically; keep the PR draft if acceptance is incomplete.

## Minimal verification set

```bash
python3 tools/validate_codex_passes.py && python3 tools/test_validate_codex_passes.py
python3 tools/validate_component_index.py && python3 tools/test_component_index.py
python3 tools/validate_repo.py && python3 tools/test_validate_repo.py
python3 tools/validate_host_packages.py && python3 tools/test_validate_host_packages.py
python3 tools/validate_security.py
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
node tools/test_desktop_focus_ui.cjs
node tools/test_micro_ui.cjs
node tools/test_desktop_viewport.cjs
node tools/test_micro_viewport.cjs
```

The existing [quality workflow](../.github/workflows/rust.yml) retains its Rust stable/1.90, native Mac/Windows and UI gates. Run all checks referenced in the selected JSON pass in addition to these common ones. See [validation history](VALIDATION.md), [index ownership map](INDEXING.md), [adapter guide](UNIVERSAL_ADAPTER.md) and [fleet-sync security gates](RB_SYNC_ROADMAP.md).

## Exact next-pass instructions

> Continue **RB-CX-00** on `codex/package-adapter-protocol`. Verify current HEAD and mainline divergence; build and test the passive RADIO catalogue with real Rust 1.90/stable; check that desktop/micro Settings and package selection do not start streams or devices; run relevant Chromium and offline index tests; investigate current-head Actions status rather than assuming old startup failures still occur; update the production checkpoint and pass-index evidence; checkpoint the existing draft PR without merging.

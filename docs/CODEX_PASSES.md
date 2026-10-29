# Codex development passes — Rust-Boy

**Branch scope, 2026-10-29:** This sequence targets draft [PR #71](https://github.com/pskeffington/pip_boy/pull/71), branch `codex/package-adapter-protocol`. The canonical machine backlog is [`catalog/codex_passes.json`](../catalog/codex_passes.json). Read the root [`AGENTS.md`](../AGENTS.md) for Codex operating rules and [production checkpoint](PROD_STATUS.md) for what was actually verified.

Rust-Boy is a modular pop-in socket: the shell owns **one package viewport**, compact navigation, host-owned Settings and permission boundaries. MAP, RADIO, TV and COMMS retain independent release lifecycles. The draft already contains a portable adapter SDK, in-process lifecycle manager, host Settings/foreground APIs, an unfinished Unix IPC precursor and a **compiled-in passive RADIO station catalogue**. It does *not* provide authenticated untrusted packages, safe dynamic installation or completed interactive native media adapters.

## Pass sequence

| Pass | Priority | Status | Focus | Prerequisites |
| --- | --- | --- | --- | --- |
| RB-CX-00 | P0 | **Accepted** | Exact-head build/CI baseline and passive Radio bootstrap verification | None |
| RB-CX-01 | P0 | **Accepted** | Safe universal data-driven viewport, Settings and fault handling | 00 |
| RB-CX-02 | P0 | **Accepted** | Backend capability intent, audio/device/transport arbitration | 00, 01 |
| RB-CX-03 | P1 | Blocked | Real interactive `rust_radio` package, explicit Play/Stop | 01, 02 |
| RB-CX-04 | P0 | **Accepted** | Unix socket ownership, authenticated peer provenance, bounded IPC | 00, 02 |
| RB-CX-05 | P1 | **Next** | `rb_gis` migration with roads, county/ZIP parity and rollback | 01, 02 |
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
python3 tools/validate_ci_policy.py && python3 tools/test_validate_ci_policy.py
python3 tools/validate_repo.py && python3 tools/test_validate_repo.py
python3 tools/validate_host_packages.py && python3 tools/test_validate_host_packages.py
python3 tools/validate_security.py && python3 tools/test_validate_security.py
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

## CI dependency elimination checkpoint

Commit `63093badc2046a59689693f68896fa502dd95234` removed external GitHub Action `uses:` calls from the draft workflow. Run `40823687340` on `5ad4327d34751cb723eae0ae7de7ef653e3702cf` isolated a macOS Node 24.22.0 mismatch. The repair pinned official Node 22.23.4 with SHA-256 checks, routed Rust toolchain setup through `scripts/ci_bootstrap.sh`, and added an offline CI-policy regression gate. Exact-head run `40824698037` then passed all seven jobs on `c425994eb8a172c357c6920e29d2b4d38cdc5050`. This remains **first-party action orchestration, not a fully self-hosted or offline build**.

Codex must inspect [run 40823597748](https://github.com/pskeffington/pip_boy/actions/runs/40823597748) and subsequent exact-head runs, confirm whether jobs actually start, evaluate the full stable/1.90/SDR/native matrix, and record failed commands and platform-specific defects. Do not reintroduce remote GitHub actions or mark a gate accepted based only on a running workflow. If checkout/toolchain provisioning fails, repair the first-party bootstrap while retaining all original validations.

## RB-CX-00 acceptance checklist and Codex handoff

**Scope:** repair/verify the passive, explicitly compiled RADIO catalog and host-owned navigation before implementing any generic external package loader. No changes to independent radio/GIS/TV/COMMS repositories are implied.

1. **Baseline:** print current branch, exact 40-character SHA, working-tree status and divergence against `origin/main`; inspect the PR head and existing workflow runs. Do not reset or overwrite local work. If GitHub Actions did not start any job, record `runner_admission_blocked` as a blocker rather than a source test failure.
2. **Offline index and CI-policy tests:** run `python3 tools/codex_next.py --json`, `python3 tools/validate_codex_passes.py`, `python3 tools/test_validate_codex_passes.py`, `python3 tools/validate_component_index.py`, `python3 tools/test_component_index.py`, `python3 tools/validate_ci_policy.py`, `python3 tools/test_validate_ci_policy.py`, `python3 tools/validate_repo.py`, `python3 tools/test_validate_repo.py`, `python3 tools/validate_host_packages.py`, `python3 tools/test_validate_host_packages.py`, `python3 tools/validate_security.py` and `python3 tools/test_validate_security.py`. Keep the output and return code for each step.
3. **Rust and static analysis:** run `cargo +stable fmt --all -- --check`, `cargo +stable check --workspace --all-targets --locked`, `cargo +stable test --workspace --locked`, `cargo +stable clippy --workspace --all-targets --locked -- -D warnings`; repeat `check` and `test` under Rust `+1.90.0` where installed. Missing compilers are `not_run`, not a pass.
4. **UI and safety:** run `node tools/test_desktop_focus_ui.cjs`, `node tools/test_micro_ui.cjs`, `node tools/test_desktop_viewport.cjs`, `node tools/test_micro_viewport.cjs`, `node tools/test_radio_ui.cjs` and `node tools/test_radio_viewport.cjs`. Confirm one foreground, persistent Settings, no root scrollbar, no raw HTML injection, and no implicit playback/network/device access from catalog selection.
5. **Evidence ledger:** for every command record `command`, `result` (`pass`, `fail`, `not_run` or `blocked`), exact commit SHA, platform, and reason or URL in `docs/PROD_STATUS.md` or `docs/VALIDATION.md`. CI requires an actual successful workflow run for that exact source revision; a zero-job run proves nothing about code quality.
6. **Promotion:** keep status `next` until all RB-CX-00 acceptance requirements pass. After positive exact-head CI evidence and review, change RB-CX-00 to `accepted` with its commit and CI URL, set RB-CX-01 to `next` in the **same** index change, run the validators again, and checkpoint. Document any still-unmet Mac/Pi acceptance separately.

### RB-CX-00 acceptance evidence — accepted

- Exact source head: `c425994eb8a172c357c6920e29d2b4d38cdc5050`; exact pull-request workflow [40824698037](https://github.com/pskeffington/pip_boy/actions/runs/40824698037), all seven jobs successful.
- Stable and Rust 1.90 passed locked compile/test; stable passed format, warnings-denied Clippy, CLI/runtime smokes and all Linux browser suites. Each Rust workspace suite reported 77 `pipboy`, 301 core, 3 SDK and 12 broker tests (393 total).
- Both Soapy-SDR and SDRconnect check, Clippy and test jobs passed without receiver discovery or capture. Native macOS passed the pinned Node v22.23.4 checksum bootstrap, native test/smoke/PTy suite and Mac-first offline MAP checks. Native Windows passed build/tests, offline smokes and ConPTY restoration.
- Desktop, micro and RADIO Chromium tests passed. Compact viewport fixtures verified no autoplay, network or device activity; Settings restoration and one foreground remained available. The Mac MAP viewport passed at 480×320, 800×480 and 320×568 without network access.
- The seven check conclusions and relevant Mac, Linux and Windows job logs were reviewed. Offline CI-policy, pass-index, component-index, host-package, repository, catalog, role, Vault, security and smoke-roster validators passed locally; local Node download was blocked by environment DNS, but the hosted Mac job verified the pinned binary.
- Acceptance-index checkpoint commit `b4719dcfa896d9d08c38e68d5c11567e8e7f9561` passed all seven CI jobs in run [40825620885](https://github.com/pskeffington/pip_boy/actions/runs/40825620885). That follow-up updated the read-only handoff regression fixture to reflect RB-CX-00's accepted state; it did not alter the recorded RB-CX-00 source acceptance.
- `hardware_gate` is `software_only`, so no field report is required for this pass. RB-CX-00 is accepted. RB-CX-01 subsequently passed its own exact-head validation, recorded below. The PR remains a draft; no merge or production release is claimed.

**RB-CX-01 design boundary (do not implement before RB-CX-00 is accepted):** a generic, schema-validated view model with safe text and input events, no raw package HTML/scripts, one responsive canvas, and host-owned Settings. Before enabling media controls or other privileged intents, require independent host capability policy and explicit operator action.

### RB-CX-01 acceptance evidence — accepted

- Exact source head `b039fbe6886920a071d836841d9cffe8fd75be99`; [run 40826559481](https://github.com/pskeffington/pip_boy/actions/runs/40826559481) completed successfully with all seven exact-head checks. The PR diagnostics reported catalog/source, stable, Rust 1.90, both SDR features, macOS and Windows all successful for that SHA.
- The Rust SDK defines bounded v1 `PackageView` heading/text/status/list blocks. Core rejects unknown or oversized view data and faults the package before foreground exposure. Desktop and micro use host-created DOM/text nodes; no package HTML, scripts, callbacks, links or privileged input actions are supported.
- Stable and Rust 1.90 each passed workspace format/check/tests/Clippy (395 tests); both SDR feature check/Clippy/test matrices passed under both toolchains. Offline catalog, host-package, component, repo, pass-index, CI-policy, security, roles, Vault and smoke validators passed.
- Desktop focus and micro interaction tests passed. Chromium desktop sizes 1440×900, 1280×800, 1024×768 and 800×600, and handheld 480×320, 800×480 and 320×568 plus Custom passed without document scrolling. Package selectors expose an accessible label and 44px touch targets; HTML-looking fixture text remained text in both renderers.
- `hardware_gate` is `software_only`. This accepts the bounded display pass only; OS grants, external loader/trust and physical-hardware acceptance remain later gates. RB-CX-02 was next and is now accepted below; PR #71 remains a draft.

## Exact next-pass instructions

> Continue **RB-CX-04** on `codex/package-adapter-protocol`. Harden the Unix adapter socket path and session provenance before accepting a peer: verify restrictive ownership and OS-reported peer identity, bound the exchange, and reject replay/stall/malformed frames. Keep the listener disconnected from a launcher, do not trust hello claims, and preserve the deny-only device broker. Run the indexed package/security checks and exact-head CI; keep PR #71 draft and `rust_radio/`, `rb_gis`, `rust_tv` and `rusti_rc` independent.

### RB-CX-02 acceptance evidence — accepted (software-only)

Exact validation head `c1df259d1e01689dcb93ed48073b250150027f56` passed all seven checks in [run 40828393462](https://github.com/pskeffington/pip_boy/actions/runs/40828393462). The implementation adds ephemeral role/package/scope approvals checked at each backend intent, revocation and role-mismatch regressions before adapter calls, and exclusive `media.audio` ownership with release after a successful stop. Stable and Rust 1.90 each passed 398 tests; both SDR feature check/test/Clippy matrices passed under both toolchains. Security/index validators, permission/broker smokes, desktop/micro interaction and desktop/handheld Chromium regressions passed. The broker remains deny-all. Durable approvals, consent UI, OS enforcement, native media and physical-device access remain outside this software-only acceptance; production release gates remain open.

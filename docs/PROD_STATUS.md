# Rust Boy production checkpoint

## Codex instruction checkpoint — 2026-10-29

**RB-CX-00 is accepted** on exact source head `c425994eb8a172c357c6920e29d2b4d38cdc5050` by [workflow run 40824698037](https://github.com/pskeffington/pip_boy/actions/runs/40824698037), all seven jobs successful. The first-party CI goal and universal-host validation gate are complete; **RB-CX-01 is now the next development pass**. PR #71 remains draft. RB-CX-00 is software-only; separate Pi/hardware acceptance remains a later release requirement.

## Workflow independence candidate

The reviewed workflow uses zero external GitHub Action `uses:` imports and exact-SHA checkout. Run [40823597748](https://github.com/pskeffington/pip_boy/actions/runs/40823597748) on `63093badc2046a59689693f68896fa502dd95234` was **cancelled**, not source-failed: its manifest job passed and the newer PR-head run cancelled the remaining in-flight jobs. Run [40823687340](https://github.com/pskeffington/pip_boy/actions/runs/40823687340) on `5ad4327d34751cb723eae0ae7de7ef653e3702cf` admitted all seven jobs. Stable, Rust 1.90, both SDR feature suites, manifest validation and native Windows passed. Linux desktop/micro/Radio browser checks passed. macOS completed compile/tests, native smokes and PTY, then failed because the runner supplied Node 24.22.0; its later Mac-first MAP step was skipped. This identified a runner-version assumption, not a Rust or adapter compile failure.

Commit `91753624bd39dfd9e7efccd85d5d1e33fe327891` pins official Node.js 22.23.4 by architecture-specific SHA-256 in `scripts/ci_bootstrap.sh`, routes stable/1.90 and native Rust setup through that repository-owned bootstrap, and adds an offline regression gate for zero Action imports, required matrices and pins. The exact-head hosted run verified the pinned Node binary on macOS and passed the complete matrix. External Rust toolchains, system packages and locked Cargo dependencies remain network dependencies; this is not a hermetic/offline build.


Updated: 2026-10-29

## Gate state

- **Latest exact PR head:** `c425994eb8a172c357c6920e29d2b4d38cdc5050`; [run 40824698037](https://github.com/pskeffington/pip_boy/actions/runs/40824698037) completed successfully with all seven checks green.
- **Current gate:** RB-CX-01 is `next`; RB-CX-00 is accepted with exact commit/run evidence in `catalog/codex_passes.json`. Its software-only acceptance does not waive desktop/mobile regression checks required by later passes.
- **Last accepted Codex pass:** RB-CX-00. **Last completed PROD roadmap gate:** PROD-0 remains **not accepted**; accepting this software validation pass does not certify the broader production baseline or release.
- **Branch:** draft [PR #71](https://github.com/pskeffington/pip_boy/pull/71), `codex/package-adapter-protocol`. First-party CI repair is included. `rust_radio/` remains an independent untracked nested repository and is untouched.
- **Relevant pinned module revision:** `rust_radio` dependency revision `3d77d11da3a31fbc3144635da6d391a3febf38b7` from the workspace lock/config. The local `rust_radio/` directory is a separate untracked nested repository and was preserved untouched.

## Implementation and evidence

- **Implemented, software validated:** `pipboy-core::package_socket` provides bounded JSON request, hello-claim and response codecs; highest-common-version negotiation; strictly increasing request IDs; and a 64-entry request correlator that validates package and ID matches. Identity in a hello remains an unauthenticated claim.
- **Implemented, software validated:** generic stream frame helpers use a 4-byte big-endian length, accept clean EOF, reject empty/truncated/oversized frames, and cap reads before allocation at 64 KiB.
- **Implemented, focused software validated:** `pipboy-core::package_transport::UnixAdapterSession` adds an absolute-deadline Unix stream handshake and serial request/response exchange; a peer's hello identity must match the expected package ID, and request IDs/package IDs are correlated. This accepts an already-open socket only; hello identity is not authentication and the transport does not authorize, dispatch, spawn, or isolate packages.
- **Implemented, software validated:** desktop and micro package selectors consume the mounted roster in `/api/state`; selection, Settings and foreground clearing update `UniversalAdapterHost` through same-origin endpoints. Both shells display host-provided views as inert text and preserve package selection across Settings/reload. The current candidate bootstrap mounts one passive RADIO catalogue, but it does not auto-select, start playback or grant hardware access.
- **Implemented, focused software validated:** `PrivateUnixListener` binds only in an existing current-user-owned private directory, refuses occupied paths, sets socket permissions to `0600`, checks the OS-reported UID and PID against the host's expected child process, and removes only the socket inode it created. Linux uses peer credentials; macOS uses `getpeereid` and `LOCAL_PEERPID`. This establishes process/account provenance at connection time, not module identity or signature; no listener is connected to a process launcher yet.
- **Still unimplemented:** timeout cancellation, domain dispatch, capability brokerage, signed/persistent installation state, process launch and isolation. The listener is not wired to a launcher; these remain later package trust/security passes.
- The listener module passed an isolated Rust 1.90 aarch64 Linux check. A full workspace Linux cross-check remains unverified because this Mac lacks `aarch64-linux-gnu-gcc` for the vendored D-Bus dependency.
- `cargo fmt --all -- --check` — passed.
- `cargo check --workspace` — passed.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `cargo test --workspace --locked -- --test-threads=1` — passed on merged candidate working tree: 77 `pipboy`, 301 core, 3 SDK and 12 broker tests (393 total), doc tests passed. Tests required local socket access; the sandboxed attempt failed three loopback fixtures with `EPERM`.
- `cargo test -p pipboy-core package_listener --locked -- --test-threads=1` — four listener security/lifecycle tests passed before merge; merged full suite includes them.
- `cargo +stable fmt --all -- --check`, `cargo +stable check --workspace --all-targets --locked`, `cargo +stable test --workspace --locked -- --test-threads=1`, and `cargo +stable clippy --workspace --all-targets --locked -- -D warnings` — passed on source revision `8a5c9d162af5ede9449dcf924f88fba15e9bdb38`.
- The same four commands with `cargo +1.90.0` — passed on the same source revision (393 Rust tests total).
- Isolated listener Linux cross-check, `cargo +1.90.0 check --manifest-path /tmp/rustboy-listener-linux-check/Cargo.toml --target aarch64-unknown-linux-gnu --offline` — passed. It compiles only `package_listener.rs` with `rustix` net/process features; it does not establish a full workspace cross-build.
- RADIO UI and viewport tests, desktop viewport (1440×900 through 800×600) and micro viewport (480×320, 800×480 and 320×568) suites — passed. These browser fixtures check no autoplay/I/O and Settings restoration.
- Pass-index, host-package, component-index, repository and security validators and their test suites — passed; security checked 282 tracked paths.
- `cargo test -p pipboy-core package_transport --locked` — passed: 6 transport tests.
- `cargo test --workspace --locked -- --test-threads=8` — passed on current controller-binding changes: 75 `pipboy`, 281 `pipboy-core` and 12 broker tests (368 total); doc tests passed (0 present). The first sandboxed attempt had three loopback permission failures; the same suite passed after granting local bind permission. The `pipboy` suite also passed five consecutive 8-thread runs at the prior checkpoint.
- `python3 tools/validate_host_packages.py` — passed.
- `python3 tools/test_validate_host_packages.py` — passed (8 tests).
- `python3 tools/validate_component_index.py` — passed.
- `python3 tools/validate_repo.py` — passed.
- `CHROME_BIN='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' node tools/test_dashboard_viewport.cjs` — passed at 480×320, 800×480, 1280×720 and 1920×1080, including default, custom, scout and security profiles. Short-screen Settings spacing now fits the role controls.
- `CHROME_BIN='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' node tools/test_desktop_viewport.cjs` — passed at 1440x900, 1280x800, 1024x768 and 800x600; checks single-view workspace, persistent Settings and restoring Radio after Settings.
- `CHROME_BIN='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' node tools/test_micro_viewport.cjs` — passed at 480×320, 800×480, 320×568 and 480×320 custom, plus desktop map coverage at 800×600, 1280×800 and 1920×1080. The fixture permits only reads and host Settings lifecycle writes.
- Combined desktop and handheld viewport evidence covers 480×320, 800×480, 320×568, 800×600, 1024×768, 1280×720, 1280×800, 1440×900, 1920×1080; no document scroll, settings restoration, or package canvas clipping assertions failed.
- `node tools/test_desktop_focus_ui.cjs`, `node tools/test_micro_ui.cjs`, JavaScript syntax checks, `tools/test_validate_host_packages.py`, `tools/validate_component_index.py`, and `tools/validate_repo.py` — passed.
- GitHub combined status and commit workflow-run queries for `2c13594` returned no status entries/runs. The repository workflow file defines CI, but the actual startup-failure cause is not established by this result.

## Accepted RB-CX-00 evidence — 2026-10-29

- Exact PR head `c425994eb8a172c357c6920e29d2b4d38cdc5050`; run [40824698037](https://github.com/pskeffington/pip_boy/actions/runs/40824698037) completed `success`, with catalog, stable, Rust 1.90, Soapy SDR, SDRconnect, native macOS and native Windows all green. The PR check diagnostic endpoint reported those same seven checks as successful for that SHA.
- Stable and Rust 1.90 each passed 393 locked workspace tests. Stable also passed formatting, warnings-denied Clippy, all desktop/micro/MAP/RADIO Chromium viewports and runtime smokes. Both SDR features passed compile, Clippy and tests without hardware capture/discovery.
- macOS passed Rust compile/tests, native offline/PTY checks, the repository bootstrap's pinned Node v22.23.4 and Mac-first offline MAP acceptance (MAP Chromium 480×320, 800×480, 320×568; no network). Windows passed native build/tests, offline smokes and ConPTY recovery.
- The RADIO viewport fixture permits only local `GET /api/radio`; opening/browsing does not issue Play or contact an external station. UI tests also passed the single foreground, no-autoplay and Settings restoration checks. No physical receiver or media output was used or claimed.
- Local stable and Rust 1.90 each passed format, locked all-target check/test, and warnings-denied Clippy. Both SDR feature matrices passed locally. All offline catalog, security, role, Vault, pass/component/CI-policy, host-package, repository and smoke-roster validators/tests passed. Local Node download was blocked by DNS, while exact-head hosted Mac verified the binary.
- Pass-index status is `RB-CX-00: accepted`, `RB-CX-01: next`. No production roadmap gate is accepted; release acceptance remains **0/12**.

## Release acceptance

- **Planned:** bind expected peer credentials to a real host launcher and verified binary identity, plus all remaining production gates.
- **Implemented but unverified:** compiled-in passive RADIO station-catalogue bootstrap and shell selection. Interactive native adapter playback, signed installation and hardware UI acceptance are open.
- **Software validated locally:** manifest registry, passive RADIO catalogue, request codec/framing, deadline-bounded Unix stream session, private Unix socket path/permissions and expected-process OS peer credential checks, and desktop/micro shell lifecycle linkage.
- **Hardware validated:** none for adapter integration.
- **Release candidate / production accepted:** neither. **Accepted roadmap gates: 0/12**; PROD-3 is a source candidate, not a hardware-accepted interactive package.

## Open defects and blockers

- The Node mismatch on run `40823687340` was repaired by the pinned official v22.23.4 bootstrap; exact-head run `40824698037` verified it and all matrix jobs passed. Earlier startup failures and cancellations remain historical CI events, not current blockers.
- One earlier full parallel run returned HTTP 422 in `manager_can_create_assign_dispatch_and_track_but_other_roles_cannot_reassign`; the failure did not reproduce in isolation, serial execution, five consecutive parallel suite runs or the latest full workspace run. The test now includes task-state context if it recurs.
- One explicitly compiled, capability-free RADIO catalogue adapter is mounted by default, but not selected automatically. MAP, TV and COMMS remain inert references; signed installation and real audio-adapter integration remain open.
- Local Mac/Pi hardware, media output and independently maintained package integration have not been accepted.
- Expected UID/PID checks do not establish package identity, signature, or binary integrity.

## Codex development handoff

`python3 tools/codex_next.py --json` is a read-only, validated handoff of the next eligible RB-CX-01 pass. The `validate_ci_policy.py` / `test_validate_ci_policy.py` guard the zero-`uses:` rule, exact-SHA checkout, stable/1.90, SDR/native matrices and pinned Node bootstrap. Index status is backed by the exact run recorded above.


The [machine pass queue](../catalog/codex_passes.json) records **RB-CX-00 accepted** with exact source SHA and successful workflow URL, and names **RB-CX-01** as the only `next` pass. Historical startup failures, cancellation and the repaired macOS Node mismatch remain documented below; the latest exact run has all checks green. See [Codex instructions](../AGENTS.md), [pass plan](CODEX_PASSES.md) and [index hierarchy](INDEXING.md).

### RB-CX-01 handoff requirements

- Implement and validate the bounded host-owned view model with safe text/input handling on desktop and micro.
- Preserve one foreground, permanent Settings, failure recovery, accessibility and no document scrolling; package selection must never start playback, network or device activity.
- Run negative/regression and desktop/micro/Radio viewport checks, then require a new successful exact-head matrix. Keep PR #71 draft.

## Next three implementation actions

1. Inspect RB-CX-01's declared renderer/SDK entrypoints and current package-view representation; design a bounded versioned schema.
2. Implement desktop and micro safe rendering with host-owned lifecycle, Settings restoration and fault behavior, plus negative tests.
3. Run offline/Rust/UI checks and exact-head CI; document evidence without merging or weakening gates.

## Release risks

No package loader or process boundary exists. Manifest metadata and the typed codec confer no trust or capability. Hardware acceptance, UI integration, package provenance/signatures, IPC security, rollback, SBOM and deployment packaging remain open.

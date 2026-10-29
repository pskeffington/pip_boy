# Rust Boy production checkpoint

## Codex instruction checkpoint — 2026-10-29

**RB-CX-00 and RB-CX-01 remain accepted. RB-CX-02 is the active, unaccepted pass.** The acceptance-index checkpoint `b4387cd2913c07355b5bc5d4d70ea2a18e6d94fe` passed all seven exact-head jobs in [workflow run 40827207962](https://github.com/pskeffington/pip_boy/actions/runs/40827207962). The current uncommitted RB-CX-02 candidate adds role-bound in-memory adapter approvals, revocation checks at each intent, and exclusive audio ownership. Stable and Rust 1.90 workspace tests (398 each), both SDR feature test matrices on both toolchains, formatting, checks and warnings-denied Clippy passed locally; exact-head CI for this candidate is pending. PR #71 remains draft. The physical-device broker remains deny-only; no OS-backed grant, persistent approval, or production release is claimed.

## Workflow independence candidate

The reviewed workflow uses zero external GitHub Action `uses:` imports and exact-SHA checkout. Run [40823597748](https://github.com/pskeffington/pip_boy/actions/runs/40823597748) on `63093badc2046a59689693f68896fa502dd95234` was **cancelled**, not source-failed: its manifest job passed and the newer PR-head run cancelled the remaining in-flight jobs. Run [40823687340](https://github.com/pskeffington/pip_boy/actions/runs/40823687340) on `5ad4327d34751cb723eae0ae7de7ef653e3702cf` admitted all seven jobs. Stable, Rust 1.90, both SDR feature suites, manifest validation and native Windows passed. Linux desktop/micro/Radio browser checks passed. macOS completed compile/tests, native smokes and PTY, then failed because the runner supplied Node 24.22.0; its later Mac-first MAP step was skipped. This identified a runner-version assumption, not a Rust or adapter compile failure.

Commit `91753624bd39dfd9e7efccd85d5d1e33fe327891` pins official Node.js 22.23.4 by architecture-specific SHA-256 in `scripts/ci_bootstrap.sh`, routes stable/1.90 and native Rust setup through that repository-owned bootstrap, and adds an offline regression gate for zero Action imports, required matrices and pins. The exact-head hosted run verified the pinned Node binary on macOS and passed the complete matrix. External Rust toolchains, system packages and locked Cargo dependencies remain network dependencies; this is not a hermetic/offline build.


Updated: 2026-10-29

## Gate state

- **Latest pushed validation head:** `b4387cd2913c07355b5bc5d4d70ea2a18e6d94fe`; [run 40827207962](https://github.com/pskeffington/pip_boy/actions/runs/40827207962) completed successfully with all seven checks green, including source/index, Rust stable, Rust 1.90, both SDR features, native macOS and native Windows.
- **Current gate:** RB-CX-02 remains the sole `next` pass and is being implemented; it is not accepted. RB-CX-00 and RB-CX-01 retain their exact evidence in `catalog/codex_passes.json`. The new source candidate is not yet committed or validated by exact-head CI.
- **Last accepted Codex pass:** RB-CX-00. **Last completed PROD roadmap gate:** PROD-0 remains **not accepted**; accepting this software validation pass does not certify the broader production baseline or release.
- **Branch:** draft [PR #71](https://github.com/pskeffington/pip_boy/pull/71), `codex/package-adapter-protocol`. First-party CI repair is included. `rust_radio/` remains an independent untracked nested repository and is untouched.
- **Relevant pinned module revision:** `rust_radio` dependency revision `3d77d11da3a31fbc3144635da6d391a3febf38b7` from the workspace lock/config. The local `rust_radio/` directory is a separate untracked nested repository and was preserved untouched.

## RB-CX-02 candidate — backend approvals and media ownership

- Added `AdapterAuthorization`, an in-memory approval authority keyed by active Rust role, package ID and manifest scope. Every scoped intent checks the host-supplied manifest declaration and the current authority before adapter invocation; approval revocation and role mismatch therefore stop at the backend boundary.
- Audio requests use a separate owner check: only the current owner may play/resume/pause/stop, a second package cannot take ownership, and a successful stop releases the owner. Role changes are rejected while an audio owner remains active.
- These approvals are neither loaded from `permissions.json` nor inferred from package manifests, UI state or role tool pins. They are not OS-enforced permissions. `rust-boy-broker` remains a separate deny-all process with no device/network handles.
- Local stable and Rust 1.90 workspace tests passed (398 each) after granting the existing loopback-only fixtures access. Both SDR feature test/check/Clippy matrices passed under both toolchains. Targeted permission and network/device/message denial tests passed; see [validation](VALIDATION.md) for the exact commands and local Chromium limitation.
- Remaining acceptance boundary: no consent UI integration, durable approval workflow, OS authorization, real media backend or external package execution is accepted. The index still leaves RB-CX-02 unaccepted until all of its code and CI evidence is reviewed.

## Implementation and evidence

- **Implemented, software validated:** `pipboy-core::package_socket` provides bounded JSON request, hello-claim and response codecs; highest-common-version negotiation; strictly increasing request IDs; and a 64-entry request correlator that validates package and ID matches. Identity in a hello remains an unauthenticated claim.
- **Implemented, software validated:** generic stream frame helpers use a 4-byte big-endian length, accept clean EOF, reject empty/truncated/oversized frames, and cap reads before allocation at 64 KiB.
- **Implemented, focused software validated:** `pipboy-core::package_transport::UnixAdapterSession` adds an absolute-deadline Unix stream handshake and serial request/response exchange; a peer's hello identity must match the expected package ID, and request IDs/package IDs are correlated. This accepts an already-open socket only; hello identity is not authentication and the transport does not authorize, dispatch, spawn, or isolate packages.
- **Implemented, software validated:** desktop and micro package selectors consume the mounted roster in `/api/state`; selection, Settings and foreground clearing update `UniversalAdapterHost` through same-origin endpoints. The accepted host mounts one passive RADIO catalogue and never auto-selects or starts playback.
- **Accepted RB-CX-01:** the SDK defines a bounded version-1 display-only `PackageView` (heading/text/status/list), the core validates it before exposing a foreground, and desktop/micro render host-created DOM using text content. Accessible native selectors meet 44px touch sizing. Exact-head run 40826559481 passed stable/1.90, both SDR features, Linux Chromium UI/viewport suites, native macOS and native Windows. Interactive package actions remain deferred to RB-CX-02.
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
- Pass-index status is `RB-CX-00: accepted`, `RB-CX-01: accepted`, `RB-CX-02: next`. No production roadmap gate is accepted; release acceptance remains **0/12**.

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

`python3 tools/codex_next.py --json` is a read-only, validated handoff of RB-CX-02. The `validate_ci_policy.py` / `test_validate_ci_policy.py` guard the zero-`uses:` rule, exact-SHA checkout, stable/1.90, SDR/native matrices and pinned Node bootstrap. RB-CX-00 acceptance is supported by run 40824698037; the RB-CX-00 index checkpoint passed all seven checks on run 40825620885; RB-CX-01 acceptance is supported by run 40826559481; the subsequent accepted-index head b4387 passed all seven checks on run 40827207962.


The [machine pass queue](../catalog/codex_passes.json) records RB-CX-00 and RB-CX-01 as accepted with exact source SHAs and successful workflow URLs; RB-CX-02 remains the only `next` pass while its candidate is validated. The latest pushed index head b4387 passed all seven CI jobs in run 40827207962. Historical startup failures, cancellation, the repaired macOS Node mismatch, and stale handoff-test fixtures are documented below; see [Codex instructions](../AGENTS.md), [pass plan](CODEX_PASSES.md) and [index hierarchy](INDEXING.md).

### RB-CX-02 current acceptance requirements

- Deny unapproved network/media/device/message intents in the backend before side effects; preserve deny-first behavior and deny-only broker.
- Test revocation and role mismatch; never infer grants from UI or manifest declarations. Package display views remain inert.
- Run indexed security and broker regressions, stable/1.90 and full exact-head CI. Keep PR #71 draft.

## Next three implementation actions

1. Finish the stable/1.90 workspace and Soapy-SDR/SDRconnect feature test matrices for the current source candidate.
2. Re-run the complete pass validators and desktop/micro/Chromium regressions; preserve the deny-only physical broker and zero external Action imports.
3. Commit and push the coherent source/docs checkpoint to PR #71, inspect all seven exact-head checks, and accept RB-CX-02 only if its backend-denial, revocation, role-mismatch and media-owner requirements all pass. Keep the PR draft and record OS-backed permissions as a later blocker.

## Release risks

No external package loader or OS process isolation exists. Manifest metadata and the typed codec confer no trust or capability. Physical hardware acceptance, interactive package intents, package provenance/signatures, IPC security, rollback, SBOM and deployment packaging remain open.

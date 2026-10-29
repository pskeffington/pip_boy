# Rust Boy production checkpoint

Updated: 2026-10-29

## Gate state

- **Current gate:** PROD-2 / host UI integration — **implementation candidate; software validated**. Desktop and micro shells now select only mounted packages via host APIs, show inert package view data, and clear foreground on shell navigation. The roster is empty, so real package acceptance remains open.
- **Last completed gate:** PROD-0 baseline repair — **not accepted**. Mainline working tree and local validation are understood; GitHub Actions evidence is currently unavailable for the inspected commit, and CI startup failure has not been diagnosed.
- **Branch:** `codex/package-adapter-protocol`; shell integration commit `c653048`. Earlier controller and transport commits: `8197ffc`, `c020483`, checkpoint `0598d8d`.
- **Relevant pinned module revision:** `rust_radio` dependency revision `3d77d11da3a31fbc3144635da6d391a3febf38b7` from the workspace lock/config. The local `rust_radio/` directory is a separate untracked nested repository and was preserved untouched.

## Implementation and evidence

- **Implemented, software validated:** `pipboy-core::package_socket` provides bounded JSON request, hello-claim and response codecs; highest-common-version negotiation; strictly increasing request IDs; and a 64-entry request correlator that validates package and ID matches. Identity in a hello remains an unauthenticated claim.
- **Implemented, software validated:** generic stream frame helpers use a 4-byte big-endian length, accept clean EOF, reject empty/truncated/oversized frames, and cap reads before allocation at 64 KiB.
- **Implemented, focused software validated:** `pipboy-core::package_transport::UnixAdapterSession` adds an absolute-deadline Unix stream handshake and serial request/response exchange; a peer's hello identity must match the expected package ID, and request IDs/package IDs are correlated. This accepts an already-open socket only; hello identity is not authentication and the transport does not authorize, dispatch, spawn, or isolate packages.
- **Implemented, software validated:** desktop and micro package selectors consume the mounted roster in `/api/state`; selection, Settings and foreground clearing update `UniversalAdapterHost` through same-origin endpoints. Both shells display only host-provided views as inert text and preserve the selected package over Settings/reload. Selection remains impossible with an empty mounted roster.
- **Still unimplemented:** secure listener/socket-path ownership, peer authentication, timeout cancellation, domain dispatch, capability brokerage, trusted installation state, process launch and isolation.
- `cargo fmt --all -- --check` — passed.
- `cargo check --workspace` — passed.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `cargo test --workspace -- --test-threads=1` — passed: 77 `pipboy`, 294 core, 3 SDK and 12 broker tests (386 total), doc tests passed. The sandbox blocked three localhost fixtures on the first attempt; the full suite passed with localhost socket access.
- `cargo test -p pipboy-core package_transport --locked` — passed: 6 transport tests.
- `cargo test --workspace --locked -- --test-threads=8` — passed on current controller-binding changes: 75 `pipboy`, 281 `pipboy-core` and 12 broker tests (368 total); doc tests passed (0 present). The first sandboxed attempt had three loopback permission failures; the same suite passed after granting local bind permission. The `pipboy` suite also passed five consecutive 8-thread runs at the prior checkpoint.
- `python3 tools/validate_host_packages.py` — passed.
- `python3 tools/test_validate_host_packages.py` — passed (8 tests).
- `python3 tools/validate_component_index.py` — passed.
- `python3 tools/validate_repo.py` — passed.
- `CHROME_BIN='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' node tools/test_dashboard_viewport.cjs` — passed at 480x320, 800x480, 1280x720 and 1920x1080; checks full-width tool details on demand, return navigation, no document scrolling, and Settings restoration.
- `CHROME_BIN='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' node tools/test_desktop_viewport.cjs` — passed at 1440x900, 1280x800, 1024x768 and 800x600; checks single-view workspace, persistent Settings and restoring Radio after Settings.
- Chromium viewport runs after the controller binding passed at handheld 480x320, 800x480 and desktop 1280x720, 1920x1080, 1440x900, 1280x800, 1024x768 and 800x600. The desktop probe verifies Settings open/close API transitions and Radio restoration.
- `node tools/test_desktop_focus_ui.cjs`, `node tools/test_micro_ui.cjs`, JavaScript syntax checks, `tools/test_validate_host_packages.py`, `tools/validate_component_index.py`, and `tools/validate_repo.py` — passed.
- `CHROME_BIN='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' node tools/test_dashboard_viewport.cjs` — failed at 480×320 on the existing Settings role-selector clipping assertion. The earlier viewport and Settings restoration assertions at that size passed; other required viewport sizes were not reached. Needs layout investigation.
- GitHub combined status and commit workflow-run queries for `2c13594` returned no status entries/runs. The repository workflow file defines CI, but the actual startup-failure cause is not established by this result.

## Release acceptance

- **Planned:** secure socket listener/peer provenance and all remaining production gates.
- **Implemented but unverified:** real installed-package bootstrap, real adapter rendering, and hardware UI acceptance.
- **Software validated:** manifest registry, request codec/framing, deadline-bounded Unix stream session precursor, and desktop/micro shell lifecycle linkage.
- **Hardware validated:** none for adapter integration.
- **Release candidate / production accepted:** neither. **Accepted roadmap gates: 0/12**; PROD-2 remains an implementation candidate pending a real mounted module and viewport/hardware acceptance.

## Open defects and blockers

- Diagnose the historical GitHub Actions `startup_failure` using an actual affected run and its job/runner diagnostics; no run is currently exposed for the inspected main commit.
- One earlier full parallel run returned HTTP 422 in `manager_can_create_assign_dispatch_and_track_but_other_roles_cannot_reassign`; the failure did not reproduce in isolation, serial execution, five consecutive parallel suite runs or the latest full workspace run. The test now includes task-state context if it recurs.
- There is no approved installed-package roster, so no package can yet be selected and package content is only exercised through synthetic API tests.
- Local Mac/Pi hardware, media output and independently maintained package integration have not been accepted. The 480×320 dashboard Settings role selector clipping failure is a live UI regression gate.

## Next three implementation actions

1. Investigate and fix the 480×320 Settings role-selector clipping, then run the full desktop and handheld viewport matrices.
2. Implement trusted first-party installed-state bootstrap and mount a real reviewed package without enabling manifest-driven code execution.
3. Add secure Unix socket creation/ownership and peer provenance checks, then retrieve an actual failed GitHub Actions run and classify startup failure.

## Release risks

No package loader or process boundary exists. Manifest metadata and the typed codec confer no trust or capability. Hardware acceptance, UI integration, package provenance/signatures, IPC security, rollback, SBOM and deployment packaging remain open.

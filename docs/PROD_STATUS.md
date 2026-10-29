# Rust Boy production checkpoint

Updated: 2026-10-29

## Gate state

- **Current gate:** PROD-1 / local adapter transport — **in progress**; PROD-2 shell integration remains open.
- **Last completed gate:** PROD-0 baseline repair — **not accepted**. Mainline working tree and local validation are understood; GitHub Actions evidence is currently unavailable for the inspected commit, and CI startup failure has not been diagnosed.
- **Branch:** `codex/package-adapter-protocol`; transport implementation `c020483`. UI implementation is `4800d3a`; protocol commits include `5749054`, `28fa747`, `c15fbe3`, and checkpoint `fac8a85`.
- **Relevant pinned module revision:** `rust_radio` dependency revision `3d77d11da3a31fbc3144635da6d391a3febf38b7` from the workspace lock/config. The local `rust_radio/` directory is a separate untracked nested repository and was preserved untouched.

## Implementation and evidence

- **Implemented, software validated:** `pipboy-core::package_socket` provides bounded JSON request, hello-claim and response codecs; highest-common-version negotiation; strictly increasing request IDs; and a 64-entry request correlator that validates package and ID matches. Identity in a hello remains an unauthenticated claim.
- **Implemented, software validated:** generic stream frame helpers use a 4-byte big-endian length, accept clean EOF, reject empty/truncated/oversized frames, and cap reads before allocation at 64 KiB.
- **Implemented, focused software validated:** `pipboy-core::package_transport::UnixAdapterSession` adds an absolute-deadline Unix stream handshake and serial request/response exchange; a peer's hello identity must match the expected package ID, and request IDs/package IDs are correlated. This accepts an already-open socket only; hello identity is not authentication and the transport does not authorize, dispatch, spawn, or isolate packages.
- **Still unimplemented:** secure listener/socket-path ownership, peer authentication, timeout cancellation, domain dispatch, capability brokerage, trusted installation state, process launch and isolation.
- `cargo fmt --all -- --check` — passed on the current transport branch.
- `cargo check --workspace --all-targets --locked` — passed on the current transport branch.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — passed on the current transport branch.
- `cargo test -p pipboy-core package_transport --locked` — passed: 6 transport tests.
- `cargo test --workspace --locked -- --test-threads=8` — passed on current working changes: 74 `pipboy`, 281 `pipboy-core` and 12 broker tests (367 total); doc tests passed (0 present). The first sandboxed attempt had three loopback permission failures; the same suite passed after granting local bind permission. The `pipboy` suite also passed five consecutive 8-thread runs at the prior checkpoint.
- `python3 tools/validate_host_packages.py` — passed.
- `python3 tools/test_validate_host_packages.py` — passed (8 tests).
- `python3 tools/validate_component_index.py` — passed.
- `python3 tools/validate_repo.py` — passed.
- `CHROME_BIN='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' node tools/test_dashboard_viewport.cjs` — passed at 480x320, 800x480, 1280x720 and 1920x1080; checks full-width tool details on demand, return navigation, no document scrolling, and Settings restoration.
- `CHROME_BIN='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' node tools/test_desktop_viewport.cjs` — passed at 1440x900, 1280x800, 1024x768 and 800x600; checks single-view workspace, persistent Settings and restoring Radio after Settings.
- `node tools/test_desktop_focus_ui.cjs`, `node tools/test_micro_ui.cjs`, and JavaScript syntax checks — passed.
- GitHub combined status and commit workflow-run queries for `2c13594` returned no status entries/runs. The repository workflow file defines CI, but the actual startup-failure cause is not established by this result.

## Release acceptance

- **Planned:** host foreground UI binding, secure socket listener/peer provenance, and all remaining production gates.
- **Implemented but unverified:** none recorded here.
- **Software validated:** manifest registry, request codec/framing and deadline-bounded Unix stream session precursor.
- **Hardware validated:** none for adapter integration.
- **Release candidate / production accepted:** neither. **Accepted roadmap gates: 0/12**; the HOST-UI-1 shell candidate has not passed controller-binding acceptance.

## Open defects and blockers

- Diagnose the historical GitHub Actions `startup_failure` using an actual affected run and its job/runner diagnostics; no run is currently exposed for the inspected main commit.
- One earlier full parallel run returned HTTP 422 in `manager_can_create_assign_dispatch_and_track_but_other_roles_cannot_reassign`; the failure did not reproduce in isolation, serial execution, five consecutive parallel suite runs or the latest full workspace run. The test now includes task-state context if it recurs.
- Desktop/micro rendering remains unbound to `ForegroundSelection` and an approved package roster; the new single-view/Settings shell is an integration candidate, not gate acceptance.
- Local Mac/Pi hardware, media output and independently maintained package integration have not been accepted.

## Next three implementation actions

1. Bind host-owned foreground selection and approved installed-package enumeration to the real desktop and micro renderers, retaining authorization and viewport regression checks.
2. Add secure Unix socket creation/ownership and peer provenance checks; do not launch untrusted processes or imply capability grants.
3. Retrieve an actual failed GitHub Actions run and classify startup failure before modifying workflow configuration; preserve checks as written.

## Release risks

No package loader or process boundary exists. Manifest metadata and the typed codec confer no trust or capability. Hardware acceptance, UI integration, package provenance/signatures, IPC security, rollback, SBOM and deployment packaging remain open.

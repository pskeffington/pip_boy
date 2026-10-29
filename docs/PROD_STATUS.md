# Rust Boy production checkpoint

Updated: 2026-10-28

## Gate state

- **Current gate:** PROD-2 / host UI integration — **in progress**; PROD-1 protocol precursor remains software validated.
- **Last completed gate:** PROD-0 baseline repair — **not accepted**. Mainline working tree and local validation are understood; GitHub Actions evidence is currently unavailable for the inspected commit, and CI startup failure has not been diagnosed.
- **Branch:** `codex/package-adapter-protocol`; latest implementation commit `4800d3a` (single-view workspace and persistent Settings). Protocol commits: `5749054` (request codec), `28fa747` (negotiation/correlation), `c15fbe3` (bounded stream framing), and `fac8a85` (checkpoint).
- **Relevant pinned module revision:** `rust_radio` dependency revision `3d77d11da3a31fbc3144635da6d391a3febf38b7` from the workspace lock/config. The local `rust_radio/` directory is a separate untracked nested repository and was preserved untouched.

## Implementation and evidence

- **Implemented, software validated:** `pipboy-core::package_socket` provides bounded JSON request, hello-claim and response codecs; highest-common-version negotiation; strictly increasing request IDs; and a 64-entry request correlator that validates package and ID matches. Identity in a hello remains an unauthenticated claim.
- **Implemented, software validated:** generic stream frame helpers use a 4-byte big-endian length, accept clean EOF, reject empty/truncated/oversized frames, and cap reads before allocation at 64 KiB.
- **Still unimplemented:** connected IPC, handshake deadlines, timeouts/cancellation, cryptographic authentication, dispatch, capability brokerage, trusted installation state and process isolation.
- `cargo fmt --all -- --check` — passed.
- `cargo check --workspace --all-targets --locked` — passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — passed.
- `cargo test --workspace --locked -- --test-threads=8` — passed: 74 `pipboy`, 275 `pipboy-core` and 12 broker tests (361 total); doc tests passed (0 present). The `pipboy` suite also passed five consecutive 8-thread runs. Earlier sandbox denial of localhost binds was resolved by granting the test process local bind permission.
- `python3 tools/validate_host_packages.py` — passed.
- `python3 tools/test_validate_host_packages.py` — passed (8 tests).
- `python3 tools/validate_component_index.py` — passed.
- `python3 tools/validate_repo.py` — passed.
- `CHROME_BIN='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' node tools/test_dashboard_viewport.cjs` — passed at 480x320, 800x480, 1280x720 and 1920x1080; checks full-width tool details on demand, return navigation, no document scrolling, and Settings restoration.
- `CHROME_BIN='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome' node tools/test_desktop_viewport.cjs` — passed at 1440x900, 1280x800, 1024x768 and 800x600; checks single-view workspace, persistent Settings and restoring Radio after Settings.
- `node tools/test_desktop_focus_ui.cjs`, `node tools/test_micro_ui.cjs`, and JavaScript syntax checks — passed.
- GitHub combined status and commit workflow-run queries for `2c13594` returned no status entries/runs. The repository workflow file defines CI, but the actual startup-failure cause is not established by this result.

## Release acceptance

- **Planned:** host foreground UI binding, external package transport and all remaining production gates.
- **Implemented but unverified:** none recorded here.
- **Software validated:** manifest registry precursor and this request codec.
- **Hardware validated:** none for adapter integration.
- **Release candidate / production accepted:** neither. **Accepted roadmap gates: 0/12**; the HOST-UI-1 shell candidate has not passed controller-binding acceptance.

## Open defects and blockers

- Diagnose the historical GitHub Actions `startup_failure` using an actual affected run and its job/runner diagnostics; no run is currently exposed for the inspected main commit.
- One earlier full parallel run returned HTTP 422 in `manager_can_create_assign_dispatch_and_track_but_other_roles_cannot_reassign`; the failure did not reproduce in isolation, serial execution, five consecutive parallel suite runs or the latest full workspace run. The test now includes task-state context if it recurs.
- Desktop/micro rendering remains unbound to `ForegroundSelection` and an approved package roster; the new single-view/Settings shell is an integration candidate, not gate acceptance.
- Local Mac/Pi hardware, media output and independently maintained package integration have not been accepted.

## Next three implementation actions

1. Connect the generic framing helpers to an explicit local transport with handshake deadlines, cancellation and malformed/truncated-frame fault tests; do not add package execution yet.
2. Bind host-owned foreground selection and approved installed-package enumeration to the real desktop and micro renderers, retaining authorization and viewport regression checks.
3. Retrieve an actual failed GitHub Actions run and classify startup failure before modifying workflow configuration; preserve checks as written.

## Release risks

No package loader or process boundary exists. Manifest metadata and the typed codec confer no trust or capability. Hardware acceptance, UI integration, package provenance/signatures, IPC security, rollback, SBOM and deployment packaging remain open.

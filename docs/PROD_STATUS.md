# Rust Boy production checkpoint

Updated: 2026-10-28

## Gate state

- **Current gate:** PROD-1 / adapter protocol contract — **in progress**.
- **Last completed gate:** PROD-0 baseline repair — **not accepted**. Mainline working tree and local validation are understood; GitHub Actions evidence is currently unavailable for the inspected commit, and CI startup failure has not been diagnosed.
- **Branch:** `codex/package-adapter-protocol`, based on `2c13594` (`docs: record pure single foreground selector and remaining GUI integration gate`). No task commit exists yet.
- **Relevant pinned module revision:** `rust_radio` dependency revision `3d77d11da3a31fbc3144635da6d391a3febf38b7` from the workspace lock/config. The local `rust_radio/` directory is a separate untracked nested repository and was preserved untouched.

## Implementation and evidence

- **Implemented, software validated:** `pipboy-core::package_socket::AdapterMessage` provides bounded JSON request encoding/decoding for activate, suspend, resume, resize, intent and shutdown. It validates protocol version, nonzero request ID, package ID, resize range and a 64 KiB frame ceiling. Two focused Rust tests pass.
- **Still unimplemented:** transport framing across streams, handshake, response/event messages, timeouts/cancellation, authentication, dispatch, capability brokerage, trusted installation state and process isolation.
- `cargo fmt --all -- --check` — passed.
- `cargo check --workspace --all-targets --locked` — passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — passed.
- `cargo test --workspace --locked -- --test-threads=1` — passed: 271 `pipboy` tests and 12 broker tests; doc tests passed (0 present). The ordinary parallel run first failed because sandbox policy blocked three synthetic loopback socket fixtures; after permission to bind localhost, one unrelated shared task-store assertion flaked under parallel execution, passed in isolation, and the serial full suite passed.
- `python3 tools/validate_host_packages.py` — passed.
- `python3 tools/test_validate_host_packages.py` — passed (8 tests).
- `python3 tools/validate_component_index.py` — passed.
- `python3 tools/validate_repo.py` — passed.
- GitHub combined status and commit workflow-run queries for `2c13594` returned no status entries/runs. The repository workflow file defines CI, but the actual startup-failure cause is not established by this result.

## Release acceptance

- **Planned:** host foreground UI binding, external package transport and all remaining production gates.
- **Implemented but unverified:** none recorded here.
- **Software validated:** manifest registry precursor and this request codec.
- **Hardware validated:** none for adapter integration.
- **Release candidate / production accepted:** neither.

## Open defects and blockers

- Diagnose the historical GitHub Actions `startup_failure` using an actual affected run and its job/runner diagnostics; no run is currently exposed for the inspected main commit.
- Parallel `pipboy` tests have at least one apparent shared task-store race (`manager_can_create_assign_dispatch_and_track_but_other_roles_cannot_reassign`); isolated and serial execution pass. Investigate fixture isolation before treating parallel execution as stable.
- Actual desktop/micro rendering remains unbound to `ForegroundSelection`; one-package UI and always-available Settings release gate is open.
- Local Mac/Pi hardware, media output and independently maintained package integration have not been accepted.

## Next three implementation actions

1. Extend the protocol contract with bounded response/event messages, handshake and deterministic sequence/correlation rejection tests.
2. Bind host-owned foreground selection and permanent Settings to the real desktop and micro renderers, retaining existing authorization and viewport regression checks.
3. Retrieve an actual failed GitHub Actions run and classify startup failure before modifying workflow configuration; preserve checks as written.

## Release risks

No package loader or process boundary exists. Manifest metadata and the typed codec confer no trust or capability. Hardware acceptance, UI integration, package provenance/signatures, IPC security, rollback, SBOM and deployment packaging remain open.

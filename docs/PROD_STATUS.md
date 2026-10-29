# Rust Boy production checkpoint

Updated: 2026-10-29

## Gate state

- **Latest locally tested source revision:** `8edff4b948b24187786dcde61db74e212b26485d`; PR #71 check diagnostics for this exact revision returned no checks, statuses, or jobs. Local tests below are not CI evidence.
- **Current gate:** PROD-3 / passive RADIO bootstrap — **local stable software and browser checks pass; pass remains open pending Rust 1.90 and exact-head CI evidence**. Desktop and micro shells now select only mounted packages via host APIs, show inert package view data, and clear foreground on shell navigation. A passive, compiled-in RADIO catalogue now populates one roster entry; interactive playback and device acceptance remain open.
- **Last completed gate:** PROD-0 baseline repair — **not accepted**. Mainline working tree and local validation are understood; GitHub Actions evidence is currently unavailable for the inspected commit, and CI startup failure has not been diagnosed.
- **Branch:** draft [PR #71](https://github.com/pskeffington/pip_boy/pull/71), `codex/package-adapter-protocol`. Previous UI/transport integration checkpoints are documented below; inspect the live PR HEAD before claiming a build or CI result. The passive RADIO pilot and new Codex indexing changes are still validation candidates.
- **Relevant pinned module revision:** `rust_radio` dependency revision `3d77d11da3a31fbc3144635da6d391a3febf38b7` from the workspace lock/config. The local `rust_radio/` directory is a separate untracked nested repository and was preserved untouched.

## Implementation and evidence

- **Implemented, software validated:** `pipboy-core::package_socket` provides bounded JSON request, hello-claim and response codecs; highest-common-version negotiation; strictly increasing request IDs; and a 64-entry request correlator that validates package and ID matches. Identity in a hello remains an unauthenticated claim.
- **Implemented, software validated:** generic stream frame helpers use a 4-byte big-endian length, accept clean EOF, reject empty/truncated/oversized frames, and cap reads before allocation at 64 KiB.
- **Implemented, focused software validated:** `pipboy-core::package_transport::UnixAdapterSession` adds an absolute-deadline Unix stream handshake and serial request/response exchange; a peer's hello identity must match the expected package ID, and request IDs/package IDs are correlated. This accepts an already-open socket only; hello identity is not authentication and the transport does not authorize, dispatch, spawn, or isolate packages.
- **Implemented, software validated:** desktop and micro package selectors consume the mounted roster in `/api/state`; selection, Settings and foreground clearing update `UniversalAdapterHost` through same-origin endpoints. Both shells display host-provided views as inert text and preserve package selection across Settings/reload. The current candidate bootstrap mounts one passive RADIO catalogue, but it does not auto-select, start playback or grant hardware access.
- **Implemented, focused software validated:** `PrivateUnixListener` binds only in an existing current-user-owned private directory, refuses occupied paths, sets socket permissions to `0600`, checks the OS-reported UID and PID against the host's expected child process, and removes only the socket inode it created. Linux uses peer credentials; macOS uses `getpeereid` and `LOCAL_PEERPID`. This establishes process/account provenance at connection time, not module identity or signature; no listener is connected to a process launcher yet.
- **Still unimplemented:** timeout cancellation, domain dispatch, capability brokerage, signed/persistent installation state, process launch and isolation. The listener is not wired to a launcher, and exact-head CI remains pending.
- Local host has only stable Rust; Rust 1.90 was not run. Linux aarch64 compilation also remains unverified because crates.io DNS failed while retrieving uncached `event-listener-strategy 0.5.4`.
- `cargo fmt --all -- --check` — passed.
- `cargo check --workspace` — passed.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `cargo test --workspace --locked -- --test-threads=1` — passed on merged candidate working tree: 77 `pipboy`, 301 core, 3 SDK and 12 broker tests (393 total), doc tests passed. Tests required local socket access; the sandboxed attempt failed three loopback fixtures with `EPERM`.
- `cargo test -p pipboy-core package_listener --locked -- --test-threads=1` — four listener security/lifecycle tests passed before merge; merged full suite includes them.
- `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, and `cargo clippy --workspace --all-targets --locked -- -D warnings` — passed on merged candidate working tree.
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

## Release acceptance

- **Planned:** bind expected peer credentials to a real host launcher and verified binary identity, plus all remaining production gates.
- **Implemented but unverified:** compiled-in passive RADIO station-catalogue bootstrap and shell selection. Interactive native adapter playback, signed installation and hardware UI acceptance are open.
- **Software validated locally:** manifest registry, passive RADIO catalogue, request codec/framing, deadline-bounded Unix stream session, private Unix socket path/permissions and expected-process OS peer credential checks, and desktop/micro shell lifecycle linkage.
- **Hardware validated:** none for adapter integration.
- **Release candidate / production accepted:** neither. **Accepted roadmap gates: 0/12**; PROD-3 is a source candidate, not a hardware-accepted interactive package.

## Open defects and blockers

- GitHub Actions `startup_failure` has zero-job evidence from earlier runs, but no diagnosis; CI for this candidate remains pending.
- One earlier full parallel run returned HTTP 422 in `manager_can_create_assign_dispatch_and_track_but_other_roles_cannot_reassign`; the failure did not reproduce in isolation, serial execution, five consecutive parallel suite runs or the latest full workspace run. The test now includes task-state context if it recurs.
- One explicitly compiled, capability-free RADIO catalogue adapter is mounted by default, but not selected automatically. MAP, TV and COMMS remain inert references; signed installation and real audio-adapter integration remain open.
- Local Mac/Pi hardware, media output and independently maintained package integration have not been accepted.
- Expected UID/PID checks do not establish package identity, signature, or binary integrity.

## Codex development handoff

The next indexing gate adds `python3 tools/codex_next.py --json`, a read-only, validated handoff of RB-CX-00 with source paths, checks and outstanding dependencies. Static source registration does not advance RB-CX-00 or establish passing CI.


The [machine pass queue](../catalog/codex_passes.json) names **RB-CX-00** as the only `next` pass: inspect exact HEAD, run the passive RADIO bootstrap and host UI regressions, and obtain actual compiler/CI evidence. See [Codex instructions](../AGENTS.md), [pass plan](CODEX_PASSES.md) and [index hierarchy](INDEXING.md). This indexing update registers new offline checks but does **not** claim they ran or passed. [Actions run 40821379695](https://github.com/pskeffington/pip_boy/actions/runs/40821379695) on indexing commit `5b4a66e` concluded `startup_failure` with **zero jobs**; it provides no Python, Rust or browser test result. Subsequent exact-head CI remains to be established.

### RB-CX-00 required verification before advancement

- Record exact branch/head and `main` divergence, successful indexed Python checks, Rust stable/1.90 workspace checks, and desktop/micro/Radio browser regressions.
- Verify the passive RADIO adapter does not start audio, reach the network, open hardware or grant scopes during bootstrap or foreground switching.
- Inspect the latest workflow admission and jobs; a zero-job `startup_failure` is a CI infrastructure blocker, not test acceptance.
- Only advance to RB-CX-01 following a successful exact-head CI run and review. The current index status remains `next`; no production gate is accepted.

## Next three implementation actions

1. Compile and test the new reviewed read-only RADIO catalogue bootstrap and desktop/micro selection on real Rust 1.90/stable and Chromium; preserve legacy playback.
2. Add secure Unix socket creation/ownership and peer provenance checks before enabling independent processes.
3. Retrieve an actual failed GitHub Actions run and classify startup failure; preserve the workflow checks as written.

## Release risks

No package loader or process boundary exists. Manifest metadata and the typed codec confer no trust or capability. Hardware acceptance, UI integration, package provenance/signatures, IPC security, rollback, SBOM and deployment packaging remain open.

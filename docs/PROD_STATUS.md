# Rust Boy production checkpoint

## Codex instruction checkpoint — 2026-10-29

**RB-CX-00/01/02/04 are accepted software-only gates.** RB-CX-04 exact source head `ab98743b453a9c33f45b5694ac60d2e7cd8f39b1` passed all seven required jobs in [run 40830133847](https://github.com/pskeffington/pip_boy/actions/runs/40830133847): source/catalog, stable, Rust 1.90, Soapy-SDR, SDRconnect, native macOS and native Windows. This accepts bounded Unix socket provenance and exchange only; there is still no approved launcher or binary identity for external package execution. PR #71 remains draft. Durable consent, OS-enforced physical access, and broader production acceptance remain open.

## Workflow independence candidate

The reviewed workflow uses zero external GitHub Action `uses:` imports and exact-SHA checkout. Run [40823597748](https://github.com/pskeffington/pip_boy/actions/runs/40823597748) on `63093badc2046a59689693f68896fa502dd95234` was **cancelled**, not source-failed. Job-level evidence: source/catalog validation passed; stable completed formatting, Node verification and Linux Chromium suites through MAP/RADIO before all-target compilation was cancelled; Rust 1.90 compiled all targets before tests were cancelled; SDRconnect check/Clippy passed before tests were cancelled; Soapy setup completed before its compile step was cancelled; native Mac and Windows builds were cancelled before their smoke steps. No job reported a failure. This was an incomplete run and supplies no acceptance evidence. Run [40823687340](https://github.com/pskeffington/pip_boy/actions/runs/40823687340) on `5ad4327d34751cb723eae0ae7de7ef653e3702cf` admitted all seven jobs. Stable, Rust 1.90, both SDR feature suites, manifest validation and native Windows passed. Linux desktop/micro/Radio browser checks passed. macOS completed compile/tests, native smokes and PTY, then failed because the runner supplied Node 24.22.0; its later Mac-first MAP step was skipped. This identified a runner-version assumption, not a Rust or adapter compile failure.

Commit `91753624bd39dfd9e7efccd85d5d1e33fe327891` pins official Node.js 22.23.4 by architecture-specific SHA-256 in `scripts/ci_bootstrap.sh`, routes stable/1.90 and native Rust setup through that repository-owned bootstrap, and adds an offline regression gate for zero Action imports, required matrices and pins. The exact-head hosted run verified the pinned Node binary on macOS and passed the complete matrix. External Rust toolchains, system packages and locked Cargo dependencies remain network dependencies; this is not a hermetic/offline build.


Updated: 2026-10-29

## Current PR and default-branch CI status — 2026-10-29

- PR #71 head `dabc18719a6d86dae7076a63732971679fefd2c9` passed all seven checks in [PR run 40859193586](https://github.com/pskeffington/pip_boy/actions/runs/40859193586). The README badge is scoped to this development branch.
- The default `main` branch remains at `629f34f0ce21e37c6d2bc6567919ccc5c862c80d`; its latest listed run [40669977589](https://github.com/pskeffington/pip_boy/actions/runs/40669977589) concluded `startup_failure` with zero jobs. The failing badge visible on the default-branch README reflects this separate main run; it is not the status of PR #71.
- The main workflow still contains third-party Action imports. The PR branch replaces these with repository-owned exact-SHA checkout and official toolchain bootstrap, preserving the stable/1.90, SDR and native platform gates. Main's badge will reflect the repaired workflow after the change is merged.
- PR #71 remains draft because RB-CX-05's independent `rb_gis` renderer parity and operator/field acceptance remain open. Do not treat the green PR CI matrix as acceptance of those gates.

## RB-CX-05 host package interoperability — local candidate

- Candidate source checkpoint: `0d648f20a96796a30beef443bfe0439439e57ff7` (same source tree on which the local commands below passed). Hosted exact-head CI has not run yet.
- The subsequent PR checkpoint `8aec54af05d02f148486867234ae79482a1fabf0` includes this source and passed all seven exact-head jobs in [run 40856786397](https://github.com/pskeffington/pip_boy/actions/runs/40856786397): source/catalog, stable, Rust 1.90, both SDR features, native macOS and native Windows. RB-CX-05 remains unaccepted for the independent parity and field limitations below.
- Documentation checkpoint `5a31883b834de4652dcbdd7d45617276cb24223e` also passed all seven exact-head jobs in [run 40857599786](https://github.com/pskeffington/pip_boy/actions/runs/40857599786), including native Windows ConPTY. This confirms the badge tracked from the PR branch is green at that checkpoint; the default-branch `main` badge remains a separate red status.
- Current source/evidence checkpoint `daf4acf99a63b5e9259f30823f299c6fcd75f34c` passed all seven jobs in [run 40858385833](https://github.com/pskeffington/pip_boy/actions/runs/40858385833). Default `main` remains separate and unchanged at the startup-failure run; PR #71 remains draft until RB-CX-05 acceptance.
- Added explicit local directory import for the documented `rb-gis-package/v1` flat GeoJSON format in the Rust-Boy MAP host. Parsing enforces one directory, no path traversal, up to 12 layers, 64 KiB manifest, 2 MiB per layer, 2,000 features/30,000 vertices per layer and existing aggregate map limits. Layer styling comes from a closed role map; source, vintage and attribution remain declared strings.
- A visible warning says package provenance is unauthenticated. Import is user-triggered, stays local, performs no automatic device or network access, and commits geometry only after the full package validates. Malformed replacements preserve last-good geometry, source metadata and camera.
- The new parser negatives and map-view last-good regression passed. All six indexed RB-CX-05 checks passed on the development worktree; Chromium passed 480×320, 800×480 and 320×568. Offline validators and `git diff --check` passed. The first sandboxed loopback test attempt returned `EPERM`; the unchanged localhost suites passed with explicit test-loopback access.
- `bash scripts/test-map-macos.command --verify --require-chromium` also passed on the candidate worktree: native Mac build, nine county-cache Rust tests, real loopback micro smoke, offline map logic, six cache replay tests and three Chromium viewports. No production devices, external providers, or independent `rb_gis` renderer were used.
- Exact commit SHA and hosted CI evidence are pending this local candidate's commit. RB-CX-05 remains unaccepted: no independent `rb_gis` renderer parity, Census SHP/DBF/ZIP/service migration, Mac operator acceptance or field report is claimed. The default `main` badge remains red until the CI repair is eligible to merge; PR #71 stays draft.

## RB-CX-05 Rust GIS geometry inspection — local candidate

- Expanded the Rust-owned Scout GIS core and `pipboy scout geo inspect` from Point-only inspection to Point/MultiPoint/LineString/MultiLineString/Polygon/MultiPolygon and nested GeometryCollection. Its stable summary reports geometry type, Feature count and raw WGS84 bounds; the validator also enforces 1 MiB files, 1,000 features, GeometryCollection nesting to six and 30,000 positions, and rejects legacy CRS declarations, malformed coordinates and unsupported geometry.
- Stable and Rust 1.90 full workspace `check`, `test`, and warnings-denied `clippy` passed locally. The unchanged stable full test command initially showed two dashboard test failures; both passed in isolation, and the unchanged full suite passed on retry. Rust 1.90's full suite passed. The Scout CLI smoke and all six indexed RB-CX-05 MAP checks passed.
- This is local read-only validation/summarization; it does not render, import native GIS data into MAP, or close independent renderer parity. Exact source SHA and hosted matrix for this candidate are pending commit. The `rb_gis` repository remains unmodified; PR #71 remains draft.

## Gate state

- **Latest completed exact-head validation:** PR checkpoint `68722d765c1df9bfd0f7c4ac9b8cb4c84b717023`; push-triggered [run 40840007336](https://github.com/pskeffington/pip_boy/actions/runs/40840007336) passed all seven jobs, including stable/Rust 1.90, both SDR features, source/catalog policy, desktop/handheld Chromium, native macOS MAP and Windows ConPTY. CI policy continues to require zero external GitHub Action `uses:` dependencies.
- **Acceptance/index checkpoint:** `7e0a19b440e9a8531390e00fb165b2b8df06f3d9`; [run 40831001927](https://github.com/pskeffington/pip_boy/actions/runs/40831001927) passed all seven jobs, including stable MAP browser regressions, Rust 1.90, SDR features, native Mac MAP and Windows ConPTY.
- **Current gate:** RB-CX-05 is next, limited to host-owned GIS integration/parity work. `rb_gis` remains an independent repository and will not be modified from this checkout. RB-CX-04 is accepted software-only; its remaining blocker is external package launch because no approved launcher or binary identity is bound to a package.
- **Last accepted Codex pass:** RB-CX-04. **Last completed PROD roadmap gate:** PROD-0 remains **not accepted**; these software pass acceptances do not certify the broader production baseline or release.
- **Branch:** draft [PR #71](https://github.com/pskeffington/pip_boy/pull/71), `codex/package-adapter-protocol`. First-party CI repair is included. `rust_radio/` remains an independent untracked nested repository and is untouched.
- **Relevant pinned module revision:** `rust_radio` dependency revision `3d77d11da3a31fbc3144635da6d391a3febf38b7` from the workspace lock/config. The local `rust_radio/` directory is a separate untracked nested repository and was preserved untouched.

## Clean `main` user build — 2026-10-29

- Fetched remote `main` at `629f34f0ce21e37c6d2bc6567919ccc5c862c80d`, exported that exact tree to a clean temporary source directory, and successfully ran `cargo +stable build --workspace --locked` on macOS ARM64.
- `cargo +stable run --locked -p pipboy -- --help` succeeded, then `python3 tools/smoke_micro.py` passed against the clean main binary. The smoke exercises the real local micro UI/API, CSP and offline startup with temporary test state; it makes no WAN or device requests.
- The README now lists Rust 1.90+, the Debian/Ubuntu C toolchain, `pkg-config` and ALSA header prerequisites, and locked workspace build/CLI commands. This build result is source-compilation evidence; it does not replace native Windows/Pi operator, hardware or field acceptance.

## RB-CX-02 accepted — backend approvals and media ownership

- Added `AdapterAuthorization`, an in-memory approval authority keyed by active Rust role, package ID and manifest scope. Every scoped intent checks the host-supplied manifest declaration and the current authority before adapter invocation; approval revocation and role mismatch therefore stop at the backend boundary.
- Audio requests use a separate owner check: only the current owner may play/resume/pause/stop, a second package cannot take ownership, and a successful stop releases the owner. Role changes are rejected while an audio owner remains active.
- These approvals are neither loaded from `permissions.json` nor inferred from package manifests, UI state or role tool pins. They are not OS-enforced permissions. `rust-boy-broker` remains a separate deny-all process with no device/network handles.
- Local stable and Rust 1.90 workspace tests passed (398 each) after granting the existing loopback-only fixtures access. Both SDR feature test/check/Clippy matrices passed under both toolchains. Targeted permission and network/device/message denial tests passed; see [validation](VALIDATION.md) for the exact commands and local Chromium limitation.
- Acceptance is software-only. No consent UI integration, durable approval workflow, OS authorization, real media backend or external package execution is accepted. The deny-only broker remains unchanged and physical operations remain unavailable.
- Exact-head evidence is validation head `c1df259d1e01689dcb93ed48073b250150027f56`, [run 40828393462](https://github.com/pskeffington/pip_boy/actions/runs/40828393462), all seven checks successful. Stable/1.90 and both SDR feature matrices passed; Chromium desktop and handheld UI regressions passed in CI.

## RB-CX-04 accepted — Unix adapter socket provenance and exchange (software-only)

- `PrivateUnixListener::accept_expected_process(expected_pid, timeout)` now performs bounded nonblocking admission, checks OS-reported UID/PID, and drops unrelated peers while continuing within one absolute deadline. Admission yields an opaque `VerifiedUnixStream`; `UnixAdapterSession` exposes only `from_verified_peer`, while raw-stream construction is private.
- The listener remains disconnected from any process launcher. A caller must still supply the expected PID and verify the approved binary; OS peer credentials and the adapter hello claim do not authenticate package identity or authorize capabilities. Unsupported platforms remain deny-only, and `rust-boy-broker` is unchanged.
- Focused regressions cover private socket ownership/mode, peer PID mismatch, bounded timeout, slow-trickled handshake, oversized frame rejection, request replay/order rejection, and a complete verified-peer session exchange. Exact-head CI run [40830133847](https://github.com/pskeffington/pip_boy/actions/runs/40830133847) passed all seven required checks on `ab98743b453a9c33f45b5694ac60d2e7cd8f39b1`.
- This gate does not approve a package launcher or binary identity, and it does not enable external package execution. The broker remains deny-only; physical permissions, process isolation and signed distribution remain open.

## RB-CX-05 host-owned MAP progress — implementation candidate

- Native RustGIS imports now reject explicit projected/unknown dataset CRSs instead of silently treating coordinate values as WGS84 degrees. Negative tests cover EPSG:3857 and malformed CRS metadata; EPSG:4326 remains accepted.
- Exact source head `0b760598f26814a8cbeb87a81a003cb90826f6d7` passed all seven required jobs in [run 40831595041](https://github.com/pskeffington/pip_boy/actions/runs/40831595041). RB-CX-05 remains an implementation candidate because independent renderer parity and field acceptance are still open.
- Added an integration regression in `tools/test_map_layers.cjs`: a corrupt RustGIS import must retain the existing last-good county geometry, show the import error and clear the file input for retry. The test exercises the live host MAP event path; it does not claim independent renderer parity.
- `node tools/test_rustgis_map_import.cjs`, `node tools/test_map_layers.cjs`, `node tools/test_map_zip.cjs` and `node tools/test_map_viewport.cjs` passed locally. Viewports passed at 480×320, 800×480 and 320×568 with no network. The local viewport server required loopback permission after the sandbox returned `EPERM`.
- `rb_gis/` remains independent and untouched. Full GIS rendering parity, Mac operator acceptance, cached regional data recovery and physical Pi acceptance remain open.
- On this PR head, `bash scripts/test-map-macos.command --verify --require-chromium` passed all five stages: native Rust build, nine county-cache tests, real loopback micro API smoke, offline MAP/GPS/county/ZIP/RustGIS logic, six synthetic native cache replay tests, and Chromium viewports at 480×320, 800×480 and 320×568. The initial sandboxed run could not bind loopback (`EPERM`); the unchanged command passed with local loopback access authorized.

### Location-driven county replacement rollback — 2026-10-29 (local candidate)

- `map.js` now stages each validated county source without publishing it. It atomically replaces the displayed package and boundary only after at least one source has loaded; failure retains the last-good package, shapes, county boundary and prior successful lookup point.
- `tools/test_map_layers.cjs` exercises the manual-location → Census county → host boundary/package path with a failure after the first incoming layer. It verifies that the last-good objects remain identical and the failed location does not become the retry-suppression point.
- Local `node tools/test_map_layers.cjs`, `node tools/test_map_zip.cjs`, and `node tools/test_rustgis_map_import.cjs` passed. Source commit `a148bfdd112917bc8de9ae66784a8e2c1c31ba87` is pushed to PR #71; exact-head run [40833358785](https://github.com/pskeffington/pip_boy/actions/runs/40833358785) passed all seven jobs after failed-job reruns. No pass acceptance is inferred.
- The read-only pass index remains RB-CX-00/01/02/04 accepted and RB-CX-05 next. `rb_gis/` remains unmodified. Independent renderer parity, operator and field evidence remain open.

### Cache selection and viewport rollback — 2026-10-29 (local candidate)

- Cached boundaries and downloaded package archives are artifacts; retrieving either no longer changes the cold-start `active_geoid`. A same-origin, explicit-confirmation `/api/map/activate` call persists the selection only after the client has parsed at least one valid non-landmark base layer. The host rejects missing confirmation, cross-origin requests, invalid GEOIDs and counties without a cached base archive.
- The county replacement transaction now also stages the requested location. Failed boundary/package validation leaves the prior viewport center, zoom, follow state, county boundary, geometry, lookup retry point and persisted county selection intact. A successful retry commits them together after base-layer validation.
- Focused cache/API, county-source, map-layer, ZIP and RustGIS import regressions pass. Stable and Rust 1.90 locked workspace checks/tests and warnings-denied Clippy pass (403 tests/toolchain); Soapy-SDR and SDRconnect each pass locked check, Clippy and tests under both toolchains. Desktop Chromium passed 1440×900, 1280×800, 1024×768 and 800×600; MAP Chromium passed 480×320, 800×480 and 320×568. Desktop focus, micro UI, MAP boot/GPS permission and offline validators pass.
- Initial exact-head CI run [40835715874](https://github.com/pskeffington/pip_boy/actions/runs/40835715874) on `6842f43aabbd22f873b24bf9744956901a6563a6` found two old MAP test fetch mocks that did not implement the new activation endpoint. I updated both fixtures and added activation assertions; the local full macOS MAP acceptance script then passed. The fixture-fix head `520b3bf2801d8a0a261ff16e96c40c3a2a02e5fb` passed all seven exact-head checks in [run 40836201769](https://github.com/pskeffington/pip_boy/actions/runs/40836201769), including stable, Rust 1.90, both SDR features, offline/source policy, desktop/handheld Chromium, native macOS MAP and Windows ConPTY. The PR-triggered run [40836201498](https://github.com/pskeffington/pip_boy/actions/runs/40836201498) also passed all seven checks on the same SHA. RB-CX-05 remains `next`; independent `rb_gis` renderer parity and Mac operator/field acceptance are open. No changes were made inside the independent `rust_radio/` repository.

## Implementation and evidence

- **Implemented, software validated:** `pipboy-core::package_socket` provides bounded JSON request, hello-claim and response codecs; highest-common-version negotiation; strictly increasing request IDs; and a 64-entry request correlator that validates package and ID matches. Identity in a hello remains an unauthenticated claim.
- **Implemented, software validated:** generic stream frame helpers use a 4-byte big-endian length, accept clean EOF, reject empty/truncated/oversized frames, and cap reads before allocation at 64 KiB.
- **Implemented, focused software validated:** `pipboy-core::package_transport::UnixAdapterSession` adds an absolute-deadline Unix handshake and serial request/response exchange. Its public constructor accepts only the opaque token produced by listener UID/PID checks; hello identity remains an unauthenticated claim. It does not authorize, dispatch, spawn, or isolate packages.
- **Implemented, software validated:** desktop and micro package selectors consume the mounted roster in `/api/state`; selection, Settings and foreground clearing update `UniversalAdapterHost` through same-origin endpoints. The accepted host mounts one passive RADIO catalogue and never auto-selects or starts playback.
- **Accepted RB-CX-01:** the SDK defines a bounded version-1 display-only `PackageView` (heading/text/status/list), the core validates it before exposing a foreground, and desktop/micro render host-created DOM using text content. Accessible native selectors meet 44px touch sizing. Exact-head run 40826559481 passed stable/1.90, both SDR features, Linux Chromium UI/viewport suites, native macOS and native Windows. Interactive package actions remain deferred to RB-CX-02.
- **Implemented, focused software validated:** `PrivateUnixListener` binds only in an existing current-user-owned private directory, refuses occupied paths, sets socket permissions to `0600`, checks the OS-reported UID and PID against the host's expected process within a bounded wait, and removes only the socket inode it created. Linux uses peer credentials; macOS uses `getpeereid` and `LOCAL_PEERPID`. This establishes process/account provenance at connection time, not module identity or signature; no listener is connected to a process launcher yet.
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
- Pass-index status is `RB-CX-00/01/02/04: accepted`, `RB-CX-05: next`. No production roadmap gate is accepted; release acceptance remains **0/12**.

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

`python3 tools/codex_next.py --json` is a read-only, validated handoff of RB-CX-05. The `validate_ci_policy.py` / `test_validate_ci_policy.py` guard the zero-`uses:` rule, exact-SHA checkout, stable/1.90, SDR/native matrices and pinned Node bootstrap. RB-CX-00 acceptance is supported by run 40824698037; the RB-CX-00 index checkpoint passed all seven checks on run 40825620885; RB-CX-01 acceptance is supported by run 40826559481; RB-CX-02 passed all seven exact-head checks in run 40828393462; RB-CX-04 passed all seven exact-head checks in run 40830133847.


The [machine pass queue](../catalog/codex_passes.json) records RB-CX-00/01/02/04 as accepted with exact source SHAs and successful workflow URLs; RB-CX-05 is the only `next` pass. Head `ab98743b453a9c33f45b5694ac60d2e7cd8f39b1` passed all seven CI jobs in run 40830133847. Historical startup failures, cancellation, the repaired macOS Node mismatch, and stale handoff-test fixtures are documented below; see [Codex instructions](../AGENTS.md), [pass plan](CODEX_PASSES.md) and [index hierarchy](INDEXING.md).

### RB-CX-05 current acceptance requirements

- Match legacy roads, counties, ZIPs, cache and location-follow behavior offline.
- Keep location ingestion in host Settings and the map near-full-canvas; verify rollback before legacy map removal.
- Run the indexed MAP regressions and stable/1.90 exact-head CI. Keep `rb_gis` independent and PR #71 draft.

## Next three implementation actions

1. Finish source/index validation for the socket acceptance checkpoint and host MAP rollback regression, then commit and push fast-forward to PR #71.
2. Inspect exact-head CI for the documentation/test checkpoint; repair any failure without weakening the matrix.
3. Continue RB-CX-05 host-owned parity work and Mac MAP acceptance while preserving `rb_gis/` as an independent repository.

## Next three implementation actions

1. Commit and push the RB-CX-04 socket-provenance candidate after the final offline/Rust/UI checks and inspect its exact-head seven-job CI run.
2. Record CI evidence and accept RB-CX-04 only if listener, verified-session, stall/replay/frame and deny-only requirements all pass at the exact head.
3. Advance to the next indexed P0/P1 host gate without launching or modifying `rb_gis`, `rust_radio`, `rust_tv` or `rusti_rc`.

## Release risks

No external package loader or OS process isolation exists. Manifest metadata and the typed codec confer no trust or capability. Physical hardware acceptance, interactive package intents, package provenance/signatures, IPC security, rollback, SBOM and deployment packaging remain open.

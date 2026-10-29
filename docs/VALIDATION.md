# Rust-Boy | Validation evidence

## First-party CI and Node bootstrap repair — 2026-10-29 (accepted)

Removed `dtolnay/rust-toolchain@stable`, `actions/checkout@v4` and `actions/setup-node@v4` from the draft workflow and replaced them with first-party checkout/bootstrap commands. The repository bootstrap pins official Node.js v22.23.4 and verifies architecture-specific SHA-256 values published in the [official Node.js release record](https://github.com/nodejs/nodejs.org/blob/main/apps/site/pages/en/blog/release/v22.23.4.md). Exact-head run [40824698037](https://github.com/pskeffington/pip_boy/actions/runs/40824698037) verified the pinned Mac Node, Mac-first MAP and all other CI jobs on `c425994eb8a172c357c6920e29d2b4d38cdc5050`. This establishes first-party GitHub Action orchestration, not a fully offline dependency supply chain.


## Admitted CI and local validation — 2026-10-29

Run [40823687340](https://github.com/pskeffington/pip_boy/actions/runs/40823687340) tested exact head `5ad4327d34751cb723eae0ae7de7ef653e3702cf`. Stable and Rust 1.90 each passed format/check/test (77 `pipboy`, 301 core, 3 SDK and 12 broker tests; 393 total); stable Clippy passed. Both optional SDR jobs passed locked workspace check, warnings-denied Clippy and all 393 tests without hardware discovery/capture. Catalog/source/security gates, all Linux JavaScript/Chromium desktop, micro, MAP and RADIO viewport suites, and the native Windows build/tests/smokes/ConPTY passed. macOS build/tests (393), offline smokes and PTY passed; the job then failed because the runner provided Node `v24.22.0`, and the required Node 22 check failed before the Mac-first MAP step.

Run [40823597748](https://github.com/pskeffington/pip_boy/actions/runs/40823597748) at `63093badc2046a59689693f68896fa502dd95234` was cancelled after a newer PR run started: manifest validation passed, and in-flight Rust/SDR/native work was cancelled with no failed step. This is not source failure evidence.

On the current local repair working tree, stable and Rust 1.90 each passed `cargo fmt --all -- --check`, locked all-target workspace check, `cargo test --workspace --locked` (393 tests), and Clippy with `-D warnings`. The first sandboxed stable test attempt had three loopback `EPERM`s; the unchanged suite passed with local loopback permission. Both `pipboy/soapy-sdr` and `pipboy/sdrconnect` passed stable locked check, Clippy and workspace test suites. Offline pass-index, CI-policy, component-index, host-package, repository, catalog, roles, Vault, security and smoke-roster validators/tests passed. `bash -n scripts/ci_bootstrap.sh` and `git diff --check` passed. The local Node v22.23.4 bootstrap is not network-validated: this environment cannot resolve `nodejs.org`.

Source commit `91753624bd39dfd9e7efccd85d5d1e33fe327891` pins Node v22.23.4 and its published architecture-specific digests, routes Rust bootstrap through the repository script, and adds a regression guard against external Action imports or dropped required matrices. Acceptance evidence is recorded below and in the pass index.

## Accepted universal-host validation gate — 2026-10-29

RB-CX-00 is accepted on exact source head `c425994eb8a172c357c6920e29d2b4d38cdc5050`; [run 40824698037](https://github.com/pskeffington/pip_boy/actions/runs/40824698037) completed successfully with all seven required jobs: catalog/source, Rust stable, Rust 1.90, Soapy SDR, SDRconnect, native macOS and native Windows. The exact-head diagnostics reported the same seven successful checks.

The successful run confirmed macOS used official Node.js v22.23.4 and passed Mac-first MAP offline/viewport smoke, Linux Chromium desktop/micro/MAP/RADIO suites, and Windows offline smoke/ConPTY behavior. Stable and Rust 1.90 ran format, locked all-target check, 393 tests and warnings-denied Clippy; both SDR feature matrices ran locked check, Clippy and all 393 tests. Offline validators and regression suites are listed in the production checkpoint. RB-CX-00 has `hardware_gate: software_only`; this evidence does not certify the broader release roadmap or physical-hardware acceptance. RB-CX-01 is next.

The acceptance-index/documentation commit `b4719dcfa896d9d08c38e68d5c11567e8e7f9561` passed all seven exact-head checks in [run 40825620885](https://github.com/pskeffington/pip_boy/actions/runs/40825620885). Its predecessor `d949ea8` exposed stale `tools/test_codex_next.py` expectations (no accepted pass and RB-CX-01 still waiting for RB-CX-00); the fixture was updated to derive accepted passes from the index and construct a valid pending-prerequisite case. The failed catalog job was not waived; the complete follow-up run passed.

## RB-CX-01 bounded package view candidate — local validation, hosted run pending

The candidate adds the SDK's strict, bounded v1 `PackageView` with heading, text, status and list blocks; core validation rejects unknown or oversized views and faults the adapter before foreground exposure. Desktop and micro map valid blocks to host-created elements with text content only. Native package selectors are accessible by label and now meet a 44px minimum touch target. Views have no package callbacks, links, HTML, scripts or privileged input actions.

Local stable and Rust 1.90 each passed format, locked all-target check, the complete workspace suite (395 tests: 77 `pipboy`, 302 core, 4 SDK, 12 broker) and warnings-denied Clippy. Each of the Soapy-SDR and SDRconnect feature matrices passed locked workspace check, Clippy and all 395 tests under both stable and Rust 1.90, without device discovery or capture. The first sandboxed full-suite attempt failed three pre-existing loopback tests with `EPERM`; unchanged stable and 1.90 suites passed with loopback permission.

The new schema/contract validator suite passed: catalog, host-package (10 tests), component index, repository, Codex pass index, CI policy and their corresponding regressions. Desktop focus and micro UI tests passed. Real Chromium desktop viewports passed 1440×900, 1280×800, 1024×768 and 800×600; handheld viewports passed 480×320, 800×480 and 320×568 plus Custom, with no document scroll, supported view rendering, safe HTML-looking text and 44px selectors. Desktop map coverage and the existing micro map/RADIO journeys remained green. Local UI tests ran with Node v26.10.0; hosted pinned Node v22.23.4 behavior remains covered by the accepted CI pipeline.

This is **not yet accepted RB-CX-01 evidence**: no hosted exact-head run has exercised these changes. The machine pass index keeps RB-CX-01 as `next` until the focused work is pushed and passes the full exact-head matrix.

## Earlier exact-head rerun — 2026-10-29

At PR head `6f1e8d276595e62930b506d5f878e0b952832777`, stable and Rust 1.90 both passed format, locked workspace check, locked workspace tests (77 `pipboy`, 301 core, 3 SDK, 12 broker; 393 total) and Clippy. The test runs used local socket access. The Actions run for this head, [40822764662](https://github.com/pskeffington/pip_boy/actions/runs/40822764662), failed at startup and exposed zero jobs/checks/statuses. GitHub Status showed Actions operational with no Oct 29 incident, which does not explain this run. RB-CX-00 remains pending CI execution and physical acceptance.

## Rust 1.90 and exact-head CI follow-up — 2026-10-29

At source revision `8a5c9d162af5ede9449dcf924f88fba15e9bdb38`, both stable and Rust 1.90 passed `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo test --workspace --locked -- --test-threads=1` (393 tests), and `cargo clippy --workspace --all-targets --locked -- -D warnings`. An isolated Rust 1.90 aarch64 Linux check of `package_listener.rs` also passed with `rustix` net/process features. The full workspace cross-build is not verified: vendored D-Bus requires `aarch64-linux-gnu-gcc`, which is absent on this Mac.

The Actions workflow run for this exact source revision, [40822394927](https://github.com/pskeffington/pip_boy/actions/runs/40822394927), concluded `startup_failure`; PR diagnostics returned zero jobs, checks and statuses. This supplies an affected run but no job logs or root-cause diagnosis. RB-CX-00 remains pending; local compiler results do not substitute for CI or field acceptance.

## RB-CX-00 local merged-branch validation — 2026-10-29

At tested source revision `8edff4b948b24187786dcde61db74e212b26485d`, after integrating the passive RADIO catalogue and Codex pass-index commits into `codex/package-adapter-protocol`, local stable-toolchain validation passed for the merged source plus `cargo fmt` cleanup in `builtin_packages.rs` and `lib.rs`. The Rust suite passed 77 `pipboy`, 301 `pipboy-core`, 3 SDK and 12 broker tests (393 total), plus doc tests. `cargo check --workspace --all-targets --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo fmt --all -- --check` passed. Workspace tests needed local socket permissions; the first sandboxed run failed three existing loopback fixtures with `EPERM`.

Pass-index, host-package, component-index, repository and security validators and their Python test suites passed. RADIO UI and compact viewport scenarios passed; desktop 1440×900 through 800×600 and micro 480×320, 800×480 and 320×568 scenarios passed. These browser fixtures verified Settings restoration and no autoplay or unintended I/O. PR diagnostics for the tested revision returned zero checks/statuses/jobs. Rust 1.90 is not installed locally. Exact-head GitHub Actions and physical Mac/Pi acceptance remain pending, so RB-CX-00 is not marked accepted. See [CI triage](CI_TRIAGE.md) and [production checkpoint](PROD_STATUS.md).

## Codex handoff projection — 2026-10-29 (source candidate)

Added the read-only `tools/codex_next.py` CLI with JSON/plain-text projection of the single validated `next` pass and its prerequisites, declared files and checks. The deterministic negative fixtures in `tools/test_codex_next.py` and CI catalog registration are source changes; executing the new checks on the current head remains pending. This CLI neither runs Git nor claims a passing CI or device acceptance.


## Codex pass indexing — 2026-10-29 (committed source; exact-head execution pending)

Added root `AGENTS.md`, `catalog/codex_passes.json`, `docs/CODEX_PASSES.md` and `docs/INDEXING.md`, with standard-library-only validation and negative tests. The validator checks stable pass IDs, declared paths, known component IDs, backward dependencies, exactly one eligible `next` pass, and structurally complete commit/CI evidence for any `accepted` state. It does **not** independently confirm that a referenced CI run passed. The CI catalog job now lists the new offline commands. [Actions run 40821379695](https://github.com/pskeffington/pip_boy/actions/runs/40821379695) for source commit `5b4a66e` returned `startup_failure` with zero jobs: **none of the new Python checks or Rust/Chromium tests ran in that workflow**. Successful exact-head execution remains unverified. Earlier results below apply only to their documented earlier revisions.


## Universal adapter SDK and host manager — 2026-10-28 (source committed; CI startup failure)

The new `crates/rust-boy-adapter-sdk` crate defines `AdapterEnvelope`, `AdapterCommand`, `AdapterReply`, `UniversalPackageAdapter` and a deny-all `ScopeAuthority` interface. `pipboy-core::universal_adapter` manages trusted in-process registrations, bounded requests/replies, one foreground package, separate Settings, scoped intents and explicit unmount/replacement. Unit test sources exercise four synthetic package IDs, lifecycle, protocol bounds, rollback, Settings persistence and oversized replies. The portable hello adapter example contains no network or device requests.

**Static checks performed:** GitHub repository tree and component ownership index matched the new SDK/host/docs paths (17 indexed components, no duplicate code ownership); `Cargo.toml` workspace membership, path dependency and `Cargo.lock` SDK entry were inspected for consistency; the SDK source has no direct `std::net`, `std::process` or `std::fs` imports. These inspections do not establish Rust compilation or runtime correctness.

**Not yet executed:** `cargo check --workspace --all-targets --locked`, `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, Mac GUI and Raspberry Pi physical tests. [Actions run 40669004392](https://github.com/pskeffington/pip_boy/actions/runs/40669004392) reported `startup_failure` before any job, not a demonstrated source-level test failure. Neither rb_gis nor TV, RADIO or COMMS is mounted through the new manager.


## Host package socket registry — 2026-10-28 (CI not executed)

The read-only `pipboy_core::package_socket` source implements a schema-1 manifest parser, reference registry, bounded explicit local directory scan, and rejection of malformed manifests, incompatible host API versions, duplicate IDs, unknown capability requests, unsafe executable manifest keys and symlinks. The `packages list|check|scan` CLI is metadata-only; `GET /api/packages` serves four `installed:false`, `activatable:false` design references. Added Rust unit regressions (including local directory scan and pure single-foreground/Settings selection), offline standard-library `tools/validate_host_packages.py` and `tools/test_validate_host_packages.py`, and a CI catalog step.

**Verification boundary:** An [Actions run after the socket-registry CI change](https://github.com/pskeffington/pip_boy/actions/runs/40661432096) returned `startup_failure` before jobs ran; it does **not** establish a code regression or successful tests. Independent inspection of the four checked-in JSON manifests found the expected IDs, versions, declared capabilities and source paths. Rust compilation, formatter, Clippy, Python CI execution, native UI acceptance and hardware interoperability are **not yet verified**. This gate cannot be marked accepted until a working runner executes the complete checks.

## Unix adapter transport precursor — 2026-10-29 (local validation)

Added `pipboy_core::package_transport::UnixAdapterSession` for an already-accepted Unix stream. The session checks the adapter's hello claim against an expected package ID, negotiates the version, and conducts serial bounded/correlated exchanges. One monotonic deadline covers the complete hello read or request/response exchange; nonblocking I/O prevents slow partial frames from resetting that deadline. Six tests cover successful exchange, package/version/sequence rejection, identity mismatch, missing or slowly trickled hello, and truncated response.

`cargo fmt --all -- --check`, workspace check and warning-denied Clippy passed. The focused transport tests passed (6), as did the full workspace suite (74 `pipboy`, 281 `pipboy-core`, 12 broker; 367 total) after granting loopback permission to existing endpoint/UDP fixtures. This only validates a connected stream session: no socket listener or path ownership, peer authentication, process launch, capability broker, dispatch or isolation exists. GitHub Actions startup failure and physical-device validation remain outstanding.


## Desktop copy and focus optimization — 2026-10-26 (CI pending)

The source-only HTML/CSS/JS pass reduces default-visible text while preserving the current role-independent **TOOLS / VIEW / MESSAGES / SETTINGS** one-screen shell. Only SETTINGS can stage a profile change; Rust-defined tool data, authorization, local storage, MAP ZIP lookup and native network/message code are unchanged. Full tool descriptions remain in hover titles and a closed **About this tool** inspector disclosure; safety information is behind **Safety & access**. Messenger mounts only when the operator selects MESSAGES, never on initial desktop load.

Added `tools/test_desktop_focus_ui.cjs` production DOM/consent tests and `tools/test_desktop_viewport.cjs` real Chromium offline fixture at 1440×900, 1280×800, 1024×768 and 800×600, checking no document overflow, visible >=44px targets, Settings-only station selection and zero unexpected initial API requests. The existing dashboard viewport test retains the 480×320, 800×480, Scout, Security and Custom 44px short-screen gates. Exact-head five-job CI and operator-Mac browser tests are pending; no physical hardware acceptance is claimed.


## NET ENGINEER HTTP extraction — 2026-10-26 (five-job CI passed; merged)

The source-only `dashboard/network_routes.rs` extraction preserves `POST /api/net/scan` and `POST /api/net/unifi` behind the original exact Host/Origin, content-type, bounded JSON and explicit `net-lab`/confirmation checks. New dashboard tests reject hostile Origin, missing consent, inappropriate roles, invalid CIDR length, invalid UniFi site IDs, public compare scope and unsupported operations **before network probes or controller access**. Prior network health and UniFi smoke checks remain in the existing five-job workflow. This extraction passed five-job [CI run 40295912014](https://github.com/pskeffington/pip_boy/actions/runs/40295912014) and merged into `main` as `870c975b`; physical UniFi/Pi interoperability remains untested.


## Native Mac real-cache integrity audit — 2026-10-26 (CI pending)

The new `tools/audit_map_cache.py` is invoked by `scripts/test-map-macos.command --audit-cache`, operating on an operator-selected existing local profile only. It starts the actual Rust loopback service and sends fixed cache-only, non-confirming HTTP requests to boundary/source endpoints; there is no outward provider call. An independent Python ZIP reader verifies archive structure, paths, decompression budget and CRC and reports failures/partial layers, while native Rust validates the county boundary GEOID and rings. `tools/test_audit_map_cache.py` uses a test-only fictional county and actual loopback server to verify missing, complete, partial, bad boundary and damaged ZIP responses without touching the operator's cache. No live Census retrieval or personal Mac test has yet been performed by this code.

## Native Mac MAP operator acceptance — pending local run (2026-10-26)

`scripts/test-map-macos.command` provides a native macOS Rust build, isolated loopback HTTP smoke, offline GIS parser/permission tests, optional real Chrome emulation and a persistent test-home launch. [Mac MAP acceptance matrix](MAC_MAP_TESTING.md) defines cold startup, live Census GEOID resolution, layer toggles, full offline restart, GPS permission behavior and draw/source timing measurements. The GitHub macOS runner may execute the automated portion after CI passes; **no user-Mac field results are recorded yet**. Do not equate CI with a passed GPS, Safari, or physical touch session.

## Messenger/PACE HTTP route separation — 2026-10-26 (five-job CI passed; merged)

The source-only Messenger handler extraction on `chore/dashboard-messenger-boundary` moves `GET /api/messages`, `POST /api/messages/send` and `POST /api/messages/retry` into `crates/pipboy/src/dashboard/messenger_routes.rs`. The sole parent Host/Origin validation and JSON parser remain unchanged. New offline Rust dashboard tests check denial of cross-origin mutations and invalid GET Host **before touching private stores**, explicit SEND/RETRY confirmation, invalid PACE tiers/text, unknown outgoing message IDs and unsupported content type. Existing desktop/micro, Messenger/PACE, SSH and workspace tests remain part of the five-job CI suite. Five-job CI [run 40295084172](https://github.com/pskeffington/pip_boy/actions/runs/40295084172) passed on combined head `da650c89b7e2302eeb9115af1bbe22c22a576eba`; PR #45 merged as `c96d3943f2cbc51cd37498fc76f9879d47fe91f5`. No live SSH, independent radio bearer or Raspberry Pi hardware test is claimed.


## Task/peer HTTP route extraction — 2026-10-26 (five-job CI passed)

`crates/pipboy/src/dashboard/task_routes.rs` now contains the Task Control and explicitly confirmed peer handoff route implementations, keeping the same public endpoints and parent same-origin gate. A new Rust regression checks that every POST route in this family rejects hostile origins **before creating local stores**. Existing manager/role scoping, optimistic revision, malformed-store preservation, SSH peer smoke tests and desktop/micro launch checks remain registered in the five-job CI matrix. This structural extraction passed GitHub Actions five-job CI [run 40291462858](https://github.com/pskeffington/pip_boy/actions/runs/40291462858), exact PR #43 head `f6cd63435a0bb14eaa4743d187ba0bcd89f3f28a`, and merged into `main` as `60c066ac3e34fe9aff8fa807e29c8b7e545ced87`. These tests do **not** certify physical SSH interoperability.


## PACE routing local validation — 2026-10-25

Exact Rust 1.90.0 passes 239 workspace tests (52 CLI/dashboard, 175 core,
12 deny-only broker), strict Clippy and formatting. The Messenger browser test
covers manual P/A selection, disabled C, local E holds/retries and actual queue
storage labels. The expanded two-store relay records SSH invocation counts:
saving/showing a PACE route and holding a message invoke no SSH; C is refused;
one A retry delivers the original held message ID and obtains a stored receipt.
The protocol/capture, dashboard and micro smoke checks also pass locally.

This is implementation evidence, not independent security sign-off, native
key-store lifecycle acceptance or physical bearer/radio interoperability.
See [PACE contract](PACE_MESSAGING.md). Publication/hosted CI status must be
recorded separately from these local results.

## Messenger A–C local validation — 2026-10-25

On Linux with exact Rust 1.90.0, this pass passes 237 workspace tests
(52 CLI/dashboard, 173 core, 12 deny-only broker), strict Clippy, formatting,
manifest/role/MSRV validators, Messenger browser tests, the existing two-store
fake-SSH relay smoke and `tools/smoke_messenger_protocol.py`. OpenSSH `-G`
independently evaluates the fallback configuration without opening a connection.
Queue tests use an injected age identity rather than claiming native credential
store acceptance; they cover restart, wrong key, modified ciphertext, preserved
corrupt originals, failed enrollment on existing data and invalid-marker denial.
Protocol tests cover golden encoding, malformed frames, source authorization,
virtual-clock faults, lost receipts, idempotent retries and forged receipts.

Native CI at this exact revision is recorded separately when available. No real
private LAN, physical USB hardware or independently reviewed crypto integration
was tested here. [Build contract](MESSENGER_BUILD_PASSES.md) and
[hardware acceptance](MESSENGER_INTEROPERABILITY.md) distinguish software evidence
from remaining field/review gates. Two pre-existing strict-Clippy issues were
also corrected: a redundant required-text boolean and a constant-emptiness test.

## Handheld viewport geometry CI — 2026-10-26

The next layout pass places STOP REFRESH and status above the inventory and keeps pagers reachable during detail scrolling. The repository's stable Linux CI now invokes `tools/test_network_viewport.cjs` with headless Chrome/Chromium and production handheld CSS/Network Engineer JavaScript. Its fixture is a synthetic, explicitly authorized response with no real network access; the test requires 480×320 menu/result acceptance plus 800×480 and 320×568 menu checks. This is a newly authored test gate, **not** a passing field or browser claim until the exact PR run is green. Real touch calibration, glove use, GPIO buttons and hardware interoperability remain outside this software gate.

## Network Engineer handheld controls — 2026-10-26

[PR #32](https://github.com/pskeffington/pip_boy/pull/32) merged the four-tool-per-page handheld selector, status strip, labeled PREV/NEXT pager and the first physical-key pager mapping to `main` at `6ece898`. Its exact PR head `e88171d7a616e63dc6d68e67fa0dfe36656d2160` passed [quality run 40268875632](https://github.com/pskeffington/pip_boy/actions/runs/40268875632), with five jobs successful (stable Rust, exact Rust 1.90.0, native macOS, native Windows and source/manifest validation).

The follow-up extends D-pad Up/Down focus traversal, PageUp/PageDown page turns and high-contrast focus/press cues. Dependency-free browser fixture journeys cover tool-page boundaries, focus, no requests while browsing, text input preservation, consent checkbox traversal and safe cancellation. Its exact-head [quality run 40270208005](https://github.com/pskeffington/pip_boy/actions/runs/40270208005) passed all five jobs (stable Rust, 1.90.0, native macOS, native Windows, manifest), and [PR #33](https://github.com/pskeffington/pip_boy/pull/33) merged at `a8fb944`. Neither automated mock DOM journeys nor CI establish glove touch accuracy, physical button wiring, 480×320 screenshot geometry or an authorized network field test.

## Network mainline and UI consolidation — 2026-10-25

Baseline [PR #15](https://github.com/pskeffington/pip_boy/pull/15) merged LAN scan/watch, read-only UniFi connected-client reconciliation and DIAG ingestion to `main` at `15c9500`. [Quality run 40216528470](https://github.com/pskeffington/pip_boy/actions/runs/40216528470) passed all five jobs at `badabc523df585e31de20337f79d03dad8298139`: Linux stable, Rust 1.90.0, native macOS, native Windows and manifest validation.

The follow-up consolidates the prior network-suite/health work into that mainline without replacing the newer inventory/DIAG adapters. Local dependency-free JavaScript journeys verify no unsolicited fetch, authorization, two-entry pagination, one owned refresh timer, hidden-tab stop, no automatic resumption, destroyed-request abort, stale-response suppression and text-only controller labels. Offline manifest, role and compiler-policy validators pass. At code head `f77df8c95233c0ec31a0c1425e34a52ab7b7cce2`, [quality run 40217917079](https://github.com/pskeffington/pip_boy/actions/runs/40217917079) passed all five jobs: stable Rust with formatting, Clippy and runtime suites; exact Rust 1.90.0; native macOS; native Windows; and manifest validation. The new loopback CLI/API health suite and the existing LAN/UniFi/DIAG tests passed on all three native platforms. A documentation-only follow-up records these results; its exact head also receives the normal quality matrix.

No real LAN/controller, native ping process, physical Pi display or game/SSH endpoint was available in this editing environment. Controlled fixtures and loopback TCP tests remain distinct from those field acceptances.


## Rust 1.90 minimum-supported toolchain transition — October 25, 2026

The Reclamation Scout development branch declares `[workspace.package] rust-version = "1.90"` and now **enforces** Rust **1.90.0** alongside stable in GitHub Actions, using an explicit `RUSTUP_TOOLCHAIN` matrix override and compiler-version assertion. This change is intended to align the compiler floor with RustGIS while keeping the Rust 2021 edition and the existing lockfile. The exact updated commit requires its own complete CI result before promotion. **Historical compiler verification correction:** An earlier job named Rust **1.81.0** installed that compiler but actually used stable (`rustc 1.99.0` in [run 40172664621](https://github.com/pskeffington/pip_boy/actions/runs/40172664621)), because the checked-in `rust-toolchain.toml` overrode `rustup default`. Thus older job labels do **not** independently verify Rust 1.81.0 compatibility. Historical passing suites below remain evidence of the code under the **compiler actually used** and of native runtime tests, not independent MSRV or RustGIS/Raspberry Pi acceptance.

Updated October 25, 2026 (America/New_York). **Verified published baseline:** [PR CI 40168791127](https://github.com/pskeffington/pip_boy/actions/runs/40168791127) passed all five jobs at `176516e`: catalog, Linux stable and Rust 1.81.0, native macOS PTY and the actual source launcher, native Windows ConPTY and deny-all physical-broker checks. This is not live PlayStation, physical-hardware, packaging or accessibility certification. **Sections farther below preserve historical development snapshots**; any older “pending”, “unexecuted” or “startup failure” wording describes the state *at the cited historical revision*, not the current status.

## Live-update test marker regression

The browser revision `09dbf2d` passed catalog, Linux stable/MSRV and native macOS, including 147 Rust tests and actual source-launcher/inventory journeys ([CI 40169810949](https://github.com/pskeffington/pip_boy/actions/runs/40169810949)). Windows passed its 146 Rust tests and inventory journey, but the delayed-reference peer again reported WSAECONNABORTED despite a successful terminal child exit.

The terminal harness had accepted the short raw-output substring `90` as evidence of the updated HP value. Connection metadata or terminal sequences could satisfy that check before the delayed update. The follow-on waits for a unique nine-digit fixture value and deliberately includes `fixture-90` in interactive announcement metadata. A local mutation check restores the old `90` marker and requires that the delayed-reference journey fail, proving the decoy catches premature exit; the unmodified suite then verifies the actual delayed value. This changes test synchronization only; the Rust browser implementation remains unchanged from `09dbf2d`. Fresh hosted Windows verification is required for this strengthened check.

## Game browser navigation follow-on

The pure `game_view.rs` renderer separates object navigation from selected JSON, keeps the selected row visible, preserves parent context, clamps text scrolling to actual wrapping, adapts between side-by-side/stacked panes, and shows resize guidance below 40×12. Named selection survives dictionary insertions. An opened path follows replacement references or returns to a surviving parent after removal. Left/Right, R and End supplement existing navigation; the 32-level bound remains.

Six new deterministic memory-render/state tests exercise a 40-item inventory, parent restoration, reordered/removed/replaced game references, Unicode text tails and resize clamping, eight viewport sizes including zero dimensions, empty objects, terminal controls and cycles/depth. Local Linux stable and Rust 1.81.0 each passed **24 CLI + 111 core + 12 broker = 147 tests**. The expanded actual-client PTY peer journey opens inventory, selects item 26, opens/returns, shrinks/restores the terminal and exits; it passed locally. Current-head hosted macOS/Windows execution remains to be verified.

## Game readiness and prompted Mac launcher follow-on

Diagnostic/snapshot readiness now requires a bounded language/version announcement as well as a complete reachable root. Malformed announcement fields are rejected and unknown fields retained. The browser distinguishes waiting for an announcement from waiting for complete data. A later split-reference update no longer incorrectly triggers the initial-root timeout after an established connection.

The Mac source launcher accepts `--diagnose` before/after an address or without an address to prompt. Prompt text goes to stderr, preserving machine-readable JSON on stdout. Missing input, repeated/unknown flags and extra addresses fail explicitly. Native Mac regression journeys exercise explicit-IP diagnostics, prompted diagnostics and an interactive prompt → live update → Ctrl+C path from a project path containing spaces and a different working directory. Literal shell-like addresses, saved network denial and corrupt preferences are also covered.

The expanded controlled-peer suite checks delayed/missing/invalid/oversized announcements, live HP updates, a split reference after the ten-second startup deadline, and disconnect/error terminal restoration in PTY/ConPTY. Local Linux stable and Rust 1.81.0 each passed **18 CLI + 111 core + 12 broker = 141 tests**. Stable formatting, Clippy with warnings denied, workspace build, catalog validation, native CLI, runtime PTY and the expanded game peer/PTY smoke suite passed. Bash syntax validation passed. The three actual-launcher journeys and refusal cases also passed on Linux with a controlled `uname` shim substituting only the launcher platform gate, using the actual compiled client and loopback peer; this is shell/CLI contract evidence, not native macOS acceptance. Actual Mac launcher execution still requires native CI at this follow-on's published head.

Published at `6979a66`: [CI 40169394680](https://github.com/pskeffington/pip_boy/actions/runs/40169394680) passed catalog, both Linux toolchains and native macOS, including the actual prompted source launcher. Windows compiled and passed its 140 unit tests, then the mock peer reported WSAECONNABORTED despite a successful terminal child exit. The follow-on accepts EOF/reset/aborted only at the final close boundary; heartbeat and active-stream failures remain errors. The later marker investigation above identifies why a successful child exit was insufficient proof of the delayed update. This historical run does not establish complete Windows acceptance.

No actual PlayStation session is available; this pass does not complete live game or gameplay-control acceptance.

## Mac → PlayStation launch pass

Adds a PlayStation IP preset (TCP 27000 by default), protocol-aware diagnostic JSON, console-specific failure advice, and `scripts/launch-playstation.command` for Finder/Terminal. Diagnostic readiness requires a complete reachable game root, not merely TCP success; selected PlayStation profile is explicitly not authenticated console identity. Other edition expansion is deferred.

Local Linux stable and Rust 1.81.0 passed **17 CLI + 111 core + 12 broker = 140 tests**; formatting, Clippy, build, source validation, native CLI and game mock-peer smoke passed. The game suite checks the preset/diagnostic contract and rejects conflicting options/hostnames. **Verified:** [PR CI 40168791127](https://github.com/pskeffington/pip_boy/actions/runs/40168791127) passed all five jobs at `176516e`, including actual native macOS launcher execution through a repository symlink containing spaces, from a different working directory, against a temporary protocol peer. Executable mode is part of the launcher contract.

An actual PlayStation game session remains unavailable; PS4/PS5 build-specific support is not attested by the mock peer. See [Mac → PlayStation setup and live acceptance](MAC_PLAYSTATION.md).

## Fallout 4 companion precursor — controlled-peer validation

The opt-in read-only game browser is implemented with a transactional binary database decoder and direct TCP client. Local stable and Rust 1.81.0 passed **16 CLI + 111 core + 12 broker = 139 tests**; stable Clippy, build, formatting, offline catalog, native CLI and runtime PTY smoke passed. `tools/smoke_game.py` passed a loopback mock game with byte-fragmented frames, heartbeat verification, late references, busy/oversize refusal, no gameplay commands/data-file writes, and interactive PTY navigation/exit restoration. The same game harness is wired into Windows ConPTY and macOS/Linux CI. New hosted CI remains to be verified at publication.

**No actual game session was available.** All-version support, AE game-side enabler/bridge, console availability, gameplay controls and map images remain open. Synthetic protocol success is not proof of live game compatibility. See [game mode](GAME_MODE.md), module `game`, evidence L38.

## TUI keyboard-interrupt pass — local checks passed

Ctrl+C now returns through normal terminal guard cleanup; Ctrl+Shift+C is also accepted when delivered as a key event. Plain C, Alt+C, Ctrl+Alt+C and key-release events do not trigger the interrupt path. The POSIX PTY regression verifies actual Ctrl+C input, zero exit status, exact terminal attributes and alternate-screen restoration. Windows ConPTY coverage adds Ctrl+C after pause/resume and shrink to 20×5. Native CI must validate that new Windows journey. This is keyboard-event handling, not external OS signal cancellation.

Local Linux stable 1.99.1 and Rust 1.81.0 each passed **16 CLI + 107 core + 12 broker = 135 tests**. Formatting, Clippy with warnings denied, workspace build, catalog validation and the updated POSIX runtime smoke passed. Modules: console/telemetry; evidence: L05/L37. See [console contract](CONSOLE.md).

## G1-B/G1-C host field-quality pass — local checks passed

Built from merged foundation `794da68`. The new `host_report.rs` provides seven typed field readings with units, quality and finite reason codes; `status --details [--json]`, `sample --details` and the console derive them from the same single snapshot. The default JSON keys are preserved; unavailable CPU is explicitly null. CPU initialization requests only CPU/RAM and gates refreshes with monotonic elapsed time; the TUI no longer sleeps to prime CPU. Poll-start intervals reset with a new session. See the [host telemetry contract](HOST_TELEMETRY.md).

Executed locally on Linux with locked dependencies:

- Stable Rust **1.99.1**: format check, workspace unit tests, Clippy with `-D warnings`, and workspace binary build passed.
- Minimum-supported Rust **1.81.0**: workspace compile/unit tests passed. Both toolchains passed **15 CLI + 107 core + 12 broker = 134 tests**.
- All nine black-box suites passed: `smoke_native_cli`, `smoke_runtime`, `smoke_notes`, `smoke_permissions`, `smoke_simulator`, `smoke_replay`, `smoke_transport`, `smoke_mock_adapters`, and `smoke_broker`.
- Offline catalog validation passed: **27 modules, 37 evidence records, 14 external sources, 8 runtime entries**.

New fixtures cover exact CPU cadence, early/repeated/backward ticks, genuine zero, unavailable CPU/core/OS/memory values, NaN/infinity/range checks, inconsistent memory, expiry, report provenance/JSON round-trip, no duplicate collection, and paused-value clearing. The existing cross-platform `smoke_native_cli.py` now also checks detailed text/JSON/NDJSON, units, null reasons, timing, argument rejection and absence of data-file/ANSI side effects. Evidence IDs: L03, L19, L31, L32, L33; modules: `stat`, `telemetry`.

Native macOS/Windows execution for this new pass must be confirmed in its associated [quality workflow](https://github.com/pskeffington/pip_boy/actions/workflows/rust.yml) before merge. No dependency or permission change, hardware enablement, stored-observation migration, measured CPU accuracy/overhead baseline or accessibility certification is claimed.

## G2-A deny-all broker protocol — cross-platform CI verified

`crates/rust-boy-broker` is a **separate**, explicitly invoked, one-shot Rust executable whose only exposed channel is inherited stdin/stdout; it never starts on the normal TUI/status/doctor/notes paths. Its v1 input is a bounded 1,024-byte JSON record with strict capability/operation, opaque `device:` selector, request identifier, duration and purpose rules. It always returns a fixed five-field `allowed:false` response; even `operator_intent:true` cannot authorize a physical interface. It does not read a real port, device, radio, BLE, network or filesystem. No OS-enforced caller identity, broker privilege boundary, trusted user confirmation or physical handle exists.

Twelve new Rust unit tests and `tools/smoke_broker.py` cover strict deny-all behavior, unknown/future fields, malformed/oversize requests, capability-direction restrictions, invalid selectors, redaction, one-shot argument handling and no unintended data-directory writes. The quality workflow now builds **both** CLI and broker with `cargo build --workspace --locked` and runs the broker smoke on Ubuntu, native macOS and native Windows. The offline catalog validator checks the broker roster against declared adapter capabilities and rejects direct physical I/O imports in the virtual-only modules. **Verified:** [GitHub Actions run 40164717116](https://github.com/pskeffington/pip_boy/actions/runs/40164717116) passed all five jobs at implementation head `6713885`. Ubuntu stable passed 13 CLI + 102 core + 12 broker = **127 Rust unit tests**, formatting, compile, Clippy, binary build and nine black-box smoke suites. Linux Rust 1.81.0 passed workspace compile/tests. Native macOS passed 12 broker tests, the new subprocess smoke and its PTY navigation/recovery journey. Native Windows passed 13 CLI + 101 core + 12 broker = **126 Rust tests**, the offline smoke suites including the broker process, and interactive ConPTY restoration. The catalog audit passed independently. The test suite proves bounded strict denial and cross-platform startup, **not** authenticated callers or OS-enforced physical device authorization. See [broker protocol](BROKER_PROTOCOL.md) and [OS broker requirements](OS_DEVICE_BROKER.md).

## Native Windows ConPTY acceptance — verified on hosted Windows runner

Previous [push run 40162057807](https://github.com/pskeffington/pip_boy/actions/runs/40162057807) and [PR run 40162059963](https://github.com/pskeffington/pip_boy/actions/runs/40162059963) passed all five jobs at `8a4f7c5`, including Linux stable/MSRV, macOS native PTY and native Windows unit/CLI tests (**115 Rust tests per toolchain**, plus eight Linux smoke scripts). That green baseline did **not** exercise Windows interactive TUI ConPTY.

The new `tools/smoke_windows_conpty.py` is a Python-standard-library-only ConPTY harness using the OS `CreatePseudoConsole`, process-startup attribute, resize and process-wait APIs. It launches the **real** Rust-Boy TUI under a temporary `PIPBOY_HOME`, checks the initial STAT render, simulated P pause/resume, 20x5 fallback and resize recovery, Q exit, and alternate-screen restoration. A separate malformed-notes journey expects a nonzero exit and restored alternate screen without rewriting corrupt notes. The test has bounded output and deadlines, reads asynchronously to avoid blocked ConPTY pipes, and cleans up child processes/handles on error. The Windows runner executes it **after** native build and offline smoke suites. **Verified:** [run 40162936734](https://github.com/pskeffington/pip_boy/actions/runs/40162936734) completed with all five jobs successful at commit `4d27208`. The Windows job ran 13 CLI and 101 core unit tests plus its offline adapter/transport/permission smoke suite. Its ConPTY test reported a passing Windows command-shell output-pipe control, actual Rust-Boy TUI launch, P pause/resume, 20x5 shrink and resize recovery, Q normal exit, malformed-notes error exit, alternate-screen restoration and no hidden writes. Ubuntu stable ran formatting, Clippy, 13 CLI + 102 core tests and eight smoke suites; Rust 1.81.0 tests and native macOS PTY passed too. Windows ConPTY is **not** a hardware permission or device broker. Accessibility and physical-device gates remain open. No physical device or real authorization is involved. [Microsoft ConPTY reference](https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session); [OS device broker contract](OS_DEVICE_BROKER.md).

## Typed adapter and native console follow-on — execution pending

The core now exports `InputSource`, `DisplayBackend` and `SerialReader` traits with bounded `MockInputSource`, `MockDisplayBackend` and `MockSerialReader` implementations in `mock_adapters.rs`. Ten new Rust unit tests exercise trait-object dispatch, explicit simulated provenance, printable 1–80 character display frames, injected 1–32 byte serial data, saved denials, no implicit confirmation, revocation/cancellation and repeatable JSON. The CLI exposes `pipboy simulate adapter input|display|serial [--confirm]` using only source-defined fixtures. A new `tools/smoke_mock_adapters.py` checks JSON and policy refusal paths. No device or network I/O was introduced.

The native CI suite now includes `tools/smoke_native_cli.py` on Linux, macOS and Windows to verify noninteractive terminal-output behavior, no ANSI escape sequences, strict status JSON, no silent filesystem writes and CLI errors. On macOS, the job additionally runs the existing POSIX PTY `tools/smoke_runtime.py` for TUI keyboard and restoration behavior. **Windows ConPTY now has a passing native test; accessibility acceptance remains open**, as do physical adapters and real device authorization. These additional checks are authored but are not yet verified for the current code revision. See [MOCK_ADAPTERS.md](MOCK_ADAPTERS.md).

## New cross-platform and virtual transport pass — execution pending

The development branch now contains `virtual_transport.rs` with a **1–64 element FIFO**, `pump(1..=128)`, oldest-event drop accounting, pre-read active lease checks and cancellation/revocation/disconnect/timeout/policy-change queue invalidation. New CLI `pipboy simulate transport ID --count N --capacity N [--confirm]` emits a synthetic-only JSON envelope and leaves no files behind. Eight new core fixtures and `tools/smoke_transport.py` cover bounds, order, drop counts, explicit confirmation, denied/corrupt policy, and no stale-event reads. No network/socket/device driver is included.

The existing first-party-only CI now also defines `native` matrix jobs for GitHub-hosted `macos-latest` and `windows-latest`, using runner-native `rustup` for stable `cargo check --workspace --all-targets --locked`, `cargo test --workspace --locked`, and `cargo build -p pipboy --locked`; offline preference, simulator, replay and transport smoke scripts run on those OSs. The Linux stable/MSRV and catalog jobs remain intact. The prior full green Ubuntu result at `8047a24` is *not* proof that the new transport or platform jobs pass. See [virtual transport](VIRTUAL_TRANSPORT.md). Real TUI/PTY, native recovery, screen-reader/contrast tests and actual hardware are not covered by the new matrix.

## Verified Ubuntu baseline and follow-on tests

[Actions run 40160220866](https://github.com/pskeffington/pip_boy/actions/runs/40160220866) passed the complete Linux quality matrix at `e36fd59`: catalog validation, formatting, stable Clippy and compile, 13 CLI + 79 core unit tests on stable and Rust 1.81.0 (**92 each**), and five CLI/TUI smoke suites. GitHub Actions were enabled throughout; the earlier zero-job failure reproduced when an external Rust setup action was used, and resolved when setup/cache used runner-native `rustup`. The account-level action policy was not directly inspected.

**Latest verified delta:** [push run 40160762403](https://github.com/pskeffington/pip_boy/actions/runs/40160762403) and [PR run 40160765839](https://github.com/pskeffington/pip_boy/actions/runs/40160765839) both passed at `f445814`. Both toolchains completed **13 CLI + 84 core = 97 unit tests** with zero failures. Stable Rust 1.99.1 passed formatting, compile, Clippy with `-D warnings`, binary build and all five black-box scripts (`smoke_runtime`, `smoke_notes`, `smoke_permissions`, `smoke_simulator`, `smoke_replay`). Rust 1.81.0 passed workspace compile and unit tests; the offline catalog job passed. New behaviors: cancel-safe mock leases, 1–60,000 ms manually injected virtual TTL, exact-deadline expiry, fail-closed backward clock, and two new deterministic replay scenarios. This is simulated, in-process policy—not a real-time OS watchdog, network interface, or physical device capability. macOS/Windows and selected real-hardware acceptance remain outstanding.

## Historical initial deterministic replay work (before verified CI)

New `device_simulator.rs` behavior models virtual disconnect/reconnect without reviving a stale broker lease. An explicit `Unavailable` error distinguishes a disconnected device from a revoked token. The new `simulator_replay.rs` implements bounded, fixed `nominal`, `disconnect-recover`, and `revoke` logical-time traces; the CLI exposes `pipboy simulate replay ID SCENARIO [--confirm]`. Each trace is labeled `simulated: true`, uses `synthetic_logical_ms` (never wall time), emits one mock-only event before simulated failures, records subsequent rejection reasons, and does not automatically restart a lease. A requested preference is insufficient, while a saved denial remains blocking.

The new source contains eight replay unit fixtures and two disconnect/reconnect broker fixtures. A separate `tools/smoke_replay.py` is wired into the stable Actions workflow; it defines golden action/time/state sequences, checks byte-identical output for repeated runs, enforces deny-before-success and malformed-schema preservation, and verifies no data directory is created by default. Real device access and OS brokering remain absent. These source and test changes are **not compiled or executed**; static source consistency cannot substitute for Rustfmt, Clippy, cargo tests, or native platform checks.

Acceptance commands include `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo build -p pipboy --locked`, and `python3 tools/smoke_replay.py` plus the three prior smoke scripts. Both stable and Rust 1.81.0 must pass. GitHub Actions has previously produced zero-job `startup_failure`; new run-level status must be checked separately. See [SIMULATION.md](SIMULATION.md).

## G1-D simulated broker follow-on (authored, unexecuted)

The latest inspected [Actions run 40159278156](https://github.com/pskeffington/pip_boy/actions/runs/40159278156) reports `startup_failure` and zero jobs at head `ecbb861b`. A static review confirms 11 distinct `sim:*` fixtures, their one-to-one mapping to the 11 external capability enums, explicit confirmation/denial checks, broker-identity lease validation, no `std::fs`, `std::net`, or sysinfo backend in the simulator source, and CI smoke-script wiring. **Static checks are not compiler or functional test results.**

The development branch now includes `crates/pipboy-core/src/device_simulator.rs` and `pipboy simulate list|run ID [--confirm]`. It contains 11 fixed, virtual fixture IDs and no OS, RF, serial, BLE, location or network I/O backend. A simulation requires a fresh explicit `--confirm`; a stored requested preference never authorizes it, while a denied preference blocks it. A per-broker, generation-scoped lease is invalidated on explicit revoke, close or denied policy reload. Events use `simulated:true` and carry no fabricated device measurements. **This prototype is not the G2 hardware broker or active real-device authorization.**

Eight new Rust tests verify inventory coverage, request-vs-confirmation, saved denials, truthful fixture JSON, lease revocation and replay protection, policy reload/fail-closed behavior and cross-broker identity. A new `tools/smoke_simulator.py` script covers CLI parsing, requested/denied preferences, no default filesystem writes, schema corruption and strict simulated JSON output; the stable workflow invokes it. The static offline catalog validator now also checks the simulator fixture roster against all 11 external capability keys.

**Executable validation has not been performed** on these changes. This environment lacks Rust/Cargo, and GitHub Actions has been returning `startup_failure` before any jobs spawn. Run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo build -p pipboy --locked` followed by `python3 tools/smoke_simulator.py`, plus existing `smoke_permissions.py`, `smoke_notes.py` and `smoke_runtime.py`. Confirm stable and Rust 1.81, then native OS acceptance. The historical passing tests are not evidence for this commit. See [SIMULATION.md](SIMULATION.md).

## Current follow-on G1-D implementation: adapter preferences (unverified)

The development branch now contains `crates/pipboy-core/src/permissions.rs`, exposed by `lib.rs`, and a CLI `pipboy permissions list|request|deny|clear`. This is a **preference-only** record for 11 named future adapter capabilities, not actual consent. It never calls `ModuleHost::grant`; stored `requested` does not permit a radio, mesh, location, BLE or network adapter to run. Preferences are in a bounded 16 KiB schema-1 `permissions.json` under `PIPBOY_HOME`, protected by a stable advisory lock and flushed same-directory replacement. Read/list creates no files, invalid or future schemas are preserved, and the explicit commands update or clear decisions.

Five new unit tests and `tools/smoke_permissions.py` were authored to cover empty defaults, no implicit grants, repeat-safe set/deny/clear, unsupported capability refusal, writer contention, corrupt/future/oversized file preservation and CLI behavior. The CI stable job now includes this smoke script. **None of these new tests have been executed.** The current execution environment has no Rust/Cargo binary or network access to clone/install the toolchain; remote GitHub Actions has been failing at startup before any jobs are created. No new build, compilation, native platform, accessibility, or runtime success is claimed. This change also does not make previously authored STATUS or scoped LIBRARY/NOTES tests pass retroactively.

Before accepting, run `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo test --workspace --locked` on stable and 1.81.0, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and both existing smoke suites plus `python3 tools/smoke_permissions.py`. Verify Linux, macOS and Windows native file replacement and advisory locking. Source/static checks only are recorded separately and do not prove compilation. See [PERMISSIONS.md](PERMISSIONS.md).

## Change under test

G1-B/G1-C/G1-D/G1-E: shared `TelemetrySession<SnapshotSource>` for TUI and finite NDJSON. The session bounds history, records source/provenance, rejects backwards collection timestamps, and calculates freshness at render time. TUI history holds 60 readings; NDJSON history holds at most the requested 1–30. No telemetry files are created. The legacy `status --json` shape is preserved.

Module IDs: `stat`, `telemetry`. Existing evidence IDs: L03, L19, L31, L32, L33. TUI and finite NDJSON now use a lazy HostTelemetryModule through the lifecycle host. TUI P revokes/resumes local host-read permission. No external capability grants or device adapters were introduced; persistent per-adapter consent and OS brokering remain planned.

## New follow-on change: status lifecycle integration (not yet executed)

On the development branch, `pipboy status` and `status --json` now call a lazy `HostTelemetryModule` through `ModuleHost` instead of directly creating the collector in `main.rs`. Running this command still explicitly requests read-only local host metrics; it does not grant device enumeration, location, network or radio access. The raw eight-field Snapshot JSON contract and human-readable status output are retained.

Three additional deterministic tests have been authored for deny-before-factory activity, one successful sample with the legacy JSON shape, and collector-start failure without fabricated measurements. These tests and the following checks **have not been run for the new commit**: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `python3 tools/smoke_runtime.py`. The 63-test evidence below belongs to the preceding development head, not this follow-on change. GitHub Actions must execute for the new commit before claiming G0/G1 acceptance.

## Executed quality checks

| Command | Observed result |
| --- | --- |
| `python3 tools/validate_catalog.py` | PASS: 27 modules, 37 evidence records, 14 external sources, 8 runtime entries |
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --all-targets --locked` | PASS on stable |
| `cargo test --workspace --locked` | PASS: 10 executable + 53 core tests |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `cargo +1.81.0 test --workspace --locked` | PASS: same 63 tests |
| `cargo build --workspace --locked` | PASS |
| `cargo check --workspace --all-targets --target x86_64-pc-windows-gnu --locked` | PASS on stable: cross-compilation check only, no native execution |
| `python3 tools/smoke_notes.py` | PASS: 24 concurrent CLI appends with retry, unique IDs, no lost notes, search, export/import/re-import and schema preservation |
| `python3 tools/smoke_runtime.py` | PASS: timed NDJSON, closed pipe, no telemetry files, TUI pause/resume, reload/oldest-note selection, resize fallback/recovery and exit/error restoration |

Stable toolchain: rustc 1.99.1 (58cf8f9c3, 2026-10-13). Minimum toolchain: rustc 1.81.0 (eeb90cda1, 2024-09-04).

The generated lockfile uses format 3 for Cargo 1.81 compatibility. Modern Cargo's fallback resolver avoids transitive dependencies requiring Rust above 1.81; the tested lock selects instability 0.3.10 and unicode-segmentation 1.12.0. Cargo 1.81 emits a harmless warning for the newer resolver configuration key. CI now uses `--locked`.

Deterministic fixtures cover bounded shared history, stale-at-expiry, no collection during render, invalid retention/expiry, clock rollback before collector activity, finite NDJSON schema round-trip, injected waits, closed output pipes, and propagation of other writer errors.

## Lifecycle integration checks

New fixtures verify no start/sample before grants and explicit start, immediate collector release on revoke, regrant without implicit restart, scoped revocation, cleanup after partial startup failure, explicit reset after unavailable, repeated poll degradation/recovery, stop/drop cleanup, lazy factory invocation, history reset after restart, and pause text without host values.

`tools/smoke_runtime.py` repeats black-box checks against the built binary: two timed NDJSON records, early completion on a closed pipe despite a 30-second requested interval, no telemetry data files, TUI pause held longer than a refresh period, resume with `1 retained / 0 dropped`, all-panel navigation, and normal/malformed-notes terminal restoration. This smoke script is added to stable CI but remote execution is unverified.

## Notes storage pass

The schema-1 filename and fields remain compatible. Writes now hold a stable fs2 advisory lock over read/modify/replace, flush a same-directory tempfile and persist by replacement without unlinking the original first. Unix additionally flushes the parent directory. Bounded reads (8 MiB), note count (10,000), IDs, text, schema versions and unexpected fields are validated. Search is read-only.

Nine new deterministic fixtures cover no-write reads, search, invalid/future schema preservation, lock contention/release, interrupted staging, failed replacement, count/size/ID limits, private Unix modes and orphaned staging. The real CLI smoke test coordinates 24 subprocess appends through up to eight simultaneous subprocesses, retries only reported lock contention, and verifies every expected entry and unique sequential ID. No actual power-failure test was performed.

New dependencies: fs2 0.4.3 and tempfile 3.27.0, reviewed against primary API/source documentation (L35/L36), compiled/tested on Linux stable/MSRV and cross-checked for the Windows GNU target. Native Windows/macOS runtime and filesystem acceptance remain open. See [storage contract](STORAGE.md).

## Console accessibility pass

Four new memory-backed tests cover navigation bounds/preserved selection, empty/planned views, every panel at seven sizes (42 combinations including 0x0, 1x1, 20x5, 39x11, 40x12, 60x16 and 100x24), and access to an older note's wrapped Unicode text tail followed by scroll clamping after enlargement.

The runtime PTY smoke test also modifies the temporary notes file externally, verifies explicit R reload, reaches the oldest note with End, shrinks to 20x5 for fallback guidance, grows back to 80x24 with selection intact, and exits without modifying the notes file. Notes are no longer re-read on each redraw, and no fixed 12-note display limit remains. Ratatui's locked 0.29.0 rendered-line-info feature is used for matching scroll bounds to the actual word wrapper.

The console pass passed 56 tests on stable and Rust 1.81.0. The updated workspace also passes the Windows GNU target compile check; native execution is not established. See [console contract](CONSOLE.md).

## Notes portability pass

Seven additional core tests verify source-preserving export, ID remapping into an existing collection, timestamp/text preservation, repeat-safe re-import, repeated-identical-note multiplicity, invalid/oversized/empty input with no write side effects, writer exclusion, corrupted canonical-file preservation, capacity/ID overflow and combined encoded-size failure without partial commit, and output failure without source changes.

The CLI smoke suite exports all 25 notes created by its append/concurrency checks, imports them into a second collection with an existing note, verifies 26 unique sequential local IDs, re-imports without rewriting, and rejects malformed/future backup input and extra/missing arguments while preserving data. The temporary input file is separate from live notes.json. No network, backup scheduler or automatic migration was added.

Current local verification: 63 tests on stable and Rust 1.81.0, Clippy, format, Windows GNU all-target compile check and both CLI/PTY smoke suites. Details: [storage contract](STORAGE.md).

## Earlier Linux smoke checks (commit 04f4f0f)

A Python subprocess/PTY harness used the built `target/debug/pipboy` with a temporary `PIPBOY_HOME` and `TERM=xterm-256color`:

- `doctor`: validated bundled catalogs successfully.
- `status --json`: parsed real host output and confirmed a positive memory total.
- `sample --count 2 --interval-ms 500`: parsed exactly two versioned JSON records; monotonic capture timestamps differed by at least 500 ms.
- Before adding notes, the temporary data directory remained empty after telemetry commands.
- Added two notes and read back ID 2, verifying repeated local persistence on Linux.
- In a 100×24 pseudo-terminal, navigated all six panels with Tab and exited with Q (exit 0).
- With deliberately malformed notes JSON, navigated to NOTES and observed an error exit (exit 1).
- Both TUI journeys restored the original terminal attributes and emitted the leave-alternate-screen sequence.

## Remaining acceptance work

Remote GitHub Actions previously failed before jobs started; its current success is not established by these local checks. Storage is published as 2223c2c. Console and portability work remain local while the automatic approval reviewer rejects publication; no approval setting was changed. Verify any eventual pushed revisions separately in Actions. macOS/Windows smoke checks, assistive-technology/contrast acceptance, panic/signal paths, native Windows/macOS note replacement, locking and durability, observation schema migrations/field-level units, persistent per-adapter consent, and device-broker enforcement remain open. G0/G1 are not declared complete, and no hardware readiness is claimed.

The optional native receiver smoke, `python3 tools/smoke_radio_sdrconnect.py`,
is classified as a manual hardware check. Build pipboy with `--features
sdrconnect` and provide an OS audio output; the test uses synthetic PCM at zero
volume and temporary preferences. `--live-ip` explicitly adds Skylab playback;
`--standalone /path/to/rust-radio-cli` also checks the standalone native player.
It never claims synthetic samples are received RF and is excluded from CI
profiles that lack audio hardware.

# Rust-Boy | Local graphical dashboard

## Vault command center — 2026-10-26

The desktop route `/` is the **internal Vault / Command Center**, not the field Pip-Boy. Its universal primary navigation is **COMMAND / MODULES / VIEWPORT / MESSAGES / SETTINGS**, with a separate explicit **RADIO / IP** button. Roles configure authorization and work queues; they do not replace the common navigation. The masthead identifies the **VAULT / COMMAND CENTER** workstation. The dedicated `/micro` route remains the field device.

**COMMAND** is the centralized mission, task, and job-assignment workspace. **Overseer** launches into COMMAND on first page load, while other roles retain their existing Module startup and can open COMMAND at any time. The new stage expands to full desktop width, hides the unrelated tool inspector, and keeps the local task board internally scrollable, leaving the document and primary controls on one screen. A command rail provides explicit **MESSAGES**, **ACTIVE VIEW**, and **MODULES** navigation.

Assignment permissions are **enforced by the Rust API**, using the active persisted `mission-board` slot. Overseer (or a deliberately configured Custom station with that capability) may create/reassign tasks and explicitly dispatch to a configured SSH peer; other profiles receive **only their role-scoped duties**, with progress and acknowledgment where allowed. The browser's authority badge is explanatory, not permission enforcement. Creation saves to the local task store and does **not** send. Peer delivery requires a paired destination and explicit confirmation; remote storage is not human acknowledgment. The handheld can still receive, inspect, and update field progress through the shared local API.

The Overseer **VIEWPORT** now renders the shared full-canvas situational map, separate from COMMAND; the map's header selector provides a return to COMMAND and other pages without covering the geometry. When a Custom station selects Task Control, its Viewport offers a link to the central Command stage instead of mounting a second board. Asynchronous task reads render into a role-specific mount node so a delayed response cannot overwrite a newly loaded role's task queue.

Run `node tools/test_desktop_focus_ui.cjs`, `node tools/test_desktop_viewport.cjs` and `node tools/test_dashboard_viewport.cjs` for Command navigation, role scope, one-screen layout and no automatic POST/dispatch. Browser and Raspberry Pi acceptance remain separate.

## Compact desktop focus layout — 2026-10-26

The shared desktop workspace retains **COMMAND / MODULES / VIEWPORT / MESSAGES / SETTINGS** navigation for every role; station/profile changes happen only in **SETTINGS**. The workspace is one full-width view; selecting a tool opens full-screen details with a return action. Station selection, six-item pages (two on short touch screens), the Custom tool editor, Reclamation Scout MAP/SITES/DUTIES, camera views and the `/micro` UI are preserved.

Opening or closing Settings updates the Rust-owned `ForegroundSelection` through same-origin `POST /api/host/settings`; `/api/state` reports its current surface so reload restores Settings when that host surface is active. The endpoint accepts only a boolean Settings transition. It cannot select a package or make a reference manifest installed; no approved package roster exists yet. Other desktop stage/pane navigation remains local to the browser.

Visible copy is now intentionally minimal: compact tool buttons with name and meaningful status, a three-value **Station / Tools / Access** band, and concise inspector and Settings labels. Full Rust-authored station/tool descriptions remain available as hover tooltips or inspector **About this tool** disclosure. **Safety & access** is also expandable. Optional guidance is closed by default; no permissions or information are removed.

Opening a non-Overseer desktop reads only `GET /api/state`; Overseer additionally opens COMMAND and reads the local task board (and configured peer metadata) without sending or mutating it. The MESSAGES stage is initially hidden and its existing local Messenger component is mounted only after that page is selected; reading local journal status does not send, retry, connect to peers, launch a LAN scan, or enable hardware. Load-station confirmation remains mandatory and a Settings preview never changes active pinned tools.

**Regression coverage:** `node tools/test_desktop_focus_ui.cjs` checks unchanged routing/pagers, consent and collapsed details. `node tools/test_desktop_viewport.cjs` uses headless Chromium and a fictional Rust snapshot at 1440×900, 1280×800, 1024×768 and 800×600, including verifying that MESSAGES triggers a local GET only on first entry. The existing `node tools/test_dashboard_viewport.cjs` additionally exercises 480×320, 800×480, Scout/Security focused views, Custom and 44px touch sizing on short screens. Physical Mac browser and Raspberry Pi acceptance remain separate.

## Unified Network Engineer

`main` includes the shared desktop/micro Network Engineer. Its 16-operation catalog arrives from Rust, with offline calculators, bounded ICMP/TCP health, private-LAN evidence and controller-connected-client inventory. The inspector mounts `assets/network.js`; tool navigation itself performs no I/O. Active tools have a separate authorization step. Inventories have pages; refresh stops on navigation, edits, hidden tabs, errors or STOP and never resumes automatically. [Operator setup and measurement definitions](NETWORK_HEALTH_UNIFI.md).

**Status:** developer-preview shell running entirely from Rust-Boy on **127.0.0.1**, with a responsive original HTML/CSS/JavaScript frontend embedded in the Rust executable. This is a real working local web UI, **not** a public website, a packaged desktop binary, a RustGIS map renderer, an ATAK client or a released handheld OS.

## Dedicated handheld micro route

Use [micro handheld mode](MICRO_UI.md) when testing a real micro-display layout instead of shrinking the desktop workstation. Start `./scripts/launch-micro.command` or `cargo run --locked -p pipboy -- dashboard --micro --open` on Mac. The local server serves **/micro** alongside the existing **/** desktop route. Both use the exact same workbench API; changing a job template in either view is reflected in the other after reload.

## NET ENGINEER active network tool

Load Head Engineer or Security Chief, open NET ENGINEER, select Live Private LAN Scan, choose local or an authorized private /24..../32 CIDR, affirm the scope, and press Scan Private LAN. Response lists detected IPs, ICMP/TCP latency evidence, and neighbor-cache sightings, not an authoritative list of all connected clients. Optional monitor refresh runs once per 60 seconds only while this view remains active. See [network diagnostics](NETWORK_ENGINEERING.md). Use the UniFi Sites/Clients/Compare modes to read the local, configured UniFi Network Integration API; no browser API key is sent. The offline CIDR calculator is still available. See [UniFi setup](NETWORK_ENGINEERING.md).

## Launch on macOS

From the current `main` branch, with Rust **1.90 or newer** installed:

```bash
git clone https://github.com/pskeffington/pip_boy.git
cd pip_boy
cargo run --locked -p pipboy -- dashboard --open
```

Or double-click `scripts/launch-dashboard.command` in Finder after cloning. The Mac launcher opens the default browser and leaves the local Rust server running in Terminal. Press **Ctrl+C** in that terminal to shut it down. The default address is **http://127.0.0.1:8765/**. If that port is in use, start the server with a different port:

```bash
cargo run --locked -p pipboy -- dashboard --port 8790 --open
```

On Linux or an appropriately configured Raspberry Pi, omit `--open` and browse to the printed local address **on that same device**. The server intentionally cannot be reached from other LAN computers; the browser and Rust process must run on the same host. `--port 0` selects an ephemeral loopback port, primarily for automated tests.

## Peer task dispatch

The desktop and micro Task Control components share the same local peer roster and task APIs. Device send/sync operations require both a configured SSH peer and a separate operator confirmation; no job selection makes a connection. The receiving Rust-Boy persists incoming assignments and reports their statuses only on explicit SSH synchronization. No email draft, automatic notifications or browser-initiated arbitrary network targets are present. See [peer security and setup](PEER_TASK_TRANSPORT.md).

## Overseer task management

Desktop **COMMAND** now hosts the shared task board in a dedicated full-width mission center, with create/assign/status/reassign, overdue counts and manual handoff/acknowledgment. Task data is separate from profile settings, so switching roles never erases tasks. Other job screens show their locally assigned queue. The clipboard/email-draft workflow is **operator-controlled** and does not imply actual delivery; authenticated remote synchronization is a future gate. See [Task Control](TASK_CONTROL.md).

## User journey (shared desktop shell)

1. **Command:** Overseer starts at COMMAND; all roles can open it. Authorized managers assign, review and explicitly dispatch jobs. Other roles see their scoped duty queues. **Modules:** The module deck displays only active pinned tools from Rust `PRESETS`/`PALETTE` and the persisted workbench.
2. **Inspector and viewport:** Select a tool to inspect/execute its allowlisted offline operation. Use VIEWPORT for its dedicated map, task, or camera panel, and MESSAGES for the same manually operated messenger entrypoint. At narrow widths, WORKSPACE and INSPECTOR switch without page scrolling.
3. **Settings → profiles:** Open SETTINGS to see the active profile and a paginated list of seven occupational presets plus Custom. Choosing a profile only stages a selection in Settings; **neither the active module deck nor live tool capabilities change** until LOAD PROFILE and the confirmation are completed. Canceling has no effect.
4. **Custom tools:** Once Custom is the active profile, SETTINGS → MANAGE CUSTOM TOOLS opens the paged module palette, with a dedicated SETTINGS return action. Planned hardware modules remain disabled.
5. **Safety:** Selecting a profile or a tool never grants permissions or initiates a network, camera, or hardware connection. Backend profile changes continue through the explicitly confirmed `POST /api/profile` contract.

## Interaction and layout

The interface uses charcoal and phosphor-green cues but **no proprietary Fallout graphics, fonts or UI code**. Its desktop shell has two functional panels (shared workspace + inspector), with a fixed COMMAND / MODULES / VIEWPORT / MESSAGES / SETTINGS tab row. Occupational roles only alter the pinned tools and permitted module data, not the navigation hierarchy. Narrow displays expose WORKSPACE / INSPECTOR as a two-way toggle; the separate `/micro` route is the preferred handheld experience. Responsive targets remain **800×480** and **480×320**, pending physical Pi acceptance.

- **Settings → profile:** select a different role and explicitly confirm LOAD PROFILE.
- **Alt+Up/Down (Settings only):** change staged profile selection; **Alt+Enter (Settings only):** open confirmation.
- **Click module:** inspect; **Run local test:** execute only the selected allowlisted offline operation.
- **Escape:** return the camera-wall fixture to four tiles, or cancel a confirmation dialog.
- **Refresh icon:** reload the saved Rust workbench state.

## Local HTTP/API contract

The Rust service embeds and serves exactly `/`, `/assets/dashboard.css`, `/assets/dashboard.js` and a small allowlisted JSON interface: `GET /api/state`, `POST /api/profile`, `POST /api/custom/add`, `POST /api/custom/remove`, `POST /api/experiment`. JSON POST requests must come from the exact same origin (`http://127.0.0.1:<port>`), carry `Content-Type: application/json`, and stay within **4 KiB**; the server enforces a strict Host header, maximum headers of **8 KiB**, no transfer encoding, no CORS and no arbitrary shell execution. The browser receives a restrictive Content Security Policy, no-store headers and frame denial. The Rust listener binds **IPv4 loopback only**; no public bind option exists.

The state endpoint deliberately excludes note contents, camera credentials, console addresses and user location. Role profiles only change layout, and safe experiments are explicitly invoked. The existing atomic/persisted WorkbenchStore remains the source of truth; corruption or unknown future schema does not trigger destructive reset.

This dashboard is a **development UI**, not a security boundary against malicious local programs with direct access to the operator's account. Do not expose or proxy it to an untrusted network. Future physical-device connectors will require OS-enforced permissions, authenticated client sessions and separate security review. A private WAN/mobile deployment needs a different architecture.

## Automated and manual acceptance

Run:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --locked
python3 tools/smoke_dashboard.py
```

The smoke script launches the **actual Rust server** on an ephemeral loopback port, fetches embedded CSS/JS/HTML, checks exact-eight template ordering and Custom blankness, performs role replacement and offline Scout/NET/byte computations, verifies persisted state, and rejects cross-origin/invalid input attempts. It runs in Linux stable CI and in native macOS/Windows smoke jobs. **Manual browser testing remains required** for Safari, Firefox/Chrome, accessible keyboard/touch operation, focus behavior, 800×480 and 480×320 breakpoints. Physical Pi touch/GPU tests and real RustGIS embedding are future gates.

## Later: embed or launch RustGIS in Scout

Do not substitute the decorative grid for navigation. RustGIS integration will require an evaluated **process/IPC or native map renderer interface**, offline layers, map pan/zoom, true GNSS consent (if enabled), waypoint/GeoJSON interoperability and Pi ARM64 GPU/performance acceptance. The compiler minimum is Rust 1.90, but no RustGIS crate is linked in this dashboard pass.

See [role roadmap](ROLE_ROADMAP.md), [Scout GIS/TAK plan](SCOUT_GIS_TAK.md), [security policy](../SECURITY.md), and [camera-wall roadmap](SECURITY_CAMERA_WALL.md).

## One-screen viewport rule (October 2026)

The desktop shell must fit one viewport without document-level scrolling. At wide widths, the central workspace and inspector remain visible together; **no persistent role rail** is displayed. Below 1100 CSS pixels, **WORKSPACE / INSPECTOR** becomes an explicit two-way screen switch. The central workspace always exposes **COMMAND / MODULES / VIEWPORT / MESSAGES / SETTINGS**. Role selection is a nested Settings page, while MANAGE CUSTOM TOOLS opens a paged subview only when Custom is active. Modules, roles, and palette use visible PREV/NEXT controls instead of long vertical lists.

### Host-owned package viewport

The package stage renders the portable v1 `PackageView` schema through host-created headings, paragraphs, status rows and lists. Content is assigned as text; package HTML, scripts, URLs and callbacks are not accepted. The view has an internal bounded scroll region, preserving document-level no-scroll behavior. Package selection remains a native keyboard-accessible control and Settings remains host-owned and always reachable. Interactive package actions are deferred until capability policy passes RB-CX-02.

**Acceptance criteria:** at normal browser zoom, do not expose a body scrollbar; keep the masthead, page switches, active screen and its navigation in the visible window; never hide the last module behind a clipping mask. At 480×320, use the dedicated `/micro` shell, which was specifically designed for the handheld. Dense data output, long device messages, authorization forms, and accessibility zoom may still use *internal* scrolling when needed; hiding essential controls is not an acceptable no-scroll implementation. The desktop radio and other future modules should use the same focused-panel/paging contract rather than adding sections below the fold.

Browser/physical hardware acceptance is still required at 480×320, 800×480, 1280×720, and 1920×1080, including focus, touch operation, enlarged fonts, and no clipped buttons. A CSS-only no-scroll result does not establish that all embedded components fit.

### Follow-up UI pass: focused subviews and browser acceptance

The responsive dashboard now opens **INSPECTOR** when a module tile is chosen on a narrow screen and provides a dedicated **MODULES** return action. Staging a role in Settings does not change the visible workspace; only a confirmed profile load replaces pinned modules. Role lists show two items per page on short (480 CSS-pixel high or smaller) viewports to retain large touch targets; module and Custom palettes show four, with six on taller displays. Keyboard Tab/Enter and native button semantics remain supported.

The Reclamation Scout viewport now separates **MAP / SITES / DUTIES** into focused screens. A site-to-map action opens MAP and requests focus on the chosen site after the map is mounted in a visible container. Security Chief separates **CAMERAS / DUTIES**; simulated cameras remain explicitly disconnected. These focused views prevent the original stacked content from being clipped by the no-scroll shell. Overflow-heavy forms, task details, and accessibility zoom retain internal scroll access.

The new `node tools/test_dashboard_viewport.cjs` Chromium gate is run by GitHub Actions. It loads the *production dashboard HTML, CSS, and dashboard JavaScript* in exact-size iframe viewports, supplies local synthetic API data, and checks page bounds, module and role paging, narrow-screen inspector navigation, workspace switches, and the Custom palette. Sizes: 480×320, 800×480, 1280×720, 1920×1080, plus Custom scenarios at 480×320 and 1280×720. No external service, peer, camera, or network scan is contacted by the fixture. Tests are a browser regression gate, not physical Raspberry Pi approval.


## Shared interface architecture — October 26, 2026

The operator's occupation is **configuration, not navigation**. Desktop uses the same five workspace tabs in all roles, while micro keeps the same six hardware-oriented tabs (STAT / GEAR / MAP / DATA / LINK / SETUP). The active Rust profile determines pinned tools, assignment scope, and optional role-specific VIEWPORT contents, without changing the universal shell. Desktop and handheld both store changes through the shared workbench API. Desktop role selection is exclusively in SETTINGS; on micro it is exclusively in SETUP's CHANGE PROFILE dialog. The micro top bar always reads FIELD TERMINAL.

Previewing a new role must never cause a profile mutation, change active tool pins, or launch a device connector. A confirmation remains mandatory before replacing the active template. New acceptance assertions verify SETTINGS-only role discovery, active-module stability during preview, five-tab desktop consistency, and the SETUP-only micro selector.

## RADIO native dock within VIEWPORT (October 26, 2026)

The common COMMAND / MODULES / VIEWPORT / MESSAGES / SETTINGS navigation has five stages. Open **VIEWPORT → RADIO / IP** to mount the native in-process radio control widget. The radio dock replaces the active view within the existing workspace rather than stacking an always-visible second section. **◀ ACTIVE VIEW** or Escape returns to the original viewport. Switching away from VIEWPORT unmounts the radio controller UI and stops local status polling, **not** audio that the user intentionally started; STOP is still an explicit action. Merely launching the desktop, choosing a role or browsing other stages never opens a stream or fetches radio status.

The desktop Chromium fixture covers explicit radio opening, no autoplay, view containment and clean unmount while preserving the one-screen 5-stage navigation. Micro radio remains accessible via LINK → RADIO / IP. Actual OS output and Pi touch acceptance remain separate gates.

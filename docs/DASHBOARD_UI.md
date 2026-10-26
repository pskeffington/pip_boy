# Rust-Boy | Local graphical dashboard

## Unified Network Engineer

`main` includes the shared desktop/micro Network Engineer. Its 16-operation catalog arrives from Rust, with offline calculators, bounded ICMP/TCP health, private-LAN evidence and controller-connected-client inventory. The inspector mounts `assets/network.js`; tool navigation itself performs no I/O. Active tools have a separate authorization step. Inventories have pages; refresh stops on navigation, edits, hidden tabs, errors or STOP and never resumes automatically. [Operator setup and measurement definitions](NETWORK_HEALTH_UNIFI.md).

**Status:** developer-preview shell running entirely from Rust-Boy on **127.0.0.1**, with a responsive original HTML/CSS/JavaScript frontend embedded in the Rust executable. This is a real working local web UI, **not** a public website, a packaged desktop binary, a RustGIS map renderer, an ATAK client or a released handheld OS.

## Dedicated handheld micro route

Use [micro handheld mode](MICRO_UI.md) when testing a real micro-display layout instead of shrinking the desktop workstation. Start `./scripts/launch-micro.command` or `cargo run --locked -p pipboy -- dashboard --micro --open` on Mac. The local server serves **/micro** alongside the existing **/** desktop route. Both use the exact same workbench API; changing a job template in either view is reflected in the other after reload.

## NET ENGINEER active network tool

Load Head Engineer or Security Chief, open NET ENGINEER, select Live Private LAN Scan, choose local or an authorized private /24..../32 CIDR, affirm the scope, and press Scan Private LAN. Response lists detected IPs, ICMP/TCP latency evidence, and neighbor-cache sightings, not an authoritative list of all connected clients. Optional monitor refresh runs once per 60 seconds only while this view remains active. See [network diagnostics](NETWORK_ENGINEERING.md). Use the UniFi Sites/Clients/Compare modes to read the local, configured UniFi Network Integration API; no browser API key is sent. The offline CIDR calculator is still available. See [UniFi setup](NETWORK_ENGINEERING.md).

## Launch on macOS

From the `feat/dashboard-ui-shell` development branch, with Rust **1.90 or newer** installed:

```bash
git clone --branch feat/dashboard-ui-shell https://github.com/pskeffington/pip_boy.git
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

The desktop Overseer Module Viewport now hosts the same task board as the handheld interface, with create/assign/status/reassign, overdue counts and manual handoff/acknowledgment. Task data is separate from profile settings, so switching roles never erases tasks. Other job screens show their locally assigned queue. The clipboard/email-draft workflow is **operator-controlled** and does not imply actual delivery; authenticated remote synchronization is a future gate. See [Task Control](TASK_CONTROL.md).

## User journey (shared desktop shell)

1. **Modules:** Start in the same FIELD WORKBENCH regardless of active occupation. The module deck displays only active pinned tools from Rust `PRESETS`/`PALETTE` and the persisted workbench.
2. **Inspector and viewport:** Select a tool to inspect/execute its allowlisted offline operation. Use VIEWPORT for its dedicated map, task, or camera panel, and MESSAGES for the same manually operated messenger entrypoint. At narrow widths, WORKSPACE and INSPECTOR switch without page scrolling.
3. **Settings → profiles:** Open SETTINGS to see the active profile and a paginated list of seven occupational presets plus Custom. Choosing a profile only stages a selection in Settings; **neither the active module deck nor live tool capabilities change** until LOAD PROFILE and the confirmation are completed. Canceling has no effect.
4. **Custom tools:** Once Custom is the active profile, SETTINGS → MANAGE CUSTOM TOOLS opens the paged module palette, with a dedicated SETTINGS return action. Planned hardware modules remain disabled.
5. **Safety:** Selecting a profile or a tool never grants permissions or initiates a network, camera, or hardware connection. Backend profile changes continue through the explicitly confirmed `POST /api/profile` contract.

## Interaction and layout

The interface uses charcoal and phosphor-green cues but **no proprietary Fallout graphics, fonts or UI code**. Its desktop shell has two functional panels (shared workspace + inspector), with a fixed MODULES / VIEWPORT / MESSAGES / SETTINGS tab row. Occupational roles only alter the pinned tools and permitted module data, not the navigation hierarchy. Narrow displays expose WORKSPACE / INSPECTOR as a two-way toggle; the separate `/micro` route is the preferred handheld experience. Responsive targets remain **800×480** and **480×320**, pending physical Pi acceptance.

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

The desktop shell must fit one viewport without document-level scrolling. At wide widths, the central workspace and inspector remain visible together; **no persistent role rail** is displayed. Below 1100 CSS pixels, **WORKSPACE / INSPECTOR** becomes an explicit two-way screen switch. The central workspace always exposes **MODULES / VIEWPORT / MESSAGES / SETTINGS**. Role selection is a nested Settings page, while MANAGE CUSTOM TOOLS opens a paged subview only when Custom is active. Modules, roles, and palette use visible PREV/NEXT controls instead of long vertical lists.

**Acceptance criteria:** at normal browser zoom, do not expose a body scrollbar; keep the masthead, page switches, active screen and its navigation in the visible window; never hide the last module behind a clipping mask. At 480×320, use the dedicated `/micro` shell, which was specifically designed for the handheld. Dense data output, long device messages, authorization forms, and accessibility zoom may still use *internal* scrolling when needed; hiding essential controls is not an acceptable no-scroll implementation. The desktop radio and other future modules should use the same focused-panel/paging contract rather than adding sections below the fold.

Browser/physical hardware acceptance is still required at 480×320, 800×480, 1280×720, and 1920×1080, including focus, touch operation, enlarged fonts, and no clipped buttons. A CSS-only no-scroll result does not establish that all embedded components fit.

### Follow-up UI pass: focused subviews and browser acceptance

The responsive dashboard now opens **INSPECTOR** when a module tile is chosen on a narrow screen and provides a dedicated **MODULES** return action. Staging a role in Settings does not change the visible workspace; only a confirmed profile load replaces pinned modules. Role lists show two items per page on short (480 CSS-pixel high or smaller) viewports to retain large touch targets; module and Custom palettes show four, with six on taller displays. Keyboard Tab/Enter and native button semantics remain supported.

The Reclamation Scout viewport now separates **MAP / SITES / DUTIES** into focused screens. A site-to-map action opens MAP and requests focus on the chosen site after the map is mounted in a visible container. Security Chief separates **CAMERAS / DUTIES**; simulated cameras remain explicitly disconnected. These focused views prevent the original stacked content from being clipped by the no-scroll shell. Overflow-heavy forms, task details, and accessibility zoom retain internal scroll access.

The new `node tools/test_dashboard_viewport.cjs` Chromium gate is run by GitHub Actions. It loads the *production dashboard HTML, CSS, and dashboard JavaScript* in exact-size iframe viewports, supplies local synthetic API data, and checks page bounds, module and role paging, narrow-screen inspector navigation, workspace switches, and the Custom palette. Sizes: 480×320, 800×480, 1280×720, 1920×1080, plus Custom scenarios at 480×320 and 1280×720. No external service, peer, camera, or network scan is contacted by the fixture. Tests are a browser regression gate, not physical Raspberry Pi approval.


## Shared interface architecture — October 26, 2026

The operator's occupation is **configuration, not navigation**. Desktop uses the same four workspace tabs in all roles, while micro keeps the same six hardware-oriented tabs (STAT / GEAR / MAP / DATA / LINK / SETUP). The active Rust profile determines pinned tools, assignment scope, and optional role-specific VIEWPORT contents, without changing the universal shell. Desktop and handheld both store changes through the shared workbench API. Desktop role selection is exclusively in SETTINGS; on micro it is exclusively in SETUP's CHANGE PROFILE dialog. The micro top bar always reads FIELD TERMINAL.

Previewing a new role must never cause a profile mutation, change active tool pins, or launch a device connector. A confirmation remains mandatory before replacing the active template. New acceptance assertions verify SETTINGS-only role discovery, active-module stability during preview, four-tab desktop consistency, and the SETUP-only micro selector.

# Rust-Boy | Micro handheld shell

## Universal sections

The bottom rail always has **STATUS / [active role] / DATA / MAP / RADIO**, in that order. The second tab uses the active role title from the Rust API; long titles truncate in the rail and remain available in the page heading. **⚙ Settings** in the header holds role selection, CRT effects, Reload and Custom pin editing. Settings does not take a sixth rail slot.

| Section | Modules and actions |
| --- | --- |
| Status | System Stat, System Diag and local task summary |
| Active role | Mission Log, Network Engineer, Byte Lab, Sim Bay, Survey GIS, Field Log and future specialist pins; Security Chief camera placeholders |
| Data | Journal, Archives, Game Link, Messenger and assigned duties |
| Map | Shared county/vector map, location and map-layer controls |
| Radio | Direct shared native Rust radio controls and output settings |

Each palette descriptor has a Rust-owned `section` field. Only pinned modules appear, each in one launcher category; shared Messenger, duties, Map and Radio remain available to every role. Unknown future section metadata defaults to the role deck. Browsing categories never starts a probe, playback, peer delivery or a hardware connection.

Role and Data decks show four tools below 260 CSS pixels of content height and six on larger screens. Status reserves space for two system tools and the task summary. The Custom palette shows two tools on the small screen and four otherwise. Settings role selection shows two profiles per page on screens at most 400 CSS pixels tall and four otherwise. Paging retains focus and staged selection. Resizing does not rebuild an open editor or active Radio view. Tool targets retain a 44px minimum height; accessibility overflow recovery remains available.

`node tools/test_micro_ui.cjs` covers section ordering, complete and exclusive module grouping, direct Radio mounting/cleanup, Settings access, paging and editor preservation with invented local fixtures. Rendered viewport and physical touch acceptance remain separate gates.

## Back navigation and temporary drafts

Back/Escape returns from a result to its inputs before leaving a tool. Network inputs return to the tool menu; the next Back exits the module. Local input drafts survive tab navigation and GIS-mode changes within this browser session, scoped by role/tool/mode. They are not written to disk and clear on reload. Completed local results focus EDIT INPUTS; editing focuses the first field. Detached requests cannot update the current status or show stale errors.

## Micro readability pass

**STATUS, [active role], DATA, MAP, RADIO** are the universal five-section labels. Settings is accessed from the header. The heading has one consistent Back control, including return from non-home tabs; Escape follows the same route. Head Engineer opens NETWORK, which presents the available operations rather than implying a scan has already started. Local tools show results separately with EDIT INPUTS preserving the entered values. Root pages retain overflow access at accessibility zoom.

## Unified Network Engineer

The Network Engineer launcher uses **four tools per handheld page** in a 2-column by 2-row deck, leaving space for the status strip and large navigation controls at 480×320. Desktop uses six tools per page. Every tool shows its purpose and whether it is an offline calculation or an authorized action. The status strip explicitly reads SESSION / STANDBY, REQUESTS / MANUAL ONLY and LAST RESULT / NOT LIVE; these are not continuous LAN telemetry. **PREV and NEXT are visible buttons with at least 84×48 CSS-pixel targets** (larger on coarse pointers) and a full-width page footer. All compact primary buttons have at least 44px height, with visible focus indicators. Keyboard-emulated D-pad Up/Down moves focus through tool tiles or visible form actions in reading order; Left/Right and PageUp/PageDown turn tool and result pages. Enter/Space use browser-native focused-button activation, with a high-contrast focus outline. The initial arrow press focuses a reachable control; page turns focus the first new tool tile. Held keys do not rapidly skip controls. In one-line fields, Left/Right preserves caret editing while Up/Down moves focus to the next field or action; multiline fields and native selectors keep arrow editing. Only keyboard focus, not an API request, changes on navigation. Neither paging method starts a scan or controller request. A new GitHub Actions real-Chromium fixture tests 480×320 and additional screen sizes against production micro/network styles, including visible paging, a reachable STOP REFRESH, and no unsolicited network requests. Real touchscreen/gloves, hardware-button/GPIO wiring and browser-zoom acceptance remain pending.

From `main`, select Head Engineer → NET ENGINEER. The shared Rust catalog has 16 operations across four handheld pages (six tools per desktop page). Select a tool, enter inputs, and use NEXT for active operations to reach a separate authorization screen. Offline calculations run directly. Results display two inventory entries per page and retain full text under DETAILS. SCAN LAN and UniFi operations offer 60-second refresh capped at 60 runs; edit/back/navigation/hidden tabs/errors stop it. Returning to the view requires another explicit Run. See [health and UniFi setup](NETWORK_HEALTH_UNIFI.md).

**Status:** functional local micro UI preview. The desktop dashboard remains available and unchanged except for a **MICRO** entry. The micro view shares the same Rust service, `PRESETS` registry, `PALETTE`, atomic `WorkbenchStore`, and allowlisted offline tool APIs. It is **not** a wearable hardware image, native RustGIS raster renderer, production GNSS service or live camera client. On the GIS feature branch, MAP can import native RustGIS vector maps and display an explicitly approved browser location.

### Browser viewport acceptance

`node tools/test_network_viewport.cjs` uses headless Chrome/Chromium with Node 22 and a loopback fixture. It measures the production Network Engineer component and production micro styles at 480×320 (menu and mocked authorized results), 800×480, and 320×568. Expected assertions include all four menu tiles visible without scroll at 480×320, touch page controls at least 84×48 CSS pixels, reachable pagers, and STOP REFRESH in the initial result viewport. A single synthetic POST is allowed only after the fixture explicitly completes the authorization form. This is not a field LAN/controller test and does not simulate a physical touchscreen or GPIO input.

## Launch on Mac

```bash
# From the feat/micro-compact-hud branch:
./scripts/launch-micro.command

# Or from Cargo:
cargo run --locked -p pipboy -- dashboard --micro --open
```

The Mac launcher opens **http://127.0.0.1:8765/micro** and stops when the Terminal process is interrupted (Ctrl+C). If the default port is busy, run `cargo run --locked -p pipboy -- dashboard --micro --port 8790 --open`. The desktop page is still **/** on the *same* local server; use the **DESK** link to return, or the desktop's **MICRO** link to switch back. On Raspberry Pi, omit `--open` and navigate to the local `/micro` URL from that Pi. The port binds **127.0.0.1 only**; it cannot be reached from another device over the LAN.

### Hardware-oriented layout

This is an independent phosphor-style renderer, not a collapsed desktop sidebar. Its main five sections use a fixed tab rail, short labels and a **no-scroll viewport** at 480×320. Tool grids and profile selection have visible Previous/Next pages; a short toast replaces the extra visible footer. Unusually long results or accessibility zoom may scroll in the detail view rather than conceal controls. CRT effects can be disabled. No original game artwork is used.

| Display | Intended adaptation | Current verification |
| --- | --- | --- |
| Desktop browser | Centered simulated handheld housing (up to 900×520) | Rust HTTP/assets and JS syntax tests |
| 800×480 | Compact five-section HUD, paged role/task selectors | Manual browser acceptance pending |
| 480×320 | Short primary screens without scroll; task list and tools paginated | Manual browser acceptance pending |
| 320px width/portrait | Reduced chrome, detail scroll fallback for accessibility | Manual browser acceptance pending |

No physical screen, Raspberry Pi graphics stack, battery sensor or refresh-rate measurement has yet been validated. **PWR is shown as "NOT DETECTED"**, not a fabricated charge percentage.

## Minimal screen contract

The five sections follow the category table above. All module editors open within their owning section and Back returns to its launcher. Map and Radio use the available page area. Security Chief cameras live in the role deck and show NO SIGNAL until an actual integration exists. Inputs/results remain separate screens; accessibility zoom restores overflow access when needed.

## Data Messenger

Handheld DATA → MESSENGER hosts a dedicated device-to-device message view for all non-Security-Chief profiles; Security Chief retains its four-camera fixture with a **MESSENGER / SSH** action. Peer selection, three-message paging, short UTF-8 text, explicit SEND/RETRY, and manual local REFRESH operate entirely through the loopback Rust API. The transport uses pinned OpenSSH and forced-command enrollment, not an Internet chat gateway. **Transport is encrypted; default journals are plaintext. Fresh journals may explicitly enroll age encryption with OS-backed keys. The protocol is not E2EE.** No messages are sent or fetched from peers by opening Messenger. See [Messenger security, pairing, and threat model](MESSENGER.md).

## Direct device task delivery

The Task Control detail now offers **SEND TO SELECTED DEVICE** only for explicitly enrolled OpenSSH peers. Once the destination durably saves the task, the manager sees its peer ID and stored receipt. Recipient operators can acknowledge and work on tasks in their own job HUD; the Overseer must deliberately request **REFRESH REMOTE STATUS** to view updates. This replaces the previous email-draft workflow; no background connections or task notifications are added. Pairing and host fingerprint verification are CLI/OS steps; see [peer setup](PEER_TASK_TRANSPORT.md).

## Overseer and role duty displays

On a new install, Overseer begins with **TASK CONTROL** pre-pinned. STAT summarizes actual locally recorded open/overdue/blocked tasks and provides direct access to the task board; the role section opens the management module. Creation, reassignment, manual handoff and acknowledgement take explicit clicks. The other job HUDs show their **assigned duties**, filtering by their job ID, rather than a generic management control panel. Older saved Overseer boards need an explicit profile reload to receive the new Task Control pin without resetting task data.

See [Task Control and manual dispatch](TASK_CONTROL.md). Neither the task HUD nor the screen's green styling implies live network delivery or authenticated operators.

## Operation

**Tabs:** STATUS, active role, DATA, MAP, RADIO. The header Settings button opens device configuration.

**Keyboard:** 1–5 selects the corresponding section when not editing; Escape closes a module, then returns to Status. Standard Tab/Shift+Tab and Enter/Space remain available. Network Engineer retains its internal directional navigation. Profile selection remains staged until LOAD ROLE.

**Profile state:** Selecting a job in the modal is only a preview. **LOAD ROLE** activates it and replaces only pinned module tiles. The change is persisted to the same workbench file used by desktop and TUI; no tools are executed and no extra permissions are granted. Custom begins empty, and only tools supported by the palette can be pinned. A documented "RESET CUSTOM" action clears Custom's pins after explicit confirmation.

**Active local tools:** NET ENGINEER performs real, explicitly authorized private-LAN ICMP/TCP sweeps (with detected-device counts, RTT and optional 60-second refresh while open). It never scans just because the user opens a role or tab. The same tool supports controller-connected client counts, UniFi site IDs and probe-versus-controller comparison when the server is explicitly configured with local TLS credentials. Other labs remain offline. See [network diagnostics](NETWORK_ENGINEERING.md).

**Active offline labs:** For a profile containing the relevant tool, tap **ROLE** → **SURVEY GIS**, **NET ENGINEER**, **BYTE LAB** or **SYSTEM STAT** (under STATUS). The result is computed locally when **RUN** is pressed, using the existing Rust `/api/experiment` endpoint. The browser never auto-fetches host telemetry. Notes/library/simulator/game display safe manual handoffs, not arbitrary command execution. Scout's MAP page can open **WAYPOINT** or **DISTANCE** directly.

**Security Chief fixture:** the role section opens four clearly disconnected camera tiles. Tap a tile to focus; tap it again, press BACK or Escape to return to the 2×2 wall. No stream URLs, snapshots, IP probing, video decoding, recording or camera sessions are present.

## Security and acceptance

The micro route `/micro` serves embedded HTML, CSS and JS through the existing Rust server. Its API calls use the exact same 127.0.0.1-only Host and Origin checks, bounded JSON requests, strict CSP and workbench state validation as the desktop UI. Micro does not store secrets or location; settings toggles remain only in page memory. Do not port-forward or publicly expose the development server.

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --locked
node --check crates/pipboy/assets/micro.js
node --check crates/pipboy/assets/tasks.js
python3 tools/smoke_micro.py
python3 tools/smoke_tasks.py
```

The new smoke script starts an actual Rust loopback server with `--micro --port 0`, confirms both desktop and micro routes, verifies the five-section markup, responsive breakpoints and no external JS URLs, then exercises profile persistence, Scout GIS, Custom pinning and denial of unauthorized requests. It is part of Linux and native macOS/Windows CI. **A passing static/HTTP test is not a visual certification.** Follow up by checking rendered browser screenshots and touch/keyboard usage at 800×480 and 480×320, including focus order, text legibility, scroll clipping, and modal operation.

**Next:** verify RustGIS vector import, GPS consent and online OSM reference at handheld size; native RustGIS raster rendering, hardware GNSS and TAK/camera transport remain separately gated.

## County-first offline cache, RustGIS import and GPS map

MAP mounts the shared [RustGIS-native map and GPS viewer](SCOUT_GPS_MAP.md), supporting explicit browser GPS permission, manual location selection, RustGIS/GeoJSON imports, source manifests, follow, accuracy, and pan/zoom. It starts on an unlocated grid, without an outbound request. **COUNTY** retrieves source-derived Census roads and water features only after the operator chooses a location and explicitly requests county data; cached, validated county geometry remains available offline. A separate **WEB ON** control allows bounded external reference queries, while saved-site privacy constraints remain in force. There is no bundled demonstration-city basemap, fictional geography, navigational certification, native RustGIS renderer, remote raster layer, or USB serial GNSS integration. [County data provenance](COUNTY_SOURCES.md) · [Lore naming](LORE_NAMING.md).

DATA → MESSENGER includes a compact [PACE route selector](PACE_MESSAGING.md) beside DEVICE.
Primary/Alternate send only on explicit actions; Contingency is disabled and
Emergency queues locally. Its banner states P/A share a LAN, C is receive-only
and E holds locally. History encryption labels follow the actual queue mode.


## Uniform role-independent shell (October 26, 2026)

The top bar is fixed RB / FIELD TERMINAL with a Settings button. Role selection remains under Settings → Change Profile. All roles share the same five sections; only their specialist pins change. Desktop keeps its existing navigation and shares profile storage with micro.

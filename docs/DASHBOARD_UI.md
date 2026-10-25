# Rust-Boy | Local graphical dashboard

**Status:** developer-preview shell running entirely from Rust-Boy on **127.0.0.1**, with a responsive original HTML/CSS/JavaScript frontend embedded in the Rust executable. This is a real working local web UI, **not** a public website, a packaged desktop binary, a RustGIS map renderer, an ATAK client or a released handheld OS.

## Dedicated handheld micro route

Use [micro handheld mode](MICRO_UI.md) when testing a real micro-display layout instead of shrinking the desktop workstation. Start `./scripts/launch-micro.command` or `cargo run --locked -p pipboy -- dashboard --micro --open` on Mac. The local server serves **/micro** alongside the existing **/** desktop route. Both use the exact same workbench API; changing a job template in either view is reflected in the other after reload.

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

## Head Engineer network tools

The NET SUITE desktop module has a Rust-provided operation picker and per-mode fields. Nine calculations stay offline. **TCP CHECK** requires a separate operator confirmation before invoking one bounded IP:port connection; it never enumerates ports or contacts hostnames. Tool details and output are presented in the same existing workbench pane. See [Network Engineering guide](NETWORK_ENGINEERING.md).

## Peer task dispatch

The desktop and micro Task Control components share the same local peer roster and task APIs. Device send/sync operations require both a configured SSH peer and a separate operator confirmation; no job selection makes a connection. The receiving Rust-Boy persists incoming assignments and reports their statuses only on explicit SSH synchronization. No email draft, automatic notifications or browser-initiated arbitrary network targets are present. See [peer security and setup](PEER_TASK_TRANSPORT.md).

## Overseer task management

The desktop Overseer Module Viewport now hosts the same task board as the handheld interface, with create/assign/status/reassign, overdue counts and manual handoff/acknowledgment. Task data is separate from profile settings, so switching roles never erases tasks. Other job screens show their locally assigned queue. The clipboard/email-draft workflow is **operator-controlled** and does not imply actual delivery; authenticated remote synchronization is a future gate. See [Task Control](TASK_CONTROL.md).

## User journey

1. **Job templates:** The left rail contains **seven preconfigured occupations**, followed by **Custom**. Click a role (or use **Alt+Up/Down**) to preview it without modifying the saved workbench.
2. **Load station:** Click **Load station**, then explicitly confirm replacement. All displayed module tiles come from the exact Rust `PRESETS` and `PALETTE` definitions, not an independently maintained browser inventory. The selection is persisted in `PIPBOY_HOME/workbench.json`; without an existing file, the default is Overseer. No device connection occurs when selecting or loading.
3. **Module deck:** Click a module to inspect its availability, scope and safe command. Implemented offline experiments include **Scout GIS** (great-circle distance or GeoJSON waypoint), **NET LAB** (IPv4/CIDR arithmetic), **BYTE LAB** (hex dump) and **HOST STAT** (one-shot local metrics). Other modules provide honest manual CLI handoffs. Gameplay and hardware are never auto-started.
4. **Custom last:** Load Custom to get a **blank** workspace. Add supported local/simulated/explicit-permission tools from the palette and remove pins. Planned hardware modules are disabled and cannot be pinned.
5. **Module viewport:** Reclamation Scout has an **empty grid viewport labeled RustGIS not attached**. Security Chief has a **four-camera interaction fixture**: click a tile to focus and click again or press Escape to return. There are no fake live frames, no camera endpoints, no feeds, no recording and no map-position sampling.

## Interaction and layout

The original design uses charcoal and phosphor-green cues but **no proprietary Fallout graphics, fonts or UI code**. The browser supports mouse, keyboard and touch. Its CSS adapts from a wide three-pane desktop to a compact vertically scrollable handheld layout; reference targets are **800×480** and **480×320**. These are **responsive design targets**, not a verified 4-inch screen, GPU or physical Pi acceptance result.

- **Alt+Up/Down:** preview previous/next job profile.
- **Alt+Enter:** start the profile-replacement confirmation.
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

# Rust-Boy | Micro handheld shell

**Status:** functional local micro UI preview. The desktop dashboard remains available and unchanged except for a **MICRO** entry. The micro view shares the same Rust service, `PRESETS` registry, `PALETTE`, atomic `WorkbenchStore`, and allowlisted offline tool APIs. It is **not** a wearable hardware image, a working RustGIS navigation viewer or a live camera client.

## Launch on Mac

```bash
# From the feat/micro-handheld-shell branch:
./scripts/launch-micro.command

# Or from Cargo:
cargo run --locked -p pipboy -- dashboard --micro --open
```

The Mac launcher opens **http://127.0.0.1:8765/micro** and stops when the Terminal process is interrupted (Ctrl+C). If the default port is busy, run `cargo run --locked -p pipboy -- dashboard --micro --port 8790 --open`. The desktop page is still **/** on the *same* local server; use the **DESK** link to return, or the desktop's **MICRO** link to switch back. On Raspberry Pi, omit `--open` and navigate to the local `/micro` URL from that Pi. The port binds **127.0.0.1 only**; it cannot be reached from another device over the LAN.

### Hardware-oriented layout

This is an intentionally separate HTML/CSS/JS shell, not a compressed desktop sidebar. It uses an original green-phosphor field terminal treatment: a top role/status ribbon, large one-screen viewport with its own scroll region, a fixed six-tab rail and a compact privacy/status footer. A static, low-cost scanline overlay and restrained glow can be disabled under CONF; the system accessibility contrast and reduced-motion settings are respected. No official Pip-Boy graphics, textures, fonts or assets are embedded.

| Display | Intended adaptation | Current verification |
| --- | --- | --- |
| Desktop browser | Centered simulated handheld housing (up to 900×520) | Rust HTTP/assets and JS syntax tests |
| 800×480 | Edge-to-edge micro terminal, fixed tabs | CSS breakpoint and manual browser check required |
| 480×320 | Smaller top bands, compact controls, scrollable viewport | CSS breakpoint and manual browser check required |
| 320px width/portrait | No horizontal page scrolling; simplified status | CSS breakpoint and manual check required |

No physical screen, Raspberry Pi graphics stack, battery sensor or refresh-rate measurement has yet been validated. **PWR is shown as "NOT DETECTED"**, not a fabricated charge percentage.

## Overseer and role duty displays

On a new install, Overseer begins with **TASK CONTROL** pre-pinned. STAT summarizes actual locally recorded open/overdue/blocked tasks and provides direct access to the task board; GEAR opens the same management module. Creation, reassignment, manual handoff and acknowledgement take explicit clicks. The other job HUDs show their **assigned duties**, filtering by their job ID, rather than a generic management control panel. Older saved Overseer boards need an explicit profile reload to receive the new Task Control pin without resetting task data.

See [Task Control and manual dispatch](TASK_CONTROL.md). Neither the task HUD nor the screen's green styling implies live network delivery or authenticated operators.

## Operation

**Tabs** (large bottom controls): **STAT** current profile/local status and shortcuts; **GEAR** pins and tool launcher; **MAP** reserved RustGIS port with functioning offline GIS/waypoint/distance lab when pinned; **DATA** local reference and research tools; **LINK** disconnected transport summary, or Security Chief's four-camera mock interaction; **CONF** role switch, CRT effects toggle, reload and Custom tool palette.

**Keyboard:** 1–6 jumps directly to pages when not editing a text field; **Escape** exits a focused module or camera, then returns to STAT. Standard Tab/Shift+Tab focus and Enter/Space activate controls. Security Chief LINK uses **1–4** for camera placeholder focus; Escape returns to the four-tile grid. The role-switch control in the header opens a scrollable modal of all seven prebuilt job templates with **Custom last**.

**Profile state:** Selecting a job in the modal is only a preview. **CONFIRM REPLACE** activates it and replaces only pinned module tiles. The change is persisted to the same workbench file used by desktop and TUI; no tools are executed and no extra permissions are granted. Custom begins empty, and only tools supported by the palette can be pinned. A documented "RESET CUSTOM" action clears Custom's pins after explicit confirmation.

**Active offline labs:** For a profile containing the relevant tool, tap **GEAR** → **SCOUT GIS**, **NET LAB**, **BYTE LAB** or **HOST STAT**. The result is computed locally when **RUN LOCAL TOOL** is pressed, using the existing Rust `/api/experiment` endpoint. The browser never auto-fetches host telemetry. Notes/library/simulator/game display safe manual handoffs, not arbitrary command execution. Scout's MAP page can open **WAYPOINT** or **DISTANCE** directly.

**Security Chief fixture:** LINK displays four clearly disconnected camera tiles. Tap a tile to focus; tap it again, press BACK or Escape to return to the 2×2 wall. No stream URLs, snapshots, IP probing, video decoding, recording or camera sessions are present.

## Security and acceptance

The micro route `/micro` serves embedded HTML, CSS and JS through the existing Rust server. Its API calls use the exact same 127.0.0.1-only Host and Origin checks, bounded JSON requests, strict CSP and workbench state validation as the desktop UI. Micro does not store secrets or location; settings toggles remain only in page memory. Do not port-forward or publicly expose the development server.

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --locked
node --check crates/pipboy/assets/micro.js
python3 tools/smoke_micro.py
```

The new smoke script starts an actual Rust loopback server with `--micro --port 0`, confirms both desktop and micro routes, verifies the six-page markup, responsive breakpoints and no external JS URLs, then exercises profile persistence, Scout GIS, Custom pinning and denial of unauthorized requests. It is part of Linux and native macOS/Windows CI. **A passing static/HTTP test is not a visual certification.** Follow up by checking rendered browser screenshots and touch/keyboard usage at 800×480 and 480×320, including focus order, text legibility, scroll clipping, and modal operation.

**Next:** after micro layout acceptance, expose a real RustGIS process/IPC or embedded mapping renderer in Reclamation Scout; then introduce separately approved GNSS/TAK and camera transports. None are implied by the current mock viewports.

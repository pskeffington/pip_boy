# Changelog

## 2026-10-25 — Persistent PACE delivery progress

- Persist tracked attempt counts, last P/A route and exchange/receipt outcomes inside the existing message journal, including its opt-in age encryption. Preserve local holds across restart.
- Commit verified receipt status and the successful attempt together. Interrupted or unconfirmed exchanges remain queued with delivery unknown; retries retain the original envelope and receiver deduplication identity.
- Serialize cooperative outbound attempts/holds with a separate nonblocking dispatch lock; inbound journal writes remain available. Add read-only `peer message-status ID ALIAS --json`.
- Display wait reasons and last route in desktop/handheld LINK, reload local evidence after retry errors and disable unavailable retry actions. Existing journals load without fabricated attempt history; older binaries cannot read records after progress metadata is written.
- Extend encrypted restart, interruption, locking, forged/lost receipt, CLI, browser and two-store relay validation. Physical hardware, native key lifecycle and independent review gates remain open.

## 2026-10-25 — Boston real street base and lore-aligned names (feature validation pending)

- Pin 10,634 true City Boston SAM street segments and 47,994 original WGS84 source vertices for a local central-Boston MAP. No fictional feature creation; show coverage and source credit.
- Render source lines under separate imported GIS and Reclamation markers in an original phosphor palette. Add BOS OFF/ON, source manifest and offline default without external tiles or automatic reachability ping.
- Rename display tools to MISSION LOG, SYSTEM STAT, ARCHIVES, JOURNAL, SIM BAY, SYSTEM DIAG, FIELD LOG and SURVEY GIS; set micro tab labels STAT/GEAR/MAP/DATA/LINK/SETUP without renaming stable IDs.
- Add provenance/rebuild tool, real-only map and naming documents, tests and HTTP smoke checks. Field validation, data release licensing and hardware acceptance remain open.


## 2026-10-25 — Reclamation map-marker integration (validation pending)

- Add opt-in **MAP → SITES** overlay and focus from site records on micro and desktop without basemap requirement.
- Introduce minimal, gated local marker endpoint (omits private notes), manual-only provenance and six-item accessible lists.
- Protect saved-site coordinates from OSM extent requests by disabling/blocking WEB on site views; clear marker memory on map unmount.
- Extend API smoke and browser map journeys; repair stale Scout profile assertion and apply Rust formatting.


## Unreleased — micro workflow continuity

- Back/Escape moves from local results to preserved inputs and from network results/authorization to inputs, then tools.
- Preserve local lab drafts in memory per role/tool/GIS mode; focus Edit after completion and the input on return. Ignore detached local responses and errors. Drafts clear on reload.

## Unreleased — clear micro navigation

- Use plain HOME / TOOLS / SETUP labels, a header Back control, quieter phosphor styling and larger readable tool labels.
- Remove duplicated home-role headings and empty lab output; separate local results from preserved input forms with EDIT INPUTS.

## Unreleased — unified Network Engineer and mainline polish (2026-10-25)

- Merge LAN discovery, read-only UniFi client inventory/reconciliation and DIAG ingestion to `main` through PR #15 after all five quality jobs passed at `badabc5`.
- Consolidate nine offline network calculators, one TCP check, bounded ICMP/TCP health and four LAN/UniFi actions into a 16-operation Rust catalog. Preserve `workbench net cidr` and existing scan/controller CLI routes.
- Share the desktop/micro Network Engineer component. Split handheld inputs, authorization and results; page inventories without truncating the structured result. Place NET ENGINEER first in the Head Engineer template.
- Own one refresh timer per mounted view: 60 seconds, maximum 60 runs, immediate cancellation on edit/navigation/hidden tabs/errors, and stale-response suppression. Cancelling a fetch does not revoke an already accepted bounded server operation.
- Replace obsolete README branch-switch instructions with a single `main` setup and current capability table. Keep physical/controller/game acceptance separate from merged preview software.
- Add controlled loopback CLI/API health checks and dependency-free UI journeys for authorization, paging, timer ownership, visibility and safe labels. No dependency or credential changes.

This file records notable changes to Rust-Boy. This repository is in **development preview**; the Rust workspace version `0.1.0` is not a published production release. Dates are in UTC unless otherwise stated.

## Unreleased — UniFi read-only controller reconciliation (2026-10-25)

- Add an optional, explicitly configured UniFi Network local Integration API GET-only adapter (TLS-verified, private IPv4 controller). List sites and paginated connected clients using a server-only API key delivered through curl stdin rather than process arguments or the browser.
- Add `net unifi sites`, `net unifi clients SITE`, `net unifi compare SITE CIDR` with explicit operator confirmation. Desktop and micro NET ENGINEER get the same three actions and show controller-connected counts separately from probe responders.
- Add response bounds, UUID validation, paging consistency checks, no redirects, no insecure TLS mode and reproducible mocked-API tests. UniFi hardware acceptance and CI remain release gates.

## Unreleased — real NET ENGINEER LAN inspection (2026-10-25)

- Add explicitly authorized private IPv4 /24..../32 ICMP echo and bounded TCP connectivity sweeps with distinct responders, ARP neighbor-cache sightings, observed-device counts, probe-specific RTT and short-interval delta.
- Enable the Head Engineer/Security Chief NET ENGINEER tool on both desktop and micro HUDs, with a confirmed Scan button and opt-in 60-second refresh while the view remains open. Add CLI scan/watch; keep the original offline CIDR planner.
- Retain local-only dashboard transport, no arbitrary shell, no Internet scanning, no credential collection; probing occurs only on deliberate operator action. Device totals are lower bounds, not controller inventories.
- [Network engineering guide](docs/NETWORK_ENGINEERING.md); live hardware and cross-platform CI gates remain to be verified before a release claim.

## Unreleased — minimalist micro HUD (2026-10-25)

- Reduce the handheld STAT page to one short role heading, action buttons and live task counters; remove redundant descriptions and decorative instrument blocks.
- Keep fixed six-tab navigation with no scrolling on normal 480×320 root pages. Paginate role selection, GEAR/DATA tools and Custom palette rather than silently clipping content.
- Use a separate compact Overseer Task Control: two-task pages, staged New Task entry, tabbed details, explicit peer-device send and status synchronization. Desktop task workflow remains unchanged.
- Offline lab input and results are separate views. Long outputs and unusual accessibility/zoom sizes may scroll within detail views to keep controls reachable.
- Add smoke checks for new concise disconnected-camera and map labels. Manual Pi and Mac browser visual acceptance is still open.

## Unreleased — authenticated peer task transfer prototype (2026-10-25)

- Replace the Overseer's email-draft controls with a **paired Rust-Boy device selector**, explicit **SEND TO DEVICE** confirmation, stored-delivery receipt, receiver-side acknowledgment and manual progress refresh.
- Build an optional OpenSSH peer backend: private `peers.json` roster, explicit `peer init/trust/add/list/send/sync` CLI, bounded typed assignment/receipt protocol and restricted SSH forced-command bridge; task GETs and job loads never connect.
- Add idempotent receiver task import, separate device IDs, origin/delivery/remote-progress fields, conflict protection and CI-tested two-home local SSH relay (not a real SSH encryption test).
- Add [device-to-device setup and security guide](docs/PEER_TASK_TRANSPORT.md); no new listening port, wireless radio transport, push notification or auto-pairing. True physical Mac-to-Pi hardware testing remains open.

## Unreleased — Overseer Task Control and operational job HUDs (2026-10-25)

- Preload `mission-board` / **TASK CONTROL** as the first Overseer module. Role-specific micro STAT screens replace the generic station page; other occupational HUDs have locally assigned task queues.
- Implement a local task store with role/person assignments, priority, due date, status, manual handoff/sent/acknowledgment fields, bounded event history, atomic writes, schema validation and revision-based conflict rejection.
- Add shared desktop/micro task board: create, manage, filter, update, reassign, copy an assignment and explicitly open an email draft. No automatic sending or authenticated remote receipt; no network delivery code.
- Add task API routes, [Task Control operator contract](docs/TASK_CONTROL.md), and cross-platform local HTTP smoke tests for ten task assignments, role filtering, persistence and privacy.
- Future work: authenticated recipient identities, secure transport and actual cross-device synchronization; UI/manual browser acceptance remains separately gated.

## Unreleased — dedicated micro handheld shell (2026-10-25)

- Add `pipboy dashboard --micro [--open]` and `/micro` with embedded `micro.html`, `micro.css` and `micro.js`, running inside the existing loopback-only Rust dashboard; the desktop route and API are preserved.
- Original compact phosphor-green field screen with optional static scanline/glow effects, top status ribbon, one scrolling viewport, and fixed six-tab thumb-sized navigation, targeting 480×320 and 800×480. No third-party artwork, game code, fonts or remote assets.
- Seven ready-made job templates remain sourced from Rust; Custom is last and starts empty. Explicitly confirmed profile switching, Custom pin/unpin, offline GIS waypoint/distance, CIDR and hex tools are available from the handheld UI.
- Scout map area is a **RustGIS not attached** reference grid; Security Chief has a disconnected four-camera focus/back layout prototype. Neither represents a live map or camera stream.
- Add Mac double-click launcher `scripts/launch-micro.command`, [micro guide](docs/MICRO_UI.md), JS syntax checks and actual loopback HTTP smoke tests for Linux/macOS/Windows CI. Manual 4-inch browser and physical Raspberry Pi tests remain separate release gates.

## Unreleased — local graphical dashboard shell (2026-10-25)

- Add a **Rust-served localhost-only dashboard** with original responsive field-console styling, desktop/4-inch reference layouts, and no new frontend or network-service dependencies.
- Load the exact `PRESETS` and `PALETTE` from Rust; seven preloaded occupational profiles are followed by blank Custom. Role previews are non-mutating; actual replacements are confirmed and persisted through the existing WorkbenchStore.
- Provide interactive, allowlisted offline tool experiments (Scout WGS84 waypoint and distance; IPv4/CIDR; hex inspection; one-shot local host reports), plus clearly marked mock-layout/placeholder ports for real RustGIS and IP-camera integrations.
- Provide a Mac double-click launcher, [dashboard operator guide](docs/DASHBOARD_UI.md), and real loopback HTTP smoke acceptance in Linux/macOS/Windows CI. The service requires strict Host/Origin/JSON request rules and deliberately exposes no arbitrary shell execution.
- This is a **development UI**, not a packaged native desktop app or a working TAK, camera, GNSS, game or RustGIS graphical integration. Exact-head CI and physical handheld/browser acceptance are separately gated.

## Unreleased — Rust 1.90 minimum compiler upgrade (2026-10-25)

- Raise the workspace minimum supported Rust compiler from **1.81** to **1.90** across all three inheriting crates; CI tests Rust **1.90.0** and current stable, plus native macOS/Windows.
- Retain Rust edition **2021**, workspace resolver **2**, current Cargo.lock and the `stable` development toolchain. Edition 2024 dependencies are supported without migrating this application; no dependency/lockfile re-resolution is introduced by this policy change.
- Remove the earlier RustGIS MSRV blocker. Direct RustGIS source, a native ARM64 renderer and TAK interoperability are still separate, unshipped milestones.
- Add version-policy validation to prevent metadata, CI and install documentation drift. Fix the CI override so the 1.90.0 job actually runs that toolchain (older 1.81-labeled jobs inadvertently ran stable), and explicitly assert the compiler version.

## Unreleased — Reclamation Scout GIS / Pi-TAK foundation (2026-10-25)

- Add an offline `scout geo` CLI with bounded RFC 7946 Point/Feature/FeatureCollection inspection, WGS84 waypoint export, and spherical distance calculations. No network/GNSS access is performed by the module.
- Make GIS Lab a ready, pre-pinned Reclamation Scout tool; preserve the other roles, offline defaults and explicit permissions.
- Add a researched ARM64 RustGIS compatibility plan and Raspberry Pi TAK integration roadmap. ATAK is Android software; OpenTAKServer runs as separate, operator-managed Raspberry Pi software. No automatic TAK/CoT session, server installation or third-party RustGIS code is bundled.
- Add exact-SHA smoke and unit acceptance gates; report them as pending until CI completes.

## Unreleased — workbench role roadmap (2026-10-25)

- Document eight Vault-Tec-inspired operational profiles with exact existing palette mapping, real-world requirements, four R0–R3 acceptance gates each, formal source provenance and a consistent distinction between canonical occupational names and Rust-Boy adaptations.
- Define the Security Chief four-feed 2×2 camera wall and one-press focus/back UX, safe manual camera import, ONVIF Profile T / RTSP interoperability plans and required credential, privacy and actual hardware test gates. **Video is not yet implemented.**
- Add machine-readable `catalog/role_roadmap.json` and `tools/validate_roles.py` to detect divergence between docs and Rust profile definitions in CI.
- **Status:** the workbench branch passed the full cross-platform quality matrix at `d7ee200`; this new Scout branch must pass independently.

## Unreleased — main-branch foundation (2026-10-25)

### Available

- Game browser with separate navigation/value panes, visible long-list selection, preserved parent context, bounded scrolling, resize recovery, and replacement/removal tracking for opened game objects. Adds pure render/state coverage and a full inventory PTY/ConPTY journey; handles Windows peer-close aborts only at the final close boundary.

- Game diagnostics require a valid language/version announcement and complete database; later split updates no longer inherit the startup timeout. The Mac launcher supports prompted diagnostics with clean JSON, with live-update/disconnect and launcher PTY regression coverage.

- Mac → PlayStation focus: executable Finder/Terminal launcher, `--playstation IP` preset, protocol-aware `--diagnose`, console-specific connection advice and native macOS launcher test from a path containing spaces. Live console acceptance remains pending.

- Opt-in Fallout 4 read-only companion browser, bounded binary state decoder and incremental updates, explicit TCP connection, heartbeat, game snapshot output, and native terminal/mock-peer tests. Actual game compatibility and gameplay controls remain unverified/unimplemented.

- TUI Ctrl+C keyboard exit uses normal terminal cleanup, including resize fallback; native PTY/ConPTY interrupt regressions. External OS signals remain a separate acceptance gate.

- Field-level host units, quality and reason codes in `status --details [--json]`, finite `sample --details` and the console. Explicit CPU warm-up/early-poll absence, per-field stale propagation and poll-start timing; collector initialization now requests only CPU/RAM, with no process inventory.
- Rust-Boy-branded native `pipboy` CLI and Ratatui TUI, with keyboard navigation, small-screen fallback and local host diagnostics.
- Bundled, license-aware source library; offline plaintext notes with search, export/import, versioned schema and cooperative writer protection.
- Finite NDJSON host sampling, in-memory observation buffers, pause/resume, and permission-aware module lifecycle.
- Adapter preference commands that record **intent only**; no physical permission or device access.
- Fixed synthetic device fixtures, deterministic replay, bounded virtual event transport and typed mock input/display/serial interfaces.
- Standalone `rust-boy-broker` deny-only request process with a strict one-shot JSON protocol; **all** physical access requests remain denied.
- Source/evidence catalogs, documented G0–G4 development roadmap and cross-platform GitHub Actions quality checks.

### Validation

- GitHub Actions covers Ubuntu stable and Rust 1.81.0, native macOS terminal PTY, native Windows ConPTY, and offline broker/adapter simulations.
- The broker implementation and final branch documentation passed all five CI jobs on [PR #1](https://github.com/pskeffington/pip_boy/pull/1). See [validation evidence](docs/VALIDATION.md).

### Not yet implemented or validated

- Privileged, authenticated and OS-enforced device authorization; real GPIO, serial, BLE, SDR, GNSS, radio, mesh or network adapters.
- Physical hardware integration, device hot-unplug and trusted real-time revoke/watchdog behavior.
- Release installers/packages, assistive-technology and broader accessibility certification, full persistence failure-injection/restore guarantees.

## Project identity

Public name: **Rust-Boy**. Repository slug: `pip_boy`. Main CLI: `pipboy`. Denial-only binary: `rust-boy-broker`. The project is independent and unofficial and does not bundle third-party game artwork or proprietary firmware.

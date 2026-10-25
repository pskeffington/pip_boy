# Changelog

This file records notable changes to Rust-Boy. This repository is in **development preview**; the Rust workspace version `0.1.0` is not a published production release. Dates are in UTC unless otherwise stated.

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

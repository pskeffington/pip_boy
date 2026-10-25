# Changelog

This file records notable changes to Rust-Boy. This repository is in **development preview**; the Rust workspace version `0.1.0` is not a published production release. Dates are in UTC unless otherwise stated.

## Unreleased — main-branch foundation (2026-10-25)

### Available

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

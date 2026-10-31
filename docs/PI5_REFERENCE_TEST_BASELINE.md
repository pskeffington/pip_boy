# Raspberry Pi 5 reference hardware acceptance — 2026-10-31

**Engineering decision:** Raspberry Pi 5 is the primary physical reference test platform for Rust-Boy's handheld/field deployment. macOS and Windows remain development and compatibility targets, but do not replace Pi 5 release acceptance.

## Test baseline to record on each device

Record board model/revision, RAM size, bootloader/firmware, OS image and kernel, architecture (`aarch64` expected for native 64-bit), Rust version, display resolution and interface, touchscreen/input controller, power source, storage type, cooling and attached radio/GNSS/camera/audio equipment. Do **not** assume RAM, display, HAT or peripheral model from board identity. Keep reproducible build and test logs with commit SHA, timestamps and device identifier free of secrets.

## Phased gates

| Gate | Acceptance on physical Pi 5 |
| --- | --- |
| PI5-00 inventory | Capture platform, OS, kernel, RAM, power, storage, display/input and attached devices; document recovery image and rollback |
| PI5-01 native boot | Clean ARM64 build/package install; offline boot, restart, graceful stop, logs, persistence and bounded resource use |
| PI5-02 UI | 480x320 and 800x480 **when supported by test display**; one foreground package, visible Settings, keyboard/touch focus, no root scroll, large targets and accessible contrast/text |
| PI5-03 MAP | Offline county cache, safe ZIP/GPS acquisition, viewport coverage, bad-data rollback, county transitions, memory and render latency |
| PI5-04 RADIO | Actual IP playback/output selection; stop, lost-stream recovery, unplugging and optional SDR tested only on attached authorized hardware |
| PI5-05 COMMS | Denied-by-default network discovery, explicit authenticated pairing, local-IP loss/rejoin, receipt/unknown delivery and offline queue recovery |
| PI5-06 host modules | Process isolation and supervised adapter start/stop/crash/restart; permission denial, manifest signature/provenance and one foreground owner |
| PI5-07 soak | Repeatable long-duration offline/on-network runs, power interruptions, thermal throttling, storage wear considerations and observability |
| PI5-08 signoff | Exact tested commit and image hash, CI job evidence, physical pass/fail record, known limitations, reproducible deployment and rollback |

## Minimum pass criteria

No emergency/camera/medical claims without device-specific acceptance. Physical outcomes are never inferred from QEMU, cross-build, CI, localhost Mac tests or synthetic fixtures. Native Pi 5 results must be recorded separately and linked in the release matrix. For constrained-memory devices prioritize bounded allocations and peak RSS, startup/idle CPU, thermals, battery/power-loss recovery and network partition behavior; choose thresholds after measuring the actual board configuration.

## Codex instructions

Software agents should build ARM64-ready code, deterministic Pi 5 test scripts and offline fixtures in GitHub without waiting for a locally attached Pi. If real Pi 5 access is unavailable, mark field gates `pending_hardware`, never passed. Preserve platform-independent core logic, explicit device broker permission boundaries, and portable logging. Platform-specific fixes belong behind tested target cfg/feature boundaries. Do not turn on hardware probing during UI initialization or tests.

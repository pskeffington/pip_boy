# Raspberry Pi 5 reference hardware acceptance — 2026-10-31

**Current test bench:** Raspberry Pi 5 with **full-sized HDMI monitor**, keyboard and mouse. See [RB-PI5-REF-A bill of materials](PI5_RECOMMENDED_HARDWARE.md). A 5-inch touchscreen is optional future hardware, not part of the present test prerequisite.

**Engineering decision:** Raspberry Pi 5 is the primary physical reference test platform for Rust-Boy's handheld/field deployment. macOS and Windows remain development and compatibility targets, but do not replace Pi 5 release acceptance.

## Test baseline to record on each device

Record board model/revision, RAM size, bootloader/firmware, OS image and kernel, architecture (`aarch64` expected for native 64-bit), Rust version, display resolution and interface, touchscreen/input controller, power source, storage type, cooling and attached radio/GNSS/camera/audio equipment. Do **not** assume RAM, display, HAT or peripheral model from board identity. Keep reproducible build and test logs with commit SHA, timestamps and device identifier free of secrets.

## Phased gates

| Gate | Acceptance on physical Pi 5 |
| --- | --- |
| PI5-00 inventory | Capture platform, OS, kernel, RAM, power, storage, display/input and attached devices; document recovery image and rollback |
| PI5-01 native boot | Clean ARM64 build/package install; offline boot, restart, graceful stop, logs, persistence and bounded resource use |
| PI5-02A UI — current | Full-sized HDMI monitor at recorded actual resolution; one foreground package, visible Settings, keyboard/mouse, no root scroll, map canvas and accessible contrast/text |
| PI5-02B UI — later | Physical touch at 480x320/800x480 if available on target device, plus eventual DSI touchscreen native resolution; touch focus, hit targets and handheld ergonomics. **Pending until touchscreen attached** |
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

## Additional available inventory — model verification required

The engineering inventory also includes **one reportedly Raspberry Pi 4** and **one device referred to as "Pi 3T"**. Treat both model labels as **unverified until hardware inspection**; do not assume that "3T" is an official Raspberry Pi model. Record the exact model reported by `cat /proc/device-tree/model`, `uname -m`, `cat /etc/os-release`, `free -h`, power adapter and attached peripherals. Preserve raw results without credentials or host-identifying details.

| Test tier | Equipment | Intended role | Status |
| --- | --- | --- | --- |
| Reference | Pi 5 + full-sized HDMI monitor | Production-target ARM64 functional and performance acceptance | Planned reference; physical results pending |
| Compatibility A | Reported Pi 4, RAM/OS unspecified | Build/startup, desktop UI, networking, GIS cache, radio, lower-resource regressions | Available per inventory; exact model not verified |
| Compatibility B | Reported \"Pi 3T\", identity unspecified | Conservative smoke tests, graceful unsupported-feature handling, minimal-footprint profiling | Available per inventory; verify model and 32/64-bit OS before setting targets |

**Policy:** Never downgrade the Pi 5 reference release gate based on successful Pi 4 or older-device testing. A failed older-board test should be classified as a compatibility finding unless that board is explicitly designated a supported release target. Run bounded tests without assuming native SDR, camera, NVMe, physical touch, sufficient RAM or hardware decoding. Explicitly capture timeouts, memory usage and thermal throttling rather than weakening core security checks. Pi 4/3T may be used immediately for software/bench evaluation while Pi 5 provisioning remains open.

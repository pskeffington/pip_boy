# Pi 5 reference hardware configuration — 2026-10-31

Engineering recommendation, not hardware already purchased or validated. The [PI5 acceptance gates](PI5_REFERENCE_TEST_BASELINE.md) govern real-device release evidence.

## Primary reference bench: RB-PI5-REF-A

| Component | Default | Rationale |
| --- | --- | --- |
| SBC | Raspberry Pi 5, 8 GB RAM | Baseline for field host, GIS, radio and comms; test 4 GB as lower-memory compatibility target later |
| OS | Raspberry Pi OS 64-bit current stable; pin exact image/build | ARM64 reproducibility; record kernel, firmware and image hash |
| Boot/recovery | 64 GB high-endurance microSD | Recovery and independently reproducible provisioning; do not assume SSD boot without configuration |
| Runtime storage | 256 GB NVMe SSD via compatible M.2 HAT+ | Local map cache, logs, packages and tests; benchmark storage and graceful power loss |
| Cooling | Official Raspberry Pi 5 Active Cooler | Continuous GIS and native process workload; record CPU temperature and throttling |
| PSU | Official 27 W USB-C 5.1 V / 5 A | Avoid peripheral current limiting; verify powered hubs for high-draw SDR or SSD |
| **Current primary display** | **Full-sized HDMI monitor**, resolution/refresh recorded at test time | Current Raspberry Pi 5 development and functional verification; keyboard and mouse |
| Later handheld display | Optional 5-inch Touch Display 2 (720x1280 native, landscape rotation subject to validation) | **Deferred** touchscreen/mobile acceptance; not required for the present test bench |
| Controls | Momentary navigation/Back/Settings buttons through safe GPIO input adapter or USB keypad | Real D-pad and glove-friendly controls, with electrical isolation and tested debounce |
| Audio | Simple powered speaker / USB audio output | Native IP radio, route and output tests |
| Network | Onboard Wi-Fi plus wired Gigabit Ethernet | Deterministic offline/link-loss tests; separate controlled lab WLAN |
| Case | Ventilated enclosure compatible with DSI, HAT+, cooler, cables and selected display | Prevent shorts, ensure access and thermal performance |

## Expansion bench: RB-PI5-REF-B

Second Pi 5 (or independently configured peer) on isolated lab network for paired SSH, authenticated device-to-device messaging, unconfirmed-delivery recovery and partition/rejoin cases. Optional receive-only USB SDR, GNSS receiver and approved Pi/USB camera are **separate** test add-ons; camera permission does not follow automatically from pairing. Use powered USB hubs where a peripheral exceeds port budgets. Do not select specific SDR, GNSS or battery model without electrical/interface verification.

## Bring-up order

1. Record board RAM/revision, PSU, SSD/HAT, DSI cable, case clearances and OS image hash.
2. Boot from recovery microSD, enable firmware-supported SSD boot deliberately, validate the NVMe filesystem and rollback boot media.
3. Verify ARM64 build and persistent local data home; capture CPU/RAM, idle/load thermals, power stability and logs.
4. **Current bench:** connect Pi 5 to a full-sized HDMI monitor with keyboard and mouse. Verify native desktop/field shell at actual HDMI resolution, Settings, one foreground module, map canvas, input focus, radio, and terminal workflows. Record screen dimensions. Retain automated/synthetic 480x320/800x480 regressions without calling them physical touch acceptance.
5. **Later stage:** test the optional 5-inch DSI touchscreen, touch/back navigation and physical handheld ergonomics as separately pending PI5-02 field acceptance.
6. Test approved native radio/IP stream and output-failure behavior; then add SDR/GNSS/camera as permission-gated incremental components.
7. Bring up isolated second peer and test link loss, replay/denial, unknown receipt, local queue and recovery. No automatic discovery or internet exposure.
8. Run restart/power-loss tests, storage corruption recovery, soak and documented operator acceptance.

## Codex acceptance instructions

Maintain hardware profile RB-PI5-REF-A and independent RB-PI5-REF-B fixtures. Build standalone `scripts/pi5/` support incrementally only after inventorying existing scripts and without claiming they exist today. Prefer immutable manifests, deterministic smoke tests, a structured machine-readable result with exact Git SHA, OS/kernel/firmware, tests, temperature/CPU/memory and device evidence. Use QEMU/cross-compile for preflight only; mark PI5-00..08 pending_hardware until physical evidence is attached.

## Primary documentation references

- https://www.raspberrypi.com/products/raspberry-pi-5/
- https://www.raspberrypi.com/documentation/computers/raspberry-pi.html
- https://www.raspberrypi.com/documentation/accessories/touch-display-2.html

Hardware component quantities, RAM and SSD size are recommended starting points, not production performance guarantees.

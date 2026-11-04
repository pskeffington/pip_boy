# Raspberry Pi 5 HDMI software smoke checklist

Status: **procedure only**. No Pi 5 was detected or tested while preparing this checklist. All physical results remain **PENDING** until recorded from the actual reference board.

This procedure supports the Pi 5 full-sized HDMI monitor, keyboard and mouse bench (PI5-02A). It does not cover the deferred touchscreen gate (PI5-02B), production release approval, or Pi 4 / unidentified Pi 3 compatibility. The detailed [Pi 5 acceptance baseline](https://github.com/pskeffington/pip_boy/blob/docs/longterm-codex-roadmap-20261031/docs/PI5_REFERENCE_TEST_BASELINE.md) and [hardware inventory](https://github.com/pskeffington/pip_boy/blob/docs/longterm-codex-roadmap-20261031/catalog/hardware_test_inventory.json) are under review in [PR #89](https://github.com/pskeffington/pip_boy/pull/89); they are not yet part of the current main checkout.

## Boundaries

- Use the Pi 5 as the release reference. A passing Mac/Windows run, ARM cross-build, emulator, or browser fixture is software evidence only.
- Pi 4 and unidentified Pi 3 devices are compatibility-only and cannot satisfy a Pi 5 gate.
- Do not probe peripherals or open network, audio, SDR, camera, GNSS, location, messaging, or package operations as part of this smoke.
- Keep host networking on loopback. Do not expose the dashboard on a LAN.
- Do not record serial numbers, MAC/IP addresses, credentials, private paths, or user data in evidence.
- Stop the UI with Ctrl+C. Do not cut power during this smoke.

## Software preflight

Run these from a clean checkout on a normal development host or the Pi. Record the Git SHA and each command's exit status. A passing preflight does not satisfy any physical gate.

~~~sh
git rev-parse HEAD
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo +1.90.0 check --workspace --locked
python3 tools/validate_host_packages.py
~~~

If the hardware inventory from its separate integration change is present, also run:

~~~sh
python3 tools/validate_hardware_inventory.py
python3 tools/test_hardware_inventory.py
~~~

Record GitHub Actions evidence separately: workflow URL, exact source SHA, actual job names and conclusions. startup_failure with zero jobs is blocked CI, not a pass. Do not label a preflight result as Pi acceptance.

## Pi 5 bench procedure

1. Confirm the physical label says Raspberry Pi 5 and connect the full-sized HDMI monitor, keyboard, mouse, stable power supply and intended storage. Record only the details in the evidence template below.
2. On the Pi, record the OS and kernel identity:

   ~~~sh
   tr -d '\0' </proc/device-tree/model
   printf '\n'
   uname -m
   uname -r
   cat /etc/os-release
   free -h
   ~~~

   These commands document the platform; they do not prove any application gate. Omit device-unique identifiers from copied logs.
3. From the checked-out source tree, start the loopback-only micro shell:

   ~~~sh
   cargo run --locked -p pipboy -- dashboard --micro --port 0
   ~~~

   Record the printed loopback URL and open it in a browser on the same Pi. Confirm the server reports 127.0.0.1; stop if it binds to a non-loopback address.
4. On the actual HDMI display, inspect the rendered page at its reported resolution. Confirm one foreground workspace, visible host Settings, usable keyboard and mouse focus, no root-page scrolling, and readable text. Do not activate MAP, RADIO or another package during this smoke.
5. In a second terminal, run the full-screen TUI:

   ~~~sh
   cargo run --locked -p pipboy -- tui
   ~~~

   Confirm Settings remains reachable. On Workbench, open the slash command palette, move through the listed offline commands, press Escape and confirm the palette closes without displaying a command result. Reopen it and run only a harmless registered offline/read-only command. Confirm a rejected or unknown command remains denied. Stop with Ctrl+C and verify the shell returns normally.
6. Stop the dashboard with Ctrl+C. Record the observed result and any failure verbatim after redacting device-unique values. Do not infer power-loss, long-soak, thermal, networking, audio, or peripheral behavior from this short smoke.

If no graphical browser or local display is available, record PI5-02A as **NOT RUN** with the reason. A successful build or server startup alone is not a UI pass.

## Evidence record

Store one record per physical run. Keep raw logs local if they could contain device-unique values; attach only reviewed, redacted evidence.

~~~json
{
  "schema_version": 1,
  "run_id": "operator-assigned opaque id",
  "recorded_at_utc": "YYYY-MM-DDTHH:MM:SSZ",
  "profile_id": "rb-pi5-ref-a",
  "physical_device_present": true,
  "device_model_observed": "Raspberry Pi 5",
  "identity_verified": false,
  "architecture": "operator-recorded uname -m",
  "os_release": "distribution and version only",
  "kernel_release": "operator-recorded uname -r",
  "ram_total": "operator-recorded total; no serial or unique id",
  "display": {
    "connection": "full-sized HDMI",
    "resolution": "width x height"
  },
  "input": ["keyboard", "mouse"],
  "source_sha": "40-character Git commit SHA",
  "ci": {
    "workflow_url": "https://github.com/owner/repo/actions/runs/00000000000",
    "job_conclusions": []
  },
  "software_preflight": {
    "cargo_fmt": "PASS | FAIL | NOT RUN",
    "cargo_clippy": "PASS | FAIL | NOT RUN",
    "cargo_test": "PASS | FAIL | NOT RUN",
    "rust_1_90_check": "PASS | FAIL | NOT RUN",
    "host_package_validator": "PASS | FAIL | NOT RUN"
  },
  "physical_gates": {
    "PI5-00_inventory": "PENDING | PASS | FAIL | NOT RUN",
    "PI5-01_native_boot": "PENDING | PASS | FAIL | NOT RUN",
    "PI5-02A_hdmi_ui": "PENDING | PASS | FAIL | NOT RUN"
  },
  "observations": [],
  "redaction_reviewed": false
}
~~~

Use **PENDING** until an operator records the physical result. For each PASS or FAIL, include the observation, timestamp and link to reviewed evidence. Never set identity_verified or a physical gate from CI, a cross-build, or a simulated device.

## Not covered

PI5-02B touch ergonomics; native package installation; real MAP data/GNSS; radio or audio output; SDR; camera; COMMS pairing/delivery; power-loss recovery; thermal/resource soak; process isolation; and production signoff remain separate gates.

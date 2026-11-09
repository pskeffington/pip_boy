# Raspberry Pi 5 HDMI acceptance procedure

Status: **procedure only**. No Pi 5 was detected or tested while preparing this document. Every physical result stays **PENDING** until an operator records it from the actual reference board.

This runbook covers the Pi 5 bench with a full-sized HDMI display, keyboard, and mouse (PI5-02A). Touchscreen ergonomics, Pi 4/other-board compatibility, signing/notarization, and production release are separate gates.

## Safety and evidence boundaries

- A Mac/Windows run, ARM cross-build, emulator, browser fixture, or synthetic adapter is software evidence only.
- Do not install discovered/reference packages. Listing, help, selection, or palette browsing must not start a package or grant authority.
- Keep the dashboard bound to loopback. Do not expose it on a LAN.
- This smoke uses the static read-only RADIO station listing only. Do not probe station hosts, enumerate SDRs, play audio, scan networks, access GNSS/camera, send messages, or upload location.
- Test offline behavior with the bench isolated from external network access. Do not change shared network infrastructure to create that condition.
- Never record serial numbers, MAC/IP addresses, credentials, private paths, or user data. Keep raw logs local if they could contain identifiers; attach reviewed, redacted evidence only.
- Stop applications with Ctrl+C. Do not cut power during this smoke.

## Software preflight

Run from a clean source checkout on the Pi or an authorized development host. Record the exact Git SHA, command, exit code, and relevant output summary. These checks do not satisfy physical acceptance.

```sh
git rev-parse HEAD
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo +1.90.0 check --workspace --locked
python3 tools/validate_host_packages.py
```

Record GitHub Actions separately: workflow URL, exact source SHA, and actual job names/conclusions. `startup_failure` with zero jobs is blocked CI, not a pass.

## Pi 5 bench procedure

### 1. Record the physical bench

Confirm the board is physically labeled Raspberry Pi 5; attach full-sized HDMI, keyboard, mouse, stable power, and intended storage. Record only non-unique platform details. If the model cannot be verified or the device is unavailable, record `NOT RUN` and the reason; do not substitute another device.

On the Pi, platform details may be collected with:

```sh
tr -d '\0' </proc/device-tree/model
printf '\n'
uname -m
uname -r
cat /etc/os-release
free -h
```

Remove device-unique identifiers and user data before sharing any output.

### 2. Check package trust and loading behavior

```sh
cargo run --locked -p pipboy -- packages list --json
```

Confirm reference entries report `installed_count: 0` and `reference_designs_only`. Do not run `packages scan`, `packages check`, or an installer as part of this smoke. In the application, inspect the available/pinned toolsets and select only an already available host-owned view. Verify that opening a listing or changing selection does not start a package or produce permission grants. If a trusted installed package is required for a particular check but none is present, record `NOT AVAILABLE`; do not treat a reference manifest as installed.

### 3. Verify startup, single foreground view, and Settings

Start the desktop shell on an ephemeral port:

```sh
cargo run --locked -p pipboy -- dashboard --port 0
```

Open the printed URL in a browser on the Pi. Confirm the server binds to `127.0.0.1`, the view shows one foreground package, and host-owned Settings is reachable. Use the physical HDMI display at its reported resolution; verify keyboard and mouse focus and check that the root page does not scroll to expose hidden sections. Stop the server with Ctrl+C, restart it once, and confirm a clean exit both times. If the printed bind address is not loopback, stop and record a failure.

### 4. Verify terminal dispatch without side effects

In a second terminal:

```sh
cargo run --locked -p pipboy -- tui
```

Confirm Settings remains reachable. In Workbench, open the command palette with `/`; inspect the displayed registered commands, then press Escape and confirm that no command ran and no result was produced. Select only an already pinned offline/read-only host tool and run a harmless registered status/diagnostic command. Try one unknown command and confirm it is denied. Do not pin a tool to imply permission. Stop with Ctrl+C and confirm the terminal returns normally.

If the expected tool or handler is unavailable, record `NOT AVAILABLE` or `NOT RUN` with the exact reason; do not substitute simulated success.

### 5. Verify offline RADIO listing and fallback

With the bench isolated from external network access, run:

```sh
cargo run --locked -p pipboy -- radio stations
```

Confirm it displays the checked-in curated station metadata without network probing, SDR enumeration, playback, or a “connected” claim. Record the output category, not private station/user data. If any network request or audio output starts, stop the application and record a failure. This verifies static listing behavior only; it is not radio reception or playback acceptance.

### 6. Verify restart and adapter fault recovery

Run the deterministic adapter lifecycle regression on the Pi if the source/toolchain is available:

```sh
cargo test --locked -p pipboy-core universal_adapter::tests::failed_suspend_detaches_only_the_failing_foreground_package
cargo test --locked -p pipboy-core universal_adapter::tests::failed_rollback_clears_stale_view
```

Record these as **software fixture results**, never physical-adapter results. If an approved physical adapter is actually installed and a separate device-specific procedure authorizes its use, capture a separate record for intentional fault and recovery testing; otherwise mark physical adapter recovery `NOT RUN`. Confirm Settings remains available after fixture/app shutdown and that no stale foreground view is reported. Do not connect or probe hardware merely to complete this checklist.

## Evidence record template

Create one record per actual physical run. Use `PENDING`, `PASS`, `FAIL`, `NOT RUN`, or `NOT AVAILABLE` exactly; do not turn CI or simulation into a physical pass.

```json
{
  "schema_version": 1,
  "run_id": "operator-assigned opaque id",
  "recorded_at_utc": "YYYY-MM-DDTHH:MM:SSZ",
  "physical_device_present": false,
  "device_model_observed": "PENDING",
  "architecture": "PENDING",
  "os_release": "PENDING",
  "kernel_release": "PENDING",
  "ram_total": "PENDING",
  "display": {"connection": "full-sized HDMI", "resolution": "PENDING"},
  "input": ["keyboard", "mouse"],
  "source_sha": "PENDING",
  "ci": {"workflow_url": "PENDING", "job_conclusions": []},
  "checks": {
    "software_preflight": "PENDING",
    "loopback_startup_and_restart": "PENDING",
    "package_reference_not_installed": "PENDING",
    "single_foreground_and_settings": "PENDING",
    "terminal_dispatch_and_unknown_denial": "PENDING",
    "offline_radio_listing": "PENDING",
    "adapter_fault_software_fixture": "PENDING",
    "adapter_fault_physical": "NOT RUN"
  },
  "observations": [],
  "redaction_reviewed": false
}
```

For every `PASS` or `FAIL`, include the observed result, timestamp, exact source SHA, and a link to reviewed, redacted evidence. Leave physical results `PENDING` until recorded on the Pi itself.

## Not covered by this smoke

Touchscreen acceptance, native package installation, live MAP data/GNSS, radio reception or audio playback, SDR, camera, COMMS pairing/delivery, network transport, power-loss recovery, thermal/resource soak, sustained process isolation, signing/notarization, and production signoff remain separate gates.

# Native Linux launcher and Nemesis Operator handoff

**Status: implementation candidate; runtime, graphical and hardware acceptance pending.**

## Native launch contract

The existing `pipboy dashboard --port 8765` is the only backend. It listens on 127.0.0.1 only; the new read-only `/api/healthz` response reports an exact local service identity without creating user data. Existing Host and Origin controls remain in force.

The XDG desktop entry runs `/usr/local/bin/rust-boy-launcher` from the KDE menu. That launcher starts the current user's `rust-boy-dashboard.service`, verifies the exact health identity directly (bypassing proxy configuration), then opens the dashboard in the default browser. `rust-boy-launcher --micro` selects the micro page. No new renderer, executable package activation or network listener is added.

The unit is installed but **not enabled**, and startup is on demand rather than at boot or login. It is a systemd **user** service with least-privilege restrictions, not a root service. A desktop session is required. Shutdown: `systemctl --user stop rust-boy-dashboard.service`. A port conflict, missing systemd user session or wrong readiness response causes a launch failure rather than opening an unrelated server.

## Build and stage in an authorized Linux builder

Run `cargo build --release --locked -p pipboy`, create a new isolated staging root, then run `bash scripts/stage-linux-native.sh target/release/pipboy /absolute/staging/root`. The script requires an existing canonical absolute staging root other than / (including paths resolving to /), rejects symlinked paths, refuses to overwrite existing managed files, and requires `readelf` (binutils) to confirm that the staged executable is an ELF matching the native Linux machine architecture (`x86_64` or `aarch64`). It stages the existing first-party LICENSE and records the native binary SHA-256. It does not install to the workstation, enable a unit, start a browser, fetch software or alter disk partitions.

The package payload consists of:

- `/usr/local/bin/pipboy` — compiled native application.
- `/usr/local/bin/rust-boy-launcher` — on-demand Linux opener.
- `/usr/lib/systemd/user/rust-boy-dashboard.service` — disabled user unit.
- `/usr/share/applications/org.rustboy.RustBoy.desktop` — XDG desktop entry.
- `/usr/share/licenses/rust-boy/LICENSE` — existing project license.
- `/usr/share/rust-boy/native-binary.sha256` — staged binary checksum.

## Nemesis Operator integration (not yet implemented in Nemesis)

Pin the **exact reviewed Rust-Boy source SHA**, inspect its MIT license and Cargo.lock dependencies, compile x86_64 in a **non-root** isolated builder, then stage the six paths in only the `mac-intel-operator` edition using its existing separate Operator ISO pipeline. Verify source/binary hashes, executable modes, disabled user-unit status, absence of Rust-Boy XDG autostart, and presence of all payload files within the actual SquashFS. Do not alter the generic rescue or Air field profiles; do not replace the existing Airplanes.live background. Source tests and Docker ISO inspection are not graphical boots.

For Pi 5/aarch64, build separately for the correct architecture and verify ELF/linkage. Do not ship the x86_64 Operator binary to Pi or enable SDR, serial, camera, GNSS, networking scans, or package adapters implicitly.

## Required gates and ownership

- **LIN-N01 — Linux host and QA:** format, check, warnings-denied Clippy, locked workspace tests, `python3 tools/test_linux_native.py`, `bash -n scripts/stage-linux-native.sh`, existing dashboard/micro CI, and actual exact-head GitHub Actions jobs.
- **LIN-N02 — Linux Operations / Nemesis:** pinned source build, deterministic standalone rootfs staging, signed Arch package resolution and Operator-only ISO payload assertions; no service auto-enablement.
- **LIN-N03 — Linux Operations:** independently authorized KDE/Wayland VM graphical launch, user-service stop/restart, full/micro keyboard and Settings focus, offline fallback and no LAN bind.
- **LIN-N04 — hardware QA:** Pi 5 full-sized HDMI physical acceptance using `PI5_CODEX_SMOKE.md`; aarch64 build, power/thermal and peripherals are separate gates.

Foundation (Mac) coordinates source pins and cross-platform architecture; Linux Operations owns Linux packaging, systemd session, Arch/Nemesis integration and deployment evidence. No acceptance of physical hardware or production security is inferred from source or ISO assembly.

**Rollback:** remove the staged package in a disposable image or stop/disable the user unit and revert the dedicated package-layer change. Never modify existing operator files, source branches or host configurations to force a deployment.

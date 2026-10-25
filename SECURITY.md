# Rust-Boy | Security and privacy

Rust-Boy is an experimental offline-first field systems console, not a safety-of-life product.

## Default permissions

- No telemetry uploads, analytics endpoints, automatic network scans, tracking, or background device access.
- The STAT panel reads host status through the sysinfo crate.
- The catalog is bundled at build time; viewing it does not connect to GitHub.
- Explicit finite host sampling emits NDJSON to stdout only; it does not automatically save data, transmit measurements, or access external hardware.
- Notes are created only through an explicit user command. They are stored in JSON under PIPBOY_HOME or the user's home directory.
- Notes are plaintext and must not contain passwords, tokens, sensitive medical records, or restricted information.

## Capability architecture

The in-process Rust `ModuleHost` provides a **best-effort policy precheck** for built-in trusted modules, not operating-system isolation. The static manifest cannot prevent malicious linked code from invoking OS functions. Hardware operations will require a separate device broker with explicit user consent and scoped handles before those adapters can be enabled. See [docs/OBJECT_MODEL.md](docs/OBJECT_MODEL.md).

## Future devices

Location, serial ports, GPIO, SDR/radio, BLE, Wi-Fi, and mesh transports require separate adapters, permission gates, simulator tests, and user-facing indicators. Do not silently initiate scans, pair devices, transmit, or modify settings. No interference, jamming, credential interception, or covert collection features.

## Third-party materials

Never import source or media solely because it is visible on GitHub. Capture repo URL, exact revision, license and notice requirements, provenance, and any asset-specific terms before reuse. Missing/ambiguous licensing means reference-only.

## Reports

File a private security report using GitHub's security advisory feature when available. Avoid publishing credentials, location traces, or sensitive sample files in issues.

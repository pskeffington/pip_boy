# Rust-Boy | Security and privacy

Rust-Boy is an experimental offline-first field systems console, not a safety-of-life product.

## Default permissions

- No telemetry uploads, analytics endpoints, automatic network scans, tracking, or background device access.
- The STAT panel reads host status through the sysinfo crate.
- The catalog is bundled at build time; viewing it does not connect to GitHub.
- Explicit finite host sampling emits NDJSON to stdout only; it does not automatically save data, transmit measurements, or access external hardware.
- Notes are created only through an explicit user command. They are stored in JSON under PIPBOY_HOME or the user's home directory.
- Notes are plaintext and must not contain passwords, tokens, sensitive medical records, or restricted information.
- Saved adapter preferences are separate from active permissions; `requested` does not open a device, access the network or authorize a receiver/transmitter.

## Capability architecture

The in-process Rust `ModuleHost` provides **lifecycle and capability policy checks** for built-in trusted modules, not operating-system isolation. The static manifest cannot prevent malicious linked code from invoking OS functions. Hardware operations will require a separate device broker with explicit user consent and scoped handles before those adapters can be enabled. See [docs/OBJECT_MODEL.md](docs/OBJECT_MODEL.md).

## G1 simulated broker (no real authorization)

`SimulatedBroker` and `pipboy simulate` operate only on 11 fixed, in-memory `sim:*` fixtures. A session requires `--confirm` in the CLI, and a saved `denied` preference always wins. Requested intent never grants a session. The simulation emits `simulated: true` and no device readings; cross-broker and revoked leases are invalid. Fixed `simulate replay` scenarios use synthetic logical milliseconds and record explicit stale-token rejection following virtual disconnect/recovery or revocation; they do not use physical clocks or recorded signals. This **cooperative in-process harness is not an OS privilege broker**, and `--confirm` must never be reused to open real devices. Real trusted consent, OS handle isolation and physical adapters remain planned. See [simulation contract](docs/SIMULATION.md).

## Future devices

Location, serial ports, GPIO, SDR/radio, BLE, Wi-Fi, and mesh transports require separate adapters, permission gates, simulator tests, and user-facing indicators. Do not silently initiate scans, pair devices, transmit, or modify settings. No interference, jamming, credential interception, or covert collection features.

## Third-party materials

Never import source or media solely because it is visible on GitHub. Capture repo URL, exact revision, license and notice requirements, provenance, and any asset-specific terms before reuse. Missing/ambiguous licensing means reference-only.

## Reports

File a private security report using GitHub's security advisory feature when available. Avoid publishing credentials, location traces, or sensitive sample files in issues.

Local host telemetry starts only after the host checks its read grant. TUI P revokes that grant immediately, drops the collector/history, and clears the displayed reading. Resume grants host read access for the current session and explicitly restarts collection. Grants are not persisted. STAT is lifecycle-managed, while trusted LIBRARY and NOTES operations use scoped `run_local` prechecks. `permissions.json` saves only future adapter intent; it is never loaded as active grants, and there is no OS broker or actual external-adapter authorization yet. See [adapter preferences](docs/PERMISSIONS.md) and [local operation controls](docs/LOCAL_OPERATIONS.md).

Notes writers use a cooperative OS lock. Readers/search are side-effect free. Schema checks reject future versions, extra fields, duplicate/zero IDs, control characters and oversized data before rewriting. Ordinary staging errors preserve the current document. Locking does not constrain external editors or untrusted code; do not delete notes.lock while any writer may be active. See [storage limits and recovery](docs/STORAGE.md).

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

Virtual lease cancellation and deadlines are only in-memory mock behavior. The broker accepts manually injected logical milliseconds rather than wall-clock time, and backward ticks invalidate the active mock lease. Neither operation is an OS-enforced device watchdog or protection for physical transports.

The bounded `VirtualTransport` queue only holds synthetic `sim:*` events; explicit CLI `--confirm` is for a single simulation and is never persisted as a device grant. The current in-process queue checks broker session state before reads and clears data after mock cancellation or revocation, but this **does not** protect against untrusted native code, enforce OS permissions, or implement encrypted/authenticated communications. A physical transport needs a separate broker, secure peer identity and real-world timeout/revocation semantics.

The typed mock input/display/serial adapters also remain in-memory only: they reject missing confirmation or saved denials, return explicit simulated records and never measure a GPIO pin, render to a physical screen, or read serial hardware. The mock display rejects control bytes/escape sequences. The serial payload is an explicitly injected fixture, not information received over a port. No real adapter authorization, encrypted channel or OS isolation is implied by these trait interfaces. See [typed adapter contract](docs/MOCK_ADAPTERS.md).

## Deny-all physical-request process (new, no hardware access)

`crates/rust-boy-broker` is now a separately built binary that reads **one** at-most-1,024-byte JSON request on standard input only when launched with `--stdio-once`. Its v1 protocol recognizes only the reviewed 11 capability/operation pairs. All responses explicitly say `allowed:false`, including a well-formed request with `operator_intent:true`; no user-controlled string, saved preference, token or identifier is authoritative. The executable has no driver, scanner, socket, device handle, privileged startup, permissions writer or persistent grant. This is a deny-by-default **protocol/process scaffold**, not an independently enforced OS authorization boundary. See [protocol](docs/BROKER_PROTOCOL.md).

## Actual device-authorization boundary (not implemented)

The project now specifies [G2 OS device broker requirements](docs/OS_DEVICE_BROKER.md): explicit device identity, trusted per-operation consent, a separately enforced OS permission boundary, ephemeral scoped handles, actual deadline/close-on-revoke behavior, and fail-closed hot-unplug and process error handling. **None of those features are enabled merely because their design is written.** Windows ConPTY validation exercises only terminal behavior and cannot grant peripheral access.

## Future devices

Location, serial ports, GPIO, SDR/radio, BLE, Wi-Fi, and mesh transports require separate adapters, permission gates, simulator tests, and user-facing indicators. Do not silently initiate scans, pair devices, transmit, or modify settings. No interference, jamming, credential interception, or covert collection features.

## Third-party materials

Never import source or media solely because it is visible on GitHub. Capture repo URL, exact revision, license and notice requirements, provenance, and any asset-specific terms before reuse. Missing/ambiguous licensing means reference-only.

## Reports

File a private security report using GitHub's security advisory feature when available. Avoid publishing credentials, location traces, or sensitive sample files in issues.

Local host telemetry starts only after the host checks its read grant. TUI P revokes that grant immediately, drops the collector/history, and clears the displayed reading. Resume grants host read access for the current session and explicitly restarts collection. Grants are not persisted. STAT is lifecycle-managed, while trusted LIBRARY and NOTES operations use scoped `run_local` prechecks. `permissions.json` saves only future adapter intent; it is never loaded as active grants, and there is no OS broker or actual external-adapter authorization yet. See [adapter preferences](docs/PERMISSIONS.md) and [local operation controls](docs/LOCAL_OPERATIONS.md).

Notes writers use a cooperative OS lock. Readers/search are side-effect free. Schema checks reject future versions, extra fields, duplicate/zero IDs, control characters and oversized data before rewriting. Ordinary staging errors preserve the current document. Locking does not constrain external editors or untrusted code; do not delete notes.lock while any writer may be active. See [storage limits and recovery](docs/STORAGE.md).

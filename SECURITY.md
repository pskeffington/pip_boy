# Security and privacy

Rust-Boy is an **experimental local field console**, not a safety-of-life, security-monitoring or production hardware-control system. The public code is MIT-licensed; that does not imply a warranty or security certification.

## What the current version does

- **Offline by default.** STAT uses local host telemetry; LIBRARY reads a bundled catalog. No account, analytics, telemetry upload, device scan or background network service is required.
- **Local notes.** NOTES stores plaintext JSON under `PIPBOY_HOME` or the platform's data directory. Do **not** store secrets, private medical records, credentials or protected field logs there. Writers use cooperative locks and atomic-replacement techniques, but external edits, power loss and untrusted local code are outside those guarantees.
- **Preference is not permission.** `permissions.json` holds `requested` or `denied` intent. Neither it nor `simulate --confirm` grants a physical device capability.
- **Simulators are synthetic.** Fixed `sim:*` fixtures, typed mock input/display/serial adapters and bounded virtual queues do not open real devices, capture live radio data, contact peers or transmit signals.
- **The separate broker denies everything.** `rust-boy-broker --stdio-once` validates one bounded JSON request and always returns `allowed:false`. It has no hardware backend, device credentials, privileged helper or authenticated IPC endpoint.

The `ModuleHost` capability checks and mock leases are cooperative **in-process** controls; they are **not** an OS sandbox for untrusted native code. A separate deny-only process is likewise **not** a working OS permission broker.

## Trust boundaries and planned hardware

| Boundary | Current protection | Not yet available |
| --- | --- | --- |
| Local telemetry | Explicit in-process host-read grant; finite sampling | Hostile-process isolation |
| Notes and preferences | Bounded schema validation, side-effect-free reads, cooperative write locking | Encrypted storage or protection against compromised users/processes |
| Simulated adapters | Explicit mock confirmation, saved-denial precedence, revocation/expiry fixtures | Real device I/O or OS-enforced grants |
| Physical broker | Strict, one-shot, deny-only request parser | Trusted caller identity, scoped OS handles, privilege separation, real timeouts/revocation |

**Before enabling a physical adapter**, Rust-Boy needs reviewed platform-specific device identity, a trusted authorization interface, independently enforced operating-system permissions, a minimal broker IPC protocol, hard timeouts, handle cleanup on revoke/crash/hot-unplug, and denial-path tests on supported devices. No GPIO, SDR, GNSS, serial, mesh, Bluetooth, radio or network interface can be enabled by the current broker.

Read the detailed [OS authorization threat model](docs/OS_DEVICE_BROKER.md), [deny-only protocol](docs/BROKER_PROTOCOL.md), [adapter preference contract](docs/PERMISSIONS.md) and [test evidence](docs/VALIDATION.md).

## Reporting security issues

Use [GitHub's private vulnerability reporting / security advisory](https://github.com/pskeffington/pip_boy/security/advisories) when available. If private reporting is unavailable, contact the repository maintainer through a suitable private channel. Do not put credentials, exploit artifacts containing personal data, location traces or sensitive disclosure details in public issues.

Please include the affected revision, platform, reproduction steps and security impact. No response-time or coordinated-disclosure service level is promised.

## Third-party code and assets

A research-catalog link is not a license grant. Review source licenses, dependency provenance, hardware rights and media restrictions before importing material. Do not add copyrighted game artwork, unlicensed firmware, live geolocation traces or secrets to this repository.

## Explicit game companion connection

`pipboy game --connect IP:PORT --confirm` authorizes one direct plaintext TCP connection to the specified Fallout 4 host. It transmits heartbeat responses only; game actions are not implemented. No discovery, DNS, automatic retry or saved state is used. Treat the endpoint as untrusted and use a trusted local network. See [game protocol bounds](docs/GAME_MODE.md). This opt-in path does not enable physical device adapters.

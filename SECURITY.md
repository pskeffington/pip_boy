# Security and privacy

Rust-Boy is an **experimental local field console**, not a safety-of-life, security-monitoring or production hardware-control system. The public code is MIT-licensed; that does not imply a warranty or security certification.

## What the current version does

- **Offline by default.** STAT uses local host telemetry; LIBRARY reads a bundled catalog. No account, analytics, telemetry upload, device scan or background network service is required.
- **Local notes.** NOTES stores plaintext JSON under `PIPBOY_HOME` or the platform's data directory. Do **not** store secrets, private medical records, credentials or protected field logs there. Writers use cooperative locks and atomic-replacement techniques, but external edits, power loss and untrusted local code are outside those guarantees.
- **Preference is not permission.** `permissions.json` holds `requested` or `denied` intent. Neither it nor `simulate --confirm` grants a physical device capability.
- **Simulators are synthetic.** Fixed `sim:*` fixtures, typed mock input/display/serial adapters and bounded virtual queues do not open real devices, capture live radio data, contact peers or transmit signals.
- **The separate broker denies everything.** `rust-boy-broker --stdio-once` validates one bounded JSON request and always returns `allowed:false`. It has no hardware backend, device credentials, privileged helper or authenticated IPC endpoint.

The `ModuleHost` capability checks and mock leases are cooperative **in-process** controls; they are **not** an OS sandbox for untrusted native code. A separate deny-only process is likewise **not** a working OS permission broker.

## Tasks, assignments and manual dispatch

Task Control persists unencrypted `tasks.json` under `PIPBOY_HOME`, separate from notes and workbench preferences. The local task store uses bounded input, explicit schema validation, a cooperative file lock and atomic replacement on accepted writes. On Unix the file is private to the account (`0600`); this **does not protect against other processes running as the same user**. Do not record sensitive patient details, security credentials, private locations or other regulated material.

An assignment copied to clipboard or opened as an email draft has **not been sent or delivered by Rust-Boy**. The separate "Mark sent" and "Record acknowledgment" actions are manual declarations and **not authenticated receipts**. The current operational role selector controls which tasks are shown and edited in the UI but **is not user login or access control**; a local user can change profile. No outbound mail, webhook, LAN sync, TAK transport or automatic task notification is implemented. A future multiuser task network needs authenticated identities, secure transport, encrypted storage/authorization, acknowledgments and replay protection. See [task contract](docs/TASK_CONTROL.md).

## Micro display and device boundary

The `/micro` view and `dashboard --micro` launch switch only the UI route; they **do not** change the Rust service's loopback binding, expose the app on LAN, or enable any new endpoint or permission. Both UIs use the same strict JSON Host/Origin/CSP rules, atomic profile store and allowlisted Rust experiments. Role changes require a visible **CONFIRM REPLACE** action. The micro CRT effect toggle is cosmetic and remains in memory only.

The Security Chief 2×2 tiles are **UI-only disconnected camera placeholders**, and Scout's crosshair grid is **not** a GNSS fix or RustGIS navigation display. The micro status must never fabricate battery charge, position, camera health or radio state. Real camera/GNSS/TAK integration requires separate protected transport and hardware testing. See [micro operator guide](docs/MICRO_UI.md).

## Local graphical dashboard security

The optional `pipboy dashboard` command creates a **local-only HTTP listener bound to 127.0.0.1**, serving embedded original HTML/CSS/JS and strictly allowlisted JSON routes. It does not bind to LAN interfaces. Browser requests have an exact Host requirement; changing saved workbench state requires a same-origin JSON POST from `http://127.0.0.1:<port>`, explicit confirmation for role replacement and bounded input (8 KiB headers, 4 KiB bodies). The response uses restrictive CSP, `nosniff`, frame denial, no CORS and no-store. No arbitrary commands, general file browsing, exposed note contents, hidden streams or credentials are part of this service.

**Limitations:** same-origin headers and loopback binding protect against common remote web attacks, but are not a security sandbox against other local applications or a compromised user account. Do not proxy, port-forward or expose this development interface publicly. A remote device/web deployment needs authenticated sessions, stronger CSRF protection, authorization and an independent review. The browser's Scout viewport is a *not-connected* reference grid and the Security Chief camera fixture does not receive video. See [dashboard guide](docs/DASHBOARD_UI.md).

## Occupational profiles and future camera sources

Profiles are declarative UI configurations only. Choosing Overseer, Security Chief, or Custom cannot grant any capability, run user code, discover a device, open a network endpoint or authenticate to a camera. The new R0 profile code is still subject to CI acceptance. The workbench state is a local JSON description of pinned tools, **not** an authorization token.

The requested Security Chief **four-camera wall** is still an [integration plan](docs/SECURITY_CAMERA_WALL.md); the dashboard has a purely local four-tile focus/back layout fixture, not working IP-camera streams. Live IP-camera integration must use explicit allowlisted endpoints, reviewed JPEG/MJPEG/RTSP decoders, credential storage outside plaintext JSON (macOS Keychain when available), TLS validation and per-feed stream limits. Disabling a feed must release resources. No background video recording, audio capture, automatic ONVIF discovery, default-password attempts, PTZ control or upload is permitted by profile activation. Retained frames must be clearly marked stale, never live.

## Scout GIS and future TAK bridge

Scout GIS is **offline and operator-invoked**: WGS84 waypoint/distance and bounded read-only GeoJSON Point inspection. It does not sample the user's position, request GNSS, scan local networks, or transmit CoT events. GeoJSON files explicitly exported by the operator contain plaintext coordinates; keep them private and use appropriate access controls. Do not add telemetry collection or persistent automatic location history by simply selecting Reclamation Scout.

TAK integration is not currently implemented. A future Raspberry Pi OpenTAKServer connector needs explicit destination approval, verified TLS identity/certificates, client/source authorization, message TTL and replay checks, rate limits, log redaction, and immediate disconnect on revocation. It must not accept or rebroadcast untrusted position/identity claims as authenticated personnel. ATAK-CIV is Android software, not a native Pi OS application. See [Scout GIS and TAK-on-Pi](docs/SCOUT_GIS_TAK.md).

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

# Security and privacy

## Public-repository leakage audit (October 26, 2026)

See the [evidence-ranked review](docs/SECURITY_REVIEW_2026-10-26.md) for findings, mitigations, residual risks and acceptance gates. The public checkout now rejects known operator journals, private credential files, strong token signatures and unreviewed external-process constructors during CI. The scan is intentionally offline and reports **file names, line numbers and categories only**, never matched secret values. It is a preventive control for the checked-out tree, **not proof that no secrets exist**, a review of Git history, credential rotation or a dependency advisory scan.

External tools used by the native console (SSH, curl, ping, ARP and browser opening) are constructed with a shared helper that strips recognized API/cloud tokens from each subprocess environment. This prevents unintentional inheritance of a UniFi API key by unrelated child tools; it does not prove the external executable is trustworthy or protect against all possible environment-variable names. Do not run Rust-Boy from a highly privileged shell containing unrelated secrets.


## Unified Network Engineer session ownership

The desktop and micro tool catalogs are metadata; mounting the component starts no request. Active operations use explicit same-origin POSTs after a separate operator assertion. Input edits, role/tool navigation, hidden documents, STOP REFRESH and request failures cancel the view's timer and browser fetch. Refresh is capped at 60 runs with 60 seconds between completed runs. A cancelled fetch cannot terminate a server operation already accepted; its existing sweep, process, timeout and request-worker bounds still apply.

The nine offline operations do not inspect the host network. Single-endpoint TCP/ICMP tests accept literal IPs, bounded samples and explicit confirmation; TCP connection failures are not represented as packet loss. Controller client names are rendered as text, never HTML. No scan result, API key or new authorization preference is persisted by this UI.

Rust-Boy is an **experimental local field console**, not a safety-of-life, security-monitoring or production hardware-control system. The public code is MIT-licensed; that does not imply a warranty or security certification.

## What the current version does

- **Offline by default.** STAT uses local host telemetry; LIBRARY reads a bundled catalog. No account, analytics, telemetry upload, device scan or background network service is required.
- **Local notes.** NOTES stores plaintext JSON under `PIPBOY_HOME` or the platform's data directory. Do **not** store secrets, private medical records, credentials or protected field logs there. Writers use cooperative locks and atomic-replacement techniques, but external edits, power loss and untrusted local code are outside those guarantees.
- **Preference is not permission.** `permissions.json` holds `requested` or `denied` intent. Neither it nor `simulate --confirm` grants a physical device capability.
- **Simulators are synthetic.** Fixed `sim:*` fixtures, typed mock input/display/serial adapters and bounded virtual queues do not open real devices, capture live radio data, contact peers or transmit signals.
- **The separate broker denies everything.** `rust-boy-broker --stdio-once` validates one bounded JSON request and always returns `allowed:false`. It has no hardware backend, device credentials, privileged helper or authenticated IPC endpoint.

The `ModuleHost` capability checks and mock leases are cooperative **in-process** controls; they are **not** an OS sandbox for untrusted native code. A separate deny-only process is likewise **not** a working OS permission broker.

## Explicit NET ENGINEER and UniFi boundaries

NET ENGINEER now supports **operator-triggered** local IPv4 network I/O; the existing offline default remains. A role selection or module pin grants no network scan. Active discovery requires a user-confirmed RFC1918 /24..../32 scope (maximum 254 addresses), with one active sweep and bounded parallelism. It sends ICMP echo probes and TCP connection attempts to a fixed list of ports, then reads the local ARP neighbor cache. It does not perform vulnerability assessment, password guessing, or Internet-wide discovery. The CLI's finite watch and the dashboard's optional 60-second refresh require deliberate opt-in; observed counts are **not** a complete network census.

An optional **GET-only UniFi Network Integration API** adapter reaches one explicitly configured private IPv4 gateway over verified TLS. API keys are environment-only, passed to curl via standard input rather than process arguments, hidden from dashboard JSON and removed from the curl child environment. Custom trusted CA PEM paths are supported; TLS bypass and redirects are not. The adapter allows only site and connected-client listing; no controller configuration or client action endpoint is reachable. The dashboard requires explicit same-origin POST confirmation and an active NET ENGINEER module before requesting controller data.

**Residual risks:** local environment variables may be readable by other software in the same OS account; compromised local hosts and custom CA files can subvert trust. A guessed site UUID is not authorization, and operator scope confirmation is not an independently verified entitlement. Device names and IPs returned to the localhost browser are sensitive network inventory; never expose the dashboard via tunnels or public proxies. Curl availability, malicious binary substitution through PATH, TLS CA enrollment, real UniFi permissions, performance and physical acceptance remain unverified. Refer to [NET ENGINEER](docs/NETWORK_ENGINEERING.md).

## Device-to-device task trust and persistent local data

The current Task Control UI uses explicitly paired OpenSSH peers, **not email**. The sender verifies a previously approved host key, uses a chosen private SSH key, disables password and SSH-agent fallback, limits execution time/output, and passes only a validated JSON assignment to a fixed remote Rust-Boy command. The recommended receiver uses a dedicated OS account and a key-specific \`authorized_keys\` **\`restrict,command="/absolute/pipboy peer bridge SOURCE"\`** rule: the Rust-Boy forced bridge allows only typed receive and matching status-receipt requests, not an arbitrary shell, port forwarding or other processes. Host key fingerprints must be verified out-of-band; \`ssh-keyscan\` output alone is untrusted.

\`peers.json\` stores peer aliases, approved SSH targets, paths to SSH keys/known_hosts and local trusted labels **but no private key bytes**. \`tasks.json\` contains plaintext assignments, nonsecret origin references, stored-delivery receipt and separately imported remote progress. Data files are private to the OS account on Unix and atomically replaced; they are **not encrypted at rest**, and a locally logged-in user can switch roles. Roles and trusted sender *labels* are not human/device cryptographic identities. The forced key-level restriction is the receiver's supported source authentication boundary in this phase.

No task is transmitted or synchronized on job selection, server startup or state GET. Sending requires explicit operator confirmation; remote progress refresh is also explicit. SSH response validation/receiver-side deduplication prevent a sender retry from silently creating repeated local tasks. There are **no** background task notifications, BLE/mesh/generic LAN listeners or direct video/GNSS actions.

A compromised receiver OS account or unrestricted SSH authorized key can still perform account-level operations: **do not use this preview for safety-critical, confidential or regulated workflows**. Physical host-key, key-revocation, Linux/macOS Pi deployment and interrupted-delivery tests are pending. See [peer transport contract](docs/PEER_TASK_TRANSPORT.md).

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

## PACE routing boundary

The [PACE selector](docs/PACE_MESSAGING.md) makes only the operator-selected
attempt. Saved RFC1918 Alternate routes retain pinned SSH identities. Route
configuration and E holds open no connection; C cannot transmit. P/A share a
failure domain, so this is not four independent operational communication paths.
Pairing, strict receipts and explicit confirmation remain mandatory. The API/UI
now distinguish enrolled age queue encryption from default plaintext history.

## Messenger queue and fallback boundary

[Build passes A–C](docs/MESSENGER_BUILD_PASSES.md) integrate a shared v1 envelope,
typed durable receipts, explicit private-IP retry preserving the pinned SSH host
identity, and opt-in age encryption for fresh message queues. Queue private keys
use native credential storage, fail closed on unlock errors, and remain distinct
from operator-managed SSH sender keys. Default/legacy message journals and other
local stores remain plaintext. This is not E2EE, anti-rollback storage or an
independently audited integration. No new live serial backend is enabled:
Meshtastic decoding accepts recorded capture files/borrowed read streams only.
[Native key lifecycle and physical tests](docs/MESSENGER_INTEROPERABILITY.md) are pending.

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

## Rust-Boy Messenger v1

Device-to-device text travels only through manually initiated, strict pinned OpenSSH connections with a dedicated forced-command key per sender. Remote receipts verify persisted application storage but not a human read. Local `PIPBOY_HOME/messages.json` is **plaintext**, not E2EE or encrypted storage. The loopback HTTP user interface does not establish per-human authentication. Do not use it for patient records, passwords, classified data, protected locations or other sensitive communication. Preserve host fingerprint out-of-band verification; do not enable SSH agent/port forwarding. See [Messenger threat model](docs/MESSENGER.md).

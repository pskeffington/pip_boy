# Rust-Boy | NET ENGINEER: live private-LAN probing

**Status:** implemented on the diagnostic feature branch, pending cross-platform CI and a real Mac/UniFi LAN acceptance pass. Not a packaged release.

NET ENGINEER is distinct from the fully offline DIAG evidence-ingestion pipeline. It is a real opt-in IPv4 network probe built in Rust, not a simulation or a CIDR-only button.

## Operator quick start

```bash
# From the repository checkout; requires Rust 1.90+
cargo run --locked -p pipboy -- net scan local --confirm
cargo run --locked -p pipboy -- net scan 192.168.1.0/24 --confirm --json
cargo run --locked -p pipboy -- net watch 192.168.1.0/24 --confirm --interval-sec 60 --count 5
cargo run --locked -p pipboy -- dashboard --micro --open
```

On the **micro display**, load **Head Engineer** or **Security Chief**, open **NET ENGINEER** via STAT quick action, GEAR or DATA, choose **SCAN PRIVATE LAN**, set **local** or a private IPv4 CIDR, tick **AUTHORIZED NETWORK**, then press **SCAN PRIVATE LAN**. Optional **REFRESH EVERY 60 SEC** repeats only while this tool view remains open. Desktop exposes the same action and can switch to the original **OFFLINE CIDR CALCULATOR**.

The **local** shortcut uses a UDP socket connect solely to query the host's outbound route; it **does not send a datagram**. It assumes a conservative /24 around the selected private IPv4 address. It cannot know the interface's real subnet mask or which physical/VLAN network you intend; choose an explicit CIDR when the suggestion is incorrect, the route is through VPN, or there is no default private route.

## What counts as a detected device

Each active sweep enumerates at most 254 usable hosts in a selected /24..../32 IPv4 network. IPv4 must remain entirely RFC1918-private. No public IPv4, broadcast range, broad /23-/8, arbitrary hostname, URL, port list, exploit template, or OS command is accepted as a scan target.

For each address, the engine attempts:

1. An OS-native single numeric ICMP echo, with no shell invocation, followed by a second echo for hosts that responded. It records ICMP RTT from the command's measured reply and the absolute two-sample RTT delta when available.
2. If ICMP is unavailable or unanswered, a bounded TCP connect on a fixed short list of conventional service ports (22, 80, 443, 445, 8080). Open and explicitly rejected connections are separately reported, with TCP connection/refusal elapsed time.
3. After the active sweep, a **read-only local neighbor-cache** check enriches the report with IPs seen in ARP data, without mislabeling them as currently responsive.

The report separates **responding_devices** from **cached_only_devices**, and also provides **total_observed_devices** (unique union of both). Neither the number of responding devices nor the observed union is guaranteed to equal **all currently connected devices**. Sleeping/offline/filtered hosts and separate VLANs may not answer, and ARP cache entries can persist after disconnect. TCP refusal is weaker evidence than an ICMP echo. Measured RTT values from different probe methods are **not directly comparable**; the two-sample difference is **not** standard packet-delay variation/jitter.

This is discovery and basic fault triage only. It does not identify manufacturer or hostnames, authenticate to routers, enumerate all endpoints across VLANs, inspect services for vulnerabilities, attempt logins, or discover private networks without operator input. Authoritative per-VLAN client inventories, RSSI, negotiated link speed and error counters require approved device/controller APIs, e.g., a separately enrolled UniFi controller with scoped read-only credentials.

## Safety, permissions, and runtime

- No scanning on application launch, role/profile load, module pin, ordinary status GET, map view, or switching tabs.
- The CLI requires an explicit --confirm; dashboard POST /api/net/scan requires NET ENGINEER pinned, exact loopback Host and Origin, JSON Content-Type, and confirm: true. Its confirmation records an operator assertion, **not** independently verified legal authorization.
- The user must own or be explicitly permitted to assess all assets in the requested network range; device possession or nearby access does not establish authority over other systems.
- One active sweep at a time; a fixed 24-worker upper bound, OS ping process deadlines, short TCP timeouts, and a 254-address hard limit. No root access, raw packet injection, stealth scanning, brute-force logins or external Internet probes.
- The service remains at 127.0.0.1. Up to 12 dashboard request worker threads keep the micro shell responsive during a sweep. A user-selected 60-second browser refresh ends when the NET ENGINEER view is closed; it is not a daemon or guarantee of continuous connectivity.
- No scan results are written to disk or sent to other devices. CLI --json emits a schema-based report; browser displays a same-origin summary.
- ICMP uses the local OS ping binary: Linux, macOS and Windows flags differ. If ping is unavailable or disallowed, TCP and neighbor-cache observations still run; missing ICMP is not proof of host failure.

## Acceptance and remaining gates

Deterministic Rust tests cover private-subnet validation, /24 bounds, IP sorting, RTT parsing, cache-only distinctions, denial before active probing and a local loopback TCP listener. Rust and JavaScript lint/build checks plus API route authorization fixtures run under the repository quality workflow.

Manual acceptance on a real authorized LAN must compare responsive IPs against a known inventory and repeated sweep results, then validate macOS/Linux/Windows ping behavior, service firewall behavior, bounded resource load, UI responsiveness, 480×320 readability, and browser monitor cancellation. No claim of complete inventory accuracy is made before this physical acceptance.

Refer to [DIAG evidence schema](DIAGNOSTICS.md) for importing authorized fault telemetry and [network architecture](ARCHITECTURE.md) for the separation of local UI from device data.

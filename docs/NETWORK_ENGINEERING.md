# Rust-Boy | NET ENGINEER: live private-LAN probing

**Status:** LAN/UniFi/DIAG software merged to `main` in PR #15 after all five CI jobs passed. Network calculators and endpoint health are consolidated in the follow-up integration. Real Mac/UniFi/Pi field acceptance remains open; no packaged release is claimed.

NET ENGINEER is distinct from the fully offline DIAG evidence-ingestion pipeline. It is a real opt-in IPv4 network probe built in Rust, not a simulation or a CIDR-only button.

## Source ownership and authorization

`crates/pipboy/src/dashboard/network_routes.rs` now contains the `POST /api/net/scan` and `POST /api/net/unifi` HTTP adapter functions. The original `dashboard.rs` remains the only local web server, enforces exact Host/Origin and request bounds, and dispatches commands only after a user action. Both handlers still require `confirm:true` and an active `net-lab` tool. CIDR authorization and finite probing are owned by `pipboy-core::network_probe`; the UniFi adapter performs read-only controller queries with strict site ID validation. This code-organization pass introduces no automatic probing, new UniFi credential path or cross-device relay.

Offline dashboard tests reject malicious origins, missing consent, unpinned roles and malformed site/scope parameters before any network operation can run. End-to-end UniFi controller behavior still requires separate owner-authorized Mac field testing; physical Pi compatibility remains unverified.

## Operator quick start

```bash
# From the repository checkout; requires Rust 1.90+
cargo run --locked -p pipboy -- net scan local --confirm
cargo run --locked -p pipboy -- net scan 192.168.1.0/24 --confirm --json
cargo run --locked -p pipboy -- net watch 192.168.1.0/24 --confirm --interval-sec 60 --count 5
cargo run --locked -p pipboy -- dashboard --micro --open
```

On desktop or micro, load **Head Engineer** or **Security Chief** and open **NET ENGINEER**. Choose **SCAN LAN**, enter `local` or a private IPv4 CIDR, select NEXT, confirm authorization and Run. Inventory results have pages. Optional 60-second refresh is capped at 60 runs and stops on editing, navigation, hidden tabs or errors. The same workspace includes nine offline calculators, PING / JITTER, TCP health and controller inventory. See the [unified health and UniFi setup guide](NETWORK_HEALTH_UNIFI.md).

The **local** shortcut uses a UDP socket connect solely to query the host's outbound route; it **does not send a datagram**. It assumes a conservative /24 around the selected private IPv4 address. It cannot know the interface's real subnet mask or which physical/VLAN network you intend; choose an explicit CIDR when the suggestion is incorrect, the route is through VPN, or there is no default private route.

## What counts as a detected device

Each active sweep enumerates at most 254 usable hosts in a selected /24..../32 IPv4 network. IPv4 must remain entirely RFC1918-private. No public IPv4, broadcast range, broad /23-/8, arbitrary hostname, URL, port list, exploit template, or OS command is accepted as a scan target.

For each address, the engine attempts:

1. An OS-native single numeric ICMP echo, with no shell invocation, followed by a second echo for hosts that responded. It records ICMP RTT from the command's measured reply and the absolute two-sample RTT delta when available.
2. If ICMP is unavailable or unanswered, a bounded TCP connect on a fixed short list of conventional service ports (22, 80, 443, 445, 8080). Open and explicitly rejected connections are separately reported, with TCP connection/refusal elapsed time.
3. After the active sweep, a **read-only local neighbor-cache** check enriches the report with IPs seen in ARP data, without mislabeling them as currently responsive.

The report separates **responding_devices** from **cached_only_devices**, and also provides **total_observed_devices** (unique union of both). Neither the number of responding devices nor the observed union is guaranteed to equal **all currently connected devices**. Sleeping/offline/filtered hosts and separate VLANs may not answer, and ARP cache entries can persist after disconnect. TCP refusal is weaker evidence than an ICMP echo. Measured RTT values from different probe methods are **not directly comparable**; the two-sample difference is **not** standard packet-delay variation/jitter.

The active probe is discovery and basic fault triage only. It does not identify manufacturer or hostnames, inspect services for vulnerabilities, attempt logins, or discover networks outside the selected scope. An optional read-only UniFi adapter supplies controller-connected clients using the official Integration API, with separate counts for connected clients and IP-address matches. RSSI, negotiated link speed, error counters, disconnected history and more detailed device statistics remain future modules.

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


## UniFi Network connected-client inventory (opt-in)

The optional adapter uses **only the official local UniFi Network Integration API**, documented in [Ubiquiti's API getting-started guide](https://help.ui.com/hc/en-us/articles/30076656117655-Getting-Started-with-the-Official-UniFi-API) and [Network's List Connected Clients endpoint](https://developer.ui.com/network/v10.6.106/getconnectedclientoverviewpage).

It performs only GET requests to:

- `/v1/sites?offset=N&limit=200`
- `/v1/sites/{siteId}/clients?offset=N&limit=200`

On a UniFi OS gateway these are prefixed by `/proxy/network/integration`. A standalone Network Application uses `/integration`. This adapter does not call client actions, changes, service configuration, guest authorization or firmware endpoints.

Configuration is **server-side only**, with no browser key input and no persistent token file:

| Variable | Example / meaning |
| --- | --- |
| `RUSTBOY_UNIFI_HOST` | Controller **private IPv4** address, e.g. `192.168.1.1` |
| `RUSTBOY_UNIFI_PORT` | `443` (UniFi OS, default) or `8443` (standalone) |
| `RUSTBOY_UNIFI_MODE` | `unifi-os` (default) or `standalone` |
| `RUSTBOY_UNIFI_API_KEY` | Official Integration API key from the controller; secret, environment only |
| `RUSTBOY_UNIFI_CA_CERT` | Optional **trusted CA PEM file** if the controller certificate is not trusted by the host OS |

On your Mac, start a shell with the configuration set, then run the CLI or dashboard from that same shell:

```bash
export RUSTBOY_UNIFI_HOST=192.168.1.1
export RUSTBOY_UNIFI_MODE=unifi-os
# Set RUSTBOY_UNIFI_PORT=8443 if running a standalone Network Application.
# Set RUSTBOY_UNIFI_CA_CERT=/trusted/ca.pem if the controller uses a private CA.
# Export your API key from a local secret manager; never place it in repo files or URLs.

cargo run --locked -p pipboy -- net unifi sites --confirm
cargo run --locked -p pipboy -- net unifi clients <SITE_UUID> --confirm
cargo run --locked -p pipboy -- net unifi compare <SITE_UUID> 192.168.1.0/24 --confirm
cargo run --locked -p pipboy -- dashboard --micro --open
```

`net unifi clients` counts **controller-reported connected clients** (wired, Wi-Fi and VPN) and reports how many provide IP addresses. A comparison joins **distinct IPv4 addresses** from that inventory to **actively responding** scan targets. A client lacking an IPv4, a different VLAN, multiple clients sharing an address, or a device without UniFi reporting cannot be paired reliably. The comparison never relabels cached-only ARP neighbors as active responses.

In the desktop/micro **NET ENGINEER** menu, choose **UNIFI SITES** first, then enter its UUID in **UNIFI CLIENTS** or **UNIFI / LAN CHECK**. Tick authorized access and press the action button. Refresh can optionally repeat every 60 seconds while the view remains open; no controller query happens on login, page load, or role selection.

### Transport / safety gates

- The controller must be a private IPv4 address (not a public host or a browser-supplied URL), and the site ID must be a UUID. Only HTTPS ports 443 and 8443 are allowed.
- Rust starts local `curl` with an allowlisted GET URL, explicit timeouts, maximum response size and **TLS certificate verification enforced**. Custom trusted CA PEM is supported; there is deliberately no `--insecure` / `-k` bypass.
- The API key is provided as a header through curl's standard input. It is not part of the command line, URL, process log, HTTP API response, or source repository. The curl child has the key environment variable removed.
- Redirects, HTTP fallback, shell interpolation, automatic API-key discovery, controller updates and external Site Manager/cloud access are disabled.
- Pagination is checked for coherent offsets, count and totalCount, with a maximum of 2,000 clients/200 sites. If an inventory changes during pagination, the adapter fails rather than claiming a complete count.
- Browser POST `/api/net/unifi` needs exact loopback Host/Origin, explicit confirmation, and NET ENGINEER pinned. Returned client names/IP addresses are private local data, so **never expose the loopback dashboard through a proxy**.

**Limitations:** the first adapter relies on an installed curl executable (not yet an embedded Rust TLS implementation), local API availability varies by UniFi Network version, and access with a real controller has not been verified. The operator must choose suitable read-only account/API permissions. A standard site inventory from the API is not an all-time client registry or a cross-vendor census.

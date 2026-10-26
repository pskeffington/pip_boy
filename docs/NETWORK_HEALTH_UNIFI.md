# Network health and UniFi setup

The unified **NET ENGINEER** workspace is available from `main` in the CLI, desktop and micro HUD. Use Head Engineer or pin `net-lab` in Custom. Opening the workspace only displays tools.

## HTTP route ownership

The HUD's manually confirmed private LAN scan and read-only controller actions are implemented in `crates/pipboy/src/dashboard/network_routes.rs`, sharing the original parent loopback Host/Origin gate. `pipboy-core::network_probe` and `pipboy-core::unifi` remain responsible for network and controller policies; `endpoint_probe` owns separate measured ICMP/TCP health operations. Moving the HTTP functions does not change controller configuration, transmit permissions, probe rates or the need for explicit operator confirmation. Fixtures test rejection before network access; live device acceptance is separate.

## ICMP and TCP measurements

```bash
cargo run --locked -p pipboy -- workbench net icmp 192.168.1.1 5 --confirm
cargo run --locked -p pipboy -- workbench net health 192.168.1.1 443 5 --confirm
cargo run --locked -p pipboy -- workbench net tcp 192.168.1.1 443 --confirm
```

PING / JITTER invokes the native OS `ping` executable for 3–8 attempts to one literal IP. It accepts a complete transmitted/received summary and corresponding successful RTT readings. Missing tooling, incomplete output or denied execution produces an error. A complete summary with no replies reports response loss without invented RTT values.

NETWORK HEALTH measures 3–8 TCP connection attempts, each limited to 450 ms, with 100 ms between attempts. TCP CHECK makes one attempt with a 900 ms timeout. Literal IPs and one port are required; no DNS resolution or application payload is used. ICMP invocation has a 9-second process deadline and a 16 KiB output limit.

| Metric | Meaning |
| --- | --- |
| ICMP RTT | Time reported by native ping for successful echo replies |
| ICMP loss | Sent minus received, from a complete native packet summary |
| TCP connect time | Handshake duration including local scheduling |
| TCP failure percentage | Failed connects / attempts; **not packet loss** |
| Variation | Mean absolute difference of adjacent successful TCP attempts, or successive successful ICMP replies |
| p95 | Nearest-rank sample percentile; with 3–8 observations this is the sample maximum |

Variation is a short diagnostic proxy. It is not RFC 3550 RTP interarrival jitter, a throughput estimate or a sustained service-level measurement. TCP gaps are excluded from adjacent-pair variation; ICMP uses successful-reply order because the parser does not preserve a complete per-attempt timing series.

## UniFi local controller

Configure the Rust process before launch. Obtain a local Integration API key from your controller and provide its trusted certificate chain. Runtime variables are:

| Variable | Value |
| --- | --- |
| `RUSTBOY_UNIFI_HOST` | Controller's RFC1918 IPv4 address |
| `RUSTBOY_UNIFI_PORT` | `443` (default) or `8443` |
| `RUSTBOY_UNIFI_MODE` | `unifi-os` (default) or `standalone` |
| `RUSTBOY_UNIFI_API_KEY` | Local integration key, supplied only to the server process |
| `RUSTBOY_UNIFI_CA_CERT` | Optional path to a trusted CA PEM file |

UniFi OS uses `/proxy/network/integration/v1`; standalone uses `/integration/v1`. The adapter lists sites and paginates connected clients. HTTPS verifies the certificate against the configured IP, so the certificate must cover that IP and chain to a trusted CA. There is no TLS verification bypass or redirect following. This implementation relies on an installed trusted `curl` executable.

API credentials pass to curl through stdin and are removed from its environment. They are not placed in process arguments, browser JSON or saved workbench files. The adapter performs GET requests only. Configure the narrowest available controller access for the inventory task.

```bash
cargo run --locked -p pipboy -- net unifi sites --confirm
cargo run --locked -p pipboy -- net unifi clients SITE_UUID --confirm --json
cargo run --locked -p pipboy -- net unifi compare SITE_UUID 192.168.1.0/24 --confirm --json
```

UniFi SITES returns site UUIDs; enter one in CLIENTS or UNIFI / LAN. Comparison reads the controller before probing the explicit private subnet. The returned counts distinguish controller clients, distinct controller IPv4s, matched responders, unmatched controller IPs and probe-only IPs. Neighbor-cache-only entries do not qualify as active matches. Counts cannot establish physical device identity or inventory every VLAN.

The retired experimental `RUSTBOY_UNIFI_IP`/TLS-name configuration from PR #14 is not used by the consolidated adapter. Use the variables above. Adopted-device health, RSSI, link speed and historical clients remain planned; the current API exposes the controller's connected-client view.

## Display and refresh

Both interfaces obtain operation descriptors from `GET /api/state`. Choose a tool, enter inputs and advance to the separate authorization screen. Offline tools calculate directly. Micro inventory pages show two entries with explicit previous/next controls; full text remains under DETAILS.

SCAN LAN and the three UniFi inventory operations offer a 60-second refresh, limited to 60 runs. STOP REFRESH, editing inputs, changing roles/tools, leaving the document, a hidden browser tab or an error stops further requests. Returning does not resume automatically. Cancelling aborts the browser fetch; an already accepted server operation can finish within its existing bounds. Results remain in the mounted view only.

## Verification and field acceptance

`tools/smoke_network_health.py` exercises offline CLI/API calculations and real TCP handshakes only against an ephemeral loopback listener. Rust tests cover statistics and native ping parsing. `tools/test_network_ui.cjs` checks authorization, paging, one owned refresh timer, navigation/visibility cancellation, stale replies and safe controller labels. `tools/smoke_network.py` retains LAN/UniFi rejection and DIAG intake checks.

Before claiming field readiness, compare actual Mac ICMP/TCP measurements with native tools, validate a real controller's certificate/key/client counts, and test the 480×320 Pi touchscreen. Automated fixtures do not complete those tests.

See [Network Engineer](NETWORK_ENGINEERING.md), [micro UI](MICRO_UI.md), [validation](VALIDATION.md) and [Ubiquiti's official local API guide](https://help.ui.com/hc/en-us/articles/30076656117655-Getting-Started-with-the-Official-UniFi-API).

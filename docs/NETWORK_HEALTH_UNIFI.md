# Rust-Boy | Live Network Health + UniFi Local API

**Scope:** an explicitly invoked, real-network diagnostic workspace in the **Head Engineer / NET SUITE** profile. Built for local Mac testing and the future Raspberry Pi handheld. This is a developer preview, not an uninterrupted monitoring service or an audited SLA sensor.

## What really runs

| Mode | Measurement | Important limitation |
| --- | --- | --- |
| **PING / JITTER** (`icmp`) | 3–8 native OS ICMP Echo Requests to one literal IP, per-reply RTT, min/mean/median/p95/max, ICMP reply loss, and mean absolute RTT delta over successive received replies | A short snapshot; RTT variation is a **jitter proxy**, not an RFC 3550 packet interarrival jitter stream. ICMP may be deprioritized, filtered or blocked. Native `ping` is required. |
| **NETWORK HEALTH** (`health`) | 3–8 TCP connect attempts to one literal IP:port, handshake time and variation, successful/failed attempts | Connection failures are **not** IP packet loss; TLS, HTTP and bandwidth are not evaluated. Each attempt times out after 450 ms; waits 100 ms between attempts. |
| **TCP CHECK** (`tcp`) | Existing one-off TCP handshake to one endpoint | One connect only; not a latency distribution. |
| **UNIFI STATUS** (`unifi`) | Read-only, HTTPS-authenticated UniFi local sites and adopted-device status from the official Network Integration API | **Two GETs maximum** (first 25 sites and up to 50 adopted devices in the selected site). Counts cover the fetched page, not necessarily the entire site. No historical WAN metrics are inferred. |

The earlier nine offline modes (IPv4/IPv6, subnet math, route planning, MAC, VLAN, MTU) are unaffected. All four live operations require a new affirmative confirmation every time. Nothing runs on profile load, page navigation or initial dashboard GET.

## Four-inch workflow

On the Mac, run the feature branch and open the micro screen:

```bash
git fetch origin
git switch --track origin/feat/network-health-unifi
cargo build --workspace --locked
cargo run --locked -p pipboy -- workbench use head-engineer --replace
./scripts/launch-micro.command
```

**Head Engineer → STATUS → HEALTH** goes directly to **PING / JITTER** (real ICMP). **UNIFI** goes directly to the local API read view. For **TCP connection health** and all engineering calculations, use **TOOLS → NET SUITE** (six operations per page). The input and result pages remain separate on 480×320. True RTT, variation, and ICMP loss are labeled distinctly from TCP connect failures.

Desktop: `cargo run --locked -p pipboy -- dashboard --open` → **Head Engineer → NET SUITE**. Select an operation, enter a target and press **Ping now / Test network / Read UniFi**. Measured statistics include a compact sample visualization.

## Real gateway tests without an API key

A UniFi gateway can be tested via literal LAN IP even when no controller API credentials are configured. Use the gateway's actual IP (the following is an **example**, not a discovered device):

```bash
cargo run --locked -p pipboy -- workbench net icmp 192.168.1.1 5 --confirm
cargo run --locked -p pipboy -- workbench net health 192.168.1.1 443 5 --confirm
```

For fault isolation, compare the gateway with a separate, authorized upstream IP using the same protocol and sample count. Expect different paths and policies: a TCP handshake to port 443 does not measure ICMP RTT, and a refused TCP connection is not packet loss. Never automate tests against third-party endpoints without authorization.

## Link the official local UniFi API

UniFi documents a **local Network API** under **UniFi Network → Integrations**. API-key authentication provides sites and adopted-device information, among other capabilities. The app uses its read-only **v1** routes, not the undocumented classic controller login API.

1. In the UniFi Network application, open **Integrations** and consult the documentation for your installed version. Create an API key and protect it as an administrative secret.
2. Identify the console's **private LAN IPv4 address**. Only private RFC 1918 or loopback IPv4 targets are accepted; the browser cannot choose another host. For UniFi OS gateways the default route is `/proxy/network/integration/v1`. Set `RUSTBOY_UNIFI_API_PATH=direct` for a local server exposing `/integration/v1`, after verifying its local documentation.
3. Establish **verified TLS**. The certificate must validate against the selected name/IP. For a local trusted CA or self-signed trust anchor, supply a PEM certificate using `RUSTBOY_UNIFI_CA_FILE`. If the TLS certificate uses a hostname, set `RUSTBOY_UNIFI_TLS_NAME` to that hostname. Rust-Boy pins its DNS mapping to the private console IP with curl `--resolve`; no public DNS lookup or cloud proxy is needed. It never uses `--insecure`.
4. Configure the variables **in the same Terminal session** that launches Rust-Boy. Do not paste an API key into the web form or commit it in a repo.

```bash
export RUSTBOY_UNIFI_IP=192.168.1.1
export RUSTBOY_UNIFI_PORT=443
export RUSTBOY_UNIFI_API_PATH=os

# Set these when required by your HTTPS certificate:
export RUSTBOY_UNIFI_TLS_NAME=unifi.example.net
export RUSTBOY_UNIFI_CA_FILE="$HOME/certs/unifi-ca.pem"

# Enter the secret without printing it or placing it in shell history.
printf 'UniFi API key: '
stty -echo
IFS= read -r RUSTBOY_UNIFI_API_KEY
stty echo
printf '\n'
export RUSTBOY_UNIFI_API_KEY

./scripts/launch-micro.command
```

The TLS name/CA above are **examples**. A default self-signed controller certificate may not have a matching IP or hostname subject alternative name; importing a CA does **not** fix a hostname mismatch. Install a properly issued certificate or correct the host and trust configuration. Avoid disabling certificate validation.

Use **HEAD ENGINEER → UNIFI → READ**, then approve the manual GET request. Or run:

```bash
cargo run --locked -p pipboy -- workbench net unifi --confirm
# Optional: select a site ID returned by the first 25-site API page
cargo run --locked -p pipboy -- workbench net unifi SITE_UUID --confirm
```

`curl` must be available on the Mac. The API key is passed to the child process over **stdin**, not command-line arguments. Curl is limited to HTTPS, explicitly pinned private-LAN IP, 4-second timeouts per call, 64 KiB per response, no redirects, no proxy, and only these two allowlisted paths:

```text
GET /proxy/network/integration/v1/sites?limit=25&offset=0
GET /proxy/network/integration/v1/sites/{siteUuid}/devices?limit=50&offset=0
```

The `direct` variant uses the same paths under `/integration/v1`. Only a validated site UUID obtained from the local sites page may be used. The JSON response contains sanitized site names, device names, models, state and validated IPs. It deliberately excludes MAC addresses, API keys, arbitrary raw API payloads, and configuration mutation endpoints. The service does not persist snapshots or record credentials.

**Security caveat:** an API key may carry broad UniFi administrative privileges even though this client uses only GET routes. Protect and rotate it accordingly. Rust-Boy's local dashboard has same-origin protections but **does not provide OS-backed multiuser authentication**; never reverse-proxy or expose it to the LAN.

## Test and acceptance contract

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --locked
python3 tools/smoke_network.py
```

Unit tests cover numeric percentiles, all-lost samples, safe input limits, native macOS/Linux/Windows ping text fixtures, TLS endpoint restrictions and UUID validation. The smoke suite uses only a temporary 127.0.0.1 TCP listener; it confirms real TCP metrics, fail-closed UniFi when unconfigured, and no implicit connections. **It does not prove connectivity to your actual UniFi system**; the manual Mac/UniFi HTTPS test remains an acceptance gate. True native ICMP execution on the user's Mac, 480×320 physical display testing and controller certificate validation also require field acceptance.

### Standards and primary references

- [Ubiquiti: Getting Started with the Official UniFi API](https://help.ui.com/hc/en-us/articles/30076656117655-Getting-Started-with-the-Official-UniFi-API)
- [UniFi Network v10.6.106 — List Local Sites](https://developer.ui.com/network/v10.6.106/getsiteoverviewpage)
- [UniFi Network v10.6.106 — List Adopted Devices](https://developer.ui.com/network/v10.6.106/getadopteddeviceoverviewpage)
- [Linux iputils ping manual](https://man7.org/linux/man-pages/man8/ping.8.html) (OS variants have different timeouts and report formatting)

## Next gate

Consider actual UniFi controller API measurements such as device throughput/utilization when supported by the installed version's OpenAPI schema; add robust read-only paging and per-device selections. For live service analytics, add separately authenticated, rate-limited DNS resolution, TLS certificate inspections, optional remote test endpoints and an explicit opt-in sample recorder. Any sustained polling needs explicit consent, limits, redaction, retention and battery budgets. No auto-discovery or port sweeping is included in this pass.

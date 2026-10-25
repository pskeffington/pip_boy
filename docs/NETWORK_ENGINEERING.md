# Rust-Boy | Network Engineering Suite

**Stage:** G1 development, first functional pass. The **Head Engineer** template now preloads **NET SUITE** as its first tool. Security Chief, University Researcher, Reclamation Scout and advanced Custom profiles also have access when NET SUITE is pinned. This is an operator's engineering toolkit, **not** a remote scanner or an automatically connected network monitor.

## Launch on Mac or the four-inch HUD

```bash
# On the feature branch:
git fetch origin
git switch --track origin/feat/network-health-unifi
cargo build --workspace --locked
./scripts/launch-micro.command

# If an older user profile has stale pins:
cargo run --locked -p pipboy -- workbench use head-engineer --replace
```

Micro: **Head Engineer → STAT → HEALTH** (ICMP) or **UNIFI**; **GEAR → NET SUITE**, or **GEAR → NET SUITE**. The mode menu shows six actions per page, with a visible next/previous page control. Choose a mode, enter data, and press **RUN**. Large computed results have their own result view with an **INPUT** return action; the primary HUD remains compact.

Desktop: `cargo run --locked -p pipboy -- dashboard --open` → **Head Engineer → NET SUITE**. The mode picker and input forms use the same operation descriptions returned by the Rust backend, rather than a separate hardcoded tool registry.

## Live link: Network Health and UniFi

The Engineer now includes **13** modes: the nine existing offline calculations plus TCP Check, TCP Health (connection timing), ICMP Echo (true RTT and reply loss), and a read-only UniFi local API snapshot. The two Health methods are intentionally distinct: **ICMP response loss is packet loss; TCP connect failures are not**. The micro STATUS screen has HEALTH and UNIFI quick entries. [Live setup and security contract](NETWORK_HEALTH_UNIFI.md).

## Preloaded operations

| Operation / CLI verb | Real function | Boundary |
| --- | --- | --- |
| **PING / JITTER** `icmp` | Native OS ICMP RTT, loss and successive-reply RTT variation | **Active**; operator confirmation, literal IP, at most 8 probes |
| **NETWORK HEALTH** `health` | 3–8 TCP handshake samples with mean/median/p95 and connect-time variation | **Active**; failures are not packet loss |
| **UNIFI STATUS** `unifi` | Read-only local UniFi sites and adopted-device snapshot via verified HTTPS | **Active**; configured private gateway, API key and confirmation |
| **IPv4 CIDR** `cidr4` (or legacy `cidr`) | Address/netmask, network, broadcast, usable range and host count | Offline; /31 counts two *point-to-point* endpoints under RFC 3021, not a broadcast LAN |
| **IPv6 CIDR** `cidr6` | Network/range for 0–128-bit prefixes, canonical compressed addresses, exact power-of-two size | Offline; no IPv6 broadcast |
| **SUBNET SPLIT** `split4` | Divide a parent IPv4 range into longer prefixes, showing the first 12 and the total count | Offline; bounded output even for /0 → /32 |
| **ROUTE MATCH** `route` | Compare destination IPv4 or IPv6 address against up to 16 entered prefixes, return the longest matching route | Offline *what-if* list; does not inspect or modify the actual routing table |
| **CIDR MERGE** `aggregate4` | Merge only a pair of exact sibling IPv4 networks into their immediate parent | Offline; rejects incorrect aggregations that would overclaim address space |
| **OVERLAP** `overlap4` | Compare two IPv4 CIDRs and report their intersection | Offline planning, not a traffic capture |
| **MAC / EUI-64** `mac` | Normalize EUI-48 MAC formats, identify group/local flags and show an illustrative modified EUI-64/link-local IPv6 value | Offline; no hardware MAC discovered; derived addresses can expose a stable identifier |
| **802.1Q VLAN** `vlan` | Calculate VLAN ID, PCP, DEI, TCI and the four on-wire tag bytes | Offline; accepts VID 1–4094, PCP 0–7, DEI 0/1; does not configure a switch |
| **MTU / MSS** `mtu` | Estimate IPv4/IPv6 TCP MSS and UDP payload from an entered IP MTU | Offline; assumes base headers; options, extensions, tunnels and lower-layer overhead require further planning |
| **TCP CHECK** `tcp` | One TCP connection attempt to one **literal IP and port**, with a fixed 900 ms attempt limit | **Active**; user confirms explicitly before each attempt; no DNS, payload transfer, port range, scheduled scans or background probes |

All modes return a bounded, copyable text result from the existing `/api/experiment` endpoint. The ICMP operation explicitly invokes the native OS ping and UniFi uses a restricted local curl process; the nine offline calculations do not spawn processes. There is no automatic network operation when a job is selected, a tool is pinned or a page is opened.

## Reproducible CLI examples

```bash
cargo run --locked -p pipboy -- workbench net list
cargo run --locked -p pipboy -- workbench net cidr4 192.168.1.50/24
cargo run --locked -p pipboy -- workbench net cidr6 2001:db8:1::abcd/64
cargo run --locked -p pipboy -- workbench net split4 10.40.0.0/24 26
cargo run --locked -p pipboy -- workbench net route 10.40.1.17 0.0.0.0/0,10.0.0.0/8,10.40.0.0/16
cargo run --locked -p pipboy -- workbench net aggregate4 10.40.0.0/24 10.40.1.0/24
cargo run --locked -p pipboy -- workbench net overlap4 10.40.0.0/16 10.40.1.0/24
cargo run --locked -p pipboy -- workbench net mac 00:1a:2b:3c:4d:5e
cargo run --locked -p pipboy -- workbench net vlan 42,5,0
cargo run --locked -p pipboy -- workbench net mtu 1500

# ACTIVE: only after verifying that this device/endpoint is yours to test:
cargo run --locked -p pipboy -- workbench net tcp 127.0.0.1 8080 --confirm
```

These CLI computations run independently of the selected job profile. GUI execution requires NET SUITE to be pinned on the active profile and rejects unpinned operation.

## Rust API and network policy

The local `GET /api/state` response includes `net_operations` (ten read-only names, example inputs, second-field labels and an `active` flag) from `pipboy-core::network_suite::OPERATIONS`. UI values are rendered through text content and native form fields. Desktop and micro share this catalog.

Offline request example:

```json
{"tool":"net-lab","mode":"split4","input":"10.40.0.0/24","extra":"26"}
```

Active check example (the browser separately requests confirmation first):

```json
{"tool":"net-lab","mode":"tcp","input":"127.0.0.1","extra":"8080","confirm":true}
```

The dashboard only accepts HTTP on `127.0.0.1` with strict Host/Origin/CSP validation. `confirm:true` is a **deliberate operator action marker, not a substitute for OS identity/authentication**. Do not publicly expose or port-forward the developer dashboard. An active TCP probe performs an actual TCP handshake to the entered destination and can produce traffic and logs on the destination. Never probe devices without appropriate authorization. No external DNS is requested when a hostname is entered: TCP targets must be literal IPs.

The core network module is pure, deterministic computation. It cannot read host interfaces, open sockets or change router/switch configuration. The separate binary's `network_probe::tcp_once` is the only active implementation, with strict literal-IP validation, a single-port attempt and a 900 ms timeout. It does **not** imply a host is secure, TLS is correct or an application is healthy; a successful connect proves only that one endpoint accepted a TCP handshake.

### Tests and boundaries

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --locked
python3 tools/smoke_network.py
python3 tools/smoke_micro.py
```

`smoke_network.py` exercises every command through the compiled binary, then an actual loopback HTTP dashboard. The only live target is an ephemeral `127.0.0.1` listener created by the test. It verifies no implicit network access on GET, required confirmation, invalid input refusal, preloaded role behavior and no unintended private data files.

**Hardware acceptance remains open:** test the NET SUITE modes and result return controls at 480×320 and 800×480 (browser zoom/keyboard/large fonts), on a 4-inch Raspberry Pi display, then test a single authorized endpoint on a real LAN with explicit permission. A passing CI integration test does not establish safety or device performance.

**Next releases (not implemented):** separately permissioned OS interface/route inventory, DNS with bounded resolver timeouts, traceroute, LLDP/CDP read-only discovery, TLS certificate inspection, packet capture, local link quality and report export. Each active function must have explicit target scope, rate/size limits, a genuine permission boundary, and proof it cannot run on profile load. Do not represent future functions as already working.

### Standards / evidence

- [RFC 3021](https://www.rfc-editor.org/rfc/rfc3021) — /31 IPv4 point-to-point addresses.
- [RFC 4632](https://www.rfc-editor.org/rfc/rfc4632) — classless route addressing and aggregation.
- [RFC 4291](https://www.rfc-editor.org/rfc/rfc4291) and [RFC 5952](https://www.rfc-editor.org/rfc/rfc5952) — IPv6 addressing and canonical text.
- IEEE 802.1Q VLAN tag fields are represented as TPID 0x8100 and a 16-bit TCI containing PCP, DEI and VID. The calculator is not a switch interoperability conformance test.

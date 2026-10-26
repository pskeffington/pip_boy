# Rust-Boy | RB-ICS Intelligence Communications System

**Status:** proposed architecture and research baseline; not a released messaging, mesh, emergency-alert, or medical system. Research/design snapshot: 2026-10-25.

**Purpose:** make operator-originated text, task updates, situation reports (SITREPs), and consented location/context available across *intermittent* authorized local networks. RB-ICS is a **local-first, delay-tolerant application overlay**, not a new radio standard, an RF repeater by itself, a cellular SMS service, or a guarantee of communication when no physical path exists.

## Verified project baseline and terminology

- **Implemented:** Task Control uses separate per-device task boards and an explicit sender-to-receiver **OpenSSH** handoff. It supports pinned remote host keys, a dedicated sender key, restricted receiver commands, bounded inputs, idempotent task import and a stored-delivery receipt. Mac-to-Pi physical acceptance is still open. See [Peer Task Transport](PEER_TASK_TRANSPORT.md).
- **Not implemented:** a general-purpose RB-ICS chat protocol, encrypted generic message mailbox, automatic discovery, native mesh, multi-hop relay, scheduled outbound retry, device-radio broker, or generic RB-to-RB notification.
- **SMS-like over Wi-Fi** means short application text messages over an IP-capable local link; it is **not carrier SMS** and cannot contact arbitrary phone numbers without a separately designed external gateway.
- **Wi-Fi LAN** can work without Internet, but still requires a usable AP, peer link or wired network. IEEE 802.11s offers AP-less multi-hop only on compatible, specifically configured hardware. An unconnected or shielded device has no magic RF path.
- **Meshtastic / LoRa** needs an actual authorized and compatible external radio and its supported client API. It is not equivalent to LoRaWAN, native Wi-Fi mesh, or BLE mesh. Low-bit-rate links must use small, airtime-governed messages.
- **Healthcare boundary:** do not put protected patient information, credentials, classified material, or operationally secret sensor traces into the current plaintext task store. The proposed RB-ICS security architecture is not a HIPAA compliance finding or an emergency-service certification.

## 1. System graph: proposed physical topology

Nodes are independent Rust-Boy installations; every edge represents a *possible* physical transport, not a guaranteed live route. Solid edges below denote an **existing task transport when a suitable IP connection already exists**. Dashed edges are **proposed/future** and need implementation, hardware, permissions and field testing.

~~~mermaid
flowchart LR
  subgraph A["Operations zone"]
    O["RB-01 Overseer"]
    LAN["Authorized local IP LAN<br/>Wi-Fi AP / Ethernet"]
    E["RB-02 Engineer"]
    O <-->|"current SSH task handoff"| LAN
    LAN <-->|"current SSH task handoff"| E
  end
  subgraph B["Boundary / relay zone"]
    G["RB-03 gateway / relay"]
    M["RB-04 portable relay"]
  end
  subgraph C["RF-shadowed zone"]
    S["RB-05 Scout"]
  end
  LAN <-.->|"planned IP messenger"| G
  G <-.->|"future 802.11s or another proven link"| S
  G <-.->|"future external Meshtastic radio"| M
  M <-.->|"future opportunistic contact"| S
~~~

**Interpretation:** A message for Scout first attempts an authenticated reachable link; if it cannot reach Scout, a permitted relay can hold the opaque message and attempt later forwarding. A trusted portable device may physically carry it to a contact zone. No delivery is inferred merely from an attempted send, an available route, or a relay receipt. Separate independent links provide redundancy only after each is field-verified.

## 2. Software architecture graph: per-device message path

Every RB participating in RB-ICS would host the same interface, policy, storage, router and transport adapter layers. **Everything in this graph is a proposed RB-ICS subsystem** except the pre-existing task-specific SSH adapter noted in the adapter box.

~~~mermaid
flowchart TB
  UI["LINK micro HUD<br/>TEXT / TASK / SITREP / LOCATION / RECEIPTS"]
  VALID["Schema, authorization, size and disclosure gate"]
  SEAL["Peer/group identity, signature and end-to-end encryption"]
  STORE["Durable encrypted inbox/outbox<br/>message ID, expiry, dedup, quotas"]
  ROUTE["Link policy and scheduler<br/>consent, priority, hop budget and retry"]
  FAIL["No verified route<br/>retain locally, show QUEUED"]
  IP["Proposed local IP messaging<br/>manual peers, later DNS-SD"]
  SSH["Existing restricted OpenSSH<br/>task transport only"]
  MESH["Future Linux 802.11s<br/>hardware-dependent"]
  LORA["Future Meshtastic bridge<br/>serial / TCP / BLE to radio"]
  BLE["Future BLE peer adapter<br/>not automatically BLE Mesh"]
  RX["Receiver: verify and store once"]
  REC["Signed storage receipt<br/>separate operator acknowledgment"]
  UI --> VALID --> SEAL --> STORE --> ROUTE
  ROUTE --> FAIL
  ROUTE -.-> IP
  ROUTE -.-> MESH
  ROUTE -.-> LORA
  ROUTE -.-> BLE
  ROUTE -.->|"reuse for TASK only"| SSH
  IP -.-> RX
  MESH -.-> RX
  LORA -.-> RX
  BLE -.-> RX
  SSH -->|"existing task import"| RX
  RX --> REC
  REC -.->|"over a later authorized path"| STORE
~~~

An intermediate relay stores and forwards opaque, authenticated content without acquiring the recipient's decryption key. Security must extend **across** transports, not rely only on transport encryption. The existing SSH task protocol does not itself provide E2E secrecy against a relay that terminates SSH; do not use it as a generic opaque-message relay without designing and verifying envelope protection.

## 3. Formal dynamic graph model

Define **G(t) = (V, E(t))**:

- **V**: enrolled RB devices plus explicit gateway/relay nodes with independently verified cryptographic identities.
- **E(t)**: an edge exists only when its adapter has observed a usable link at time t; edge attributes record **transport**, **authenticated destination**, **last measured availability**, **error/loss observations**, **latency**, **payload capacity**, **power budget**, **permission scope**, and **last observation age**. Unknown values remain unknown, never zero or good.
- **P(t)**: a candidate path through *authorized* edges for a given recipient and message class. A graph path is not a promise of end-to-end delivery.
- **No path**: retain a bounded, encrypted message in the originating or permitted relay queue until a valid opportunity arises or expiry is reached.

Start with direct preferred links and a bounded opportunistic-forwarding policy. Avoid unrestricted broadcast flood; use a per-message hop limit, explicit allowed relay set, deduplication cache, expiry, finite airtime/storage quotas and congestion backoff. Optimize using **measured**, not invented, link-quality data. Do not claim compliant BPv7 packet encoding unless actual RFC 9171 conformance tests pass.

## 4. RB-ICS message envelope v1 (proposed)

A minimum logical record, not a deployed wire format:

~~~text
outer routing metadata:
  version, message_id, destination_id/group_id, expires_at,
  max_hops, remaining_hops, approximate byte length, security_suite_id
authenticated encrypted inner content:
  origin_device_id, kind [TEXT|TASK|SITREP|WAYPOINT|HEALTH|ALERT],
  created_at, sequence/nonce, sensitivity, content,
  provenance [observed|operator_reported|derived], confidence/quality,
  optional map reference and consented coordinates
receipt (separate authenticated object):
  message_id, receiving_device_id, state [stored|operator_acknowledged],
  receipt_timestamp, signature
~~~

Identity and encryption are implementation design gates. Prefer established, reviewed protocol libraries; evaluate RFC 9180 HPKE for end-to-end content protection and RFC 9172 BPSec where exact BPv7 interoperability is chosen. Key verification is independent of peer discovery and IP subnet location. Human operator authorization must be separately checked; a device's certificate or the job HUD role is not proof of an operator's right to read all messages.

Do not expose message body, patient identifiers, precise coordinates or intelligence contents as relay metadata. Use transport-adapter fragmentation only when the underlying transport and resource caps support it. In a constrained radio profile prefer short structured updates over arbitrary file transfer.

## 5. Transport research and decision table

| Link / mechanism | Proposed role | Evidence and limits | Decision |
| --- | --- | --- | --- |
| Paired OpenSSH on LAN | Existing controlled task handoff | Requires IP reachability, enrolled host fingerprint, dedicated keys, receiver SSH; no queue or native mesh | **Keep** task path, preserve current security boundary |
| Local Wi-Fi / Ethernet IP protocol | First new SMS-like RB text and signed receipts | No Internet needed if devices share a permitted reachable IP network; cannot cross segmentation by assumption | **First implementation target** |
| DNS-SD / mDNS | Optional IP peer discovery | RFC 6763/6762 describes service discovery, **not authentication**; typically link-local; segment policies may block it | **Second**, opt-in with manual peer fallback |
| IEEE 802.11s | AP-less Wi-Fi multi-hop on Linux | Linux wireless documents mesh-point support; chipset/driver/OS and bridging validation required | **Hardware lab gate** |
| Meshtastic via serial/TCP/BLE client | Low-bandwidth alternate RF path | Official client API uses protobuf ToRadio/FromRadio; hardware, region, packet size, airtime and firmware matter | **External radio adapter gate** |
| Bluetooth Mesh | Alternative managed flooding / directed forwarding | Bluetooth SIG specifies relays and low-payload messaging; a generic BLE GATT connection is **not** Bluetooth Mesh | **Research only**, not assumed available on Mac/Pi |
| Physical carry / later encounter | Store-carry-forward between partitions | DTN overlay can synchronize when nodes later meet through a validated local link | **After durable queue** |
| Carrier SMS | Contact a public telephone number | Requires cellular carrier or external SMS gateway; Wi-Fi application texts are not SMS | **Out of initial scope** |
| LoRaWAN | Wide-area IoT protocol | Official LoRa Alliance describes a star-of-stars gateway/network-server topology, not Meshtastic peer mesh | **Do not substitute** for mesh |

## 6. Message states and 480x320 LINK HUD

Keep visible states honest:

- **DRAFT**: composed locally; not committed for transport.
- **QUEUED**: durably stored at sender; recipient has not been reached.
- **IN TRANSIT**: a transport attempt is underway; not a receipt.
- **RELAY STORED**: an authorized intermediate returned a verified durable receipt; final recipient not proven.
- **DELIVERED TO DEVICE**: recipient returned an authenticated, durable message-ID receipt; not a human read/acknowledgment.
- **ACKNOWLEDGED**: a specific authorized recipient explicitly acknowledged.
- **EXPIRED / FAILED / REVOKED**: distinct terminal states with reason; do not silently mark delivered.

Suggested LINK root: **INBOX (n) | OUTBOX (n) | COMPOSE | PEERS | NETWORK** in a compact paged view. Detail view: type, peer, priority, age, actual delivery state, verified transport, a manual retry/cancel, and separate ACK. A small health strip shows **LAN / MESH / RADIO / OFFLINE**, with **UNKNOWN** when there is no measurement. Do not render an aspirational mesh as a live connection. A local urgent label is a scheduling preference, **not a guarantee of emergency delivery**. Never replace established emergency or hospital communications.

## 7. Secure-by-design / no implicit communications

1. **Pair and authorize before sending:** peer identities verified out-of-band, identity keys protected by OS facilities; no automatic trust on discovery. No user-selected role alone grants a network privilege.
2. **End-to-end authenticated encryption for relay use:** intermediate nodes cannot read protected content. Authenticate receipts and reject replayed/expired envelopes. Establish revocation, group-key rotation and key-loss policy before external exposure.
3. **Bound all resources:** per-peer/per-channel quotas, maximum payload, hop/duplicate/expiry budgets, fair queues and transmission duty regulation. Avoid unbounded automatic scanning or retry storms.
4. **Consent and privacy:** no silent GPS, RF probes, network enumeration, health records, recordings or location sharing. Imported situational reports include source provenance and uncertainty, not false certainty.
5. **Store locally but securely:** encrypt mailbox content at rest, use atomic replacement and corruption quarantine, and protect secrets from logs, backups and UI browser history.
6. **Separate infrastructure and operator identity:** an IP route, physical proximity, advertised host name, SSH account or role name does not by itself prove application-level authorization; see NIST SP 800-207.
7. **Field conditions:** document RF shadowing, local network authorization, battery and interference restrictions; no claim of hospital, disaster-response, public-safety or medical compliance from laboratory tests.

## 8. Development gates and objective acceptance

**D0 — research / graph (this document):** current vs future labeled, threat model and transport constraints documented. No runtime feature is implied.

**D1 — typed message foundation:** Rust core schema and migrations, validated payload limits, local encrypted inbox/outbox, message-ID dedup, expiry, provenance and failure-state tests. All networks disabled unless operator chooses a messaging session.

**D2 — direct IP lab:** two physically separate authorized RB nodes, verified pairing, operator-initiated short text and receipt over a permitted LAN without Internet; cross-platform error handling. Current SSH tasks remain unchanged.

**D3 — intermittent and multiple nodes:** three-plus RB instances, store-and-forward with controlled reconnection, restart/power-cut recovery, duplicate/replay rejection, TTL loops bounded, key revoke and clock-disagreement tests; verify no plaintext at a relay.

**D4 — alternate transports:** Linux/Pi 802.11s feasibility only after chipset/driver validation; Meshtastic client adapter and local radio approval; opt-in discovery, band/airtime budget and RF shadow tests. Record actual packet delivery, link age and energy use rather than estimating success.

**D5 — field acceptance:** 480x320 visibility and keyboard accessibility, authenticated device vs human ACK distinctions, explicit non-delivery when disconnected, lab report of delivery and latency distributions under tested RF environments. Security review, threat model, service authorization, and healthcare/privacy review before any live sensitive-data use.

For simulation, include: network partition, no Internet but intact LAN, AP failure, asymmetric link, delayed encounter, disconnected relay, key loss/revocation, malicious sender, duplicate message, full queue, expired message, system restart and unreachable final destination. **Success** is correct, measurable state and eventual delivery *only when a tested contact path returns before expiry*; there is no unconditional delivery guarantee.

## 9. Primary research and interpretation

| Source | Supported decision | Not evidence for |
| --- | --- | --- |
| [IETF RFC 9171 — Bundle Protocol v7](https://www.rfc-editor.org/rfc/rfc9171.html) | DTN overlay and store-carry-forward model | An already BPv7-compliant Rust-Boy implementation or a prescribed routing algorithm |
| [IETF RFC 9172 — BPSec](https://www.rfc-editor.org/rfc/rfc9172.html) | End-to-end integrity/confidentiality for DTN bundles | Automatic key management or protection from local device compromise |
| [IETF RFC 9180 — HPKE](https://www.rfc-editor.org/rfc/rfc9180.html) | Candidate reviewed cryptographic construction | Signed operator identity, group management or implementation correctness by itself |
| [IETF RFC 6763 — DNS-SD](https://www.rfc-editor.org/rfc/rfc6763.html) | Local IP service discovery patterns | Peer authorization or cross-VLAN reachability |
| [Linux Wireless — IEEE 802.11s](https://wireless.docs.kernel.org/en/latest/en/developers/documentation/ieee80211/802.11s.html) | Native AP-less Wi-Fi mesh as a platform-dependent option | Automatic support on macOS or every Pi radio |
| [Bluetooth SIG — Mesh Networking Primer](https://www.bluetooth.com/bluetooth-mesh-networking-primer/) | Relaying, TTL, managed flooding / directed forwarding | Generic BLE being equivalent to Bluetooth Mesh |
| [Meshtastic — Mesh Broadcast Algorithm](https://meshtastic.org/docs/overview/mesh-algo/) | Existing purpose-built LoRa mesh behaviors | Reliable connectivity through all concrete walls or bulk transfer |
| [Meshtastic — Client API (Serial/TCP/BLE)](https://meshtastic.org/docs/development/device/client-api/) | External-radio software integration interface | Native Rust-Boy LoRa functionality without hardware |
| [LoRa Alliance — LoRaWAN Architecture](https://lora-alliance.org/about-lorawan-old/) | Star-of-stars architecture and gateway/server requirement | Equivalence with Meshtastic multi-hop peer mesh |
| [NIST SP 800-207 — Zero Trust](https://csrc.nist.gov/pubs/sp/800/207/final) | No implicit trust based on LAN presence | Automatically compliant identity and access control |
| [HHS — HIPAA Security Rule](https://www.hhs.gov/hipaa/for-professionals/security/index.html) | Administrative, physical and technical safeguards for applicable ePHI | Authorization to process patient data in the current Rust-Boy preview |

**Research conclusion:** start with an encrypted, auditable **message-and-queue abstraction** that can use the existing permitted IP network. Keep other media behind tested adapters, rather than starting with a promise of seamless radio mesh. Local-first safety means both useful offline state **and** accurate communication failure reporting.

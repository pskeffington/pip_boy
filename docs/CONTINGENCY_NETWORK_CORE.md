# Contingency communications core — field architecture

**Status: research and fail-closed policy core committed; no LoRa or alternate-link transmission implemented.** Source matrix: [contingency_transport_matrix.csv](research/contingency_transport_matrix.csv). Existing [PACE plan](COMMUNICATIONS_REDUNDANCY_PLAN.md) and [Messenger security contract](MESSENGER.md) remain authoritative.

## Core topology

The application must distinguish (a) the handheld/host-to-radio link (USB, BLE, serial, TCP), (b) the over-the-air network (LoRa packet mesh, IP Wi-Fi), and (c) the recipient's application-level authentication and persistence. A successful device write or mesh routing ACK is **not** proof that Rust-Boy Messenger persisted a message at the destination.

```text
LINK Compose / Task Queue (no implicit scanning or transmission)
   -> canonical application message and authenticated envelope [SECURITY GATE]
   -> durable queue with expiry, recipient, key ID and dedupe [SECURITY GATE]
   -> operator-approved policy and dependency audit
        A. current pinned OpenSSH (code implemented, field validation pending)
        B. independently powered Ethernet/Wi-Fi / alt IP OpenSSH [PROPOSED]
        C. Meshtastic USB-protobuf radio [RESEARCH]
        D. MeshCore companion USB/BLE radio [RESEARCH]
        E. Reticulum / LXMF gateway [RESEARCH]
        F. offline physical handoff [PROCEDURE]
   -> recipient verifies independent app identity, stores, signs receipt
```

The Rust library module `pipboy-core/src/contingency.rs` defines typed Bearer, PaceTier, Readiness, BearerSpec, eligibility and DeliveryEvidence. It performs **no I/O**. Every bearer defaults disabled; none of the radio or alternate modes is code-available. Its eligibility rule intentionally requires an authenticated end-to-end envelope even for the pre-existing SSH entry: it is a **future routing policy**, not the current Messenger sending path. Do not wire this gate into v1 without a reviewed cryptographic envelope. RelayObservation/LinkAccepted may never become Stored.

## Candidate matrix

| Family | Handheld connection | Dependency | Suited to | Not suited to | Next gate |
| --- | --- | --- | --- | --- | --- |
| Meshtastic | USB serial **first**, later bonded BLE; TCP only on trusted local network | A Meshtastic-compatible LoRa node, correct frequency/modem config, antenna, local RF conditions | Tiny commands, short text and acknowledgments with intermittent relay paths | Bulk maps, large task payloads, high-rate polling, automatic discovery | Protocol version / real firmware fixtures |
| MeshCore | USB companion first, separately paired BLE | Compatible MeshCore companion and repeater ecosystem | Short addressed text / path-aware packet tests | Meshtastic interoperability (protocols differ), implicit E2EE | Pin companion protocol/firmware; isolated adapter |
| Reticulum + LXMF | Locally managed gateway over explicit serial or loopback service | Running RNS/LXMF process and configured interface | Asynchronous messaging via multiple media | Assume native Rust support, auto-connect public peers or forwarding without review | Sandbox gateway, interop fixtures |
| 802.11s / batman-adv / private Ethernet | Normal OS IP socket and existing SSH | Two independent equipped nodes and OS-managed links | Normal-size text and task data with alternate local IP | Guarantee of independent failure domain without power/topology audit | Pinned host-key + physical topology failover |
| Wi-Fi Aware / Wi-Fi Direct | Hardware/OS-specific pairing and IP session | Supported OS, radio and permissions | Short-range phone/handheld direct connection | Universal macOS/Linux/Windows support | Platform-specific validation |
| LoRaWAN | Gateway/server API | LoRaWAN gateway, server and backhaul | Telemetry and regional sensor uplinks | No-gateway P2P mesh substitute | Optional telemetry-only connector, lower priority |

Meshtastic protobuf data payload is **not** arbitrary unlimited application bytes. Published SDK framing describes a 233-byte Data.payload limit and a 239-byte over-air envelope; reserve a smaller application budget (initial planning cap **128 bytes**, subject to actual authenticated envelope overhead and firmware validation). Packet IDs/hop counts/ACKs belong to the Meshtastic radio layer; the app must not compound firmware retries. Do not chunk encrypted text indiscriminately; reassembly should be bounded, authenticated, expiring and tested against partial-message resource exhaustion. MeshCore's companion protocol has independent framing, message-size and key assumptions; do not mix radio identities or shared PSKs across protocols.

## Security posture

- Meshtastic's default channel key is public and cannot be treated as confidential. Shared-channel PSKs permit any key holder to read channel messages. Direct-message encryption, signed broadcasts, ACK proof and AES-CCM behavior depend on current firmware/configuration. No forward secrecy; header metadata remains visible. Verify public identity keys out-of-band. See source CT-01.
- Meshtastic host-to-radio TCP/4403 is not encrypted by PhoneAPI. Never expose it to a hostile LAN or Internet; start with physically controlled USB. BLE requires deliberate bonding and should not be silently discovered.
- LoRa is not an IP connection. Do not run OpenSSH directly over Meshtastic packets or claim a connection-based SSH session can survive a minute-long radio partition.
- App-level E2EE and cryptographically authenticated receipts remain **unimplemented**. Until independently reviewed, do not forward confidential field messages over research radio connectors. Do not claim an authenticated envelope simply because a channel or device is encrypted.
- Radio duties/airtime, frequency, power and permissible encryption depend on jurisdiction, license and hardware configuration; some amateur radio services prohibit obscured messages. Regulatory review and authorized test location are mandatory.
- Disable default position broadcasting and gateway/MQTT forwarding for private deployment unless a separately consented, reviewed profile explicitly permits them. GPS and nearby node IDs are privacy-sensitive metadata.
- A provider-wide cellular outage can also affect emergency gateways; Wi-Fi and Ethernet may share a switch. Track actual independent power/backhaul, not marketing labels.

## Adapter contract and implementation slices

**C1 — Pure core (implemented, tests):** bearer catalog, readiness and fail-closed eligibility, delivery evidence taxonomy. No code path sends packets or accesses serial devices. Introduce a future interface (not yet released) with named methods: `capabilities()`, `health(passive)`, `connect(approved_peer)`, `send_authenticated_envelope()`, `receive()`, `cancel()`. Never expose unauthenticated send.

**C2 — Meshtastic host adapter (design gate):** detect only an operator-selected OS serial path, no automatic BLE or network probing. Pin protobuf schema/firmware versions. Decode ToRadio/FromRadio strictly; use a dedicated application PortNum **only where registered/authorized**, otherwise a documented integration channel. Validate maximum packet length, node-to-device identity binding and recipient ID; no auto-forward. Await signed app receipt. Reject oversize rather than silently truncating. Firmware manages RF retries.

**C3 — Alternative lab adapters:** MeshCore companion (separate protocol parser), RNS/LXMF loopback bridge, Wi-Fi Direct/Aware as platform-specific connectors. No cross-adapter identity assumption. Perform interoperability tests with exact firmware, frequencies, USB descriptors and OS API versions. Reticulum discovery can announce automatically and must be explicitly disabled for offline-first testing.

**C4 — Transport policy:** route scoring by independent failure domain, measured link age, airtime budget, battery budget, tested path, confidentiality approval and deadline. Manual fallback only at first. Enforce rate limits, TTL, backoff, maximum hops, bounded reassembly, epoch validity, cancellation and duplicate prevention. A local radio ACK does not advance delivery to receiver-stored.

**C5 — Physical acceptance:** bench two nodes in isolated RF and Ethernet/Wi-Fi environments. Test disconnected Internet, disabled access point, no GNSS, no gateway, 10/60-minute partitions, 0/5/20/50% loss, competing traffic, oversized frame, bad key, replay, cold restart, USB unplug and battery reduction. Record each hop's ACK separately from remote stored receipt. Ensure 480×320 HUD exposes OFFLINE / RADIO LINK / QUEUED / STORED / NO PATH unambiguously.

## Minimal release tests before enabling a radio button

1. Authenticated application identity is bound to pre-approved operator-trusted peer, independent of Meshtastic or MeshCore node number.
2. No packet leaves a device without explicit enrollment and operator consent; no automatic discovery, MQTT bridge, or position telemetry.
3. Validated and bounded packet size, rate, airtime, reassembly memory, TTL and queue entries.
4. Outbound message encrypted/authenticated **before** crossing a relay; inbound message not displayed or stored until verified.
5. Tested real device-to-device delivery, not just host-to-radio write success; sender stores durable acknowledgment only from intended final recipient.
6. Known legal RF configuration and evidence of physical compatibility.
7. Bench + field performance logs, exact version pins and explicit failures for offline/no route.

## Priority

**First** independently powered Ethernet / local Wi-Fi with existing pinned SSH (fastest path with verified primitives). **Second** Meshtastic via USB after crypto-envelope gate, as a *short-message contingency*. **Third** MeshCore and Reticulum via separately reviewed adapters. Keep LoRaWAN in the telemetry track because gateway/backhaul requirements undermine a no-infrastructure fallback. No software can guarantee RF propagation through metal infrastructure or deep tunnels.

## Implementation roadmap

See [communications development roadmap](COMMUNICATIONS_DEVELOPMENT_ROADMAP.md) for M00–M12 sequencing, workstreams, immediate implementation passes, and verification criteria.

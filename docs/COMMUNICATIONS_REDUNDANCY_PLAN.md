# Rust-Boy Communications Redundancy Plan

**Status:** evidence-based design and acceptance roadmap, NOT an implemented multi-link communications system. The working Messenger v1 baseline is manually initiated, pinned OpenSSH over an available IP connection, with a bounded local queue, explicit retry and receiver-storage acknowledgment. It is **not E2EE** or encrypted at rest. See [Messenger](MESSENGER.md) and the [source-by-source literature matrix](research/communications_redundancy_lit_matrix.csv).

## Evidence framework

CISA's Primary/Alternate/Contingency/Emergency (PACE) guidance requires feasible, suitable, distinguishable and complete backup methods; distinct links should not share their most likely failure points (COM-01/02). NIST SP 800-34 and SP 800-53 CP-8 emphasize documentation, single-point-of-failure review and alternate telecommunications (COM-03/04). CISA NECP calls for governance, training and exercises as well as technology (COM-05/11). RFC 4838/9171 define disruption-tolerant store-and-forward architecture, RFC 9172 covers bundle security, RFC 9420 informs asynchronous group E2EE, and NIST SP 800-57 informs key lifecycle (COM-06–10). These are **engineering references, not proof of functionality**.

## Proposed PACE matrix

| Tier | Candidate | Failure dependencies | Status | Trigger |
| --- | --- | --- | --- | --- |
| **P / primary** | Pinned OpenSSH Messenger over the existing trusted Ethernet/Wi-Fi IP path | Device, SSH service/key, Wi-Fi AP/router/switch, power | **Implemented in code**; real Mac–Pi field validation pending | Authenticated receiver unavailable / transport timeout |
| **A / alternate** | Independently provisioned cellular or second physical IP connection, still using strict SSH peer authentication | Carrier, modem, battery, cell tower/backhaul; verify independence from primary | **Proposed**, no failover adapter | Primary unavailable; operator approves separate healthy route |
| **C / contingency** | Opportunistic store-and-forward with an approved, lawful direct link or mesh adapter | Local radio propagation, airtime rules, power, peer availability | **Research only**; no operational mesh or RF messenger | Both IP paths unavailable; secure envelope and adapter validated |
| **E / emergency** | Authorized physical rendezvous, hand-carried encrypted export or human communications SOP | Personnel, physical access, device power and chain of custody | **Procedure only**, no export tool | All electronic communications unavailable |

**Independence cannot be inferred from two network interfaces.** Cellular and fixed Internet may share upstream power/backhaul; Ethernet and Wi-Fi may share one router. Create a failure-domain inventory listing physical route, ISP, provider, local access point, tower, power, cloud/key dependency, shared fiber, and device storage. Where every link fails in tunnels, underground spaces or shielded areas, show **NO PATH**, never fictitious signal.

## Proposed architecture

1. LINK UI composes a canonical, bounded message with source, destination, unique ID, created/expiry time, priority and schema version.
2. **Before enabling untrusted relays**, a reviewed end-to-end authenticated envelope, offline identity verification and secure device-key lifecycle must protect content irrespective of transport. Existing SSH alone does not meet this requirement.
3. A durable queue persists, deduplicates and retains original content after abrupt restarts. A message never becomes delivered merely because a first hop accepted it.
4. A transport policy interface exposes capabilities, health, send, receipt and disable with per-adapter authorization and bounded retry. The operator can explicitly stop a route.
5. The PACE selector uses **measured** availability and audited physical independence, not just Wi-Fi/cellular labels. No background scanning or radio transmission is enabled by opening LINK.
6. The final receiving device validates identity and message integrity, atomically persists the message and returns a receiver-bound stored receipt; separate human-read acknowledgment remains a distinct event.

RFC 9171 is an interoperability specification, not a synonym for any proprietary packet queue. Do not advertise BPv7/BPSec conformance until exact interoperable framing, convergence layers and key security profiles are tested. Do not assume MLS RFC 9420 supplies transport routing or compromised-endpoint security.

## Fault and security acceptance matrix

| Failure case | Required behavior | Proof |
| --- | --- | --- |
| Router/AP failure | Queue remains durable; alternate route offered only when authorized | Controlled AP shutdown + recover to verified receiver |
| Common carrier/backhaul outage | Both correlated routes marked unavailable | Provider dependency fault injection |
| Network partition / delayed peer | Retain TTL-bound envelope, no duplicate inserts on retry | Reorder, resume and 1/10/60 minute outage suite |
| Malicious host key / spoofed peer | Refuse connection; no downgrade or TOFU | Host-key mismatch, unauthorized peer and malicious receipt vectors |
| Replay and route loops | Deduplicate by cryptographically bound origin ID, TTL and hop count | 1000 repeated/altered frame attempts |
| Power loss during persistence | Preserve journal or return explicit failure; do not invent receipt | Kill/restart at each atomic state change |
| Storage exhaustion | Bounded queue; signal full; no silent eviction | Disk full and storage-cap tests |
| Relay compromise | Content unavailable to relay **only after E2EE implementation passes** | Offline ciphertext inspection and independent security review |
| RF blackout underground | Report NO PATH, offer safe manual handoff | Physical basement/tunnel tests |
| Revoked/expired identity offline | Deny sending/acceptance under explicit offline key-validity rules | Stale, revoked and rotated key fixtures |

## Gated roadmap and proposed success measures

**G0 — Literature / inventory (current pass).** Freeze authoritative source roster and compile capability/failure-domain matrix. Clearly label present vs proposed vs tested. Gather actual gateway, antenna, carrier, battery and receiver identities. No transport changes.

**G1 — Core security.** Threat-model identities, metadata, compromised endpoints and offline revocation; select reviewed E2EE library and OS-backed key storage; specify secure envelope, authenticated delivery receipt, TTL and encrypted disk queue. A security review and malicious-frame tests gate deployment.

**G2 — Redundancy adapters.** Build a narrow transport trait with capability discovery *without automatic discovery traffic*, health, authorized send and receipt. First demonstrate **alternate independent IP**. Evaluate RFC 9171/BPSec implementation before any custom DTN claim. RF/mesh adapters remain lab-only until lawful hardware and air-interface approval.

**G3 — Repeatable experiments.** Two isolated devices plus a loss/latency simulator: loss 0/5/20/50%; RTT 50 ms/1 s/10 s; outage 1/10/60 min; constrained 1–10 kb/s capacity; corrupted messages, duplicate storms, clock skew, battery shutdown and storage exhaustion. Report receiver-persisted delivery rate, P50/P95 latency for *delivered* items plus undelivered fraction, duplicate count, unauthorized accepts, energy use, queue survival and on-air bytes. Record actual hardware and firmware and archive raw results.

**G4 — Field readiness.** Test Mac↔Pi host-key enrollment, physical connectivity and reverse pairing; verify emergency no-link SOP, 480×320 readability and accessibility, human drills, authorization/retention policies and independent security review. Never present a proposed tier as live.

**Proposed release assertions, not measured outcomes:** zero unauthorized messages accepted, zero false delivery/read receipts, no silent restart loss in fault suite, no unprompted link discovery or transmissions, and documented failure-domain independence for each available tier. Availability percentages are **not** estimable without physical trials and realistic outages.

## Actionable engineering checklist

- [x] Authoritative research matrix with traceable source URL, limitation and requirement mapping.
- [x] PACE deployment scenario and shared-failure warnings.
- [x] Proposed message/adapter invariants and fault test plan.
- [ ] Verify source versions/errata at implementation time.
- [ ] Inventory actual device interfaces, antennas, service providers and power domains.
- [ ] Security architecture review: authenticated E2EE and encrypted queue.
- [ ] Secure key enrollment, rotation, revocation and offline expiry policy.
- [ ] Independent alternate-IP adapter, manually confirmed.
- [ ] DTN/mesh laboratory prototype, with no default RF transmission.
- [ ] Physical no-link/tunnel and power-outage test evidence.
- [ ] Documented operator drills and release signoff.

**Scope note:** All radio operation requires applicable jurisdictional approval and compatible equipment. No emergency-services interoperability or RF capability is presumed by the presence of this document.

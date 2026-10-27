# Hybrid Rust-Boy field architecture: peer nodes + optional mini-server

**Status: topology design and deterministic, fail-closed simulation only. NOT a deployed mesh, authenticated service, persistent relay, or validated emergency channel.** Existing per-device OpenSSH pairing and manual task/message transfer remain the live compatibility baseline. The separate `rust-boy-broker` is a deny-all physical device broker, **not** a field networking server.

## Decision

Each field Rust-Boy owns its local queue, journal, identity, assignments and offline UI. A lightweight optional `rust-boy-relay` mini-server can later coordinate reachability, relay encrypted envelopes and retain bounded undelivered ciphertext; it must never be a command authority merely because it is online.

```text
                  DESKTOP VAULT COMMAND
                (authority after verified enrollment)
                         |
              OPTIONAL MINI-SERVER
             (store/forward and rendezvous)
                  /      |      \
               RB-A ---- RB-B    RB-C
                  \______/
       direct links only when mutually enrolled and authorized
```

No central server is required for locally stored assignments or operation of the HUD. A mesh requires a **real bearer** (IP LAN, hotspot, separately reviewed Wi-Fi Direct or Meshtastic hardware), not just a graph of software peers. Mesh radio and generic IP streaming have different bandwidth, duty-cycle and latency requirements.

## What to build

1. **Field agent** (Pi/Mac/other device): opt-in background agent with bounded authenticated listener, local queue, unique protected key, deny-by-default APIs, disconnect/rejoin, and resource budgets. Do not expose current loopback dashboard to LAN.
2. **Direct peer path**: keep known-peer pinned OpenSSH for v1, then introduce mutual device authentication and separate signed membership/role grants. Discovery only lists candidates; it never trusts them.
3. **Mini-server**: deployable Rust daemon on a Pi/desktop/VM, optional and replaceable. Allow no browser access to secrets. Use bounded TLS-protected ingress; encrypted-at-rest payloads, TTL, quotas, per-org isolation, audit, admin-controlled revocation, metrics and graceful restart. Do not centralize private keys or enable blanket database replication.
4. **Transport selection**: direct wired/LAN > mutually approved hotspot > managed relay > offline local hold. Low-bandwidth radio requires its own strictly scoped minimal alert protocol, replay protection and hardware/legal testing. No attempt to tunnel raw STREAM video over mesh radio.
5. **Sync v2**: authenticated event envelopes with source/destination, org, dataset scope, sequence, immutable event ID, content digest, signed/verified origin, replay ledger, idempotent durable receipt, bounded retry/backoff and per-dataset conflict policy. A delivered packet is not a human-read receipt.
6. **UX**: desktop Settings > Devices lists enrolled peers and honest last-verified state; COM > Links allows explicit connect/retry. Field unit shows paired command device and queue count. The five handheld primary tabs remain STATUS / STREAM / COM / DATA / RADIO plus inline Settings gear; do not add a sixth major page.

## Failure modes and fallback

| Condition | Expected behavior |
| --- | --- |
| Command desktop offline | Field unit retains tasks and records progress locally; no fake live status |
| Relay unavailable | Direct approved peer link if present; otherwise hold |
| Peer on same Wi-Fi but not enrolled | Ignore for data exchange; optionally show untrusted candidate during explicit discovery |
| Spoofed discovery or IP | Never changes pinned identity or dataset grants |
| Revocation freshness unavailable | Hold data requiring fresh membership; do not silently downgrade |
| Network partition and duplicate messages | Deduplicate by authenticated event identity; never last-write-wins across assignment owners |
| Pi reboot mid-transfer | Recover journal and retry without counting unacknowledged packets as stored |
| Restricted/regulated data | Block until a separately validated institutional security release |

## Initial acceptance gates

- [x] A deterministic route-selection simulator that defaults to offline hold and rejects unverified peers, stale revocation and protected data.
- [ ] Dedicated opt-in field agent protocol with real mutual identity.
- [ ] Independent mini-server that cannot authorize commands.
- [ ] Transport-level authentication, anti-replay, secure key storage, and revocation tested on devices.
- [ ] Two field devices and a desktop exchange public-demo assignments without a relay.
- [ ] Add relay, take it offline, restore it, and reconcile idempotently.
- [ ] Mac/Pi hardware, constrained RF environment, accessibility and security review.

The pure simulator in `pipboy-core::field_topology` is not connected to live transport or authorization and must never be used as an allow decision.

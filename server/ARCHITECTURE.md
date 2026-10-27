# Mini-server and field-node architecture

```text
VAULT COMMAND (desktop)  -- enrolled, authenticated IP --> FIELD RB-A
          |                                              |
          +-- optional mini-server [opaque queue / relay] +-- trusted direct FIELD RB-B
                       (not an assignment issuer)                     |
                                                                   offline journal
```

The portable field runtime must continue operating when all links fail. Peer-to-peer networking requires an actual underlying bearer; a web UI, IRC room, or routing table alone does not create a mesh.

## Security boundaries

- Server remains optional and separate from the loopback-only dashboard and physical-device broker.
- Mini-server may store or forward only per-destination, authenticated, authorization-checked **opaque** messages; no raw task database replication, no arbitrary HTTP proxy, and no trust-on-first-use enrollment.
- Devices hold their own unique keys and journals; only a properly enrolled Command operator/device can issue assignments, and receiver policy is authoritative.
- Discovery, IP address, mDNS, RF proximity, or a server route are reachability hints, not identity.
- Per-device storage, TTL, replay, quotas, operator audit, revocation freshness, safe clock handling and recovery must be verified before production.
- Current `server/mini-server` is deliberately a **no-socket scaffold** and never reports an online peer.

## Observability

Display separately: configuration state, last authenticated check, transport reachable, remote durable receipt, human acknowledgment and latest sync cursor. On startup show unavailable/unconfigured instead of a fabricated LIVE status. COM contains device links and queue health; Settings contains enrollment and secrets; DATA holds assignments. Maintain the five-tab handheld rail.

## Release constraints

Gate real network listeners behind explicit opt-in deployment configuration and negative security testing. Avoid exposing the existing dashboard on a LAN as a shortcut. Do not send STREAM video over constrained radio or claim emergency availability without hardware tests.

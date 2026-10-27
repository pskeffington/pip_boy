# Rust-Boy Server — dedicated deployment workspace

This directory owns the optional **mini-server/relay**, independent of the existing Rust-Boy localhost dashboard and the deny-all physical `rust-boy-broker`.

## Directory index

| Path | Responsibility |
| --- | --- |
| [mini-server](mini-server/) | Minimal fail-closed Rust daemon scaffold. No listening socket, peer forwarding, user authentication or background work exists yet. |
| [mining communications matrix](MINING_COMMUNICATIONS_MATRIX.md) | High-priority NIOSH/NIST/MSHA underground technology and fault tests |
| [literature matrix](LITERATURE_MATRIX.md) | NIST, hospital, tunnel and distributed communications source-to-test crosswalk |\n| [build priorities](BUILD_PRIORITIES.md) | Ranked executable engineering backlog and first three increments |
| [roadmap](ROADMAP.md) | Ordered phases, dependencies, gates, deliverables and acceptance evidence |
| [architecture](ARCHITECTURE.md) | Field agent, optional server, peer routes, failure modes and data ownership |

Corresponding documentation: [organizational sync](../docs/RB_SYNC_ROADMAP.md), [peer SSH baseline](../docs/PEER_TASK_TRANSPORT.md), [architecture](../docs/ARCHITECTURE.md) and [documentation index](../docs/README.md).

Run the inert server scaffold with:

```sh
cargo run -p rust-boy-mini-server -- status
cargo test -p rust-boy-mini-server
```

A status report proves the binary runs, **not** that it is a functioning service. Deployment, port exposure, enrollment and protected data are gated separately. Keep credentials and journal contents out of Git. No general hospital or emergency data in development fixtures.

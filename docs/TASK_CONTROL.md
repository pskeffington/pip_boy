# Rust-Boy | Overseer Task Control

Task Control is the Overseer's preloaded **management** module and is also available by explicit Custom pin. It creates, prioritizes and reassigns operational work across the seven job titles and tracks the work's life cycle on the manager's device. Other job HUDs display only their locally recorded assigned tasks. A job profile is an operational view, **not a login identity or security privilege**.

## Assignments and device delivery

1. Open `./scripts/launch-micro.command` on Mac or the desktop dashboard. Select Overseer and **TASK CONTROL**. On an installation with an older saved Overseer profile, deliberately reload its profile to receive the new tool pin.
2. Create a task with title, assigned operator, job station, priority, due date and instructions. It is **local and unsent** until explicitly dispatched.
3. Under the task detail, choose an enrolled peer from **PAIRED RUST-BOY** and click **SEND TO SELECTED DEVICE**. A separate confirmation is required; it invokes OpenSSH with pinned host-key and client-key checks.
4. When the receiver has actually persisted the assignment, the Overseer records the peer alias, remote task ID and time as **delivered**. This is not yet human acknowledgment. Offline/failed connections do not mark delivery.
5. On the recipient device, open the assigned job queue, **ACKNOWLEDGE RECEIPT**, and update work status. The recipient stores its own copy with a source reference and distinct local ID.
6. The Overseer explicitly clicks **REFRESH REMOTE STATUS** to fetch receiver-reported acknowledgment and status over SSH. The manager's local status is intentionally not overwritten.

There is **no email handoff** in the new task UI, no outbound task broadcast, no automatic peer discovery and no background polling. The older local manual handoff API remains for backwards-compatible v1 records, but the supported active route is **device-to-device**.

The complete [Device-to-device setup and security guide](PEER_TASK_TRANSPORT.md) documents the two-device Mac/Raspberry Pi pairing, restricted SSH bridge, host fingerprint verification, delivery model, data limits and tests.

## Data safety

The independent JSON task store is private to the current OS account but **unencrypted at rest**. Its schema, event history, revision checks and atomic writes preserve local state when a peer is unavailable. Never put sensitive patient data, passwords or protected incident evidence into the development task board. Current role labels do not authenticate humans. The SSH receiver should be a dedicated low-privilege account with a key-level `restrict,command="... peer bridge SOURCE"` restriction.

### Acceptance

```bash
cargo test --workspace --locked
node --check crates/pipboy/assets/tasks.js
python3 tools/smoke_tasks.py
python3 tools/smoke_peer.py
```

The peer smoke test uses a local replacement for the SSH executable and demonstrates two separate Rust-Boy data roots; it does **not** certify real OpenSSH network authentication. Cross-device live hardware acceptance remains a separate release gate.

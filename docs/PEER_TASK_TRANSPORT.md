# Rust-Boy | Device-to-device task assignments over OpenSSH

**Status:** an explicit, operator-initiated Rust-Boy-to-Rust-Boy task transport for macOS and Linux/Raspberry Pi. Windows compilation is covered by CI, but Windows SSH path handling and physical peer delivery are not yet validated. The native task storage, sender/receiver envelope, idempotent import, receipt validation and progress reconciliation are implemented. **No auto-discovery, background connection, relay, BLE/mesh or new listening service is enabled.** Physical Mac-to-Pi SSH integration and hardware UX still require field acceptance.

## How it works

```text
OVERSEER RUST-BOY (Mac)                     SCOUT RUST-BOY (Pi)
+ CREATE TASK in Task Control               Own tasks.json / own workbench.json
+ choose paired peer alias                  SSH server with dedicated key
+ CONFIRM SEND                              authorized_keys forced Rust-Boy bridge
  -> Rust-Boy launches OpenSSH  ---------->  pipboy peer bridge overseer-a
     pinned host key + client private key    checks SSH_ORIGINAL_COMMAND allowlist
     no agent / forwarding / password        validates sender label and packet
     bounded JSON assignment on stdin       inserts unique inbound task atomically
  <---------- SSH stdout receipt ----------  returns stored receiver task ID
+ records verified storage receipt          recipient sees local job queue
  "DELIVERED" != "ACKNOWLEDGED"              + ACKNOWLEDGE and update status
+ explicit REFRESH REMOTE STATUS ---------->  SSH reports from receiver
  remote_progress != local status
```

A **task is not sent by creating it**, selecting a role, opening Task Control, loading a dashboard, or reading a task. Only the operator's `SEND TO SELECTED DEVICE` confirmation (or CLI `peer send ... --confirm`) launches SSH. **Receipt means the trusted SSH destination returned a durable storage identifier, not that a person accepted the work.** Actual operator acknowledgment requires an action on the receiving Rust-Boy followed by an explicit sender refresh. No background polling is implemented.

### First-time pairing checklist (Mac → Raspberry Pi)

Both devices must have Rust-Boy installed and available as a local executable. On macOS and Pi, OpenSSH client/server must be installed separately. SSH is the actual encrypted/authenticated network transport; Rust-Boy **does not operate a TCP listener for task traffic**. Follow your OS's SSH server setup and use a dedicated, least-privilege account on the Pi.

**On the Pi (receiver):**

```bash
pipboy peer init scout-one --confirm
pipboy peer trust overseer-a --confirm
```

Prepare a **dedicated public key** for Rust-Boy delivery in the Pi account's `~/.ssh/authorized_keys`. Restrict that key so it cannot run a shell, forward ports or open PTYs. Example, replacing the executable path and key material with your own:

```text
restrict,command="/absolute/path/to/pipboy peer bridge overseer-a" ssh-ed25519 AAAA... rustboy-commander
```

The **bridge** checks the original requested remote command against exactly two operations: `pipboy peer receive` and `pipboy peer receipts overseer-a`. It additionally requires the incoming packet's `from_device` to equal `overseer-a`. Every sender should use its own restricted SSH key. The peer's `trust` label is **not** a credential on its own; SSH public-key authentication plus the forced command provide the operator/device boundary. Keys with unrestricted remote-shell access should not be used for this task channel.

**On the Mac (Overseer):**

```bash
pipboy peer init overseer-a --confirm

# Generate a distinct sender key (the private key remains on the Mac).
ssh-keygen -t ed25519 -f "$HOME/.ssh/rustboy_scout_ed25519"

# Obtain the Pi host public key through a trusted administrator/channel.
# ssh-keyscan may *collect* it but does not authenticate it by itself.
ssh-keyscan -t ed25519 scout.local > "$HOME/.ssh/rustboy_known_hosts"

# Compare this fingerprint out-of-band with the fingerprint shown directly
# by the Pi administrator, before trusting it:
ssh-keygen -lf "$HOME/.ssh/rustboy_known_hosts"

pipboy peer add scout-one pi@scout.local \
  "$HOME/.ssh/rustboy_scout_ed25519" \
  "$HOME/.ssh/rustboy_known_hosts" --confirm

pipboy peer list
```

On the receiver, compare the server fingerprint from `ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub` using a trusted console, not just the same untrusted network. Copy only the *public* sender key into the Pi `authorized_keys` line above; **never copy the private key**. The Mac's key and pinned known-hosts paths must be absolute. Spaces or shell metacharacters in these paths are intentionally rejected for conservative SSH option parsing. When SSH requires a passphrase prompt, BatchMode refuses rather than falling back to an interactive/password or agent login; provision an appropriately protected dedicated key.

**Device aliases match receiver identities:** the Mac alias `scout-one` must equal the Pi's `peer init scout-one` ID. Remote `user@host` is restricted to ordinary SSH username and DNS/IPv4 characters; no URL, command or flags can be inserted as a destination.

### Send and receive

On the Mac, open either `./scripts/launch-micro.command` or `pipboy dashboard --open`. **Overseer → TASK CONTROL → New assignment → choose assigned role → Send to Selected Device**. The paired device selector appears only after the CLI enrollment above. The operator confirms the connection separately. The UI will show delivery only after the receiving Rust-Boy confirms a persisted task.

CLI alternative:

```bash
pipboy peer send 1 scout-one --confirm
```

On the Pi, select the job that matches the assignment (for example `pipboy workbench use reclamation-scout --replace`). Its micro **STAT / Assigned Duties** queue shows the incoming work on that Pi. Open the assignment, click **ACKNOWLEDGE RECEIPT**, then use the task status controls to report `in_progress`, `blocked` or `done`. These actions are written to the receiving Pi's **own** task board, not the Mac's.

Back on the Mac, explicitly click **REFRESH REMOTE STATUS** or run:

```bash
pipboy peer sync scout-one --confirm
```

The Overseer then sees the latest receiver-reported status and whether the operator acknowledged. The local managerial status remains untouched, and stale browser revisions are rejected. There is no automatic sync, recipient discovery, broadcasting, remote shell access from dashboard controls or email draft.

### Storage and packet contract

```text
PIPBOY_HOME/
  peers.json        # local ID, trusted source labels, peer alias/SSH paths (not key bytes)
  peers.lock        # cooperative, process-local peer registry lock
  tasks.json        # local assignments, origin, delivery and remote progress
  tasks.lock        # independent task-board lock
  workbench.json    # active job template and pins only
```

The receiving device reads a bounded 8-KiB, versioned JSON `RemoteAssignment` on SSH stdin; validates input, trusted source label, type, role, due date and size; and saves only one local task for a given sender/device + source task ID/revision. An exact resend is idempotent (the receiver's local progress remains untouched); a mismatched upstream revision is rejected rather than overwriting work. The sender validates the SSH response version, claimed receiver device, source device, source task ID and nonzero stored ID **before** recording device delivery. The 64-KiB response cap, session timeout and no forwarding/agent controls bound the sender process. The restricted bridge is the recommended recipient authorization layer.

Stored task state distinguishes **local task status**, **receiver-stored delivery**, **manually acknowledged inbound work**, and **explicitly synchronized remote progress**. Only the receiving device can mark its own human acknowledgment; the sender cannot silently infer it. A disconnected Pi leaves the original tasks unchanged.

Task and peer files are plaintext (Unix task/peer files are `0600`, directories `0700`), **not encrypted at rest**. Do not store credentials, identifiable clinical notes, or secret location/intelligence data. SSH protects the transmission when host keys and account keys are verified, but it does not make a compromised destination safe.

### Security limitations / next engineering gates

- No verified hardware/software attestation; the SSH host key identifies the server, and a dedicated public key + forced bridge scopes which Rust-Boy operations that sender may execute. A human's identity is not independently established.
- Same local account processes can access plaintext files and switch job role; job names are not access-control roles. A dedicated receiver OS account and key-level restrictions are recommended.
- SSH host key enrollment is **manual and out-of-band verified**, not TOFU/auto-accept. Losing/rotating SSH keys requires explicit administrator action.
- No automatic status push, queues for offline outbound devices, peer discovery, live camera/video or TAK radio bridge. A remote device must be running SSH and a compatible Rust-Boy executable.
- Still requires real Mac-to-Pi testing: correct key restrictions, different networks, loss/retry cases, power interruption, low-res UI controls and negative host-key tests. The CI test's local substitute for SSH does **not** prove live network encryption or host fingerprint validation.

### Test

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --locked
python3 tools/smoke_peer.py
```

The deterministic smoke relay starts two **separate Rust-Boy homes** and tests explicit pairing, denied untrusted sender, strict SSH flags, sender/receiver durable storage, exact resend idempotency, receiver's job-specific queue, acknowledgment, progress and on-demand sender reconciliation. On POSIX, the SSH executable is *replaced by a strict local relay* for testability. Real OpenSSH cryptographic verification is delegated to SSH and requires field acceptance. See [OpenSSH ssh_config(5)](https://man.openbsd.org/ssh_config) and [OpenSSH authorized_keys documentation](https://manpages.debian.org/trixie/openssh-server/authorized_keys.5.en.html).

## Future RB fleet synchronization

The organization-aware [RB <-> RB synchronization roadmap](RB_SYNC_ROADMAP.md) proposes opt-in nearby discovery, administrator-issued device identities, signed/scoped enrollment, bounded event-delta reconciliation and a future shared command-center fleet view. **None of those new capabilities are active.** The current one-way OpenSSH trust and manual key/fingerprint ceremony remain authoritative until each future gate passes. Proximity and common IP/subnet are never enrollment credentials. Current task and peer stores remain plaintext and must not hold PHI or regulated institutional data.

## LINK Messenger

The same pinned OpenSSH transport now supports a separate **Messenger** operation. The dedicated authorized_keys forced bridge additionally permits exactly `pipboy peer message-receive`, bound to its named source key. Messages live in a separate bounded plaintext journal rather than the task board. See [Messenger operator guide and security limitations](MESSENGER.md). One-way task pairing is sufficient to send a message in one direction; receiving a reply requires reverse pairing.

# Rust-Boy Messenger | LINK

**Future communications resilience:** [PACE fallback architecture and source matrix](COMMUNICATIONS_REDUNDANCY_PLAN.md). Alternate IP, DTN/mesh, emergency transfer and E2EE are planned, not current Messenger capabilities.

## Released secure-transport scope

Messenger v1 provides peer-to-peer text conversations in handheld LINK and the desktop dashboard, using the **existing pinned OpenSSH device-to-device channel**. There is no cloud chat provider, direct Internet chat service, automatic background connection, discovery, inbound web listener, or remote message poll. A paired peer must be chosen and an operator must explicitly press SEND or RETRY.

**Security boundary:** pinned SSH host-key verification, dedicated sender SSH identity, strict OpenSSH command flags, and a receiver-side forced command tied to the exact sender device label. The receiver validates both source and destination device IDs. Receive-only devices can view inbound conversations but cannot SEND until reverse SSH pairing is complete. It persists the message before issuing a typed, sender/receiver-bound receipt. Replays of the same original ID and content are idempotent; changed-content replay is rejected. Successful receipt means STORED ON REMOTE DEVICE; it does **not** mean read by a human.

**This is NOT end-to-end encrypted and NOT encrypted at rest.** SSH encrypts message traffic on the authenticated device-to-device connection. The local message journal is plaintext, with restricted Unix file/directory permissions and platform-dependent Windows ACL inheritance. The local loopback UI does not independently authenticate OS users; job templates are not authorization roles. A compromised account, receiver, or privileged local process can read chat history. Do not send secrets, health records, sensitive location information or classified material with this version.

## Enrollment: Mac Overseer → Raspberry Pi Scout

The receiver needs Rust-Boy, OpenSSH server and a dedicated least-privilege user account. Example **on the Scout**:

```bash
pipboy peer init scout-one --confirm
pipboy peer trust overseer-a --confirm
```

Place only the sender's **public** key in the Scout account's authorized_keys, forced to one Rust-Boy bridge:

```text
restrict,command="/absolute/path/to/pipboy peer bridge overseer-a" ssh-ed25519 AAAA... rustboy-overseer
```

On the **Overseer**:

```bash
pipboy peer init overseer-a --confirm
ssh-keygen -t ed25519 -f "$HOME/.ssh/rustboy_scout_ed25519"
ssh-keyscan -t ed25519 scout.local > "$HOME/.ssh/rustboy_known_hosts"
ssh-keygen -lf "$HOME/.ssh/rustboy_known_hosts"
pipboy peer add scout-one pi@scout.local \
  "$HOME/.ssh/rustboy_scout_ed25519" \
  "$HOME/.ssh/rustboy_known_hosts" --confirm
```

**Verify the collected host fingerprint out-of-band against the recipient administrator's trusted console** (for example with ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub). ssh-keyscan is merely collection, not authentication. Never use unverified or auto-accepted host keys. A key requiring interactive unlock cannot be used by the current batch-only transport; protect its OS storage accordingly. To exchange replies, perform **separate reverse pairing** using a distinct Scout sender key and trust scout-one on the Overseer. Do not copy private keys to the recipient.

## Operator workflow

Open **LINK** on the handheld (Security Chief: press MESSENGER / SSH), or the desktop Messenger panel. Select an enrolled peer, type up to **1,024 UTF-8 bytes**, and press **SEND VIA SSH**. The text is queued locally before a single SSH attempt. If the recipient verifies and stores it, delivery becomes STORED; otherwise it remains QUEUED and must be retried by an explicit operator click. REFRESH reads the **local** journal only. No background retries, push notifications, read receipts, group rooms, attachments, message deletion or live presence.

For CLI, text arrives via stdin rather than an argument which could be recorded in shell history:

```bash
printf 'Station ready' | pipboy peer message-send scout-one --confirm
pipboy peer message-retry 1 scout-one --confirm
```

Storage is PIPBOY_HOME/messages.json (default ~/.pipboy/messages.json), with independent messages.lock. The journal refuses unrecognized/corrupt data without reset; atomic writes avoid partial replacement. Limits: 192 retained records and 256 KiB total; no silent pruning, no implicit remote request or reconnect. Sender timestamps are untrusted; receiver assigns its own local arrival time. Message text is displayed using textContent and refuses terminal-control and bidirectional-spoofing characters.

## Architecture and follow-on security gates

The existing forced bridge now permits three precisely matched operations: task receive, sender-specific task receipts and message receive. The messenger is **not** a custom encryption protocol; OpenSSH provides authenticated transport. Trusted sender labels require matching key-level forced-command restrictions for protection against impersonation. The local dashboard uses its existing exact loopback Host / Origin check on writes and strict JSON limits, but **does not offer separate user login or an encrypted message database**.

Before use for sensitive content: design and independently evaluate an E2EE protocol with established libraries, secure OS-managed device keys, identity fingerprints, rotation/revocation, authenticated local sessions, retention/deletion, compromised-endpoint response and human read receipt semantics. OWASP recommends established authenticated encryption and secure key management rather than homegrown ciphers: https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html .

## Verification

Cargo unit tests cover strict text/journal constraints, receipt states, stored-message replay, corruption preservation and invalid destinations. The integration smoke in tools/smoke_peer.py exercises two isolated devices through a **fake local SSH relay** enforcing the restricted remote command. Its success tests protocol and state, **not real network cryptography**. tools/test_messenger_ui.cjs covers explicit browser sends, safe display, queued retries and zero automatic remote contact. Live macOS-to-Raspberry-Pi SSH enrollment, host-key verification, Windows ACLs, data retention, and field delivery still require real hardware validation.

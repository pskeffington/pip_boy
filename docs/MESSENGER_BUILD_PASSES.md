# Messenger build passes A–C

This pass defines the transport contract, integrates it into LINK's existing
OpenSSH path, and adds opt-in encrypted queue storage. It supplies a passive
Meshtastic stream decoder and a recorded-USB capture reader. **Live USB access,
physical interoperability and independent security review remain open.**

## Acceptance status

| Pass | Implemented | Remaining release gate |
| --- | --- | --- |
| A — canonical protocol | Fixed-order v1 JSON, strict bounded decoding, typed durable receipt, `MessageTransport`, integration with existing SSH sender/receiver; deterministic drop/delay/duplicate/reorder/lost-receipt simulation | Transport-specific integration and field loss/latency measurements |
| B — crypto and keys | age 0.12.1 file encryption, separate X25519 storage identity in keyring 3.6.3; atomic encrypted queue; wrong-key/corruption/locked-store fail closed | Independent integration/security review; native OS credential lifecycle tests; recovery, rotation/revocation, migration and rollback policy; E2EE protocol selection |
| C — local-IP fallback | Explicit RFC1918 IPv4 retry; original enrolled host retained as SSH `HostKeyAlias`; typed receipt unchanged; OpenSSH's own `-G` evaluates effective routing/trust options in tests when installed | Real two-host private-network test, wrong-host-key test and independent acceptance sign-off |
| C — passive USB | Bounded fragmented/noisy Meshtastic stream decoder; opaque FromRadio payloads; read-only adapter boundary and regular capture-file CLI | OS-enforced serial broker, generated/pinned protobuf decoding, real USB/firmware interoperability, reset/hot-unplug tests |

Implementation tests do not establish independent human review, an audited
Rust-Boy integration, radio coverage or delivery across physical hardware.

## A: canonical message and transport contract

The existing v1 wire shape is retained so enrolled receivers remain compatible:

```json
{"schema_version":1,"from_device":"overseer-a","to_device":"scout-one","origin_id":1,"body":"Station ready","sent_at":1792972800}
```

The canonical encoder emits exactly these fields in this order, compact UTF-8
JSON with no newline. This is **Rust-Boy's serialization convention**, not RFC
8785 JSON canonicalization and not a signed JSON format. Receivers accept valid
JSON field ordering but reject unknown/duplicate fields, extra JSON, unsupported
versions, invalid IDs/text and frames over 8 KiB. Text is at most 1,024 UTF-8
bytes. Device IDs are the existing bounded `valid_device_id` vocabulary.

Identity is `(from_device, to_device, origin_id)`; the transport authenticates
`from_device` separately from its label. IDs are monotonic within the persistent
sender journal, and every retry retains the same content and `sent_at`. Do not
reset a sender journal and reuse its device label: enroll a new label for a reset
identity. No deletion/ID-reset mechanism is supplied. This is not a generic
cross-epoch deduplication protocol. The recipient records local arrival time;
`sent_at` is untrusted and is not an ordering/expiry authority. New inbound
records retain `remote_sent_at` to reject changed-timestamp retries. Legacy
inbound records without that optional field retain body-based replay behavior.

A successful response has exactly this shape:

```json
{"schema_version":1,"kind":"message_stored","receiver_device":"scout-one","from_device":"overseer-a","origin_id":1,"remote_id":1}
```

`STORED` means receiver persistence, never human reading. Sender commits its state
only after checking version, kind, source, destination, original ID and nonzero
remote ID. A timeout, forged receipt or lost receipt leaves the sender queued.
Receiver storage is idempotent, so an explicit retry after a lost receipt is safe.
An authenticated peer can lie about persistence; the receipt is not remote-disk
attestation.

`MessageTransport::exchange` performs one bounded attempt. It cannot silently
switch recipients, downgrade authentication, discover peers or spawn automatic
retries. `FaultSimulator` uses a virtual clock and explicit delay offsets per
attempt: `[]` drops, `[20]` delays, `[20,21]` duplicates; scheduling another frame
at `[2]` reorders it. A separate flag drops receipts after receiver persistence.
Stable event sequence breaks equal-time ties. Capacity is 256 pending events,
16 copies per scheduling call. No sockets, radio or sleeps are used. These are
message faults, not a model of RF propagation.

## B: opt-in encrypted persistent queue

On a **fresh** Messenger journal, enroll before the first message:

```bash
pipboy peer message-key-init --confirm
```

The command generates an age X25519 identity using the library's randomness and
saves its private string exclusively through the platform credential backend:
macOS Keychain, Windows Credential Manager or Linux Secret Service. It checks a
credential round trip before atomically writing `messages.recipient`, a public
age recipient that identifies the credential. Linux needs a running, unlocked
Secret Service session; a headless Pi without one cannot enable this mode. No
mock keyring, environment private key, private-key export or plaintext fallback
is configured by the application.

The same `messages.json` path now contains a binary age file when enrolled. The
whole message book, IDs and delivery metadata are encrypted before the atomic
temporary file is written; plaintext serialization/decryption buffers use
zeroization on drop. In-memory `Message` strings and UI responses remain visible
to the authorized local process. Cooperative file locks serialize writers and
Unix directory sync follows replacement. Queue limits remain 192 records and
256 KiB of decoded JSON, with 4 KiB allowance for encrypted-file overhead.

Enrollment refuses **any existing journal**, including an empty one, and any
existing enrollment marker. There is no automatic plaintext migration: old
backups, snapshots and storage blocks cannot be erased by rewriting the file.
Existing installations retain their prior plaintext behavior until an explicit,
reviewed migration is available. Missing/locked/malformed/mismatched credentials
or corrupt ciphertext stop enrolled loads and writes. The public marker is not
secret; retain it with the encrypted journal. Key loss can make the queue
unrecoverable. Do not delete a credential to rotate it; old ciphertext would
become unreadable. No rotation/delete convenience command is supplied.

This encrypts **the queue at rest**. SSH continues to authenticate/encrypt the
link using its existing distinct sender and host keys. Those SSH private keys
are still managed by the operator's OpenSSH setup; batch-only transport does not
unlock passphrase-protected keys through an agent. This pass does not add E2EE,
forward secrecy for stored history, sender authentication through age, rollback
resistance, protected UI login, native SSH-key lifecycle management, or protection
against a compromised OS account. An older valid journal can be replayed by an
attacker able to replace local files. Public-recipient file encryption alone is
not proof of who wrote a replacement file.

## C: operator-selected local-IP fallback

After a normal attempt leaves a message queued:

```bash
pipboy peer message-retry-local 1 scout-one 192.168.1.20 --confirm
```

Only RFC1918 IPv4 literals are accepted. There is no discovery, DNS-derived
fallback list, listener or unencrypted LAN protocol. The enrolled username,
private identity file and dedicated known_hosts file remain unchanged. OpenSSH
receives `Hostname=192.168.1.20` for routing and `HostKeyAlias=scout.local` for
the enrolled host identity. `CheckHostIP=no` prevents a second address identity
from replacing the enrollment identity; `StrictHostKeyChecking=yes` still
requires the pinned host key. User SSH config is disabled with `-F none`.
The same session deadline, one connection attempt, restricted receiver command
and application receipt apply. Fallback is CLI-only and explicitly invoked.
No assertion that this private address belongs to the peer is trusted until
OpenSSH verifies the enrolled key.

The test also asks **OpenSSH itself** to evaluate the effective config with `-G`
when the executable is installed. This validates option interpretation against
an independent implementation. It does **not** perform a network handshake or
constitute independent security sign-off.

## C: passive Meshtastic USB boundary

`meshtastic_passive::read_once` accepts only `Read`, with a 516-byte buffer. The
incremental decoder recognizes `94 c3`, a two-byte big-endian payload length and
1–512 payload bytes, following the upstream Python stream interface. It
resynchronizes around noise and oversized/zero-length frames. State is bounded
by one frame. Framing is not protobuf or cryptographic validation. Frames remain
**untrusted opaque FromRadio bytes**, do not enter the message book and cannot
produce chat receipts.

An operator can inspect a previously recorded binary USB capture:

```bash
pipboy peer meshtastic-capture /absolute/path/usb-capture.bin --confirm
```

The CLI accepts a regular file, rejects a directly named device/symlink and
bounds input to 1 MiB. It reports only frame count; no payload, node database or
location is printed or retained. This is not a protected file-reader sandbox
against hostile same-account file replacement. It never enumerates or opens a
serial port, sends wake bytes/config/heartbeat/ToRadio, joins a mesh or changes a
radio's state. It does not use upstream `SerialInterface` constructors, which
can write during connection. A real adapter must receive a broker-authorized
read-only stream rather than acquire a port itself. The existing deny-only
broker stays unchanged. Silent passive capture is possible if firmware emits
nothing without a config request; silence is not proof of no RF traffic.

See [hardware acceptance record](MESSENGER_INTEROPERABILITY.md) for the required
next evidence. Ordinary Meshtastic text, channel encryption, LoRaWAN and generic
LoRa transport are not interchangeable with authenticated Rust-Boy delivery.

## Primary source matrix

| Source | Applied requirement | Evidence limit |
| --- | --- | --- |
| [age Rust API and format implementation](https://docs.rs/age/0.12.1/age/) · [upstream security policy](https://github.com/str4d/rage/security) | Use library encryption/randomness and explicit X25519 identity; disable optional plugins/default features | Dependency selection is not an audit of Rust-Boy integration |
| [keyring 3.6.3](https://docs.rs/keyring/3.6.3/keyring/) | Select explicit native backends; fail on unavailable credentials; no filesystem private-key fallback | OS session, unlock and credential ACL behavior need native tests |
| [OpenSSH ssh_config](https://man.openbsd.org/ssh_config) | Distinguish connection `Hostname` from authentication `HostKeyAlias`; strict host checks and fixed options | Effective config is not two-host interoperability |
| [Meshtastic Python stream interface](https://github.com/meshtastic/python/blob/master/meshtastic/stream_interface.py) | Match USB frame marker, big-endian length and 512-byte ceiling; no upstream connection side effects | Framing alone does not validate FromRadio fields or node identity |
| [Meshtastic serial interface](https://python.meshtastic.org/serial_interface.html) | Treat DTR/reset and port setup as explicit hardware risks, outside passive parsing | Must verify actual USB device/firmware behavior |

The dependency versions are recorded in Cargo.lock. Sources informed the design;
no source is presented as certifying this implementation.

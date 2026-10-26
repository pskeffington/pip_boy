# Rust-Boy PACE messaging

PACE means **Primary, Alternate, Contingency, Emergency**. Rust-Boy now exposes
an explicit route plan per paired peer, a manual route selector in LINK and
bounded CLI operations. It does not yet provide four independent communication
bearers: P and A share the IP/OpenSSH stack, C cannot transmit, and E preserves
messages locally without delivering them.

## Current route contract

| Tier | Current action | Identity and receipt | Failure boundary |
| --- | --- | --- | --- |
| P — Primary | One explicit attempt to the enrolled SSH target | Dedicated sender key, pinned recipient host key, strict envelope and durable stored receipt | Network/address/server failures leave message QUEUED |
| A — Alternate | One explicit SSH attempt through a saved private IPv4 address | Same enrolled host key through `HostKeyAlias`; same envelope, message ID and receipt rules | Helps address/name resolution problems; does not survive a failed shared LAN/AP, dead peer or missing SSH server |
| C — Contingency | Show Meshtastic as receive-only/unvalidated; sending is refused | Recorded FromRadio frames are not authenticated Rust-Boy chat | OS serial broker, protocol/authentication and physical radio tests remain pending |
| E — Emergency | Queue or retain the message locally, with no transport call | No remote receipt is generated; status remains QUEUED / NOT DELIVERED | Retention only; no emergency-service contact, courier transfer or independent communication path exists |

The plan reports `automatic_failover:false` and `independent_bearers:false`.
Configured IP routes are labeled `configured_unprobed`, never ONLINE or healthy
without measurements. Displaying the plan, changing UI selection or saving a
route opens no connection. There is no hidden attempt at another tier when a
send fails, especially after an authentication failure. Unpaired devices cannot
send or use a local-hold action to create a new addressed message.

A full operational PACE plan still needs independently tested bearer diversity
and an external emergency communication method. The four labels alone do not
supply those capabilities.

## Configure and use

Use existing [peer enrollment](MESSENGER.md) first. Then configure the alternate
address explicitly; the public examples below are private lab fixtures:

```bash
pipboy peer pace-local scout-one 192.168.1.20 --confirm
pipboy peer pace-show scout-one --json
```

`pace-show` reads local configuration only. The alternate address lives in
`PIPBOY_HOME/message-pace.json` under a strict version-1 schema with at most 16
unique peers and 8 KiB of JSON. Updates use a cooperative writer lock and atomic
replacement; Unix creation uses private permissions and directory sync.
Malformed, unknown-version, public-IP and duplicate-peer configuration is
rejected while preserving the original. The file contains peer labels and local
addresses, not credentials or message contents. Those labels/addresses can still
reveal network topology; keep the file private. Plan configuration is not an OS
authorization token.

Clear an address without changing SSH pairing:

```bash
pipboy peer pace-local scout-one clear --confirm
```

In desktop or handheld LINK, select DEVICE and then PACE. Primary uses **SEND
VIA SSH**. A configured Alternate uses **SEND VIA LOCAL IP**. Contingency is
disabled for sending. Emergency uses **QUEUE / HOLD** and reports **HELD LOCALLY /
NOT DELIVERED**. Queued messages offer explicit RETRY using the currently
selected tier. Changing the tier never sends by itself. A held message remains
eligible for a later explicit P or A retry, including after restart; no stored
message is resent.

For an existing queued record, CLI uses its immutable message ID:

```bash
pipboy peer message-pace 1 scout-one emergency --confirm
pipboy peer message-pace 1 scout-one alternate --confirm
```

The first command retains the existing record and opens no connection; the
second makes one pinned SSH attempt through the saved alternate address.
Legacy `message-send`, `message-retry` and `message-retry-local` remain available.
Legacy primary CLI calls do not depend on a PACE plan file. The direct-IP command
accepts its explicit IP rather than saving a plan.

## API and storage truthfulness

`GET /api/messages` now includes `pace`, a derived plan for each conversation,
and the actual `storage` mode. Storage is either `plaintext_local_only` or
`age_os_keyring`; the UI checks those values and displays the corresponding
warning. Encrypted mode still requires successful OS key-store access. This is
queue-at-rest encryption, not E2EE. [Encryption contract](MESSENGER_BUILD_PASSES.md).

Existing same-origin POSTs `/api/messages/send` and `/api/messages/retry` accept
an optional `tier`: `primary`, `alternate`, `contingency`, or `emergency`.
Omitting it preserves Primary compatibility. Unknown/non-string tiers fail.
Every write still requires `confirm:true` and existing Host/Origin checks. An
unconfigured Alternate or receive-only Contingency fails before a new message is
queued. Send/retry responses identify `pace_tier`, `held_locally` and
`stored_on_receiver`; a local hold cannot claim a remote stored receipt or a
human read receipt.

The shared routing core validates peer pairing and the selected route before
queueing. A network failure after queueing preserves the message; a later retry
keeps the original sender ID, body and timestamp. The receiver deduplicates that
identity even if an earlier receipt was lost. Neither the selected tier nor an
IP address can substitute for cryptographic peer authentication.

## Evidence and next gates

- Unit tests verify strict plan persistence, missing routes, pairing precedence,
  blocked radio sends, local hold resolution and corruption preservation.
- Browser tests cover route changes with zero send/retry requests, disabled C,
  explicit A sends, E holds/retries that remain queued, receive-only users and
  correct encrypted/plaintext history labeling.
- The two-store relay smoke records connection counts, verifies saved routes and
  local holds make zero SSH calls, rejects C without queue/contact changes, then
  delivers the same held record through A with the original ID and one receipt.
  This relay remains a protocol fixture, not real network crypto validation.
- [Physical/native acceptance](MESSENGER_INTEROPERABILITY.md) remains pending:
  Mac/Pi local-IP handshake and wrong-key rejection, OS credential lifecycle,
  broker-authorized read-only USB, pinned protobuf interoperability and radio
  hardware evidence. Independently review the crypto integration before sensitive
  operational use.

Next development: safe key recovery/rotation and migration design; typed local
intelligence-report templates separating observations from assessments; scoped
OS serial authorization; independently authenticated mesh transport; and measured
bearer diversity. Do not mark C or E as operational communications until those
specific capabilities have passed acceptance.

## Planning sources

The U.S. Army describes PACE as an order of communication precedence and notes
that a plan can list fewer systems when four viable methods do not exist:
[A Short Note on PACE Plans, Infantry, July–September 2013](https://www.benning.army.mil/infantry/magazine/issues/2013/Jul-Sep/pdfs/JUL-SEP13web.pdf).
That supports explicitly recording unavailable tiers rather than inventing
readiness. [The Army's 2021 interoperability discussion](https://www.army.mil/article/242238/industry_insight_communications_skills)
also emphasizes that backup methods need interoperability. These are planning
sources, not endorsements or certification of Rust-Boy.

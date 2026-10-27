# RB <-> RB synchronization | Institutional and proximity roadmap

**Status: DESIGN / NOT DEPLOYED.** This document specifies a prospective multi-device subsystem for the open-source Rust-Boy (`pip_boy`) project. It does **not** assert that proximity discovery, automatic sync, multi-tenant authorization, E2EE, or HIPAA-ready clinical data exchange exist. The current working baseline is explicit enrolled-peer OpenSSH task and Messenger traffic with bounded packets and manually requested sends/status sync. Do not conflate passing synthetic tests with live radio, enterprise, or clinical acceptance.

**Organization prerequisite:** [Common Vault organizational chart](VAULT_ORG_CHART.md) defines the stable command/reporting template and [machine-readable organization positions](../catalog/vault_org_chart.json). **This chart is not an authentication or authorization implementation**; R1 must separate org governance, named operator appointments, device enrollment and scoped grants before using it to route synchronized records.

**ORG-1 structural core present (not authorization):** [`pipboy-core::organization`](../crates/pipboy-core/src/organization.rs) defines independently scoped org/vault/units/operators/appointments/devices and draft grants with deterministic structural and cross-tenant rejection tests. This does not verify a signer, log in an operator, enable pairing, persist a fleet roster, activate a grant, or synchronize data. R1 remains open until authenticated enrollment and a reviewed policy decision point are implemented.

**Execution tracker:** [GitHub issue #62 — R0–R3](https://github.com/pskeffington/pip_boy/issues/62). The tracking issue is planning work, not an implementation acceptance.

**Decision (2026-10-26):** A desktop Vault Command Center may originate missions, assign work to an authorized organizational fleet, and reconcile field-device reports. Handhelds own their local progress. Device proximity is only a routing hint. Authorization is anchored to organization, authenticated human/operator, authenticated device and per-dataset policy; transport selection cannot change those rights.

## 1. System picture and operating modes

```text
                          VAULT / COMMAND CENTER
                    assignment authority + audit + sync
                       org-bound policy and roster
                                   |
                        authenticated transport
                                   |
             +---------------------+---------------------+
             |                     |                     |
        RB HANDHELD A          RB HANDHELD B        RB STATION C
        same wired/LAN         nearby Wi-Fi         remote/queued
             |                     |                     |
        local queue + immutable-origin events + bounded receipts
             \_________________ approved exchanges _____________/
```

No shared database mount or automatic replication of every device's home directory. A device can belong to zero or one active organizational security domain in v1; multi-organization membership is deferred until isolated credentials, stores and policy have been independently validated. Devices from different organizations must **not** combine records simply because they share a room, subnet, QR code, or software installation.

Supported *goals*, not presently delivered features:

| Mode | How a peer becomes reachable | Identity proof | Allowed synchronization |
| --- | --- | --- | --- |
| Already paired, known IP or hostname | Existing OpenSSH roster; explicit local-IP alternate | Existing pinned SSH host key and restricted sender command | **Now:** explicit one-way task send, Messenger send, task receipt pull; manual only |
| Nearby on a common LAN | Later, opt-in mDNS/DNS-SD discovery or entered address (not a source of identity) | Pre-enrolled peer fingerprint/certificate and org membership | After review, only policy-approved low-risk queued updates |
| No infrastructure, physically near | Later, mutually approved device hotspot/Wi-Fi Direct or BLE discovery bootstrap; no implied Bluetooth data plane | Out-of-band QR/SAS fingerprint check plus device enrollment | After separate hardware/security gate; use authenticated higher-capacity data plane when possible |
| Keyed/enrolled together by an administrator | Unique **per-device** keys and org-issued, scoped enrollment; offline signed invitation possible | Trust anchor + human approval + short enrollment expiry + role/policy checks | Approved datasets only; not an all-access shared group password |
| Different networks / intermittently disconnected | Explicit configured route, later a mutually authorized gateway or store-and-forward relay | End-to-end authenticated origin + org authorization; relay never grants permissions | Bounded retry/delta after connectivity returns, with audit and retention |
| Meshtastic / LoRa | Research-only pending OS broker, duty-cycle and firmware gates | RB-level authenticated envelope and replay protection; LoRa channel membership is insufficient | Tiny, explicitly allowed metadata/alerts only; never raw PHI or bulk task history |

**No fallback may replace a pinned identity with an IP, SSID, device name, BLE MAC, mDNS TXT value, discovery response, LoRa node ID, or unauthenticated QR payload.** Unknown devices may at most be presented as untrusted pairing candidates while an operator explicitly runs discovery. No passive inventory of bystanders by default.

## 2. Entities and authority (org-aware design)

Use separate identifiers; do not overload the existing `peer.alias`, job profile, IP address, or key fingerprint:

- **organization_id**: randomized stable tenant/domain identifier; distinguish `independent`, `company`, `hospital`, `community`, or `incident` as *policy presets*, not authority.
- **device_id / device_key_id**: per-device persistent identity, separately generated authentication key, renewal/revocation state and hardware/software evidence when supported. Never use one private key across a fleet.
- **operator_id**: separately authenticated human or narrowly scoped service identity; local Rust workbench profile is a presentation/tool preset and **not** proof of human authorization.
- **membership**: signed or otherwise verified org-scoped enrollment with `not_before`, `expires_at`, allowed datasets/capabilities and optional deployment/site/team constraints.
- **assignment origin**: issuer org, authoritative command center, author identity, immutable mission/task ID, origin revision, target device or org role, and delivery policy.
- **trust state**: `unseen -> discovered_untrusted -> pairing_pending -> enrolled -> suspended/revoked/expired`. Discovery and pairing do not bypass an admin's data-access policy.

The first organizational implementation should use a signed enrollment manifest or PKI-backed certificate rooted in an organization key, plus device-specific identity keys, and mutual endpoint authentication. Administrators need an allowlist, device retirement/replacement, delegated enrollment with expiry, revocation publication, key rollover, inventory/export and audit review. Operator permissions must be checked by the native server/service for **every** sync action and every record read/write, not by a browser role label.

Single-device independent users should not be forced to run an enterprise CA: preserve the current manually verified OpenSSH peer route as a documented `standalone` compatibility mode. A hospital deployment must opt in to the institutional mode with an administrator and formal security review.

### Pairing ceremonies (ranked)

1. **Existing SSH peer (compatibility):** keep `peer init/add/trust` and restricted bridge. The operator compares host-key fingerprint out-of-band; no trust-on-first-use auto-accept.
2. **Direct two-device pairing (planned):** two-sided operator consent; compare short authentication string (SAS) or QR-encoded *public* key fingerprint out of band; bind challenge/response to a fresh session nonce. Display organization and intended grants before accepting. QR carries no private key, bearer credential or PHI.
3. **Institutional enrollment (planned):** administrator issues short-lived, single-use invitation bound to an org, a target device key, intended capability set and expiry. Enrollment completes only after server-side approval and certificate/signature verification. Offline invitations need explicit freshness/expiry policy and anti-replay accounting.
4. **Revocation (release-blocking):** deny new sessions to revoked devices; rotate group credentials and retire stale entitlements. Offline peers cannot learn a revocation immediately: constrain offline leases, clearly label stale membership, restrict writes after expiry and require reconnection for sensitive grants.

## 3. Synchronization contract (new v2; preserve v1)

Keep `RemoteAssignment`, `RemoteMessage` and current OpenSSH bridge commands exactly compatible during transition. Define a separate, version-negotiated **RB sync v2** namespace and storage adapter; no unannounced change to existing v1 JSON packets.

Proposed *schema fields* (not yet a released wire schema):

```text
SyncEnvelopeV2 {
  version, organization_id, workspace_id, dataset, object_type,
  object_id, origin_device_id, origin_seq, event_id,
  parent_revision_or_hash, operation, created_at_untrusted,
  classification, intended_recipients, expires_at,
  payload_hash, bounded_payload, authenticated_origin
}
SyncReceiptV2 {
  version, organization_id, receiver_device_id, event_id,
  state: stored | rejected | conflict | quarantined,
  receiver_cursor, durable_receipt_id
}
```

**Identity and replay:** `event_id` scoped by (org, origin device/key epoch, monotonic sequence); preserve opaque immutable origin history and enforce idempotency on any bearer. Validate frame signature/MAC and channel binding through reviewed libraries; a self-asserted `organization_id` is never sufficient. Use length bounds, version negotiation, recipient binding, replay window, duplicate detection and protocol downgrade rejection. Ignore peer clock for conflict priority or authorization; require separate credible time source or fail closed when an expiry cannot be established. Signed timestamps alone do not prove sender clock accuracy.

**Delta exchange:** authenticated capability negotiation -> local dataset authorization -> bounded per-peer manifest/cursors -> send/receive small event batches -> receiver validates and fsyncs -> receiver issues typed durable receipt -> sender updates local cursor. Backpressure, quota ceilings, resumable chunks and rejection diagnostics are mandatory. Never interpret a missing receipt as evidence of failure or success; retries must be idempotent. Derive a digest/hash for detection, not as a substitute for authorization.

**Ownership/conflicts (no blanket last-write-wins):**

| Domain | Authority | Conflict rule | Sync default |
| --- | --- | --- | --- |
| Mission, job assignment and reassignment | Issuing Command Center | Single issuer plus optimistic revision; receiver reports status separately; conflicting issuer commands quarantined for review | Enrolled org + approved assignment subscription |
| Field acknowledgment / progress | Recipient RB device and named operator | Append-only, origin-sequenced events; sender aggregates without overwriting recipient history | Enrolled specific issuer, minimal fields |
| Messenger | Named sender; recipient local journal | Immutable message/receipt identity, idempotent insert, no cross-peer merge | Explicit opt-in only |
| Site observations / geospatial notes | Authoring device + site owner | Append observations; edits require versioned author lineage and explicit merge review | Disabled until data-class and consent gates |
| Settings, device keys, permissions, roster, private file paths | Local device/org administrator | No peer replication by default; separate managed configuration channel later | **Never** |
| Camera streams, raw telemetry, location trails, attachments | Separate subsystem owner | No bulk sync without data minimization, consent, retention and bandwidth design | **Never in initial release** |
| Clinical/ePHI | Regulated entity, approved clinical workflow | Separate regulated-data service; not placed in general task/message bodies | **Blocked** |

Explicitly distinguish **queued**, **sent/transport attempted**, **stored at peer**, **acknowledged by operator**, **work in progress**, **completed**, and **reconciled at command**. "Stored" does not mean "read"; a network handshake is not a durable receipt.

## 4. Security policy and protected-data boundaries

**Classification profiles:** `public_demo` (synthetic/public), `internal` (non-sensitive organizational data), `restricted` (controlled enterprise), `regulated` (clinical/ePHI and other specifically governed content). The first sync release only supports `public_demo`; expand to `internal` after authenticated identities, per-operator authorization, encrypted-at-rest storage and reviewed audit/retention. `restricted` and `regulated` remain blocked until an independent release gate approves their respective controls.

- Default deny cross-org transfers, discovery-driven connections, background peer probing and unattended secrets export.
- Separate operator approval to enroll a peer, grant a dataset and enable a **bounded** sync schedule. No implicit write access to every dataset once a device is paired.
- Enforce least privilege: e.g. Command may issue assignments; field RB may acknowledge/update only assigned work. The current mutable local role preset must not be relied upon for this.
- Authenticate and encrypt every link: preserve pinned OpenSSH in Phase 1, evaluate independently reviewed TLS 1.3 mutual-auth or QUIC+mTLS for later persistent sync; do not invent bespoke link crypto. TLS transport protection is not end-to-end protection against compromised gateways or endpoints.
- Evaluate reviewed E2EE (e.g. MLS, RFC 9420, for **group messaging only**) after membership, offline state and key-rotation design; do not claim that `age` file encryption provides message E2EE or that MLS automatically solves task authorization.
- Device keys stay in OS-protected storage/hardware where available; no exportable fleet-wide shared secret. Headless Linux/Pi key custody must be solved without a plaintext fallback.
- Keep durable, tamper-evident audit events for enrollment, permission change, send attempt, receipt, status merge, conflict, revocation and break-glass access. Define retention, pruning and capacity limits before production; avoid sensitive content in logs.
- Lost/stolen device plan: suspend, revoke, rotate affected keys, quarantine pending outbound events, invalidate stale offline memberships and flag last-seen state. Offline revocation is **bounded but not instantaneous**.
- Data protection: existing `tasks.json` and `peers.json` are plaintext. **Never use them to hold PHI or sensitive organizational material** until encryption, migrations, key recovery, backup, access control and audit have been reviewed.

**Healthcare gate:** HIPAA requires administrative, physical and technical safeguards for ePHI, including access control, audit, integrity, person/entity authentication and transmission security. These requirements cannot be satisfied by an open-source license, an encrypted link, a role dropdown, or an organization's use of Rust-Boy alone. Require a regulated-data scope review, named accountable operator identities, minimum-necessary policies, documented risk analysis, emergency access/break-glass with audit, secure device lifecycle, business-associate/contract review where applicable, patient-record integration review and field validation **before handling ePHI**. No clinical decision support or life-critical alert delivery should rely on an unvalidated sync channel.

For public, company and community deployments, document a separate organization policy template that does not assert HIPAA certification. Open source enables adoption and independent audits but does not provide security approval, operational support or compliance by default.

## 5. Proposed PACE connectivity / fallback

Transport choice is **separate** from record authorization. After the user enrolls a peer and a dataset, prefer the safest available previously trusted route:

1. **P / Primary:** enrolled OpenSSH via known hostname or organizational LAN; operator-initiated now, bounded scheduled deltas later.
2. **A / Alternate:** enrolled OpenSSH using a private-IP route while retaining pinned `HostKeyAlias` and SSH identity. Explicit now; future automated switching only after tested trust equivalence, consent and bounded retry.
3. **C / Contingency:** approved direct Wi-Fi or hotspot and, later, a reviewed high-bandwidth local tunnel or peer gateway. Proximity discovery is not a data channel and does not grant rights.
4. **E / Emergency:** queue/hold locally and optionally transmit only a minimal, signed, non-sensitive distress or status marker on an independently validated radio bearer. No automatic dumping of task/medical records.

LAN availability and Wi-Fi Direct support vary by Mac, Pi and corporate/hospital Wi-Fi policies; do not assume multicast, BLE pairing, peer-to-peer Wi-Fi, inbound SSH or SSH port 22 are permitted. Support an administrator-provided address and explicit configured route if discovery is blocked. Never require an external cloud relay for basic local operation. Institutional gateways may eventually offer managed access, but the endpoint remains authoritative for dataset permissions.

## 6. Execution phases and acceptance gates

| Pass | Implementation deliverable | Release gate / deterministic tests |
| --- | --- | --- |
| **R0 - freeze current baseline** | Inventory `peers`, `message_protocol`, `message_pace`, `messenger`, `tasks`, `peer.rs`, `dashboard/task_routes`, UI and storage. Publish v1 compatibility and sensitivity policy; no new network behavior. | CI baseline; write/read side-effect inventory; v1 strict round trip and old-store compatibility |
| **R1 - identity and trust model** | `sync_identity` and `sync_policy` pure core modules; org/device/operator identities, invitation and trust-state transitions; scoped consent and revocation semantics. No sockets. | Wrong org/key, replay, unsigned invite, expired membership, stale offline lease, privilege escalation all fail closed; no secrets in outputs |
| **R2 - typed event log / delta engine** | Append-only dataset events and origin cursors; manifest, bounded batches, per-record permission filters, receipts, conflict quarantine and durable sync ledger. Use synthetic in-memory transports first. | Two- and three-device model tests for duplicates, drop, reordering, crash after fsync, lost receipt, restart, replay, queue quota and independent role/status updates |
| **R3 - paired IP sync** | Bind R2 to existing pinned OpenSSH restricted bridge as a new opt-in command; explicit `sync preview` / `sync now` / `sync stop`, read-only discovery of trusted endpoint capability. No unauthenticated listener. | Two real Macs/Pi hosts, pinned-key mismatch, wrong key, revoked device, network interruption, concurrent edits, no silent downgrade and bounded byte/time caps |
| **R4 - nearby discovery + managed enrollment** | Optional short-lived local discovery; operator-reviewed QR/SAS flow; org admin grant, trusted roster and safe path selection. No automatic trust on sight. | Spoofed mDNS/BLE, replayed QR, swapped IP, shared subnet of different tenants, device replacement and consent-revocation tests |
| **R5 - policy-controlled background reconciliation** | Explicit schedule enablement only for pre-approved `public_demo` datasets; low-power/backoff, visible stop state, per-peer bandwidth/queue limits, user/admin audit. | Zero background traffic before enable; no unbounded retry; no data after revocation; disconnect/rejoin, power loss, low battery, time drift, battery/RF metrics |
| **R6 - organization/fleet scale** | Multiple operators, org-scoped command consoles, device enrollment/reassignment, audit exports, delegated admin, staged rollouts and deployment documentation. Evaluate network relay/gateway without cross-org bypass. | 3, 10, 100-node load simulations; multi-tenant isolation, identity/role separation, backups/recovery, key rotation, failover, SSO/PKI integration review |
| **R7 - protected data and specialty transport** | Separate assessed clinical/ePHI store and connectors (if approved); later active BLE/Wi-Fi Direct/LoRa only through reviewed OS broker and suitable cryptographic framing. | Security/privacy/accessibility review; hospital risk assessment, actual Mac/Pi/network hardware testing, emergency workflows, and independent deployment approval |

**Priority:** R0 -> R1 -> R2 -> R3 first. Do **not** begin active BLE/LoRa or hospital/regulated transfer as a shortcut around stable organizational identity and an audited synchronization ledger.

### Minimum first full demonstration (non-clinical synthetic fixtures)

1. Overseer (Vault Command desktop) and two independent field RBs, each with unique keys, enrolled into one test organization.
2. Command creates mission `M-001` containing jobs `J-001` / `J-002`, assigned to different devices. Only explicitly authorized job fields synchronize.
3. Field A receives and acknowledges its assignment, goes offline, records `in_progress` then `done`; Field B records `blocked`. Return to trusted LAN, run sync, and display independent receipt/ack/progress in Command.
4. Simulate duplicate delivery, lost receipt, adversarial wrong-org third RB, revoked RB, changed task after sender retry, and device restart; verify zero unauthorized records, no duplicate task and visible conflict states.
5. Unpair, lose Wi-Fi and stop all sync; cached local job records remain available, no hidden transport retries and no false completion message.

Example acceptance SLO **targets, not claims**: zero unauthorized transfers in negative tests; exact-once *effective* storage under duplicates using idempotency (not exactly-once network delivery); 100% explicit record-rejection diagnostics; bounded payload/queue sizes; zero network packets before enrollment/enablement; no document-level UI scrolling while Command presents sync status.

## 7. File ownership and UI work plan

- `pipboy-core/src/sync_identity.rs`: policy-only device/org/operator model, no filesystem or socket.
- `pipboy-core/src/sync_policy.rs`: authorization + classification + expiry/revocation decisions with pure deterministic tests.
- `pipboy-core/src/sync_protocol.rs`: independent versioned events, receipts, replay/dedup and no-drop conflict model.
- `pipboy-core/src/sync_store.rs`: transactional, encrypted/approved local ledger and per-peer cursors; never duplicate private keys.
- `pipboy/src/sync_ssh.rs`: opt-in transport adapter reusing existing pinned peer setup; separate explicit bridge command.
- `pipboy/src/sync_discovery.rs`: later opt-in discovery only, bound to OS/network broker consent, no passive network sweep.
- `pipboy/src/dashboard/sync_routes.rs`: same-origin UI read-only previews; POST actions with server-side identity/policy checks, confirmations and bounded jobs.
- `assets/command-sync.js/css`: Vault Command's **FLEET / DISPATCH / RECEIPTS / CONFLICTS** secondary subviews, populated only after opt-in and backed by the Rust API; no auto-pairing from HTML.
- `assets/micro.js`: field **LINK** status shows enrolled org/device, queued receipts and assigned jobs; no bulk roster or enterprise administrative controls on the handheld by default.
- `tools/test_sync_*.{cjs,py}` and Rust `#[test]`: synthetic security regression and actual two-host acceptance remain separate.

Preserve the single-page desktop requirement: Command owns assignment and fleet status, the map stays full canvas in Viewport, and setup/keys/organizations live in universal Settings. Organization identity and verified access cannot be replaced by a UI role selector.

## 8. Primary-source design matrix

| Reference | Architectural consequence | Not evidence of |
| --- | --- | --- |
| [NIST SP 800-207: Zero Trust Architecture](https://csrc.nist.gov/pubs/sp/800/207/final) | No trust from physical/LAN proximity; per-connection device and subject authentication/authorization | A certified Rust-Boy zero-trust implementation |
| [NIST SP 800-63B-4: Authentication](https://csrc.nist.gov/pubs/sp/800/63/B/4/final) | Verify human/operator and phishing-resistant or channel-bound cryptographic authentication as appropriate | A specific approved RB authenticator |
| [NIST SP 800-57 key management](https://csrc.nist.gov/projects/key-management/key-management-guidelines) | Key lifecycle, recovery, rotation, expiry/revocation and independent per-device keys | An RB key-management audit |
| [RFC 9420: Messaging Layer Security](https://www.rfc-editor.org/rfc/rfc9420.html) | Evaluate reviewed asynchronous group key management only for a future E2EE messenger | Task syncing or access control being solved by MLS |
| [RFC 8446: TLS 1.3](https://www.rfc-editor.org/rfc/rfc8446.html) | Consider authenticated encrypted network sessions for future long-lived peers | Cross-org policy or disk protection |
| [HHS HIPAA Security Rule](https://www.hhs.gov/hipaa/for-professionals/security/index.html) | Separate clinical-data program with administrative, technical and physical controls | Compliance merely because source is open |
| [OpenSSH host-key and forced-command docs](https://man.openbsd.org/sshd) | Keep v1 pinned host identity + restricted bridge; preserve compatibility | Independent field interoperability |
| [Current RB peer/task protocol](PEER_TASK_TRANSPORT.md), [Messenger A-C](MESSENGER_BUILD_PASSES.md), [PACE](PACE_MESSAGING.md) | Reuse shipped bounded envelopes and deterministic fault fixtures before adding new data planes | Released proximity or automatic synchronization |

**Exit definition:** A multi-device RB system is ready for organizational pilot only after named authenticated identities, managed enrollment/revocation, least-privilege dataset contracts, encrypted and audited storage, two-host/field acceptance, and an accountable operator experience have all passed. A regulated healthcare pilot requires an additional independent compliance/privacy and clinical operational review; pilot status is not a claim of certification.

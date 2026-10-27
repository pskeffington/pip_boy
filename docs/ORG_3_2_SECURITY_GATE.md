# ORG-3.2 — durable trust foundation (security review required)

Status: **draft implementation, NOT an authorization service**. No HTTP, SSH, tasks, Vault Command, or RB-to-RB route invokes these modules.

## Scope in this branch

- `organization_custody`: unique Ed25519 operator/device identities generated from system randomness, retained through the existing platform keyring integration; scoped, domain-separated proof signing; no plaintext fallback.
- `organization_durable`: explicit `AdministratorApproval` gate for out-of-band public-root fingerprint verification, a local exclusive lock, atomic + fsync state snapshots, restart-persistent challenge replay rejection, and signed full-revocation snapshots with generation and expiry checks.
- No automatic enrollment, acceptance from discovery, role-dropdown authority, or regulated-data support.

## Important unresolved security boundaries

1. **Root substitution/rollback:** organization state is currently a local file. An adversary able to replace both the stored root and the snapshot can forge a new trust universe; file permissions alone are not secure root pinning. Before production, pin the root fingerprint and monotonic revocation floor *independently* in a protected administrator-controlled hardware/OS trust store, and compare them on every open. Detect restoration of an old disk image using a platform monotonic counter or an independently durable authority.
2. **Administrator authentication:** `AdministratorApproval` is an interface contract only. Do not wire it to a UI role selector, peer claim, generic yes/no button, or a network-supplied administrator flag. Require approved strong, auditable local administrator authentication, separate from operator and device identities.
3. **Key custody:** OS keyrings vary by platform and headless Linux boot environment. Keyring use is not proof of hardware-backed non-exportability or resistance to same-user compromise; independently test lock/unlock, service-account permissions, key loss, backup, and recovery. Disallow plaintext fallback.
4. **Revocations:** signed snapshots are bounded to 24 hours; offline devices cannot know an immediate revocation until they reconnect. Deployments must set a stricter maximum offline lease when needed and prohibit any sensitive operation after expiry. Revoked devices must be quarantined at every endpoint, even when other replicas have not yet synchronized.
5. **Replay:** nonce persistence survives ordinary process restart. Loss of the entire ledger, filesystem rollback, copied state, symlink attacks, and crash injection need additional tests and control-plane protection before release. A persistent ledger is not by itself an authenticated TLS/SSH channel.
6. **Authorization:** signed evidence does not validate local operator sessions, authenticated time, grant issuer authority, or fresh transport export. Existing `organization_authority` returns evidence only; no route may interpret it as a permission.

## Validation matrix

| Case | Expected |
| --- | --- |
| Unapproved administrator root fingerprint | reject without provisioning |
| Keyring unavailable; credential mismatch; reenrollment | fail closed; no plaintext fallback |
| Duplicate nonce and process restart | reject duplicate without consuming another slot |
| Full nonce journal | reject new authorization, never evict silently |
| Forged, wrong-org, unsorted or expired revocation snapshot | reject |
| Lower/equal revocation generation | reject |
| Crash before fsync/rename, crash after rename before directory fsync | verify recoverability and never acknowledge a non-durable mutation |
| Two writers, symlink swap, chmod failure | fail closed |
| Backup restored with old root/nonce/revocation floor | reject using independently protected root/counter (**not implemented**) |
| Headless Raspberry Pi + macOS keyring interoperability and permissions | field test (**not run**) |
| Offline device receives stale revocation while disconnected | deny after local lease expiry; reflect uncertainty while offline |
| Live Vault Command / RB sync authorization | remain disconnected until real-device security review |

Run once Rust toolchain and dependencies are available:

```bash
cargo test -p pipboy-core organization_custody
cargo test -p pipboy-core organization_durable
cargo test --workspace
```

**Release gate:** branch must remain draft until independent trust-anchor protection, administrator authentication, rollback resistance, trusted clock/channel binding, security regression tests, and actual Mac/Pi two-device validation are complete. No clinical/ePHI or life-critical use.

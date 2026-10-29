# Rust-Boy | OS device authorization boundary

**Status:** an isolated, **deny-all** one-shot stdio process with a bounded protocol is [cross-platform CI-verified](https://github.com/pskeffington/pip_boy/actions/runs/40164717116) (see [BROKER_PROTOCOL.md](BROKER_PROTOCOL.md)); there is still **no authenticated OS-enforced broker**, live device driver, trusted consent service, privileged helper or hardware probe. The in-process simulator and separate deny-only process do not supply real authorization.

The RB-CX-02 host-policy candidate adds ephemeral role/package/scope checks and exclusive logical audio ownership for trusted in-process adapters. It does not change the broker protocol or permit physical access. Network, device, location, radio and peripheral operations remain unavailable through the deny-only broker until OS-enforced authorization is independently implemented and accepted.

## Principle: saved preference is never an active grant

The existing `permissions.json` can contain `requested` or `denied` intent.
Neither `requested` nor the `pipboy simulate --confirm` switch is a grant for
real serial ports, GPIO, display, GNSS, SDR, mesh, BLE or network interfaces.
`denied` must continue to block the mock as well as any future real adapter.

The real broker must be a separate, privilege-minimized OS process or other
independently enforced resource boundary. Rust traits, in-process state, a
filename, and a cryptographically unprotected JSON flag do **not** constrain
untrusted linked code. There is no universal "device permission" API across
Linux, macOS and Windows.

## Proposed real authorization design (not implemented)

```text
Keyboard / user request
    |
Explicit device identity, capability, operation, duration, purpose
    |
Trusted confirmation UI (not persisted as authority)
    |
OS-enforced broker IPC and caller identity check
    |
Ephemeral broker-held resource handle
    |
Explicit start -> bounded operation -> revoke/close/expire
    |
Observable audit/status: denied, active, revoked, error
```

1. **No discovery without opt-in.** Resolve a user-selected, stable device
   identity from an OS-provided source. No automatic scanning, pairing,
   listening, transmitting or network discovery.
2. **Least privilege.** The broker exposes only enumerated operations for
   specific device IDs and allowed directions. Receive-only cannot imply
   transmission; display write cannot imply arbitrary filesystem writes.
3. **Trust the OS, not an in-memory token.** Authenticate the caller, restrict
   IPC endpoints, and keep authoritative OS handles within the broker or
   transfer only narrowly scoped handles with independently enforced ACLs.
4. **Fresh and narrow authorization.** Each real operation must have a
   narrowly scoped one-time approval, a deadline enforced by a real clock,
   an explicit stop control, and a unique non-replayable session identity.
   `permissions.json` records intent only, never session credentials.
5. **Fail closed.** On broker disconnect, malformed IPC request, denial,
   hot-unplug, timeout, process crash or suspend/recovery, close the OS handle,
   clear buffers and report unavailable. Do not silently replace the target
   device or fall back to broader permissions.
6. **Provenance.** Real observations need source/device identity, units,
   quality, freshness, and a distinction between recorded, synthetic and
   measured data. Test fixtures may not be represented as hardware evidence.
7. **No embedded secrets.** Long-lived credentials or authentication keys
   must not be stored in Rust-Boy notes or plain adapter preferences.

## Platform-specific broker design questions

| Platform | Design work before implementation | Required test evidence |
| --- | --- | --- |
| Windows | Enumerated device identity, restricted named-pipe/IPC ACL, signed helper/update path where applicable, OS handle lifetime | Reject unauthorized callers, impossible cross-device escalation, close on termination/hot-unplug |
| macOS | User-approved device selection, applicable privacy/TCC restrictions, scoped IOKit/serial access and sandbox boundary | Denial/consent flow, permission revocation, sleep/wake recovery, zero silent device probes |
| Linux | udev identity and permissions, restricted helper/Unix socket and ownership, safe hot-unplug handling | Unauthorized caller denied, device symlink substitution denied, no root daemon by default |

A cross-platform interface may report `Unavailable`, but it must not mask
platform security differences. No new privileged helper should be installed
or started as a side effect of TUI launch, `doctor`, `status`, or library reads.

## Required gates before real adapters

- **G2-A — design review:** deny-all stdio grammar and source-level no-I/O gates are authored; selected device models, OS-specific handle boundary, authenticated IPC, documented minimal permissions and threat model review remain open. The stand-alone parser does not meet the full gate.
- **G2-B — isolated authorization:** dedicated trusted consent UI,
  per-device session scope, hard OS permission check, structured denial and
  revocation tests, secure update/install route. This gate is not met.
- **G2-C — simulator parity:** mock and physical interfaces have equivalent
  bounded cancel/timeout/disconnect semantics, but physical deadlines must use
  a trustworthy clock and real handle close. Deterministic fixtures stay mock.
- **G2-D — platform evidence:** selected physical hardware on each supported
  OS, hot-unplug recovery, failure injection, accessibility review and
  device-I/O traces showing **no unapproved operations**. This gate is not met.

## ConPTY is a terminal test, not a device broker

`tools/smoke_windows_conpty.py` uses Microsoft's native pseudoconsole API
to test Windows terminal interaction, resize behavior and terminal restoration
inside GitHub Actions; the new interactive regression passed in [run 40162936734](https://github.com/pskeffington/pip_boy/actions/runs/40162936734). It does **not** grant Rust-Boy access to any external
device, and it does not implement or validate the proposed broker.

Primary platform references:
- [Microsoft: Creating a Pseudoconsole Session](https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session)
- [Microsoft: UpdateProcThreadAttribute](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute)

Related: [security](../SECURITY.md), [adapter preferences](PERMISSIONS.md),
[virtual transport](VIRTUAL_TRANSPORT.md), [typed mock adapters](MOCK_ADAPTERS.md),
and [G2 roadmap](ROADMAP.md).

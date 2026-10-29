# Rust-Boy | Deny-all physical broker protocol v1

**Gate:** G2-A protocol precursor. **Status:** separate deny-only binary, verified by [Linux/macOS/Windows CI](https://github.com/pskeffington/pip_boy/actions/runs/40164717116), with no device backends; not a real OS-permission broker. Every request returns `allowed:false`, including well-formed requests declaring explicit operator intent.

## Run explicitly

After a workspace build, launch the stand-alone executable ONLY when an
operator or test specifically requests it. It is not part of TUI startup,
`status`, `doctor`, saved preferences or background services.

```bash
cargo build --workspace --locked
echo '{"schema_version":1,"request_id":"0123456789abcdef","capability":"serial.read","device_id":"device:demo-serial","operation":"read","duration_ms":1000,"purpose":"test only","operator_intent":true}' | cargo run -p rust-boy-broker -- --stdio-once
```

The response is a single JSON line:

```json
{"schema_version":1,"allowed":false,"status":"denied","reason":"physical_device_backend_disabled","source":"rust-boy/deny-all-physical-broker"}
```

The executable exits after processing one bounded input line; it is not a
daemon or remotely accessible IPC endpoint. It opens no files, device handles,
network sockets, named pipes, radios, GPIO or serial ports. Its only I/O is the
existing standard input/output/error descriptors inherited from its invoking
process. The operating system gives the executable a process boundary, but
**there is not yet a secure OS-broker authorization boundary**, authenticated
caller identity, device identity attestation, privilege isolation or a trusted
consent UI. A user-authored input flag is **not consent**.

## Request schema and rejection policy

| Field | Required type | Validation |
| --- | --- | --- |
| schema_version | integer | Exactly 1; no future-schema fallback |
| request_id | string | Exactly 16 ASCII hexadecimal characters; not a credential or session ID |
| capability | string | One of the 11 declared external capabilities |
| device_id | string | Opaque `device:` prefix plus 1–48 ASCII alphanumeric, underscore or hyphen characters; not a path |
| operation | string | Exact reviewed operation paired with its capability |
| duration_ms | integer | 1–30,000; no infinite operation |
| purpose | string | 1–120 printable ASCII bytes with nonblank content |
| operator_intent | boolean | Required, but **never** authorizes real devices |

The exact capability/operation pairs are:

| Capability | Operation |
| --- | --- |
| device.enumerate | enumerate |
| gpio.read | read |
| display.write | write |
| serial.read | read |
| gnss.read | read |
| sdr.receive | read |
| ble.discover | discover |
| mesh.transmit | transmit |
| sensor.read | read |
| audio.play | play |
| network.fetch | fetch |

Every response has `allowed:false` and `status:"denied"`. A well-formed
request with `operator_intent:true` returns
`physical_device_backend_disabled`; without declared intent it returns
`operator_intent_required`. Invalid schema, unknown fields, wrong operations,
filesystem-like selectors, malformed JSON and oversized frames return
`invalid_request`. All denial responses have the same five fields: schema,
allow flag, status, reason and source. The broker does not echo request content,
device paths, caller intent, a token or a handle.

The separate Unix adapter socket precursor does not connect to this broker and
cannot grant physical access. Its verified peer stream only binds a session to
OS-reported process/account credentials; it does not verify package binaries,
authorize capabilities, or change the broker's deny-all behavior.

The maximum input frame is 1,024 bytes, including the final newline. The
process reads at most 1,025 bytes into memory and handles only the first line.
Its success exit code (0) means the **protocol responded**, not that access was
granted. Invoking it without exactly `--stdio-once` returns a usage error
(exit code 2) and writes no JSON response. No command-line argument can turn
authorization on.

## Security and tests

`crates/rust-boy-broker/src/main.rs` has 12 Rust unit tests for denial,
capability/operation binding, device selectors, limits, missing intent, version
drift, malformed/oversized frames and output redaction. The black-box
`tools/smoke_broker.py` starts the executable as a short-lived subprocess
and checks deterministic denial, strict protocol, no unintended writes and
no inherited access. Linux stable, Rust 1.81.0 and native macOS/Windows CI
compile/test the new workspace package; stable and native jobs build the
executable and run the smoke harness. The offline catalog validator checks
the broker's capability list against the core's 11 adapter declarations.

**A green deny-all broker test does not meet G2-B authorization.** Prior to
enabling ANY physical adapter, Rust-Boy needs a separately enforced OS broker,
caller authentication, user-selected verified device identities, trusted
per-session approval, OS ACL/sandbox constraints, real monotonic timeouts,
revocation that closes OS handles, hot-unplug and crash-recovery tests and
accessibility review. Saved `permissions.json` intent and the simulator's
`--confirm` are not transferable to this real-device authority.

Related: [OS device broker design](OS_DEVICE_BROKER.md), [security](../SECURITY.md),
[preferences](PERMISSIONS.md), [mock adapters](MOCK_ADAPTERS.md), and
[validation](VALIDATION.md).

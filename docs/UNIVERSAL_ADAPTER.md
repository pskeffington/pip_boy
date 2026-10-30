# Rust-Boy universal package adapter (v1 development contract)

**Status (2026-10-29): first-party Rust SDK and host manager compile and pass local tests.** The live Radio output path now holds a host-wide, non-preemptive audio lease. MUSIC remains a distinct design fixture; its trusted adapter, audio bridge, worker/input path, and shell mount are still open. The handheld reference row has a distinct phosphor-green treatment; this does not make it installed or activatable. The manager is not yet bound to the desktop/micro UI and is not an external plug-in loader or OS sandbox.

## Goal

Rust-Boy behaves like a universal socket: the shell owns the compact package switcher, the **one foreground package** viewport, persistent **Settings**, organizational trust, and network/device grants. Packages own their internal tools and independently maintained source repositories. The host must not contain hardcoded MAP/TV/RADIO/COMMS algorithms.

## Code ownership

| Layer | Rust source | Role |
| --- | --- | --- |
| Portable SDK | [`rust-boy-adapter-sdk`](../crates/rust-boy-adapter-sdk/src/lib.rs) | Versioned wire vocabulary, viewport, reply, `UniversalPackageAdapter` trait and deny-all scope policy interface |
| Core host | [`universal_adapter.rs`](../crates/pipboy-core/src/universal_adapter.rs) | Trusted adapter registration, lifecycle orchestration, one-package foreground ownership, host Settings, conservative intent gating |
| Manifest registry | [`package_socket.rs`](../crates/pipboy-core/src/package_socket.rs) | Strict schema-1 manifests, five inert reference fixtures, explicit local discovery |
| Shell UI | `crates/pipboy/assets/` | **Not yet bound** to the universal adapter manager; existing UI kept as compatibility baseline |
| Device / network broker | `crates/rust-boy-broker/` | Deny-only precursor; no external module hardware privilege bridge yet |

The lightweight SDK depends only on `serde` and `serde_json`. The five independently developed packages can depend on this crate rather than on `pipboy-core` (which includes unrelated host and radio dependencies). Pin a reviewed Git commit when using the SDK in another repository; do not follow an unbounded main branch in released binaries.

The [machine-readable adapter contract](../catalog/host_adapter_contract.json) indexes version, bounds, command/reply vocabulary, the five package IDs and host ownership for automation. Its consistency with the Rust SDK is checked by the offline catalog validator.

## Shared audio ownership

[`audio_focus.rs`](../crates/pipboy-core/src/audio_focus.rs) provides one
host-wide in-process coordinator. Live RADIO startup and output replacement
must acquire `HOST_AUDIO_FOCUS` as `AudioModule::Radio` before opening output;
the engine retains the RAII lease until it has stopped or failed closed.
`UniversalAdapterHost` reserves the same broker for RADIO, MUSIC and TV before
delivering Activate/Resume. The adapter must stop its output before
acknowledging Suspend/Shutdown; only then does the host release its reservation.
Failed cleanup keeps the reservation, including after manager drop, until
process exit. Contenders are denied without preempting the active owner. The
broker does not yet cross process boundaries, and real MUSIC/TV output engines
are not integrated; they remain independent adapter work.

## Common module contract

A trusted native adapter implements:

```rust
use rust_boy_adapter_sdk::{
    AdapterEnvelope, AdapterReply, AdapterError, UniversalPackageAdapter,
};

struct MyModule;

impl UniversalPackageAdapter for MyModule {
    fn package_id(&self) -> &str { "org.example.my-module" }

    fn handle(&mut self, request: AdapterEnvelope)
        -> Result<AdapterReply, AdapterError>
    {
        match request.body {
            rust_boy_adapter_sdk::AdapterCommand::Activate { .. } =>
                Ok(AdapterReply::Ack),
            rust_boy_adapter_sdk::AdapterCommand::Suspend =>
                Ok(AdapterReply::Ack),
            rust_boy_adapter_sdk::AdapterCommand::Shutdown =>
                Ok(AdapterReply::Ack),
            _ => Ok(AdapterReply::Ack),
        }
    }
}
```

Run the dependency-light demonstration using:

```bash
cargo run --locked -p rust-boy-adapter-sdk --example hello_adapter
cargo test --locked -p rust-boy-adapter-sdk
cargo test --locked -p pipboy-core universal_adapter
```

The full [hello example](../crates/rust-boy-adapter-sdk/examples/hello_adapter.rs) makes **no network or device request**. It demonstrates a package responding to a versioned activation envelope, not a proof of actual package installation.

### Protocol v1

For a separate package binary, use the bounded `RBPK` stdio transport in
[`PACKAGE_PROCESS_PROTOCOL.md`](PACKAGE_PROCESS_PROTOCOL.md) and
`rust-boy-adapter-sdk::process_protocol`. It maps this adapter vocabulary to
strict binary frames; it does not authorize scopes or make child code a
sandbox. The host process supervisor and installed Music mount remain pending.

- Version: `protocol=1`, independent from manifest `schema_version=1` and `host_api=1`.
- Request: `AdapterEnvelope { protocol, package_id, request_id, body }`; request IDs are monotonic within the host instance.
- Commands: `activate(viewport)`, `suspend`, `resume(viewport)`, `resize(viewport)`, `intent(action, required_scope, payload)`, `shutdown`.
- Replies: inert `ack` or `view(state)` JSON. The host renderer must use safe DOM/text APIs, **never** execute JavaScript/HTML supplied by a package.
- Bounds: 16 KiB serialized request/reply; 4 KiB intent payload; viewport dimensions 320..4096 wide, 240..4096 high.
- A host suspend means **loss of foreground ownership**, not implicit permission for background transmissions. A separate reviewed service layer must grant, supervise and revoke any permitted background work.

An example activation envelope:

```json
{
  "protocol": 1,
  "package_id": "org.rustboy.gis",
  "request_id": 1,
  "body": {
    "command": "activate",
    "viewport": { "width": 480, "height": 320 }
  }
}
```

## Host mounting and removal

`UniversalAdapterHost::mount_trusted(manifest, runner)` only accepts **explicitly supplied trusted built-in** implementations whose IDs match a validated manifest. It rejects design fixtures marked `design_fixture_not_installed`, `external_process` entrypoints, and duplicate IDs. The host's bootstrap code, **not** a scanning JSON file, must supply the runner. A manifest with no `status` is **not proof of signature, installation, or organizational approval**.

Once mounted, `select(id)` delivers activation/resume, suspending the previous foreground adapter first. Failed switches try to restore the old view. `open_settings()` suppresses package presentation, while `close_settings()` can restore the prior package. `unmount(id)` calls shutdown then removes metadata. Failures retain Settings and must never let one module overwrite another module's viewport. The manager attempts best-effort cleanup on drop; OS kill/revocation is still a future gate.

`intent(...)` accepts only `ui.*` actions without a scope; other actions require a capability declared in the manifest and approved by a separate `ScopeAuthority`. `DenyAllScopes` is the default. No capability declarations or background flags produce privileges; in-process trusted adapters are **not** isolated from OS I/O, so trust is a construction-time condition, not a secure execution proof.

## Five distinct packages

| Navigation | Independent source | Adapter rule |
| --- | --- | --- |
| MAP | [rb_gis](https://github.com/pskeffington/rb_gis) | Full-canvas map only, location acquisition in host Settings, no auto-download |
| TV | [rust_tv](https://github.com/pskeffington/rust_tv) | One native player viewport, explicit stream selection; audio ownership policy remains open |
| RADIO | [rust_radio](https://github.com/pskeffington/rust_radio) | Native radio controls; requested audio/SDR/network capabilities need host authority |
| COMMS | [rusti_rc](https://github.com/pskeffington/rusti_rc) | Message interface with explicit connect/send consent; background receipt separately gated |
| MUSIC | [rust_music](https://github.com/pskeffington/rust_music) | Distinct lossless package; must use the host audio lease; trusted adapter and user input remain open |

**All five remain independent development repositories.** The core tests use synthetic adapters for each package identity; no real stream, RF device, IRC server, or GIS network endpoint is opened by these tests.

## Acceptance ladder

| Gate | Required result | Current |
| --- | --- | --- |
| ADAPTER-SDK-1 | SDK crate, stable typed envelope, bounded JSON, example | Source committed; build not verified |
| ADAPTER-HOST-1 | One trait, five synthetic package identities, lifecycle/error/Settings tests, unmount | Local stable/1.90 core and host tests pass with loopback-only cases excluded by sandbox; full CI pending for current changes |
| HOST-AUDIO-1 | Shared non-preemptive lease across native Radio and hosted Radio/Music/TV lifecycles | Local arbitration, Settings release, failed suspend/shutdown retention tests pass; real Music/TV engines open |
| HOST-UI-1 | Bind shared foreground to micro and desktop, keep permanent Settings, no split panes | Open |
| ADAPTER-MAP-1 | Real rb_gis adapter and offline ZIP/county parity | Open |
| ADAPTER-TV-1 | Native rust_tv display/audio contract | Open |
| ADAPTER-RADIO-1 | rust_radio interface and audio ownership | Open |
| ADAPTER-COMMS-1 | rusti_rc transport integration and explicit consent | Open |
| ADAPTER-IPC-1 | Sandboxed process runner, validated wire transport, signed installation, OS-enforced grants | Open |
| ADAPTER-REL-1 | Mac/Pi viewport, fault, disconnect, replay, security and resource-budget field acceptance | Open |

**Verification limitation:** Rust stable and 1.90.0 passed the offline core/application suites with only loopback socket tests excluded because this sandbox denies local socket creation. Warnings-denied Clippy passed on both toolchains. Hosted exact-head CI is still required for the full unfiltered matrix. This verifies the host reservation logic, not live shell mounting or native MUSIC/TV output. See [validation](VALIDATION.md) and [host roadmap](HOST_PACKAGE_ROADMAP.md).

# Rust-Boy universal package adapter (v1 development contract)

**Status (2026-10-29): RB-CX-00 accepted; RB-CX-01 implementation in progress.** Exact-head first-party CI passed RB-CX-00 on `c425994eb8a172c357c6920e29d2b4d38cdc5050` (run [40824698037](https://github.com/pskeffington/pip_boy/actions/runs/40824698037)). The bounded host-rendered view contract below is being added in RB-CX-01. This remains distinct from an external plug-in loader, OS sandbox, or production release. A read-only first-party RADIO catalogue is mounted by explicit compile-time bootstrap; MAP, TV and COMMS remain unmounted.

## Goal

Rust-Boy behaves like a universal socket: the shell owns the compact package switcher, the **one foreground package** viewport, persistent **Settings**, organizational trust, and network/device grants. Packages own their internal tools and independently maintained source repositories. The host must not contain hardcoded MAP/TV/RADIO/COMMS algorithms.

## Code ownership

| Layer | Rust source | Role |
| --- | --- | --- |
| Portable SDK | [`rust-boy-adapter-sdk`](../crates/rust-boy-adapter-sdk/src/lib.rs) | Versioned wire vocabulary, viewport, bounded data-only `PackageView`, `UniversalPackageAdapter` trait and deny-all scope policy interface |
| Core host | [`universal_adapter.rs`](../crates/pipboy-core/src/universal_adapter.rs) | Trusted adapter registration, lifecycle orchestration, one-package foreground ownership, host Settings, conservative intent gating |
| Manifest registry | [`package_socket.rs`](../crates/pipboy-core/src/package_socket.rs) | Strict schema-1 manifests, four inert reference fixtures, explicit local discovery |
| First-party bootstrap | [`builtin_packages.rs`](../crates/pipboy-core/src/builtin_packages.rs) | Passive pinned `rust_radio` catalogue; zero capabilities, no playback or network I/O |
| Shell UI | `crates/pipboy/assets/` | Mounted-roster selection and host Settings linked; package view schema maps only to host-created DOM, legacy tools retained |
| Device / network broker | `crates/rust-boy-broker/` | Deny-only precursor; no external module hardware privilege bridge yet |

The lightweight SDK depends only on `serde` and `serde_json`. The four independently developed packages can depend on this crate rather than on `pipboy-core` (which includes unrelated host and radio dependencies). Pin a reviewed Git commit when using the SDK in another repository; do not follow an unbounded main branch in released binaries.

The [machine-readable adapter contract](../catalog/host_adapter_contract.json) indexes version, bounds, command/reply vocabulary, the four package IDs and host ownership for automation. Its consistency with the Rust SDK is checked by the offline catalog validator.

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

- Version: `protocol=1`, independent from manifest `schema_version=1` and `host_api=1`.
- Request: `AdapterEnvelope { protocol, package_id, request_id, body }`; request IDs are monotonic within the host instance.
- Commands: `activate(viewport)`, `suspend`, `resume(viewport)`, `resize(viewport)`, `intent(action, required_scope, payload)`, `shutdown`.
- Replies: inert `ack` or `view(state)` JSON. A `view` must validate as `PackageView { version: 1, title, blocks }`; blocks are `heading`, `text`, `status`, or `list` data only. The core rejects malformed/unknown blocks and faults the adapter before exposing the view. Desktop and micro render supported blocks with host-created elements and text nodes; they **never** execute package-supplied JavaScript, HTML, links, or callbacks.
- Bounds: 16 KiB serialized request/reply; 4 KiB intent payload; viewport dimensions 320..4096 wide, 240..4096 high.
- View bounds: title 80 bytes, at most 24 blocks, text 512 bytes, status labels 80/value 160 bytes, and lists of at most 24 items of 160 bytes each. The current view contract is display-only; package input actions and host capability dispatch belong to the later RB-CX-02 security gate.
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

## Four initial packages

| Navigation | Independent source | Adapter rule |
| --- | --- | --- |
| MAP | [rb_gis](https://github.com/pskeffington/rb_gis) | Full-canvas map only, location acquisition in host Settings, no auto-download |
| TV | [rust_tv](https://github.com/pskeffington/rust_tv) | One native player viewport, explicit stream selection; audio ownership policy remains open |
| RADIO | [rust_radio](https://github.com/pskeffington/rust_radio) | Native radio controls; requested audio/SDR/network capabilities need host authority |
| COMMS | [rusti_rc](https://github.com/pskeffington/rusti_rc) | Message interface with explicit connect/send consent; background receipt separately gated |

**All four remain independent development repositories.** The core tests use synthetic adapters for each identity plus a read-only pinned `rust_radio` catalogue; no stream, RF device, IRC server or GIS endpoint is opened on host registration or catalogue selection.

## Acceptance ladder

| Gate | Required result | Current |
| --- | --- | --- |
| ADAPTER-SDK-1 | SDK crate, stable typed envelope, bounded JSON, example | Workspace tests and build pass; protocol preview, not a published compatibility promise |
| ADAPTER-HOST-1 | One trait, four synthetic adapters, lifecycle/error/Settings tests, unmount | Workspace tests pass for trusted in-process fixtures |
| HOST-UI-1 | Bind shared foreground to micro and desktop, keep permanent Settings, no split panes | Desktop and micro selectors use shared host foreground APIs and inert text views; the newly compiled passive RADIO catalogue is mounted but its real browser acceptance is open |
| ADAPTER-IPC-LISTENER-0 | Private Unix listener with expected-process peer credentials | Focused tests pass on macOS; listener is not wired to a launcher, adapter session, or signature verification |
| ADAPTER-RADIO-CATALOG-0 | Compile-time `rust_radio` metadata adapter without audio/network/background grants | Candidate; new Rust, CI and browser validation pending |
| ADAPTER-MAP-1 | Real rb_gis adapter and offline ZIP/county parity | Open |
| ADAPTER-TV-1 | Native rust_tv display/audio contract | Open |
| ADAPTER-RADIO-1 | rust_radio interface and audio ownership | Open |
| ADAPTER-COMMS-1 | rusti_rc transport integration and explicit consent | Open |
| ADAPTER-IPC-1 | Sandboxed process runner, validated wire transport, signed installation, OS-enforced grants | Open |
| ADAPTER-REL-1 | Mac/Pi viewport, fault, disconnect, replay, security and resource-budget field acceptance | Open |

**Verification limitation:** Local workspace tests, Clippy and repository validators pass. The current PR head has no GitHub check/status entries, and Mac/Pi field acceptance remains open. See [validation](VALIDATION.md) and [host roadmap](HOST_PACKAGE_ROADMAP.md).

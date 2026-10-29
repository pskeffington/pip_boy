# rust-boy-adapter-sdk

Small, host-agnostic Rust package contract for independent Rust-Boy modules.

**Development preview:** Protocol version 1, not yet a stable published crate or signed-package release. This crate provides typed versioned `AdapterEnvelope` requests, `AdapterReply` results, the `UniversalPackageAdapter` trait, bounded serialization, and deny-all scope policy primitives. It does **not** load modules, grant device/network permissions, open sockets, render HTML, or sandbox package code.

```bash
cargo test --locked -p rust-boy-adapter-sdk
cargo run --locked -p rust-boy-adapter-sdk --example hello_adapter
```

For a separate repository, depend on this Git source at an explicitly reviewed commit and import the `rust_boy_adapter_sdk` crate. The package should remain independently runnable outside Rust Boy and handle `Activate`, `Suspend`, `Resume`, `Resize`, `Intent` and `Shutdown` through the same protocol.

See [the universal adapter integration guide](../../docs/UNIVERSAL_ADAPTER.md), [host package socket contract](../../docs/PACKAGE_SOCKET.md) and [phased host roadmap](../../docs/HOST_PACKAGE_ROADMAP.md). The first adapter targets are `rb_gis` (MAP), `rust_tv` (TV), `rust_radio` (RADIO), and `rusti_rc` (COMMS). None is represented as installed or mounted in the live Rust-Boy shell.

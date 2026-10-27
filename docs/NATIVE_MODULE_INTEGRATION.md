# Rust-Boy native module integration: rust_radio / rust_stream / rusti_rc

## First implementation gate

`rust-boy-module-host` is a **compiled, network-free native Rust lifecycle/policy contract** in this workspace. It is not a plugin loader and does not claim that any of the three external repositories has been linked, rendered or validated on Mac/Pi. Its unit tests cover deny-by-default, unavailable status, duplicate start prevention and per-module capability isolation.

**Integration model:** use pinned Git revisions (or vendored release artifacts with audited provenance) and Cargo dependencies, never spawn a separate program or launch a browser window as a substitute for native integration. Preserve independent MIT source repositories and make package versions and licenses visible in About/Settings. When importing upstream crates, update `Cargo.lock` and run MSRV/CI checks rather than tracking mutable `main`.

| External repository | Native ownership | Rust-Boy surface | Current boundary |
| --- | --- | --- | --- |
| `pskeffington/rust_radio` | `rust-radio-core`, `rust-radio-audio`, `rust-radio-gui` | RADIO, Settings > Audio | `RadioPanel::show(ui)` is egui-only; Rust-Boy desktop and micro are served from a localhost web UI. Keep existing working in-process Rust-Boy radio until an explicit Rust service API migration and audio regression pass. |
| `pskeffington/rust_stream` | `rust-stream-core` and opt-in GStreamer backend | MEDIA / SECURITY / NEWS / TRAFFIC | `Playback::poll()` supplies RGBA frames to a **host-owned renderer**; the standalone egui demo is not embeddable as a browser iframe. Never call a stream LIVE without actual decoded frames. |
| `pskeffington/rusti_rc` | `rusti-adapter`, `rusti-proto`, opt-in separate TLS client | MESSAGES > IRC (untrusted) | Root portable adapter is receive-only text parsing. TLS client is in a separate `net/` workspace. IRC nicknames are not organizational identities. |

## Host integration sequence

1. Pin and vendor/lock reviewed revisions of each independent crate; add opt-in cargo features `native-radio`, `native-stream`, `native-irc`. Do not enable media networks at compile, startup or view-open time. Test builds on the declared Rust MSRV and the actual Mac toolchain.
2. Bridge Rust Radio's in-process audio controller to Rust-Boy's existing HTTP dashboard radio endpoints, preserving current Play/Stop, output-device selection and stored favorites. Avoid two concurrent radio engines; remove Rust-Boy's copied player only after regression tests.
3. Add a process-local media manager to the native Rust server. All decoded frames remain local, with bounded latest-frame buffering and explicit start/stop requests. Render locally in existing HUD, with a single-page 2x2 or focused layout. No arbitrary remote source URL, embedded service site, unapproved redirect, unauthenticated internal camera or automatic playback.
4. Mount `rusti-adapter` as the validation/sanitization layer for display-only IRC messages, feature-gated and disabled by default. Separately integrate a verified-TLS client with user-initiated connect/disconnect and OS-secret references. Never interpret IRC traffic as Command assignments or verified identities.
5. Connect mounted module lifecycles to `rust-boy-module-host::NativeModule`. Role/tool visibility is not permission; explicit host permissions and institutional enrollment remain separate. A module's `Stopping` state must hold its slot until cancellation finishes.
6. Test shutdown, reconnect, source switch, offline state, blocked IP/SSRF, redirect policy, radio-output changes and concurrent 4-stream pressure, then validate real Mac and Raspberry Pi display/audio before making these the default paths.

## Acceptance checklist

- [x] Shared native Rust host state/capability contract with offline unit tests.
- [ ] Pinned upstream Cargo sources and committed lockfile.
- [ ] Native Radio adapter using the upstream controller with parity against current on-device Mac playback.
- [ ] Native Stream adapter using decoded RGBA frames inside Rust-Boy HUD.
- [ ] IRC client network transport with verified TLS, opt-in connection, sanitization and secret-store lifecycle.
- [ ] UI navigation RADIO / MEDIA / MESSAGES and shutdown/revocation wiring.
- [ ] CI, Mac physical playback, Pi touchscreen/ARM and real-device security passes.

`rust-boy-module-host` intentionally does not initiate connections, load dynamic libraries, or assign trust to external messages. Treat all open acceptance items as pending.

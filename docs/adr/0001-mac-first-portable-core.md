# ADR 0001 — Mac-first development with portable Rust core and future Raspberry Pi adapters

- **Status:** Accepted as development direction (2026-10-26); Raspberry Pi hardware acceptance remains open
- **Decision owners:** Rust-Boy project maintainers
- **Scope:** Architecture and release qualification, not procurement

## Context

Rust-Boy already runs as a Rust workspace with local desktop/micro HUDs, synthetic fixtures, a deny-only peripheral broker and separate operator-gated network adapters. The team is building and testing on a Mac before purchasing or constructing a dedicated Raspberry Pi handheld. Audio, GPS, SDR and future button/battery interfaces will vary by OS and hardware.

The current [repository map](../REPO_STRUCTURE.md) and [architecture guide](../ARCHITECTURE.md) are authoritative for existing code; this record states a target layering decision, **not** a claim that all modules are already hardware-neutral.

## Decision

1. **One Rust codebase.** Continue to use the existing Cargo workspace and shared `Cargo.lock`. Do not fork a Raspberry Pi product or a second independent HUD.
2. **Mac-first verification.** Develop and field-test features on the Mac. Isolate OS/hardware interfaces behind explicit traits or narrow adapters where they are actually needed; use deterministic fixtures for absent devices.
3. **Pi-later qualification.** Before custom Pi hardware is selected, record required interfaces: audio output/Bluetooth, receive-only SDR, GNSS, 480×320 display/touch/buttons, network, storage, battery and thermal telemetry. Pi binaries and peripheral tests must be verified separately.
4. **Boundaries and consent.** Core business rules, message integrity, safe status and storage must not depend on UI/browser trust. Physical access requires explicit grants and audited adapters; the current broker stays deny-only.
5. **Compatibility.** Preserve `pipboy` CLI, `pip_boy` repository, `PIPBOY_HOME`, data schemas and existing loopback API contracts unless separately migrated.
6. **Evidence labels.** Distinguish source present, CI verified, Mac field tested, Pi hardware tested and released. Neither a successful Mac test nor cross-platform `cargo check` proves an ARM64 peripheral works.

## Considered alternatives

| Option | Assessment |
| --- | --- |
| One Mac-only application followed by a separate Pi rewrite | Reject: duplicates protocol, role and privacy logic; raises regression risk |
| Buy/freeze Pi parts immediately, then change software to fit | Defer: prematurely constrains display, power and SDR interface choices |
| Build a full abstraction framework before actual adapters exist | Reject: speculative interfaces increase maintenance and complexity |
| Preserve workspace; extract small adapters when justified | **Chosen:** tested code remains shared; hardware is introduced incrementally |

## Consequences

- A new hardware adapter needs an explicit capability/permission contract, failure states, deterministic mock tests and an optional field acceptance plan.
- Audio and radio routing remain in-process Rust, while Bluetooth pairing is delegated to OS services.
- No new crate is required merely because a file is large. Extract code only after tests define its current behavior.
- The [component ownership index](../../catalog/component_index.json) lists authoritative entrypoints on `main`; radio work in PR #35 may be added when merged and tested.
- Future ARM64 deployment may require native build dependencies, ALSA/PipeWire configuration, device permissions, kernel/USB support and new packaging/release tests.

## Confirmation

Validate the ownership map with `python3 tools/validate_component_index.py` and `python3 tools/test_component_index.py`, then use the normal workspace CI gates. Before stating Pi compatibility, build on or for the selected Pi target and test its actual audio, RF, GNSS and display peripherals. Record results in [validation evidence](../VALIDATION.md).

## References

- [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Embedded Rust portability](https://doc.rust-lang.org/embedded-book/portability/index.html)
- [Rust API documentation guidelines](https://rust-lang.github.io/api-guidelines/documentation.html)
- [MADR architectural decision templates](https://adr.github.io/adr-templates/)

# Rust-Boy package socket contract (draft v1)

Status: HOST-SOCKET-0 design/fixture only. No external executable loader, runtime sandbox, or rb_gis integration is claimed.

## Pop-in model

The **socket** belongs to Rust-Boy; packages are independently maintained, installed and replaced. A package must not modify host navigation, Settings, role authority, device credentials, networking policy, or other packages' UI. The host shows **exactly one** package in its foreground viewport. The socket is a **versioned protocol**, not a Rust `dyn Trait` binary ABI and not an OS container.

Package discovery reads an installed manifest from a local allowlisted package directory; discovery must do **zero I/O other than local manifest reads**. Resolve package ID -> compatible runner -> package view and service capabilities. The host must retain Settings when the package fails.

### Required manifest keys

- `schema_version`: fixed integer 1 in first gate.
- `package_id`: lowercase stable reverse-DNS identifier, never inferred from UI label.
- `name`, `version`, `publisher`, `license`, `source_repository`: package identity/provenance, not proof of authenticity.
- `host_api`: exact supported socket protocol major version (1).
- `entrypoint`: `type` (`external_process`, `builtin_adapter`); `command` only for allowlisted/signed installed packages at future runtime gate. Manifest must not cause execution during scanning.
- `ui`: `mode` (host_view) and `min_width`/`min_height`; package content only.
- `capabilities`: explicit requested scopes, denied until host policy approves.
- `background`: `allowed` boolean; explicit service contract independent of foreground selection.

Reference fixtures in [host package fixtures](../catalog/host_packages/) are **examples only**, not installed package inventory or executable instructions. Never trust publisher/source strings to authorize execution.

## Initial four-package integration set

The following source repositories are **independent development projects**, not installed or executable host packages. Their manifests are schema-1 **design fixtures** only.

| Display | Repository | Fixture | Intended foreground |
| --- | --- | --- | --- |
| MAP | [rb_gis](https://github.com/pskeffington/rb_gis) | [GIS fixture](../catalog/host_packages/rb_gis.example.json) | Full map viewport |
| TV | [rust_tv](https://github.com/pskeffington/rust_tv) | [TV fixture](../catalog/host_packages/rust_tv.example.json) | Single TV player viewport |
| RADIO | [rust_radio](https://github.com/pskeffington/rust_radio) | [Radio fixture](../catalog/host_packages/rust_radio.example.json) | Native radio controls |
| COMMS | [rusti_rc](https://github.com/pskeffington/rusti_rc) | [Comms fixture](../catalog/host_packages/rusti_rc.example.json) | Native message interface |

All four must implement the same host adapter boundary: version negotiation, no-IO discovery, one foreground viewport, host-owned Settings, explicit capability grant, fault isolation and deterministic unavailability. TV may not automatically fetch streams, RADIO may not automatically access SDR, COMMS may not automatically join channels, MAP may not automatically acquire location or download layers. Background declarations in fixtures are **requests**, never grants.

Integration order: manifest validation for all four first, then independently validated MAP / RADIO / TV / COMMS adapters. Module internals and independent CI remain in their own repositories.

## GIS extraction: strangler adapter

Existing Rust-Boy map behavior is the compatibility baseline. GIS implementation and county/ZIP logic move to `pskeffington/rb_gis` in deliberate slices. Maintain a temporary Rust-Boy-owned **MAP adapter** at the socket boundary, instead of reimplementing GIS in host shell.

1. Define the map request/response contracts: current location (explicit opt-in), ZIP/county resolve, offline cached layers, viewport projection, markers, download status, errors and provenance.
2. Identify current map handlers, cached schemas and UI assets; capture golden fixtures for ZIP -> town -> county and county boundary handoff, offline restart, map occupies available viewport and no background acquisition.
3. Build a self-contained rb_gis library/CLI API with deterministic fixtures and no compulsory network. Do not remove working Rust-Boy map routes until parity is demonstrated.
4. Pin an rb_gis revision as a trusted **builtin adapter** first. Later bridge it through versioned IPC for independently deployed packages and isolation. The host invokes map via adapter and never imports GIS implementation types in its shell.
5. Run desktop and handheld parity, error, cache migration and rollback tests. Only then remove duplicate map implementation and update ownership indexes.

### Socket protocol outline

`host -> package: activate(viewport, grant_handles), suspend, resume, deactivate, resize, intent(payload)`

`package -> host: render_state, request_capability, emit_event, health, fatal_error`

All messages are bounded and versioned. Module content is clipped to one host viewport. The socket denies implicit location/device/network access and never lets packages forge host-origin control actions. Use opaque handles for granted capabilities rather than passing bearer credentials. Do not use an unconstrained shell command from user-provided JSON.

### Disconnect, failure and recovery

No installed package -> show host-owned empty state with Settings. Missing/incompatible GIS -> MAP unavailable, other packages usable. Package crash -> release foreground ownership, retain host chrome, fail closed. Reinstall/update -> explicit user action and verified package integrity. Broker revocation -> close handles and force package out of privileged operations. Switching away from MAP must not start a GIS download; map cache persists under the existing data contract until migration is accepted.

## Acceptance gates

| Gate | Change | Pass condition |
| --- | --- | --- |
| SOCKET-0 | Machine-readable manifest fixtures and documented schema | JSON parses; duplicate IDs/version mismatches reject in planned registry; no scan-side effects |
| SOCKET-1 | Host-owned one-package viewport and permanent Settings | 480x320, 800x480, 1024x768, 1440x900; no host scroll; MAP takes full remaining viewport |
| GIS-EXTRACT-1 | Inventory legacy map interfaces/cache fixtures | Map API/cache/test owner map is complete |
| GIS-EXTRACT-2 | rb_gis domain parity with deterministic fixtures | ZIP/county/offline tests match old results; no automatic network |
| GIS-EXTRACT-3 | Trusted pinned rb_gis adapter | MAP navigation loads adapter; zero GIS implementation in shell; no regression |
| SOCKET-2 | Process IPC and explicit package lifecycle | Independent module versioning and crash isolation tested |
| SOCKET-SEC-1 | Capability broker, signing and isolation policy | Unauthorized package denied; revocation and role tests pass |
| GIS-EXTRACT-4 | Remove old GIS implementation after cutover | No duplicate logic; rollback/upgrade, Mac/Pi acceptance |

Avoid conflating *socket compatibility* with OS-level process isolation. Until SOCKET-SEC-1, accept trusted first-party adapters only.

See [host phased roadmap](HOST_PACKAGE_ROADMAP.md), [object model](OBJECT_MODEL.md), and [map testing](MAC_MAP_TESTING.md).

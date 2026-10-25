# Rust-Boy | Modules, features, and capability plan

**Status: roadmap, not finished hardware.** Source of truth: [`catalog/module_plan.json`](../catalog/module_plan.json). Snapshot: 2026-10-24.

- **Implemented, unverified** means code exists but successful compilation, CI, and platform smoke tests are not yet demonstrated.
- **Planned** means design and acceptance criteria only; no hardware, GNSS, SDR, mesh or remote service connectivity is represented as live.
- A capability listed here is a **required permission**, not one currently acquired or enforced by an OS.
- Modules are grouped by dependency gate; timing is intentionally unspecified.

## G0 — Initial native baseline

| ID | Module | Priority | State | Prerequisites | Evidence |
| --- | --- | --- | --- | --- | --- |
| `stat` | Host status | P0 | implemented unverified | — | [L03](LITERATURE_MATRIX.md#l03), [L19](LITERATURE_MATRIX.md#l19), [L31](LITERATURE_MATRIX.md#l31) |
| `library` | Reference library | P0 | implemented unverified | — | [L10](LITERATURE_MATRIX.md#l10), [L19](LITERATURE_MATRIX.md#l19) |
| `notes` | Offline field notes | P0 | implemented unverified | — | [L04](LITERATURE_MATRIX.md#l04), [L19](LITERATURE_MATRIX.md#l19) |

### `stat` — Host status

**Purpose:** core · **Gate:** G0 · **Priority:** P0 · **Status:** implemented_unverified.  
**Requested capabilities:** `host.read`.

**Planned behaviors:** CPU/memory/OS/uptime snapshot; Human-readable and JSON output.

**Acceptance criteria:** 1. Status runs without network or devices  
2. No fabricated hardware values

### `library` — Reference library

**Purpose:** core · **Gate:** G0 · **Priority:** P0 · **Status:** implemented_unverified.  
**Requested capabilities:** `files.read`.

**Planned behaviors:** Bundled GitHub source catalog; License and reuse classification.

**Acceptance criteria:** 1. Catalog validates unique IDs  
2. No third-party source vendoring

### `notes` — Offline field notes

**Purpose:** core · **Gate:** G0 · **Priority:** P0 · **Status:** implemented_unverified.  
**Requested capabilities:** `files.write`.

**Planned behaviors:** User-directed create/list notes; Local JSON persistence.

**Acceptance criteria:** 1. No write without explicit user action  
2. Malformed notes rejected

## G1 — Local observability and governance

| ID | Module | Priority | State | Prerequisites | Evidence |
| --- | --- | --- | --- | --- | --- |
| `telemetry` | Telemetry event stream | P0 | planned | `stat` | [L01](LITERATURE_MATRIX.md#l01), [L02](LITERATURE_MATRIX.md#l02), [L03](LITERATURE_MATRIX.md#l03), [L19](LITERATURE_MATRIX.md#l19), [L25](LITERATURE_MATRIX.md#l25), [L27](LITERATURE_MATRIX.md#l27), [L30](LITERATURE_MATRIX.md#l30), [L31](LITERATURE_MATRIX.md#l31), [L32](LITERATURE_MATRIX.md#l32), [L33](LITERATURE_MATRIX.md#l33) |
| `storage` | Storage and schema service | P0 | planned | `notes` | [L04](LITERATURE_MATRIX.md#l04), [L09](LITERATURE_MATRIX.md#l09), [L19](LITERATURE_MATRIX.md#l19), [L26](LITERATURE_MATRIX.md#l26), [L33](LITERATURE_MATRIX.md#l33) |
| `power` | Power and resources | P1 | planned | `stat` | [L03](LITERATURE_MATRIX.md#l03), [L27](LITERATURE_MATRIX.md#l27), [L29](LITERATURE_MATRIX.md#l29) |
| `network` | Local network link status | P1 | planned | `stat` | [L20](LITERATURE_MATRIX.md#l20) |
| `settings` | Settings and permission UI | P0 | planned | `storage` | [L16](LITERATURE_MATRIX.md#l16), [L22](LITERATURE_MATRIX.md#l22) |
| `accessibility` | Accessible console | P0 | planned | `stat`, `settings` | [L05](LITERATURE_MATRIX.md#l05), [L18](LITERATURE_MATRIX.md#l18) |
| `diagnostics` | Diagnostics and evidence audit | P0 | planned | `library`, `settings` | [L16](LITERATURE_MATRIX.md#l16), [L17](LITERATURE_MATRIX.md#l17), [L26](LITERATURE_MATRIX.md#l26), [L32](LITERATURE_MATRIX.md#l32), [L34](LITERATURE_MATRIX.md#l34) |

### `telemetry` — Telemetry event stream

Partial implementation: TUI and finite NDJSON use a lazy `HostTelemetryModule` through lifecycle/capability orchestration. TUI P revokes/resumes local host reads, clearing buffered values. Local Linux tests pass; persistent per-adapter consent, OS brokering and broader G1 acceptance remain open.

**Purpose:** core · **Gate:** G1 · **Priority:** P0 · **Status:** planned.  
**Requested capabilities:** `host.read`, `files.write`.

**Planned behaviors:** Provenance-tagged samples; NDJSON export and interval controls; Bounded in-memory buffers.

**Acceptance criteria:** 1. Tests cover source, clock and sample freshness  
2. No background uploads or implicit files

### `storage` — Storage and schema service

**Purpose:** core · **Gate:** G1 · **Priority:** P0 · **Status:** planned.  
**Requested capabilities:** `files.read`, `files.write`.

**Planned behaviors:** Versioned local collections; Import/export and backup; Atomic write validation.

**Acceptance criteria:** 1. Old schemas migrate with fixtures  
2. Interrupted write never silently destroys existing notes

### `power` — Power and resources

**Purpose:** core · **Gate:** G1 · **Priority:** P1 · **Status:** planned.  
**Requested capabilities:** `host.read`.

**Planned behaviors:** Battery state if OS provides it; Disk and thermal indicators; Low-power refresh profiles.

**Acceptance criteria:** 1. Unsupported sensors read unknown not zero  
2. Idle profiles measured on supported OS

### `network` — Local network link status

**Purpose:** core · **Gate:** G1 · **Priority:** P1 · **Status:** planned.  
**Requested capabilities:** `host.read`.

**Planned behaviors:** Read-only interface and link diagnostics; No active scanning; Offline health indicators.

**Acceptance criteria:** 1. Default makes no outbound requests  
2. Network data reports source and freshness

### `settings` — Settings and permission UI

**Purpose:** core · **Gate:** G1 · **Priority:** P0 · **Status:** planned.  
**Requested capabilities:** `files.read`, `files.write`.

**Planned behaviors:** Explicit per-adapter permission choices; Reversible configuration; Disabled-by-default external I/O.

**Acceptance criteria:** 1. Permissions tested before adapter callback  
2. Settings migration and revocation tests

### `accessibility` — Accessible console

**Purpose:** presentation · **Gate:** G1 · **Priority:** P0 · **Status:** planned.  
**Requested capabilities:** none.

**Planned behaviors:** Keyboard-only navigation; Contrast and color-independent status labels; Reduced-motion / low-bandwidth mode.

**Acceptance criteria:** 1. 80x24 and smaller terminal fixtures  
2. No status relies on color alone

### `diagnostics` — Diagnostics and evidence audit

**Purpose:** core · **Gate:** G1 · **Priority:** P0 · **Status:** planned.  
**Requested capabilities:** `files.read`.

**Planned behaviors:** Schema checks and adapter readiness; Human-readable diagnostics; Feature readiness truth table.

**Acceptance criteria:** 1. No device probing without consent  
2. Every check reports pass/fail/not-run

## G2 — Wearable I/O and simulation

| ID | Module | Priority | State | Prerequisites | Evidence |
| --- | --- | --- | --- | --- | --- |
| `gpio` | Physical controls | P1 | planned | `settings` | [L06](LITERATURE_MATRIX.md#l06), [L07](LITERATURE_MATRIX.md#l07) |
| `devices` | Device broker | P0 | planned | `settings`, `diagnostics` | [L02](LITERATURE_MATRIX.md#l02), [L03](LITERATURE_MATRIX.md#l03), [L07](LITERATURE_MATRIX.md#l07), [L14](LITERATURE_MATRIX.md#l14) |
| `display` | Wearable display layer | P1 | planned | `accessibility`, `gpio` | [L05](LITERATURE_MATRIX.md#l05), [L18](LITERATURE_MATRIX.md#l18) |
| `serial` | Serial transport | P1 | planned | `devices` | [L14](LITERATURE_MATRIX.md#l14) |
| `simulator` | Hardware event simulator | P0 | planned | `gpio`, `display` | [L05](LITERATURE_MATRIX.md#l05), [L25](LITERATURE_MATRIX.md#l25), [L26](LITERATURE_MATRIX.md#l26) |

### `gpio` — Physical controls

**Purpose:** hardware · **Gate:** G2 · **Priority:** P1 · **Status:** planned.  
**Requested capabilities:** `gpio.read`.

**Planned behaviors:** Rotary encoder/button event inputs; Debounce and long-press handling; Keyboard-equivalent action mapping.

**Acceptance criteria:** 1. Replay fixture on non-Pi host  
2. Recover cleanly from device unavailability

### `devices` — Device broker

**Purpose:** hardware · **Gate:** G2 · **Priority:** P0 · **Status:** planned.  
**Requested capabilities:** `device.enumerate`.

**Planned behaviors:** Explicit device selection; Capabilities and permission broker; Safe disconnect and timeouts.

**Acceptance criteria:** 1. Unknown/unapproved device stays closed  
2. Revoke permission blocks adapter invocation

### `display` — Wearable display layer

**Purpose:** presentation · **Gate:** G2 · **Priority:** P1 · **Status:** planned.  
**Requested capabilities:** `display.write`.

**Planned behaviors:** 480x320 and 720x720 layout profiles; No-flicker refresh; Frame snapshots.

**Acceptance criteria:** 1. Layout fixtures at each profile  
2. Accessible fallback without visual effects

### `serial` — Serial transport

**Purpose:** hardware · **Gate:** G2 · **Priority:** P1 · **Status:** planned.  
**Requested capabilities:** `serial.read`.

**Planned behaviors:** Selected-port read-only session; Framed input with size limits; Device disconnect recovery.

**Acceptance criteria:** 1. No implicit port enumeration or open  
2. Malformed frames and unplug tested

### `simulator` — Hardware event simulator

**Purpose:** testing · **Gate:** G2 · **Priority:** P0 · **Status:** planned.  
**Requested capabilities:** none.

**Planned behaviors:** Deterministic event playback; Mock GPIO/serial/display adapters; Recorded golden traces.

**Acceptance criteria:** 1. CI runs without hardware  
2. Replay gives same state transitions

## G3 — Field sensors and communications

| ID | Module | Priority | State | Prerequisites | Evidence |
| --- | --- | --- | --- | --- | --- |
| `gnss` | GNSS navigation input | P1 | planned | `devices`, `storage` | [L08](LITERATURE_MATRIX.md#l08), [L09](LITERATURE_MATRIX.md#l09) |
| `map` | Offline cartography | P1 | planned | `storage`, `gnss`, `display` | [L09](LITERATURE_MATRIX.md#l09), [L10](LITERATURE_MATRIX.md#l10), [L11](LITERATURE_MATRIX.md#l11) |
| `radio` | Receive-only SDR | P2 | planned | `devices`, `telemetry` | [L20](LITERATURE_MATRIX.md#l20), [L24](LITERATURE_MATRIX.md#l24), [L30](LITERATURE_MATRIX.md#l30) |
| `adsb` | ADS-B receiver overlay | P2 | planned | `radio`, `map` | [L23](LITERATURE_MATRIX.md#l23), [L24](LITERATURE_MATRIX.md#l24), [L30](LITERATURE_MATRIX.md#l30) |
| `ble` | Nearby BLE adapter | P2 | planned | `devices`, `settings` | [L12](LITERATURE_MATRIX.md#l12), [L13](LITERATURE_MATRIX.md#l13), [L22](LITERATURE_MATRIX.md#l22), [L28](LITERATURE_MATRIX.md#l28) |
| `mesh` | Authenticated local messaging | P2 | planned | `ble`, `serial`, `settings` | [L04](LITERATURE_MATRIX.md#l04), [L12](LITERATURE_MATRIX.md#l12), [L15](LITERATURE_MATRIX.md#l15), [L21](LITERATURE_MATRIX.md#l21), [L22](LITERATURE_MATRIX.md#l22), [L28](LITERATURE_MATRIX.md#l28), [L29](LITERATURE_MATRIX.md#l29) |
| `sensors` | Environmental sensor adapters | P2 | planned | `devices`, `telemetry` | [L06](LITERATURE_MATRIX.md#l06), [L20](LITERATURE_MATRIX.md#l20), [L29](LITERATURE_MATRIX.md#l29) |
| `weather` | Local weather panel | P2 | planned | `sensors` | [L04](LITERATURE_MATRIX.md#l04), [L19](LITERATURE_MATRIX.md#l19) |
| `audio` | Field audio outputs | P2 | planned | `settings` | [L18](LITERATURE_MATRIX.md#l18) |

### `gnss` — GNSS navigation input

**Purpose:** field · **Gate:** G3 · **Priority:** P1 · **Status:** planned.  
**Requested capabilities:** `gnss.read`.

**Planned behaviors:** gpsd and NMEA adapters where allowed; Fix quality and staleness; Opt-in coordinate history.

**Acceptance criteria:** 1. No-fix and stale-fix shown explicitly  
2. Location history disabled by default

### `map` — Offline cartography

**Purpose:** field · **Gate:** G3 · **Priority:** P1 · **Status:** planned.  
**Requested capabilities:** `files.read`.

**Planned behaviors:** GPX/GeoJSON import; User-supplied licensed offline tiles; Track overlays and source badges.

**Acceptance criteria:** 1. Attribution persisted with map bundle  
2. No OSM public raster tile scraping or prefetch

### `radio` — Receive-only SDR

**Purpose:** field · **Gate:** G3 · **Priority:** P2 · **Status:** planned.  
**Requested capabilities:** `sdr.receive`.

**Planned behaviors:** Explicit selected receive-only device; Read-only signal status and sample metadata; Replay data fixtures.

**Acceptance criteria:** 1. No transmit API in default build  
2. Disconnect/timeout and resource limits tested

### `adsb` — ADS-B receiver overlay

**Purpose:** field · **Gate:** G3 · **Priority:** P2 · **Status:** planned.  
**Requested capabilities:** `sdr.receive`.

**Planned behaviors:** Receive-only 1090ES / optional 978 UAT decoders; Aircraft records with timestamp and validity; Local-only track rendering.

**Acceptance criteria:** 1. Malformed frames rejected  
2. No surveillance claims beyond received, unverified data

### `ble` — Nearby BLE adapter

**Purpose:** field · **Gate:** G3 · **Priority:** P2 · **Status:** planned.  
**Requested capabilities:** `ble.discover`.

**Planned behaviors:** User-initiated discovery; Manual peer and peripheral selection; Pairing/connection indicators.

**Acceptance criteria:** 1. Zero scanning on startup  
2. Consent revoked shuts down adapter

### `mesh` — Authenticated local messaging

**Purpose:** communications · **Gate:** G3 · **Priority:** P2 · **Status:** planned.  
**Requested capabilities:** `mesh.transmit`.

**Planned behaviors:** Manually selected peers and transport; Replay-protected message envelopes; Explicit send confirmation.

**Acceptance criteria:** 1. Threat model and cryptographic review before TX  
2. No automatic broadcast or trust by proximity

### `sensors` — Environmental sensor adapters

**Purpose:** field · **Gate:** G3 · **Priority:** P2 · **Status:** planned.  
**Requested capabilities:** `sensor.read`.

**Planned behaviors:** Temperature/pressure/air quality on approved hardware; Units/calibration and provenance; Stale-data alarms.

**Acceptance criteria:** 1. Missing calibration flagged  
2. Read-only hardware paths verified

### `weather` — Local weather panel

**Purpose:** field · **Gate:** G3 · **Priority:** P2 · **Status:** planned.  
**Requested capabilities:** `files.read`.

**Planned behaviors:** Local sensor summary; Optional manually imported forecasts; Freshness, timestamp and units.

**Acceptance criteria:** 1. Forecasts never shown as live measurement  
2. Works with zero network connectivity

### `audio` — Field audio outputs

**Purpose:** presentation · **Gate:** G3 · **Priority:** P2 · **Status:** planned.  
**Requested capabilities:** `audio.play`.

**Planned behaviors:** Local media and accessibility cues; User-supplied licensed files; Mute and quiet-mode controls.

**Acceptance criteria:** 1. No bundled copyrighted Fallout audio  
2. Readable non-audio equivalents

## G4 — Distribution and ecosystem

| ID | Module | Priority | State | Prerequisites | Evidence |
| --- | --- | --- | --- | --- | --- |
| `plugins` | Versioned module API | P1 | planned | `devices`, `simulator`, `diagnostics` | [L01](LITERATURE_MATRIX.md#l01), [L02](LITERATURE_MATRIX.md#l02), [L15](LITERATURE_MATRIX.md#l15), [L16](LITERATURE_MATRIX.md#l16), [L17](LITERATURE_MATRIX.md#l17), [L25](LITERATURE_MATRIX.md#l25) |
| `feeds` | Opt-in external data connectors | P2 | planned | `settings`, `storage`, `network` | [L04](LITERATURE_MATRIX.md#l04), [L11](LITERATURE_MATRIX.md#l11), [L19](LITERATURE_MATRIX.md#l19) |
| `release` | Reproducible release pipeline | P0 | planned | `plugins`, `accessibility`, `simulator` | [L03](LITERATURE_MATRIX.md#l03), [L16](LITERATURE_MATRIX.md#l16), [L17](LITERATURE_MATRIX.md#l17), [L27](LITERATURE_MATRIX.md#l27), [L34](LITERATURE_MATRIX.md#l34) |

### `plugins` — Versioned module API

**Purpose:** platform · **Gate:** G4 · **Priority:** P1 · **Status:** planned.  
**Requested capabilities:** `files.read`.

**Planned behaviors:** Signed declarative manifests; Versioned typed adapter contracts; Static registration first, no arbitrary dlopen.

**Acceptance criteria:** 1. Unknown or unsigned modules rejected  
2. Never grant raw OS privileges via metadata alone

### `feeds` — Opt-in external data connectors

**Purpose:** platform · **Gate:** G4 · **Priority:** P2 · **Status:** planned.  
**Requested capabilities:** `network.fetch`.

**Planned behaviors:** User-configured endpoints; Request size/rate/time limits; Offline caches with provenance.

**Acceptance criteria:** 1. No automatic network requests in default config  
2. Source failures do not block local console

### `release` — Reproducible release pipeline

**Purpose:** platform · **Gate:** G4 · **Priority:** P0 · **Status:** planned.  
**Requested capabilities:** `files.read`.

**Planned behaviors:** Signed multi-platform builds; SBOM and supply chain attestations; Hardware and accessibility smoke matrix.

**Acceptance criteria:** 1. Release requires green jobs and human acceptance  
2. No SLSA compliance claim without evidence

## Deferred or explicitly excluded

- No background Bluetooth/Wi-Fi sweeps, unsolicited transmissions, RF jamming, or control of operational infrastructure.
- No proprietary Fallout game assets or radio content included in the distribution.
- No claim that LoRaWAN and Meshtastic use interchangeable network protocols.
- No loading untrusted native plugins as shared libraries until a separate ABI, sandbox and security review is completed.

## Traceability

For every implementation PR, link a module ID, an evidence ID, an acceptance test, and permission/capability review. The `pipboy plan` and `pipboy evidence` CLI commands will report these manifests once the core loader is integrated.

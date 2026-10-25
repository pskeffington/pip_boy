# Rust-Boy | Role-by-role capability roadmap

Reviewed: **2026-10-25**. Repository: `pip_boy`; product: **Rust-Boy**. This is a **roadmap**, not a claim that every feature is implemented or hardware-certified. The authoritative machine-readable matrix is [catalog/role_roadmap.json](../catalog/role_roadmap.json); it must match `PRESETS` in [workbench.rs](../crates/pipboy-core/src/workbench.rs).

## Product principles

1. **Ready out of the box:** Seven occupational profiles ship pre-pinned; a clean installation currently selects **Overseer Operations**. **Custom** is the deliberately blank, advanced option.
2. **A profile is a workspace, not a privilege:** Selecting or pinning a profile cannot open a camera, discover nearby hardware, use a PlayStation connection, acquire a location, or run an arbitrary program. Each real connector requires a separate user action and OS-enforced grant where appropriate.
3. **Two real display targets:** Develop initially on macOS (keyboard/Terminal) while defining a future **4-inch handheld** at a proposed 800×480 reference resolution and 480×320 fallback. These are design targets, not selected hardware specifications. Layouts must be readable, responsive, keyboard-first, and have touch targets when touch is available.
4. **Truthful states:** Distinguish static layout, simulation, real local tools, explicit opt-in connectors, stale data, and entirely unimplemented features. A pinned icon is not proof of a live feed.
5. **No IP confusion:** Vault-Tec terminology and citations inspire the profile names; tool design, UX, code, and visuals are original. No Vault-Tec affiliation or game artwork license is claimed. See [lore matrix](VAULT_TEC_LORE_MATRIX.md).

## Handheld micro display shell

[Micro terminal UI](MICRO_UI.md) provides the compact visual shell shared by the seven occupational templates and Custom: a top status band, six fixed thumb-sized navigation tabs, one active viewport and the existing confirmed profile save. Scout's geospatial page allows genuine offline waypoint/distance calculations, but a **working RustGIS map is deferred**. Security Chief's four-camera page is a no-video fixture with one-tap focus/back; genuine camera streams are a separate integration gate. The micro shell does not grant physical permissions or claim Pi hardware acceptance.

## Base dashboard and template activation

The [graphical dashboard implementation](DASHBOARD_UI.md) provides a shared UI foundation **before** role-specific extensions. Its left rail lists the seven preconfigured roles in canonical order with **Custom last**, offers non-mutating role previews and explicitly confirmed loading, and places pinned modules in a responsive deck. Custom is empty until deliberately populated. Reclamation Scout reserves a viewport for a future **real RustGIS launcher/renderer**; Security Chief currently uses four camera **mock-layout controls**, not network video. The Rust service is local only and uses the established WorkbenchStore, so CLI/TUI/web selections share one source of truth. Browser usability, accessibility and 4-inch hardware acceptance remain independent gates.

## Shared program gates and prerequisites

| ID | Deliverable | Hard exit criterion |
| --- | --- | --- |
| **CI_GREEN** | Verify the Rust 1.90 minimum, stable compiler and native platform checks at each release candidate | Stable, Rust 1.90, macOS and Windows quality jobs green at the exact shipped SHA |
| **HANDHELD** | Layout system for 4-inch displays, deterministic input state machine, feature flag for terminal vs graphical renderer | 2×2 card grid, focus control, keyboard navigation, tiny-window recovery, contrast and action labels verified; no assumed hardware performance |
| **OS_BROKER** | Truly enforced OS scoped network/camera/serial/GNSS resource grants, isolated handles, revocation | Deny-by-default, explicit consent, saved-denial precedence, spoof/corrupt-input tests, revoke and hot-unplug negative tests |
| **PLUGIN_SECURITY** | Typed, versioned and trusted extension contract | Unsigned/invalid module cannot run; no network/FS/device rights derived from a preset or config file |

**Release order:** verify exact Rust 1.90/compiler selection in CI first; finish honest R0 role-activation and desktop panel acceptance; implement R1 self-contained workflows; add R2 explicit live integrations; only then consider R3 field-readiness claims. Prioritize Security Chief's 2×2 camera UI at R1 because it is a specific user requirement. Do not mistake roadmap dependencies for delivery dates. Historical 1.81-named matrix jobs used stable because of a toolchain-file override; new 1.90 checks enforce the compiler identity.

## At-a-glance roster

| Profile ID | Job station | Lore type | Existing palette tools | Future modules |
| --- | --- | --- | --- | --- |
| `overseer` | Overseer Operations | canonical job | `mission-board`, `host`, `notes`, `library`, `game` | `alerts`, `reports` |
| `head-engineer` | Head Engineer Station | canonical job | `host`, `net-lab`, `byte-lab`, `simulator`, `notes` | `serial-console`, `sensor-console`, `device-graph` |
| `security-chief` | Security Chief Station | canonical job | `host`, `net-lab`, `byte-lab`, `notes` | `camera-wall`, `camera-health`, `event-log` |
| `vault-doctor` | Vault Doctor Station | canonical job | `library`, `notes`, `host` | `environment`, `checklists`, `reference-index` |
| `university-researcher` | University Research Station | role inspired | `byte-lab`, `library`, `notes`, `simulator`, `net-lab` | `data-lab`, `experiment-log`, `graph-view` |
| `reclamation-scout` | Reclamation Field Kit | role inspired | `geo-lab`, `game`, `net-lab`, `notes`, `host` | `map`, `coordinate-lab`, `gnss`, `track-log`, `tak-bridge`, `rustgis-adapter` |
| `records-custodian` | Records Custodian Station | role inspired | `library`, `notes`, `byte-lab` | `document-index`, `sha256`, `artifact-bundle` |
| `custom` | Custom Workbench | rust boy original | None (empty by design) | `layout-editor`, `module-sdk`, `sandbox-runtime` |

## Detailed role plans

### Overseer Operations (`overseer`)

**Purpose:** Coordinate people, systems and incidents from a concise command dashboard.  
**Lore:** canonical job · `VT-01`, `VT-03`, `RB-01`  
**Preloaded now:** `mission-board`, `host`, `notes`, `library`, `game`  
**New modules:** `alerts`, `reports`

**Operational needs**

- **must — At-a-glance system/companion health and truthful stale states** (`partially_available`).
- **must — Persistent delegated mission tasks with assignee, role, priority, due date, status, and manual acknowledgment** (`code_present_ci_pending`).
- **should — Manual email/clipboard task handoff, local status tracking, immutable-revision edit conflicts, and audit history** (`partially_available`).
- **later — Opt-in live sources and redacted report export** (`planned`).

| Gate | Deliverable | Acceptance test | State |
| --- | --- | --- | --- |
| **R0** Preset, launch and honest status | Overseer selected by default on a fresh install; Five pre-pinned tools including Task Control are visible and inert until launched; Existing boards survive upgrade | Fresh install shows Overseer profile without writing until user changes it; No automatic PlayStation/network connection; Linux/macOS/Windows CI passes | code present ci pending |
| **R1** Offline command desk | Persistent local Task Control: create/assign/reassign, priority, due dates, status and acknowledgment; Explicit copy/email-draft handoffs and manually confirmed sending; local event log and revision conflicts; At-a-glance micro/desktop role-aware task queues and operator summaries | Create, complete and reassign ten tasks offline with restart persistence and validation; Attempted stale revisions fail without overwriting; corrupt/future board preserved; No mail, camera, network or background notification is sent by loading a profile; delivery is not inferred | code present ci pending |
| **R2** Consent-based integrations | Explicitly connected game observations and permitted sensor status; Incident-to-task linking and event acknowledgment | Disconnect immediately marks readings stale; No network activity until a user explicitly connects | planned |
| **R3** Field command acceptance | Resource-aware handheld overview; Signed/redacted local reports and restore | Operator completes status-task-report journey on 4-inch target; Storage rollback and interrupted write are tested | planned |

### Head Engineer Station (`head-engineer`)

**Purpose:** Troubleshoot embedded systems, networks and hardware using reproducible diagnostics.  
**Lore:** canonical job · `VT-01`, `RB-01`  
**Preloaded now:** `host`, `net-lab`, `byte-lab`, `simulator`, `notes`  
**New modules:** `serial-console`, `sensor-console`, `device-graph`

**Operational needs**

- **must — Hex dumps, checksums, IPv4/CIDR math, host diagnostics and fixture runs** (`code_present_ci_pending`).
- **must — Device inventory and repeatable diagnostic recipes** (`planned`).
- **should — Read-only USB/serial log console with explicit scope** (`planned`).
- **later — Physical GPIO, environmental sensors and managed test rigs** (`planned`).

| Gate | Deliverable | Acceptance test | State |
| --- | --- | --- | --- |
| **R0** Ready engineering desk | Pinned host/NET/BYTE/SIM/NOTES palette; Offline CIDR, byte dump and CRC32 commands | Golden CIDR /24 /31 /32 and CRC32 vectors pass; All tools remain inactive until invoked | code present ci pending |
| **R1** Real offline engineering lab | Bounded hex/file-header inspector and developer test recipes; Diagnostic session reports with reproducible parameters | Input limits and invalid file/hex handling tested; Fixture replay produces the same output on repeat | planned |
| **R2** OS-authorized device diagnostics | Separately reviewed read-only serial gateway; Device identity, per-device consent, expiry and hot-unplug | USB device cannot open without OS permission and explicit session grant; Unplug/revoke closes handles and clears buffers | planned |
| **R3** Embedded hardware integration | Sensor modules and typed adapter SDK; Battery/runtime budgets and device-specific compatibility matrix | Physical device tested on macOS and chosen handheld prototype; No transmission capability is implied by receiver diagnostics | planned |

### Security Chief Station (`security-chief`)

**Purpose:** See four authorized IP-camera feeds on a small screen, instantly focus one, and investigate incidents.  
**Lore:** canonical job · `VT-01`, `STD-01`, `STD-02`, `STD-03`, `STD-05`, `STD-06`  
**Preloaded now:** `host`, `net-lab`, `byte-lab`, `notes`  
**New modules:** `camera-wall`, `camera-health`, `event-log`

**Operational needs**

- **must — Four-camera 2x2 wall with one-key focus and back** (`planned`).
- **must — Manual, permissioned import of IP cameras (JPEG/MJPEG first; RTSP/ONVIF later)** (`planned`).
- **must — Per-feed connected/loading/stale/disconnected/auth-error state and timestamps** (`planned`).
- **should — Minimal network/incident diagnostics and bounded evidence notes** (`partially_available`).
- **later — Opt-in motion/alarm events, export and optional PTZ** (`planned`).

| Gate | Deliverable | Acceptance test | State |
| --- | --- | --- | --- |
| **R0** Security desk baseline | Pinned host, network math, byte inspection and notes; No camera connection without explicit configuration | Security profile renders without claiming feeds live; Offline baseline is usable without network | code present ci pending |
| **R1** Four-feed UI with deterministic camera fixtures | 2×2 thumbnail wall and four separate feed state machines; 1–4 choose, Enter expands, Esc returns, loading/error/stale labels; 4-inch readable layout and input focus | All four synthetic feeds animate independently; Focus/back and missing/stale feeds tested at 800×480 and compact fallback; No external endpoints contacted | planned |
| **R2** Authorized real camera streams | Manually entered allowlisted HTTPS JPEG/MJPEG sources; Per-camera credentials in secure OS storage; strict TLS verification and redaction; RTSP H.264/H.265 decoder path and ONVIF Profile T compatibility by model | Four verified real feeds display simultaneously and can focus; No URLs/credentials in logs or public config; No surprise recording/discovery, audio or PTZ | planned |
| **R3** Operational security acceptance | Reconnect/backoff, bandwidth and frame freshness indicators; Optional local events and user-confirmed evidence export; Unplug, timeout, session expiry and offline recovery | Stale frames never carry a live indicator; Operator selects feed <1s under lab fixture load; Camera field matrix documents latency/memory/CPU on Mac and handheld | planned |

### Vault Doctor Station (`vault-doctor`)

**Purpose:** Maintain non-diagnostic health/environment reference and field observations without exposing patient records.  
**Lore:** canonical job · `VT-01`, `RB-01`  
**Preloaded now:** `library`, `notes`, `host`  
**New modules:** `environment`, `checklists`, `reference-index`

**Operational needs**

- **must — Trusted offline medical/public-health references with provenance** (`partially_available`).
- **must — Environmental observations: temperature, humidity, water and air quality (non-medical)** (`planned`).
- **should — Structured checklists with source-linked thresholds and units** (`planned`).
- **later — Explicitly consented sensors and privacy-governed exports** (`planned`).

| Gate | Deliverable | Acceptance test | State |
| --- | --- | --- | --- |
| **R0** Research-and-notes starter | Offline library, manual notes and host condition tools | No health inference or clinical diagnosis is claimed; No protected patient data are seeded | code present ci pending |
| **R1** Environmental observation desk | Manual readings with units, timestamps and missing-value explanations; Hazard-specific safety checklists backed by references | Synthetic and real/manual readings distinguish provenance; Out-of-range values do not trigger unsupported medical recommendations | planned |
| **R2** Opt-in environmental sensing | Approved temperature/humidity and non-clinical sensor adapters; Calibration record, source quality and stale-device warnings | Consent and calibration test gates pass; No clinical sensor device claimed compliant by default | planned |
| **R3** Field validation and privacy | Accessible checklist export; Privacy review before any person-specific data flow | No plaintext protected health record workflow offered by default; End-to-end offline reference-checklist-export acceptance | planned |

### University Research Station (`university-researcher`)

**Purpose:** Run small, reproducible experiments and inspect data without requiring proprietary applications.  
**Lore:** role inspired · `VT-01`, `RB-01`  
**Preloaded now:** `byte-lab`, `library`, `notes`, `simulator`, `net-lab`  
**New modules:** `data-lab`, `experiment-log`, `graph-view`

**Operational needs**

- **must — Reproducible sampling and synthetic test runs** (`partially_available`).
- **must — CSV/JSON inspection, simple descriptive statistics and unit-aware charts** (`planned`).
- **should — Experiment manifest, seeds and deterministic export** (`planned`).
- **later — Restricted plugin sandbox for advanced computation** (`planned`).

| Gate | Deliverable | Acceptance test | State |
| --- | --- | --- | --- |
| **R0** Preloaded research bench | Byte, reference, note, simulator and subnet tools | All experiments identified as synthetic where appropriate; No arbitrary scripts executed on profile activation | code present ci pending |
| **R1** Offline data lab | Bounded CSV/JSON import and schema reporting; Count/mean/median/variance and small-screen sparkline viewer | Known numerical fixtures agree with trusted reference values; Invalid, huge and missing inputs fail without changing raw data | planned |
| **R2** Reproducible sessions | Versioned experiment manifest with parameter set, seed and source hash; Saved/replayable in-memory simulation recipes | Exact replay generates matching deterministic artifacts; No implicit upload or network access | planned |
| **R3** Optional advanced computation | Versioned typed WASM plugin contract with capability gates; Snapshot/restore and independent process isolation tests | Untrusted plugin cannot read files, network or hardware without external enforcement; Process crash leaves project usable | planned |

### Reclamation Field Kit (`reclamation-scout`)

**Purpose:** Build a truly offline WGS84 field kit on Mac and Pi, interoperable with RustGIS and eventually opt-in TAK nodes.  
**Lore:** role inspired · `VT-02`, `VT-03`, `STD-04`, `STD-07`, `STD-08`, `STD-09`, `STD-10`  
**Preloaded now:** `geo-lab`, `game`, `net-lab`, `notes`, `host`  
**New modules:** `map`, `coordinate-lab`, `gnss`, `track-log`, `tak-bridge`, `rustgis-adapter`

**Operational needs**

- **must — Local WGS84 waypoints, approximate great-circle distances and bounded GeoJSON Point inspection** (`partially_available`).
- **must — Offline trip notes, saved landmarks and position uncertainty labels** (`planned`).
- **should — Opt-in GNSS, multi-geometry GeoJSON/GPX exchange and permitted TAK CoT interoperability** (`planned`).
- **later — Portable RustGIS-style map rendering, offline tile caching and Raspberry Pi display performance** (`planned`).

| Gate | Deliverable | Acceptance test | State |
| --- | --- | --- | --- |
| **R0** Preloaded field desk | Pre-pinned GIS Lab plus Game, NET, notes and host tools; Offline waypoint GeoJSON, spherical distance and bounded Point-only file inspection | GeoJSON exports [longitude,latitude] and accepts only -90..90 lat and -180..180 lon; All GIS commands remain offline with no GNSS, console or TAK connection on profile activation; Unit, smoke and Rust 1.90/macOS/Windows CI gates pass before release | code present ci pending |
| **R1** Offline RustGIS-compatible field mapping | Portable GeoJSON field layer, GPX imports, waypoints and manually selected cached offline tiles; Evaluate rustgis-geom/rustgis-data adapter separately, without silently increasing the Rust 1.90 MSRV; Touch/keyboard map navigation and DMS coordinate formats | RustGIS reads exported waypoint layers and Rust-Boy inspects RustGIS Point layers without conversion errors; Golden dateline, polar, invalid-coordinate and lossless waypoint round trips; No network access required for local map navigation | planned |
| **R2** Opt-in GNSS and TAK interoperability | Explicit OS-granted GNSS with accuracy/stale indicators and revocation; Bounded Cursor-on-Target XML/Protobuf adapter with operator-selected destinations and TLS identity; Test with independent OpenTAKServer on Raspberry Pi OS and a civilian TAK client | No position sharing before the user enables it, selects recipient and approves each session; Local CoT fixture tests pass; live Pi server interoperability is field-tested and credential-safe; Revocation stops transmission and marks positions stale | planned |
| **R3** Raspberry Pi handheld GIS acceptance | ARM64 Pi 5 native Scout build and tested RustGIS ARM64 deployment; Four-inch readout with waypoint and offline map controls, keyboard/touch fallback; Power/performance measurements plus independent Mac/PS4 game companion acceptance | Offline waypoint, coordinate and cached-map use passes on actual Pi hardware; Measured cold start, frame latency, RAM/CPU and battery draw recorded without guessed metrics; TAK server/client and PlayStation tests are reported separately from mock connections | planned |

### Records Custodian Station (`records-custodian`)

**Purpose:** Catalog documents and make integrity, provenance and export understandable.  
**Lore:** role inspired · `VT-03`, `STD-05`  
**Preloaded now:** `library`, `notes`, `byte-lab`  
**New modules:** `document-index`, `sha256`, `artifact-bundle`

**Operational needs**

- **must — Fast local index and search without uploading content** (`partially_available`).
- **must — SHA-256 file integrity, not CRC32 as a security claim** (`planned`).
- **should — Source citations, tagged records, import/export and schema migration** (`planned`).
- **later — Versioned optional encrypted archives and trusted sharing** (`planned`).

| Gate | Deliverable | Acceptance test | State |
| --- | --- | --- | --- |
| **R0** Curated archive desk | Library, notes and byte-lab tools | Notes remain local/plaintext with warnings; References and URLs preserve third-party licensing context | code present ci pending |
| **R1** Integrity workflow | Read-only, bounded SHA-256 streaming file hash; Source tag, timestamp, provenance and searchable index | NIST SHA-256 golden vectors pass; Malformed/oversized records never corrupt archive | planned |
| **R2** Reproducible packages | Versioned export manifest with checksums and import validation; Restorable snapshots and conflict-safe updates | Tampered archive refuses silent import; Import/export preserves source and author metadata | planned |
| **R3** Optional private archives | Per-user encryption architecture and key-loss recovery policy; Auditable, opt-in external sharing | No credentials or contents copied to logs; Recovery and migration tests pass before encrypted storage claim | planned |

### Custom Workbench (`custom`)

**Purpose:** An empty, user-configured developer playground with explicit capabilities and pluggable modules.  
**Lore:** rust boy original · `RB-01`  
**Preloaded now:** empty Custom layout  
**New modules:** `layout-editor`, `module-sdk`, `sandbox-runtime`

**Operational needs**

- **must — Start truly empty and pin/unpin tools without execution** (`code_present_ci_pending`).
- **must — Save/load layout and preserve profile compatibility** (`partially_available`).
- **should — Custom dashboard layout, typed module manifests and safe tooling** (`planned`).
- **later — Isolated user extensions, access reviews and portable profiles** (`planned`).

| Gate | Deliverable | Acceptance test | State |
| --- | --- | --- | --- |
| **R0** Blank advanced profile | Custom profile with zero pinned tools; CLI and TUI palette picker; Local versioned workspace storage and strict IDs | Switching to Custom requires explicit replacement and leaves an empty board; No side effects until a tool is invoked | code present ci pending |
| **R1** Composable developer workspace | Saved tile/order/visibility layout, 4-inch and desktop variants; Import/export versioned profile JSON with dry-run validator | Schema migration preserves existing board and rejects unknown privilege fields; Every tile supports keyboard selection | planned |
| **R2** Typed extension SDK | Manifest and capability declaration with signed/verified packages; WASM-isolated pure computations before OS device adapters | Invalid plugin schema and unsupported capabilities fail closed; No external execution or network via profile import | planned |
| **R3** Advanced hardware playground | Per-module operating-system scoped broker permissions and lease expiry; Recovery from plugin crash and device unplug | Capability revocation immediately closes OS resource handles; Unsafe module cannot escape its declared interface | planned |

## Integration and acceptance notes

**Scout GIS and Raspberry Pi TAK:** [Scout GIS + TAK architecture](SCOUT_GIS_TAK.md) explains the independent GeoJSON bridge, future RustGIS version gate, ARM64 testing and the separation of ATAK (Android), OpenTAKServer (Pi) and future CoT integration. No TAK network connection is active.

**Security camera wall:** [camera implementation contract](SECURITY_CAMERA_WALL.md) defines the requested four-camera 2×2 UI and one-key focus/back behavior, source trust, video decoder boundary, credential handling, test plan and hardware acceptance. Do not label the Security profile camera-capable until real feeds are actually rendered. ONVIF Profile T and supported RTSP/JPEG/MJPEG are proposed compatibility surfaces, not installed drivers.

**Game companion:** The Mac-to-PS4 browser is a separately authorized, read-only experimental mode. Device identification and live-build compatibility need actual PS4 acceptance; simulated peers alone cannot certify the feature. See [Mac/PlayStation](MAC_PLAYSTATION.md).

**Lore evidence:** [VAULT_TEC_LORE_MATRIX.md](VAULT_TEC_LORE_MATRIX.md) tags canonical jobs separately from adaptations and cites primary publisher material. Technical standards are cited for camera, geodata, audit records, and accessibility requirements.

**Validation:** `python3 tools/validate_roles.py` checks the role/phase/source/profile IDs, exact preloaded tool order and documentation link coverage. It does not prove Rust compilation, camera streaming or hardware security. See [CI validation](VALIDATION.md).

# Rust-Boy | Vault-Tec lore and technical literature matrix

Reviewed **October 25, 2026**. Every citation is a bibliographic pointer, **not** permission to reuse game assets, logos, voice, interface art or proprietary code. A quest role may be real within game lore without being an official Vault-Tec corporate job outside that fictional scenario. All corresponding computer utilities are original Rust-Boy assignments.

## Source matrix

| ID | Source | Grounded evidence | Limit for this project |
| --- | --- | --- | --- |
| `VT-01` | [Bethesda Support: Overseer, Overseen (Fallout 76)](https://help.bethesda.net/app/answers/detail/a_id/49597/) · official game support | Overseer, Security Chief Robinson, Head Engineer Brass, Doctor Loris and Professor-Bot are quest roles; the scenario is a Vault-Tec University training simulation. | Does not establish an exhaustive corporate personnel chart or real job specification. |
| `VT-02` | [Bethesda Support: Reclamation Day (Fallout 76)](https://help.bethesda.net/app/answers/detail/a_id/43149) · official game support | Pip-Boy, Overseer's Mission, Vault 76 departure and Wasteland exploration. | The title Reclamation Scout is an adaptation, not an attested Vault-Tec position. |
| `VT-03` | [Bethesda: Fallout Shelter](https://fallout.bethesda.net/en/games/fallout-shelter) · official game publisher | Overseer administration, Dwellers' assigned jobs, radio and Vault protection and wasteland exploration. | Neither Records Custodian nor University Researcher is represented here as an exact canonical employee title. |
| `STD-01` | [ONVIF Profile T](https://www.onvif.org/profiles/profile-t/) · technical standard | Current camera video interoperability target: H.264/H.265, imaging, metadata and alarm features. | Interoperability depends on tested camera/client conformity and codec support. |
| `STD-02` | [ONVIF: Transition away from Profile S](https://www.onvif.org/pressrelease/onvif-to-end-support-for-profile-s/) · technical announcement | Profile S deprecation; Profile T preferred for newer integrations. | Legacy cameras may still require separately tested paths. |
| `STD-03` | [IETF RFC 7826: RTSP 2.0](https://www.rfc-editor.org/rfc/rfc7826) · technical standard | Media session setup/control protocol fundamentals. | Does not imply every consumer camera implements RTSP 2.0; compatibility must be negotiated and tested. |
| `STD-04` | [IETF RFC 7946: GeoJSON](https://www.rfc-editor.org/rfc/rfc7946) · technical standard | GeoJSON positions are longitude-latitude in WGS 84 (CRS84 axis order). | Location data may be sensitive and must be protected by explicit storage/sharing policy. |
| `STD-05` | [NIST SP 800-92: Computer Security Log Management](https://csrc.nist.gov/pubs/sp/800/92/final) · technical guidance | Planning and management of incident logs and evidence. | Reference guidance, not implementation certification. |
| `STD-06` | [W3C WCAG 2.2](https://www.w3.org/TR/WCAG22/) · technical standard | Readable controls, input targets and keyboard operability guide handheld UI acceptance. | A desktop terminal prototype does not establish WCAG conformance for a future handheld. |
| `RB-01` | [Rust-Boy role architecture](https://github.com/pskeffington/pip_boy/blob/feat/rust-boy-workbench/crates/pipboy-core/src/workbench.rs) · project definition | Role preset IDs, currently pinned tool sets and independent Custom sandbox. | Code presence on development branch is not cross-platform CI acceptance. |

| `STD-07` | [OpenGeos RustGIS — architecture, licensing, ARM64 builds](https://github.com/opengeos/RustGIS) · upstream software | RustGIS L0 geometry and L1 data are useful interoperability references; Apache-2.0, available Linux ARM64 distribution; workspace minimum Rust 1.90. | Rust-Boy uses Rust 1.90+; full RustGIS source is not currently embedded or verified on Raspberry Pi. Keep independent GeoJSON bridge until compatibility gates pass. |
| `STD-08` | [TAK Product Center — ATAK, TAKX, PiTAK, TAK Server](https://tak.gov/products) · official product | ATAK-CIV is Android software, TAKX includes a Linux variant, and PiTAK is a transport-specific TAK ecosystem product. | PiTAK does not imply an entire ATAK mobile client on Raspberry Pi OS; Linux TAKX ARM64 support has not been established. |
| `STD-09` | [OpenTAKServer — Raspberry Pi OS installation](https://docs.opentakserver.io/installation/installation.html) · upstream software | OpenTAKServer supports Raspberry Pi OS Bookworm-or-newer with a documented Pi installer; this is a TAK-compatible server for client data exchange. | Separate software, not embedded in Rust-Boy; operator must review installer, certificates, access policy and live ARM64 hardware behavior. |
| `STD-10` | [IETF RFC 7946 — GeoJSON](https://www.rfc-editor.org/rfc/rfc7946) · technical standard | Positions use geographic longitude, latitude WGS84 axis order; GeoJSON Feature and FeatureCollection are suitable portable interchange artifacts. | Raw bounding boxes may be broad across antimeridian; the Scout CLI Point inspection and newer graphical polyline/polygon renderer have separate format and size bounds. |

## Canonical roles vs adaptations

| Rust-Boy profile | Classification | Basis | Mapping policy |
| --- | --- | --- | --- |
| Overseer Operations | canonical job | `VT-01`, `VT-03`, `RB-01` | Real-world tool needs are project design, not game claims |
| Head Engineer Station | canonical job | `VT-01`, `RB-01` | Real-world tool needs are project design, not game claims |
| Security Chief Station | canonical job | `VT-01`, `STD-01`, `STD-02`, `STD-03`, `STD-05`, `STD-06` | Real-world tool needs are project design, not game claims |
| Vault Doctor Station | canonical job | `VT-01`, `RB-01` | Real-world tool needs are project design, not game claims |
| University Research Station | role inspired | `VT-01`, `RB-01` | Real-world tool needs are project design, not game claims |
| Reclamation Field Kit | role inspired | `VT-02`, `VT-03`, `STD-04` | Real-world tool needs are project design, not game claims |
| Records Custodian Station | role inspired | `VT-03`, `STD-05` | Real-world tool needs are project design, not game claims |
| Custom Workbench | rust boy original | `RB-01` | Real-world tool needs are project design, not game claims |

**Canonical / evidenced** in Bethesda's *Fallout 76* quest **Overseer, Overseen**: Overseer, Security Chief, Head Engineer, Doctor. The University Researcher profile is **inspired by** Vault-Tec University and Professor-Bot, not documented as an identical staff title. Reclamation Scout is **inspired by** Vault 76's Reclamation Day, and Records Custodian by recordkeeping and archival themes. Custom is a wholly original Rust-Boy developer mode.

## Technology evidence vs gameplay evidence

The camera project should plan around [ONVIF Profile T](https://www.onvif.org/profiles/profile-t/) while accepting that older devices may need separately gated compatibility, and use RTSP with tested decoder capabilities rather than assuming every camera negotiates the same version. The [Profile S transition](https://www.onvif.org/pressrelease/onvif-to-end-support-for-profile-s/) is a reason to favor current standards, not a reason to break existing devices without notice.

The field-mapping project must preserve **longitude, latitude** ordering in GeoJSON, report coordinate precision and protect location history. Security case logs should follow reviewed provenance and retention policies, not silently record video. Small-screen design follows published accessibility principles, with actual validation required on the selected hardware.

## Reclamation Scout interoperability note

RustGIS is an independent Apache-2.0 project, with separate Linux ARM64 releases and a workspace requiring Rust 1.90 or newer. Rust-Boy uses 1.90+ but keeps its own independent Scout GIS Lab and exchanges **RFC 7946 GeoJSON Point** documents without linking or copying RustGIS source. Direct RustGIS geometry/data dependencies are a later optional compatibility gate.

TAK's official ATAK-CIV runs on Android, not directly on Raspberry Pi OS. On a Pi, prefer a native Rust-Boy Scout interface plus separately deployed **OpenTAKServer** and an operator-approved CoT integration. **PiTAK** on TAK.gov is a transport option, not evidence that the full Android ATAK app works on Pi OS. TAKX Linux availability likewise does not establish ARM64 support without testing.

## Implementation links

- [Role roadmap](ROLE_ROADMAP.md) — key needs, gate acceptance, dependency graph, preloaded tool status.
- [Camera wall plan](SECURITY_CAMERA_WALL.md) — detailed physical security use case.
- [Machine-readable source/roadmap](../catalog/role_roadmap.json) — source IDs and role IDs validated against implementation.
- [Rust workbench palette](../crates/pipboy-core/src/workbench.rs) — current code, which does not imply that planned integrations are shipped.

## Boston real geometry and user-visible lore crosswalk — 2026-10-25

The canonical vs. inspired role distinctions above remain unchanged. The [operator naming policy](LORE_NAMING.md) maps the six visible tabs (STAT, GEAR, MAP, DATA, LINK, SETUP) and lore-inspired names to stable internal tool IDs. **RADIO** is reserved for actual RF hardware; LINK is IP/SSH. The [Boston basemap](BOSTON_BASEMAP.md) consists only of real City SAM WGS84 street coordinates with original phosphor styling, never fictional vaults, gameplay streets or invented landmarks. Verify data redistribution rights before packaging.

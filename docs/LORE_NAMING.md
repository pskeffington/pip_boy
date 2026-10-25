# Rust-Boy | Lore-aligned operator naming

**Rust-Boy** remains an independent, unofficial field computer. Bethesda's primary-source [Fallout 4 Pip-Boy companion announcement](https://bethesda.net/en-US/news/fallout-4-pip-boy-app-now-available) describes Stats, Inventory, Map, Radio and holotape games; [Overseer, Overseen](https://help.bethesda.net/app/answers/detail/a_id/49597/) establishes Overseer, Head Engineer, Security Chief and Doctor as game roles. These are **naming inspirations**, not evidence of in-game-equivalent product functions or endorsement.

## Compact six-tab micro terminology

| Visible tab | Actual Rust-Boy meaning | Alignment / boundary |
| --- | --- | --- |
| STAT | Device status and current role | Stats-inspired; not game health points |
| GEAR | Available pinned tools | Inventory-inspired; not in-game player inventory |
| MAP | Actual WGS84 geographic data and opt-in location | Maps; **no fictitious Boston geometry** |
| DATA | Local records, notes and technical evidence | Data/quest idea |
| LINK | IP/SSH peer tasks and game companion | Communications; **not** RF or Bluetooth |
| SETUP | Profile/configuration | Rust-Boy original |

The internal page identifiers (stat, gear, map, data, link, conf), tool IDs, storage names and CLI commands must remain unchanged.

## Tool-name crosswalk

| Stable ID | Visible name | Real function / lore rationale |
| --- | --- | --- |
| mission-board | **MISSION LOG** | Overseer assignments and status, inspired by quest/mission records |
| host | **SYSTEM STAT** | Host CPU/RAM and data quality, not fictional character attributes |
| net-lab | **NET ENGINEER** | LAN tools and UniFi readings; Head Engineer duty |
| diag | **SYSTEM DIAG** | Authorized local evidence/fault review |
| byte-lab | **BYTE LAB** | Offline hex/byte utilities |
| simulator | **SIM BAY** | Synthetic training fixtures only |
| library | **ARCHIVES** | Technical reference library |
| notes | **JOURNAL** | Local text notes, not recorded holotapes |
| reclamation | **FIELD LOG** | Field conditions, priorities and site progression |
| geo-lab | **SURVEY GIS** | Real WGS84 waypoint and distance operations |
| game | **GAME LINK** | Read-only game companion, no gameplay control |
| radio / mesh / ble / nfc / gpio | RADIO/SDR / MESH / BLE / NFC/RFID / GPIO | Planned hardware, still unavailable |

**RADIO** is reserved for actual radio hardware. Do not call SSH peer transport a radio or mesh. **HOLOTAPE** is reserved for real media functionality, not text notes. Do not call diagnostics V.A.T.S.

## Map name and verified source rule

The default MAP presentation may evoke the Fallout Commonwealth through original green phosphor graphics, but it identifies the data as **BOSTON / REAL STREETS**. All drawn roads, labels and coordinates must derive from a documented real producer; no made-up Diamond City, Goodneighbor, vaults, paths or game-world distances are allowed in the actual Boston basemap.

The [lore source matrix](VAULT_TEC_LORE_MATRIX.md) differentiates canonical game occupations, role-inspired adaptations and Rust-Boy-original modules. Use the same distinctions for user documentation. Never bundle official game icons, fonts, music, interface art, map imagery or trademark styling. Keyboard use and text legibility take priority over retro decoration.

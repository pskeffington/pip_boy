# Rust-Boy | Brand and identity guide

**Public name:** Rust-Boy  
**Descriptor:** Offline-first field systems console  
**Positioning:** An independent Rust-native toolkit for local diagnostics, modular field utilities, and future wearable computing.

This is the authoritative product-naming reference for this repository. The brand is a working project identity, not a claim of trademark registration or clearance.

## 1. Name hierarchy

- **Rust-Boy** — product name in all prose, documentation titles, terminal headers, issue/PR descriptions, and release notes. Use the exact spelling, capitalization, and hyphen.
- **RUST-BOY** — acceptable only as an intentional uppercase, monospaced UI heading.
- **pip_boy** — existing GitHub repository slug. Do not rename the repository or links as part of a branding-only pass.
- **pipboy** — stable CLI executable and existing Rust package identifier. Its display header says Rust-Boy; commands remain `pipboy` until a separately tested migration is approved.
- **pipboy-core** — internal library crate; retained to avoid unnecessary dependency and import changes.
- **rust-boy-broker** — separate, currently deny-only executable for strict physical-device request parsing. Its name does **not** imply actual hardware authorization.
- **PIPBOY_HOME** / `~/.pipboy` — existing settings and data paths, retained to protect stored notes and scripts.

Do not invent a general-purpose `rust-boy` CLI or a new storage path: the interactive command is still `pipboy`, and `rust-boy-broker` is a separate **deny-only** protocol binary. User-facing prose always says **Rust-Boy**; code and paths must accurately reflect the current implementation.

## Original phosphor handheld styling

Micro mode uses an original dark-green field monitor treatment with subtle scanlines, restrained glow, legible monospaced status text and a six-tab lower rail. It conveys the **retro-futurist wrist computer** category without cloning a copyrighted game interface, importing art, or implying endorsement by Bethesda/Vault-Tec. Static fixtures must state when there is no camera frame, GIS renderer, GNSS or power sensor. The screen remains keyboard/touch usable with effects disabled for accessibility.

See the [micro UI operator guide](docs/MICRO_UI.md), [lore naming crosswalk](docs/LORE_NAMING.md) and [Boston source policy](docs/BOSTON_BASEMAP.md).

## Dashboard visual identity

The local dashboard adopts its own charcoal and green phosphor visual language, a generic `RB/` mark, original typography and responsive module cards. It uses no official Fallout, Pip-Boy or Vault-Tec images, icons, proprietary interface code or font assets. Seven role titles are inspired by the documented job taxonomy and Custom is an original advanced-workbench designation. Distinguish a dashboard **template** from an approved capability: the appearance of a camera tile or map viewport cannot imply a live video feed or working navigation. See [graphical UI guide](docs/DASHBOARD_UI.md).

## Lore-inspired station naming

The user-facing Vault-Tec occupational terms are **theme inspiration**, not a claim of sponsorship, brand license or exact game-function replication. Explicitly classify canonical jobs (Overseer, Head Engineer, Security Chief, Doctor) versus original adaptations (University Researcher, Reclamation Scout, Records Custodian, Custom). Code/tool descriptions must remain accurate for their implemented real-world behavior; do not copy game graphics, logos or recorded dialogue. See [source-backed lore matrix](docs/VAULT_TEC_LORE_MATRIX.md) and [role requirements](docs/ROLE_ROADMAP.md).

## 2. Standard descriptions

**Short:** Rust-Boy is an offline-first, Rust-native field systems console.

**Long:** Rust-Boy is a modular, independent field-computing toolkit for desktop terminals and future wearable systems. It provides real local diagnostics, a curated source library, and user-controlled notes, while keeping mapping, radio, device inputs, and mesh communications behind explicit implementation and permission gates.

**Tagline:** *Offline first. Native by design.*

Product copy should be factual and restrained. Do not describe planned modules as deployed, a device as connected without evidence, or an untested build as production-ready.

## 3. Visual direction

Build an original **instrument-console** design rather than copying Fallout's artwork or proprietary interface elements. Prefer strong contrast, geometric spacing, dense but legible status data, unambiguous live/planned states, and minimal animation.

| Token | Hex | Role |
| --- | --- | --- |
| `--rb-bg` | `#0A120D` | Deep graphite-green foundation |
| `--rb-panel` | `#13231A` | Secondary panel surface |
| `--rb-accent` | `#7BE9A6` | Active data and focus state |
| `--rb-text` | `#ECF6EE` | Primary text |
| `--rb-muted` | `#AFC8B5` | Supporting labels |
| `--rb-warning` | `#E8B96E` | Caution and degraded state |
| `--rb-error` | `#F3918B` | Explicit error state |

These are design intentions, not an assertion that contrast or accessibility compliance has been tested. Verify WCAG text contrast and color-blind legibility when implementing a graphical theme. The existing Ratatui UI uses terminal colors; do not assume it already applies the hex palette.

**Typography:** system monospace for command outputs and instrument readouts; accessible system sans-serif for prose and long-form documentation. Do not bundle commercial or game-specific fonts.

**Map provenance:** Only actual Boston SAM street geometry and traceable real GIS layers are drawn; stylization must never fabricate landmarks, paths or gameplay locations.

**Motion:** static by default. Any CRT/scanline aesthetic must be optional, low-intensity, and disabled in reduced-motion mode. Never sacrifice clarity to imitate a game UI.

## 4. Interface language

- Visible micro navigation is `STAT`, `GEAR`, `MAP`, `DATA`, `LINK`, `SETUP`; stable internal page and tool IDs remain unchanged.
- Display names follow [LORE_NAMING](docs/LORE_NAMING.md): MISSION LOG, SYSTEM STAT, NET ENGINEER, SYSTEM DIAG, ARCHIVES, JOURNAL, SIM BAY, FIELD LOG, SURVEY GIS and GAME LINK. `LINK` is IP/SSH, **not** radio/mesh.
- Use `READY`, `PLANNED`, `UNAVAILABLE`, and `ERROR` accurately; never treat missing telemetry as zero.
- Label observed values with source and freshness where practical.
- Explain when radio is receive-only, device permissions are absent, or location services are off.
- Keep keyboard navigation and exit instructions visible.

## 5. Intellectual property boundaries

Rust-Boy is **independent and unofficial**. It is not connected to or endorsed by Bethesda Softworks, the Fallout franchise, The Wand Company, or any other third-party source catalog entry. Mention their names only when identifying actual upstream projects, compatibility targets, or attribution.

Do **not** copy Fallout logos, Pip-Boy product trade dress, original game screen assets, music, voice recordings, proprietary device firmware, or unlicensed repo artwork. Avoid claims of official compatibility unless verified. An MIT-licensed software repository can still contain separately restricted trademarks or artwork. The external reference catalog is not an approved asset pool.

## 6. Brand change checklist

- [x] README title, product description, and docs index use Rust-Boy.
- [x] Security, architecture, roadmap, and research documents use Rust-Boy as the project name.
- [x] CLI help, errors, and terminal header identify Rust-Boy.
- [x] Technical paths and example commands remain valid.
- [x] The brand is not presented as an authorized Fallout or Wand Company product.
- [ ] Accessibility, legal, and code quality checks are performed before tagging a release.

## 7. Scope

Naming and design conventions are active now. Original logo artwork, trademark investigation, any migration of the main command to `rust-boy`, packaging, and a fully themed GUI require separate review and implementation gates. Passing CI for the deny-only `rust-boy-broker` is not real-device certification.

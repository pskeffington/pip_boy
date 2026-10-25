# Rust-Boy | External Pip-Boy research library

This is the **Rust-Boy** project's reference library. The Pip-Boy names below identify independently owned external projects, not Rust-Boy modules or endorsed assets.

Repository discovery snapshot: 2026-10-24. Source metadata and license indicators are catalogued in ../catalog/sources.json. This is a reference library, not a vendored-code directory. Check the upstream license and exact commit again before adapting anything.

The companion [literature matrix](LITERATURE_MATRIX.md) maps peer-reviewed research, official specifications and first-party documentation to the [planned module catalog](MODULE_CATALOG.md). Unlike the external project inventory below, the literature matrix represents design evidence, not code reuse authorization.

## Strongest architectural references

| Project | Observed design | Fit | License indicator |
| --- | --- | --- | --- |
| nyzoli/PipBoyCRT | Rust + Ratatui, system status, weather, radio and terminal panels | Native dashboard conventions | MIT |
| zapwizard/pypboy | Python/Pygame with Raspberry Pi, GPIO inputs, map caching and radio files | Hardware controls, small display layouts | MIT |
| thewandcompany/pip-boy | Device modification tool with buttons, encoders and on-device JavaScript API | Replica capability and physical input reference | MIT |
| CodyTolene/pip-boy-3000-holotapes | TypeScript-to-Espruino user apps | Versioned app packaging ideas | MIT |
| RobCoIndustries/pipboylib | JS Fallout 4 companion-state protocol tooling | Optional game-companion concepts only | Apache-2.0 |
| AidansLab/PipBoy3000Companion | PC game state, named pipes, USB serial device synchronization | External integration pattern | CC BY-NC-SA 4.0 stated by README |
| SurvivorGrim/PipBoy-Pi5 | Raspberry Pi 5 components, switches, screen and mechanical parts | Hardware bill of materials | No repo license detected |
| Rimocide/pipboy.rs | Rust/Ratatui/sysinfo terminal monitor | Telemetry display pattern | No repo license detected |

Additional candidates in sources.json: grieve/pypboy, ColtonMcCasland/Raspipboy-3000, Theblastback/PipOS, Seltyk/pipboy, karanehra/pipboy-terminal, nishad2m8/Pip-Boy-Weather-Clock.

## License / source handling

- MIT / Apache-2.0: potential compatibility for *code* with notice obligations, subject to inspecting actual files and dependencies. Links are used for architectural research only in G0.
- GPL-3.0: keep as a conceptual reference; code combination requires separate copyleft review.
- CC BY-NC-SA 4.0: noncommercial/sharealike and content-vs-code implications make incorporation inappropriate without explicit review.
- Missing, custom, or NOASSERTION: do not copy, vendor or distribute code or artwork.
- Individual artwork, schematics, fonts, game audio and product marks can have separate rights even if repository code is licensed.

## Evaluation checklist

For each project: verify repository owner, primary branch and revision, explicit LICENSE file, dependency licenses, supported device model, OS requirements, maintenance recency, hardware protocol claims, test coverage, security implications, offline suitability, and attribution requirements.

## Original design decisions

Use independent Rust code, sysinfo for host read-only metrics, Ratatui for terminal drawing, offline user-owned notes, and opt-in adapter boundaries. Native devices and companion protocols are later modules—not dependencies of the G0 release. No Fallout assets are included.

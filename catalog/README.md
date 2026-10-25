# Rust-Boy research and module catalogs

These version-controlled JSON files document **engineering evidence and planned capabilities**. They do not contain hardware drivers, bundled third-party source code, active device grants or a permission sandbox.

| Manifest | Contents |
| --- | --- |
| [`literature.json`](literature.json) | 37 cited research/official-source records with IDs, limitations and engineering implications |
| [`module_plan.json`](module_plan.json) | 27 module definitions: 3 implemented module sources with additional acceptance gates, 24 planned modules |
| [`sources.json`](sources.json) | Curated external GitHub/reference projects and licensing dispositions |

The `status` field is a development/acceptance label: `implemented_unverified` does **not** mean no CI exists, and `planned` does not mean an adapter is available. `gate` (G0–G4) indicates sequencing rather than a release date. `capabilities` describes future permission requirements, not authorization. `evidence` links to source records, not licensed source code ready to copy.

The [offline catalog validator](../tools/validate_catalog.py) checks IDs, dependency relationships, declared capabilities and the deny-only broker vocabulary. See the [module catalog](../docs/MODULE_CATALOG.md), [literature matrix](../docs/LITERATURE_MATRIX.md) and [security policy](../SECURITY.md) before enabling any new component.

All upstream repositories and creative works retain their respective copyright and license conditions. A reference is not permission to reuse code, trade dress, artwork or firmware.

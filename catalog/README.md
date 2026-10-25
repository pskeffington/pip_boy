# Rust-Boy | Research and module catalogs

These are version-controlled **planning and provenance manifests**. They are not executable adapters, vendor source archives, or permissions grants.

- [`literature.json`](literature.json): 25 primary/official-source references with verification dates, research-to-module mapping, limitations, and acceptance tests.
- [`module_plan.json`](module_plan.json): 27 proposed software modules, release gates, dependency edges, capabilities, features, evidence IDs, and verification criteria.
- [`sources.json`](sources.json): external GitHub implementation references with licensing gates.

Field definitions:
- `status=implemented_unverified` indicates a G0 implementation is present, **not** that CI has passed; `planned` means no implementation claim.
- `gate=G0..G4` identifies sequencing, not a scheduled date or a release version.
- `capabilities` records permissions that a future adapter would require; it is **not** an OS sandbox or proof of enforcement.
- `evidence` links to `literature.json` entries, not to source code to copy.
- `acceptance` items become test/inspection gates when a feature is implemented.

All upstream works retain their own copyright. Consult [the literature matrix](../docs/LITERATURE_MATRIX.md), [module catalog](../docs/MODULE_CATALOG.md), and [architecture](../docs/OBJECT_MODEL.md) before implementation.

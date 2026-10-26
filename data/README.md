# Rust-Boy data layout

Version-controlled `data/` contains **public reference indexing policy and generated lookup metadata**, not operator records. The existing `catalog/` is the canonical source for module, role, literature and upstream-reference JSON, and remains at its current paths for Rust `include_str!` and validator compatibility. `docs/` is the source for human contracts. Neither directory is copied or moved here.

| Location | Contract |
| --- | --- |
| [`index/manifest.v1.json`](index/manifest.v1.json) | Explicit opt-in **file allowlist** with stable IDs, domains, titles and enabled flags. Only paths listed here may be ingested. |
| [`index/public-index.v1.json`](index/public-index.v1.json) | Deterministically generated, versioned inverted index compiled into the Rust binary. No document bodies stored. |
| [`schemas/`](schemas/) | Reference JSON schemas for code review and external tooling. The standard-library builder enforces its own strict validation. |
| [`index/README.md`](index/README.md) | Build, check and query commands. |

**Outside this tree:** `PIPBOY_HOME` holds operator notes, task boards, message queues, map caches, survey sites, diagnostics and similar runtime state. These are not inputs to the public index. Keep them out of Git and out of the default index. No filesystem crawling, network fetches, symlink following or background indexing occurs.

The compiled index is an offline discovery convenience, not an authorization boundary or a secure search service. Git blob IDs are source-drift fingerprints, **not cryptographic signatures**. See [data indexing contract](../docs/DATA_INDEX.md) and [storage privacy](../docs/STORAGE.md).
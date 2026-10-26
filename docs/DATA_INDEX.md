# Rust-Boy | Public data indexing contract (v1)

## Scope and layout

Rust-Boy now has a deterministic **read-only, offline public reference index**. This is specifically for locating the project's vetted documentation and catalogs on desktop, micro-handheld builds and from the CLI. It is **not** a recursive index of a user's device, their notes or geospatial tracks. All existing data-store paths remain unchanged.

| Path | Responsibility |
| --- | --- |
| `catalog/{literature,module_plan,role_roadmap,sources}.json` | Canonical structured source/evidence data; no relocations. |
| `docs/*.md`, selected root `.md` | Human-readable source files, added one by one to the allowlist. |
| `data/index/manifest.v1.json` | Authoritative reviewed allowlist and per-entry `enabled` bit, stable ID, source path and domain (`docs` or `catalog`). |
| `data/index/public-index.v1.json` | Generated compact metadata plus token → document ID postings and Git blob fingerprints. |
| `data/schemas/` | Declarative v1 schema references; strict standard-library validator is authoritative in CI. |
| `PIPBOY_HOME/*` | **Excluded:** all runtime collections, secrets, caches, messages, notes, assignments, telemetry, and private location data. |

The index stores IDs, titles, summaries, source paths, byte lengths, fingerprints and inverted term postings. It deliberately **does not duplicate document bodies**, ingest hidden files, follow symlinks or open arbitrary paths. All indexed source files are already committed to the public repository. The index is bundled in the Rust binary with `include_str!`, so `pipboy index` works with no network or source checkout.

## Operator controls

```bash
# Native Rust CLI, offline and read-only:
cargo run --locked -p pipboy -- index status
cargo run --locked -p pipboy -- index status --json
cargo run --locked -p pipboy -- index list --domain catalog
cargo run --locked -p pipboy -- index search 'unifi' --domain docs
cargo run --locked -p pipboy -- index search 'geospatial' --json
cargo run --locked -p pipboy -- index validate

# Maintainer controls (run at repository root):
python3 tools/index_data.py list --domain docs
python3 tools/index_data.py search 'offline map' --domain docs
python3 tools/index_data.py build
python3 tools/index_data.py check
```

`--domain catalog|docs` narrows results without broadening ingestion. Search applies token-based **AND matching**, with case-insensitive ASCII words of 2–40 characters. There is no implicit live rescan, user file index, full phrase/ranking engine or background synchronization. `validate` checks the embedded Rust index structure; only the source-checkout Python `check` compares every listed file against its actual current content.

## Index governance and data safety

1. Add entries to `manifest.v1.json` with unique, lexicographically sorted stable IDs. Only explicit `catalog/*.json`, `docs/*.md`, and a small fixed allowlist of top-level Markdown files are legal. No globbing is supported. Use `enabled: false` to exclude a listed source from publication; treat changes as reviewed source-control changes.
2. Include only public-source, distributable text. Do not include `PIPBOY_HOME`, logs, env files, notes, messages, task boards, profiles, trace dumps, private GIS, location breadcrumbs, credentials or downloaded third-party archives. The builder refuses path traversal, symlinks, unknown domains, oversized files (>256 KiB per input) and missing entries.
3. Generate with `python3 tools/index_data.py build` after source changes, and commit the manifest, edited public sources and index together. `check` recomputes the entire snapshot and fails CI on stale bytes, changed manifest metadata or source fingerprints. Git blob fingerprints detect ordinary source drift but are **not a security integrity guarantee**.
4. CI runs Python manifest/index checks plus Rust unit tests, including invalid scope, malformed token and path-traversal rejection. Runtime queries read only compiled data and grant no filesystem/network/device access.
5. Treat indexing policy changes as security review points; avoid expanding to runtime data until separately consented, encrypted where appropriate, with retention/deletion semantics and a threat model. Indexing never grants copyright or licensing permissions.

## Acceptance and limitations

The v1 acceptance gate is: deterministic rebuild, drift detection, domain-scoped native queries, no network or runtime reads, and regression tests for manifest/path restrictions. This does not certify on-device search UX, field hardware, encrypted operator search or full-text ranking. A future DATA HUD panel can call the pure Rust core index; it must not silently introduce runtime discovery or private-content indexing.

Related: [data tree](../data/README.md) · [source catalog](../catalog/README.md) · [storage](STORAGE.md) · [security](../SECURITY.md) · [architecture](ARCHITECTURE.md).
# Public reference index

From the repository root:

```sh
python3 tools/index_data.py status
python3 tools/index_data.py list --domain catalog
python3 tools/index_data.py search 'network health' --domain docs
python3 tools/index_data.py build
python3 tools/index_data.py check
```

The manifest is the only approved source roster. `enabled: false` excludes the entry and its searchable terms from the generated artifact; add or enable entries through source review. Rebuild and commit `public-index.v1.json` with any edited indexed source or manifest. CI checks exact generated content and rejects stale indexes. `build` replaces only the generated index via a temporary file. `check`, `status`, `list` and `search` do not write.

On an installed/offline Rust-Boy unit the compiled `pipboy index status|list|search|validate` commands work with **no repository checkout**; they use the immutable snapshot from the build and never index private files. Running `pipboy index validate` checks embedded index structure, while `python3 tools/index_data.py check` verifies source freshness inside a source checkout.

The query engine uses case-insensitive ASCII terms of 2–40 characters and intersects terms (AND). It does not support phrase ranking, arbitrary filesystem indexing, or private-data search. Catalog licenses remain **reference-only**; indexing a metadata record does not grant permission to import upstream source.

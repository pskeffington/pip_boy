# Codex instructions — Rust-Boy

This is the root entrypoint for Codex on the Rust-Boy repository. Before editing, consult [Codex pass guide](docs/CODEX_PASSES.md), [machine pass index](catalog/codex_passes.json), [component ownership](catalog/component_index.json), [production status](docs/PROD_STATUS.md) and [host roadmap](docs/HOST_PACKAGE_ROADMAP.md). They are branch-specific development records, **not production or hardware acceptance**.

## Each requested development pass

1. Inspect `git status --short --branch`, HEAD, remote branch and open PR before changing anything. Do not discard, reset, stash or overwrite somebody else's edits, and never force-push.
2. Distinguish `main` from the draft `codex/package-adapter-protocol` branch. Other independent PRs for the server, organization, avatars, radio, cameras, emergency tools or GIS are not automatically merged or tested.
3. Select the sole `status: next` entry in `catalog/codex_passes.json`, verify prerequisite pass evidence, then read only its declared entrypoints, guides and checks. A blocked pass may be researched but not accepted.
4. Implement one bounded, reviewable change and add negative and regression tests. Preserve legacy behavior and provide rollback when migrating packages.
5. Run the applicable offline, Rust, Node/Chromium and hardware gates. Record exact commit SHA, commands, outcomes and unrun limitations in `docs/PROD_STATUS.md` and, for substantive findings, `docs/VALIDATION.md`. Source-only tests and empty CI status are not passing evidence.
6. Update machine indexes and documentation together. Commit a coherent checkpoint to the active authorized development branch and synchronize the PR description. Merge only after verified CI/review gates; do not bypass failed checks to save context.

## Rust-Boy non-negotiables

- Exactly **one foreground package** occupies desktop/handheld workspace. Host owns slim navigation, ever-present Settings, role/policy boundary and capability arbitration. No root-level scrolling or package takeover of Settings.
- `rb_gis`, `rust_radio`, `rust_tv` and `rusti_rc` remain independent. The compiled RADIO catalogue is **passive metadata only**; selecting it must not connect, play audio or open hardware. Preserve the existing explicit native Radio player.
- A scanned manifest, package name, claimed `hello` identity, local network proximity or key label is not trust. Backend grants are deny-first; untrusted packages require authenticated provenance and OS isolation.
- Never start audio, messages, network scans, county downloads, BLE/SDR access or background services merely because a user opens or switches a package.
- Preserve Rust 1.90 minimum, `pipboy` CLI, `pip_boy` repository, existing routes, saved schema formats, `PIPBOY_HOME` and Mac-first/Pi-later boundaries.
- Keep the deny-only device broker deny-only until security gates pass. No operator data, keys, PHI, private caches or captured traffic in source control.

## Minimum offline and source gates

```bash
python3 tools/validate_codex_passes.py
python3 tools/test_validate_codex_passes.py
python3 tools/validate_component_index.py
python3 tools/test_component_index.py
python3 tools/validate_repo.py
python3 tools/test_validate_repo.py
python3 tools/validate_host_packages.py
python3 tools/test_validate_host_packages.py
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

For affected UI, run the pass's dedicated Node/Chromium viewport suite. Real Mac audio and Raspberry Pi devices need independent physical evidence. Never claim a command passed if it could not be run. Consult [index maintenance](docs/INDEXING.md) for drift controls.

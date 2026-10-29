# Codex instructions — Rust-Boy

This is the root entrypoint for Codex on the Rust-Boy repository. Before editing, consult [Codex pass guide](docs/CODEX_PASSES.md), [machine pass index](catalog/codex_passes.json), [component ownership](catalog/component_index.json), [production status](docs/PROD_STATUS.md) and [host roadmap](docs/HOST_PACKAGE_ROADMAP.md). They are branch-specific development records, **not production or hardware acceptance**.

## Each requested development pass

Start with `python3 tools/codex_next.py` or `python3 tools/codex_next.py --json` for a validated, read-only pass handoff. The output never runs work or proves acceptance.


1. Inspect `git status --short --branch`, HEAD, remote branch and open PR before changing anything. Do not discard, reset, stash or overwrite somebody else's edits, and never force-push.
2. Distinguish `main` from the draft `codex/package-adapter-protocol` branch. Other independent PRs for the server, organization, avatars, radio, cameras, emergency tools or GIS are not automatically merged or tested.
3. Select the sole `status: next` entry in `catalog/codex_passes.json`, verify prerequisite pass evidence, then read only its declared entrypoints, guides and checks. A blocked pass may be researched but not accepted.
4. Implement one bounded, reviewable change and add negative and regression tests. Preserve legacy behavior and provide rollback when migrating packages.
5. Run the applicable offline, Rust, Node/Chromium and hardware gates. Record exact commit SHA, commands, outcomes and unrun limitations in `docs/PROD_STATUS.md` and, for substantive findings, `docs/VALIDATION.md`. Source-only tests and empty CI status are not passing evidence.
6. Update machine indexes and documentation together. Commit a coherent checkpoint to the active authorized development branch and synchronize the PR description. Merge only after verified CI/review gates; do not bypass failed checks to save context.

## RB-CX-00 execution order and stop conditions

For the currently indexed `RB-CX-00`, perform a read-only baseline first: `git status --short --branch`, `git rev-parse HEAD`, `git fetch origin` (only if network access is explicitly available), and `git log --oneline --max-count=5`. Before changing tracked files, compare the current branch against `origin/main`, inspect PR #71, and preserve independent open PR changes.

Run `python3 tools/codex_next.py --json`, then `python3 tools/validate_codex_passes.py`, `python3 tools/test_validate_codex_passes.py`, `python3 tools/validate_component_index.py`, `python3 tools/test_component_index.py`, `python3 tools/validate_repo.py`, `python3 tools/test_validate_repo.py`, `python3 tools/validate_host_packages.py` and `python3 tools/test_validate_host_packages.py`. Record each result independently; a source inspection is not test execution.

Then run locked Rust format/check/test/Clippy with stable and Rust 1.90.0, plus desktop/micro browser fixtures. Check that the compile-time RADIO catalogue stays dormant until manually selected, remains read-only and capability-free, and that navigating through Settings restores the single foreground. Existing native Radio playback must remain explicit.

**Stop and report instead of advancing** on any failed required check, zero-job GitHub Actions `startup_failure`, stale remote branch, unresolved merge conflict, missing dependencies, or missing hardware evidence. Do not bypass checks, grant devices, launch an external package, force-push or mark `RB-CX-00` accepted to unlock `RB-CX-01`.

After a passing exact-head CI run and independent review of the relevant platform evidence, update the pass record, checkpoint, and PR. Only then advance `RB-CX-01` as the next eligible work. Refer to [pass-specific checklist](docs/CODEX_PASSES.md).

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

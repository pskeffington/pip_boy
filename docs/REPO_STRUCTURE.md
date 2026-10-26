# Rust-Boy repository structure and maintenance

Rust-Boy keeps a deliberately small top level. **Do not relocate operational data paths or rename crates to improve appearance**: the `pipboy` CLI, `pip_boy` repository, and `PIPBOY_HOME` environment variable are compatibility contracts.

## Canonical ownership

| Path | Purpose | Change rule |
| --- | --- | --- |
| [`../crates/pipboy-core`](../crates/pipboy-core) | Shared Rust model, policy, persistence and capability definitions | Keep business logic out of browser JavaScript; add unit tests with changes |
| [`../crates/pipboy`](../crates/pipboy) | Native CLI/TUI, loopback HTTP server, embedded dashboard/micro assets | Keep endpoints bounded and operator gated; share one core backend |
| [`../crates/rust-boy-broker`](../crates/rust-boy-broker) | Deny-only peripheral broker prototype | Do not enable physical I/O without separate threat review |
| [`../catalog`](../catalog) | Machine-readable evidence, sources, modules and role roadmaps | Stable IDs and source provenance; run catalog and role validators |
| [`README.md`](README.md) | Operator, architecture and acceptance contracts | Add a link to this index for new operator-facing guides |
| [`../tools`](../tools) | Offline validation, smoke tests and fixture helpers | Prefer Python standard library for manifest checks; no network activity in validators |
| [`../scripts`](../scripts) | Explicit local entry-point launchers | Keep launchers portable and free of implicit network actions |
| [`../examples`](../examples) | Clearly labeled deterministic examples | No private telemetry, credentials, or user data |
| [`../.github/workflows/rust.yml`](../.github/workflows/rust.yml) | CI build, Rust/toolchain and navigation gates | New quality checks must execute on pull requests |

Operator state belongs under **`PIPBOY_HOME`**, not in source control. This includes notes, task boards, peer configuration, messaging queues, geospatial caches, diagnostics and other local records. See [storage](STORAGE.md) and [security](../SECURITY.md). Do not add example files taken from real users or field devices.

## Documentation entrypoints

Start at the [project README](../README.md), then the [documentation index](README.md), [architecture](ARCHITECTURE.md), [storage contract](STORAGE.md), and [validation evidence](VALIDATION.md). Keep the root README as a concise operator-oriented landing page; put module internals and research matrices under `docs/` or `catalog/` instead of appending extensive technical content to the repository root.

## Machine-readable component responsibility map

The [component ownership and acceptance index](../catalog/component_index.json) tracks mainline subsystem entrypoints, their engineering guides, tests, responsibility boundaries and physical-hardware gates. It is intentionally **not** an exhaustive inventory of every source file or a license/permissions roster. The [organizational optimization research](ORGANIZATIONAL_OPTIMIZATION.md) explains the evidence and proposed sequencing; [ADR 0001](adr/0001-mac-first-portable-core.md) records the Mac-first/Pi-later decision.

Run `python3 tools/validate_component_index.py` and `python3 tools/test_component_index.py` when an externally visible component entrypoint, guide, test or hardware-gate label changes. CI validates both using standard-library-only, offline checks. Keep Raspberry Pi acceptance distinct from portable code compilation; `field_pending` identifies open device acceptance and `deny_only` is not a grant. Avoid unreviewed duplicate ownership of the same source entrypoint.

## Change and cleanup controls

1. **Avoid duplicate authority.** Rust core owns module catalogs and permissions; desktop and micro shells consume the same APIs. Never fork another independent JavaScript roster of roles or network tools.
2. **Keep compatibility.** Do not move existing `catalog/*.json` inputs, change stored JSON schemas or rename public CLI flags without migration and tests.
3. **Separate previews.** Radio, mapping, messaging and indexing work can remain on their own feature branches until their exact-head CI and relevant hardware gates are understood. Do not collapse unrelated open PRs into cleanup work.
4. **Treat tracked fixtures as public.** Clean out caches, temporary outputs and secrets from local working trees; do not delete durable operator records as part of a repository tidy.
5. **Validate on changes.** Run `python3 tools/validate_repo.py` and `python3 tools/test_validate_repo.py` after editing the root or documentation navigation, alongside `python3 tools/validate_catalog.py`, `python3 tools/validate_roles.py`, and the Rust workspace checks.

The repository check validates mandatory navigation, root-level Markdown and nested Markdown under `docs/`, `catalog/`, and an optional public `data/` directory. It checks inline and reference-style local link destinations while ignoring fenced examples, external URLs and footnotes. Missing paths, escaping links and symlinked documentation are rejected. The scope never extends into `PIPBOY_HOME` or other operator data. The [CI smoke roster](../tools/run_smokes.py) is the authoritative list for the shared Linux and native-platform CLI checks. Register every new `smoke_*.py` explicitly and run `python3 tools/run_smokes.py --check`; see [CI troubleshooting and coverage](CI_TRIAGE.md). These checks confirm source-tree link integrity and test coverage registration, **not** external URL validity, physical hardware acceptance or a finished release.

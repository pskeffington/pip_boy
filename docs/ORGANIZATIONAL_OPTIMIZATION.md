# Rust-Boy | Organizational optimization research

**Research baseline:** main branch, commit `f2467c1`, 2026-10-26. This is a software architecture, source governance and eventual hardware-portability assessment; it does not reorganize runtime data or certify Raspberry Pi hardware.

## Evidence-based assessment

The main branch contains **185 tracked files**, **42 Markdown documents under `docs/`**, **53 Rust source files**, and three Cargo workspace crates. The documentation index already links all 41 other documentation pages: **do not rebuild a navigation hierarchy that already works**. There are **13 Rust sources larger than 20 KiB**. Notable concentration points: `crates/pipboy/src/dashboard.rs` (82,905 bytes), `crates/pipboy-core/src/tasks.rs` (37,608), `crates/pipboy/src/peer.rs` (33,265), and `crates/pipboy-core/src/workbench.rs` (31,071). These are **file size indicators, not measured complexity scores**.

Existing strengths: shared `Cargo.lock`, cross-platform Rust/JS validation, explicit smoke roster, offline documentation-link validator, deny-only device broker, and a top-level [repository structure guide](REPO_STRUCTURE.md). The organizational gap is not absent folders: it is a missing **machine-checkable component ownership / acceptance map** and a risk of growing mixed responsibilities inside large binary modules.

**Current design constraint:** develop/test on macOS, then prototype on Raspberry Pi Linux ARM64. The open [RADIO branch PR #35](https://github.com/pskeffington/pip_boy/pull/35) demonstrates Mac field testing but remains separate from this mainline organizational pass. A Mac test is not automatically Raspberry Pi acceptance.

## Research matrix

| Evidence | Verified guidance | Rust-Boy application | Acceptance |
| --- | --- | --- | --- |
| [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html) | Packages share root lockfile, target output and common commands; metadata, dependencies and lints can be inherited | Preserve three-crate workspace; use explicit boundaries before proposing more crates | `cargo test --workspace --locked`, `cargo metadata --format-version 1` |
| [Cargo metadata](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html) | Exposes machine-readable workspace membership and resolved dependency graph | Future dependency audit should derive from Cargo, not maintain a second package manifest | Workspace packages in index match actual manifests |
| [Rust API guidelines](https://rust-lang.github.io/api-guidelines/documentation.html) | Crate-level API documentation and meaningful examples clarify public contracts | Extract stable trait/API seams incrementally; document examples with corresponding tests | No public API behavior drift when extracting code |
| [Embedded Rust portability](https://doc.rust-lang.org/embedded-book/portability/index.html) | Hardware abstraction contracts separate device-specific drivers from consumers | Keep OS audio, serial, RF, GNSS and GPIO behind explicit adapter contracts | Mac mock tests first; Pi hardware acceptance separately |
| [Diátaxis](https://diataxis.fr/) | Distinguishes tutorials, how-to, reference and explanation | Classify existing guides without mass-renaming or moving docs | Existing documentation index and local links remain valid |
| [MADR / architecture decisions](https://adr.github.io/adr-templates/) | Small decision records capture context, options and tradeoffs | Record Mac-first/Pi-later decision under [ADR 0001](adr/0001-mac-first-portable-core.md) | Later changes can supersede the ADR explicitly |
| [GitHub Actions concurrency](https://docs.github.com/en/actions/concepts/workflows-and-actions/concurrency) | Workflow concurrency groups can cancel older runs for the same branch | Push coherent batches, not dozens of sequential micro-commits, to preserve meaningful CI evidence | Exact proposed merge commit has complete 5-job green result |
| [GitHub CODEOWNERS](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners) | Path owners can be designated; enforced reviews are a separate repository setting | Defer named reviewer rules until maintainers/roles justify them; start with functional ownership | No invented human/team approvers |

## Proposed component map and boundaries

[Machine-readable component index](../catalog/component_index.json) is an **entrypoint inventory**, not an exhaustive map of every Rust file. Each record identifies a responsibility boundary, key code entrypoints, operator/engineering docs, existing offline tests and the outstanding hardware gate. The [offline validator](../tools/validate_component_index.py) checks references, path escapes, duplicate code owners, required crate entrypoints and CI registration; [fixture tests](../tools/test_component_index.py) challenge the validator without opening hardware.

```text
                 Mac browser + native CLI/TUI
                           |
            Desktop HUD / Micro HUD (same API)
                           |
             pipboy (local HTTP/OS adapters)
                           |
        pipboy-core (domain rules and durable intent)
                           |
           explicit peripheral abstraction seam
             /              |              \
       simulated          macOS OS        future Pi Linux
       adapters           interfaces      audio/SDR/GNSS/GPIO
                           |
          deny-only broker until separate approval
```

This is the **target separation of responsibilities**. Today some core modules still perform bounded OS/network I/O; do not claim the layers are perfectly isolated. A source extraction must preserve the existing data, permission and API contracts.

## Prioritized action register

| Priority | Scope | Change | Definition of done |
| --- | --- | --- | --- |
| P0 | Ownership and navigation | Add component index, offline path/gate validator and tests; link existing repository guides | All map paths resolve and existing repo validators pass |
| P0 | Branch/CI hygiene | Preserve single-topic PRs; batch logically related edits; do not combine radio, GIS, messaging and cleanup | No unrelated commit makes hardware acceptance appear passed |
| P0 | Mac-first portability | Keep `PIPBOY_HOME`, `pipboy` CLI and local API stable; record interface seams in ADR | Mac integration tests and fixtures; no premature Pi hardware claim |
| P1 | Local HTTP host | Extract `dashboard.rs` into coherent route groups only after related feature PRs settle | Exact endpoint status/body/security and micro/desktop tests unchanged |
| P1 | Domain modules | Review `tasks.rs`, `peer.rs`, `workbench.rs` by responsibility, not raw file length | Smaller independently testable seams with zero schema migration |
| P1 | Performance / build | Collect cargo timing, binary size, memory and CI duration baselines; do not guess savings | Reproducible before/after figures on identical host/toolchain |
| P2 | Hardware adapters | Prototype Linux output, SDR read-only, GNSS and physical controls using opt-in adapters | Actual Pi peripheral tests and power/thermal records |
| P2 | Release | Distinguish `source_present`, `software_verified`, `mac_field_accepted`, `pi_field_accepted` and `released` | Each label backed by a named test/evidence record |

## Completed boundary passes (post-research)

- **O1, PR #42:** Machine-readable component index and Mac-first/Raspberry Pi-later ADR merged after five-job CI. This did not relocate code or operator data.
- **O2, PR #43:** Task Control / explicit peer handoff HTTP handlers extracted to `dashboard/task_routes.rs`, with same-origin regression and five-job CI; merged as `60c066ac`.
- **O3, current review:** Messenger/PACE HTTP read/send/retry handlers isolated to `dashboard/messenger_routes.rs` with unchanged loopback router, PACE rules and pinned SSH backend. CI and external hardware acceptance must be recorded independently before merging.
- **Next:** Consider geospatial cache and bounded network probe HTTP adapters one subsystem at a time, after finalizing active GIS and RADIO feature branches. Do not infer better runtime performance from reduced file length alone.

## Non-goals and safety

- Do **not** move operator records out of `PIPBOY_HOME`, rewrite persisted formats, or turn a read-only receiver into a transmitter.
- Do **not** rename established crates, CLI flags, six HUD tabs, or `pip_boy` paths to improve aesthetics.
- Do **not** equate passing CI with physical hardware acceptance.
- Do **not** centralize secrets, device IDs, user coordinates, Bluetooth pairing data or private telemetry in `catalog/`.
- Do **not** introduce a plugin framework or a new crate before a stable interface and tested maintenance benefit are demonstrated.

## Review cadence and metrics

At each subsystem PR: update the component map if public entrypoints changed, keep [documentation links](REPO_STRUCTURE.md) valid, run relevant fixture tests and smoke checks, and attach CI plus field limitations to the PR. Monthly or before hardware freeze: inspect growth of large files, duplicate policy definitions, test durations and unresolved physical acceptance gates. **A decrease in source-file size alone is not evidence of improved architecture.**

Related: [architecture](ARCHITECTURE.md), [object model](OBJECT_MODEL.md), [validation record](VALIDATION.md), [storage contract](STORAGE.md), [contribution policy](../CONTRIBUTING.md).

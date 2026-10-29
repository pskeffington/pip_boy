# Machine indexing and source-of-truth hierarchy

Rust-Boy keeps **separate indexes for separate kinds of evidence**. They are source navigation tools, not runtime package grants, executable manifests, synchronization schedulers or field acceptance certificates.

## Canonical read order

| Index | Answers | Authority |
| --- | --- | --- |
| [`AGENTS.md`](../AGENTS.md) | How Codex conducts a safe bounded pass | Codex instructions |
| [`catalog/codex_passes.json`](../catalog/codex_passes.json) | Which development pass is next, what depends on what, what to test | **Pass backlog** |
| [`catalog/component_index.json`](../catalog/component_index.json) | Who owns a source entrypoint, which tests/docs apply | **Component ownership** |
| [`docs/PROD_STATUS.md`](PROD_STATUS.md), [`docs/VALIDATION.md`](VALIDATION.md) | What actually ran; current limitations and evidence | **Validation checkpoint** |
| [`docs/HOST_PACKAGE_ROADMAP.md`](HOST_PACKAGE_ROADMAP.md) | Host architecture and security gate ordering | **Design contract** |
| [`catalog/module_plan.json`](../catalog/module_plan.json), [`catalog/role_roadmap.json`](../catalog/role_roadmap.json), [`catalog/vault_org_chart.json`](../catalog/vault_org_chart.json) | Domain capabilities, user roles and organization design | Planning only |
| [`catalog/host_adapter_contract.json`](../catalog/host_adapter_contract.json) | Protocol constants and SDK vocabulary | Contract, not trust |
| [reference package fixtures](../catalog/host_packages), [compiled RADIO manifest](../catalog/builtin_packages/rust_radio.json) | Four inert reference designs vs. an explicitly compiled read-only catalogue | Neither grants capabilities |
| [`catalog/sources.json`](../catalog/sources.json), [`catalog/literature.json`](../catalog/literature.json) | Provenance, licensing and research limitations | References, not implementation |

## Index update matrix

| Change | Update | Offline checks |
| --- | --- | --- |
| Add/reprioritize a Codex pass or blocker | `catalog/codex_passes.json`, [pass guide](CODEX_PASSES.md), [checkpoint](PROD_STATUS.md) | `validate_codex_passes.py`, `test_validate_codex_passes.py` |
| Move an entrypoint, ownership or acceptance check | `catalog/component_index.json` and relevant authored guide | `validate_component_index.py`, `test_component_index.py` |
| Change SDK protocol or module fixture | `catalog/host_adapter_contract.json` or fixture, adapter guide | `validate_host_packages.py`, `test_validate_host_packages.py` |
| Add domain feature/role | `catalog/module_plan.json`, `catalog/role_roadmap.json` | `validate_catalog.py`, `validate_roles.py` |
| Add/remove navigation | Root `README.md`, `docs/README.md`, [repo structure](REPO_STRUCTURE.md), `catalog/README.md` | `validate_repo.py`, `test_validate_repo.py` |
| Claim release, security or hardware completion | [checkpoint](PROD_STATUS.md), [actual test evidence](VALIDATION.md) | Exact-head CI and real-device gates |

**Structural guarantees:** Codex IDs are stable `RB-CX-00`…; each pass has existing source/doc/check files, known component owners, ordered backward dependencies and exactly one `next` pass. `accepted` must cite exact commit SHA and CI run URL; field gates also need a checked-in report. The validator checks record shapes but cannot independently verify whether the remote CI actually passed. Human/agent review of real evidence remains mandatory.

**Ownership repair:** `crates/pipboy/src/dashboard.rs` belongs to the `local-hud` component. The universal adapter uses that route, but cross-component references should not duplicate its `code` ownership. Component docs may refer to shared integration paths without claiming a second owner.

**Branch boundary:** as of October 29, 2026, the pass backlog applies to draft PR #71 on `codex/package-adapter-protocol`. `main` and separately open server, security, camera, avatar and native-module PRs are not implicitly included. Do not mark an external branch merged based on a reference in a plan.

The [repository structure guide](REPO_STRUCTURE.md) governs storage and compatibility: never scan `PIPBOY_HOME`, mutate operator state, import secret/captured device data or execute a manifest as part of indexing. The new validators use the Python standard library and read checked-in paths only; compilation, browser acceptance and Pi hardware tests remain independent.

```bash
python3 tools/validate_codex_passes.py && python3 tools/test_validate_codex_passes.py
python3 tools/validate_component_index.py && python3 tools/test_component_index.py
python3 tools/validate_repo.py && python3 tools/test_validate_repo.py
python3 tools/validate_host_packages.py && python3 tools/test_validate_host_packages.py
```

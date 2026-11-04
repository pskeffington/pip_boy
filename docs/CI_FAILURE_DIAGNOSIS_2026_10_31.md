# CI failure diagnosis — 2026-10-31

## Method and scope

Inspected actual GitHub Actions runs and failing job logs for active PRs #80, #82 and #86, and checked current-run admission for #88, #87, #84 and the documentation PR #89. Failures below are observed at their **listed head SHA**, not necessarily at a later merge commit. No code fix or rerun is claimed in this document.

| PR / exact SHA | Workflow | Observed failures | Diagnosis / required next patch |
| --- | --- | --- | --- |
| [#80](https://github.com/pskeffington/pip_boy/pull/80) `00204d7` | [41017465891](https://github.com/pskeffington/pip_boy/actions/runs/41017465891) | stable formatting; Rust 1.90 Clippy; SoapySDR Clippy | In `crates/pipboy/src/terminal_commands.rs`, unused `const MAX_INPUT: usize = MAX_COMMAND_BYTES;` at line 10 triggers `dead_code` with `-D warnings`. Remove unused alias and its otherwise-unused import if appropriate; retain bounded parser's core check. Run `cargo fmt --all`, commit formatting of terminal_commands.rs; run all native/feature jobs again. Both Clippy failures share this root cause. |
| [#82](https://github.com/pskeffington/pip_boy/pull/82) `4515a74` | [41021037131](https://github.com/pskeffington/pip_boy/actions/runs/41021037131) | stable formatting only | `rustfmt --check` demands multiline formatting in `crates/pipboy-core/src/observation.rs`, around lines 212/225. Run formatter, commit the formatted file, rerun exact-head jobs. No functional test failure identified. |
| [#86](https://github.com/pskeffington/pip_boy/pull/86) `38525d8` | [41024821378](https://github.com/pskeffington/pip_boy/actions/runs/41024821378) | stable formatting only | `rustfmt --check` demands expanded `assert_eq!`/assertion formatting in `crates/pipboy-core/src/universal_adapter.rs`, around 718, 754 and 775. Run formatter, commit exact diff and rerun jobs. No functional test failure identified. |
| [#88](https://github.com/pskeffington/pip_boy/pull/88) `457d965` | [41127024688](https://github.com/pskeffington/pip_boy/actions/runs/41127024688) | none in this run | Workflow reported success. Still draft and not a release acceptance. |
| [#87](https://github.com/pskeffington/pip_boy/pull/87) `9608dbe` | [41025584846](https://github.com/pskeffington/pip_boy/actions/runs/41025584846) | none in this run | Workflow reported success. SCAN remains inert. |
| [#84](https://github.com/pskeffington/pip_boy/pull/84) `ea643c4` | [41023392472](https://github.com/pskeffington/pip_boy/actions/runs/41023392472) | none in this run | Workflow reported success; separate merge review required. |
| [#89](https://github.com/pskeffington/pip_boy/pull/89) `fd64e5a` | [41175155912](https://github.com/pskeffington/pip_boy/actions/runs/41175155912) | none | Seven executed jobs passed on this previously tested head; this diagnosis commit requires its own new exact-head run. |

## Repair order

1. **TC-A02/#80:** remove the unused MAX_INPUT alias while preserving parser's real MAX_COMMAND_BYTES invariant; format file; run stable/1.90 Clippy, SoapySDR and full workspace. Avoid broad parser changes and retain offline-only command policy.
2. **#82/#86 independently:** run `cargo fmt --all` in each branch and commit the corresponding minimal formatting diff; no changes to behavior or interfaces.
3. Reconcile #80, #82, #86 with latest main **after** passing their own checks; run head-specific CI again if rebasing changes SHA. Check #71's merge conflict separately; no job run was returned for the inspected #71 head, so its status is not accepted.
4. Review and merge in dependency order; never infer actual merge readiness solely from a green workflow or GitHub's mergeable flag.

## Guardrails

A formatting error is a quality-gate failure, not evidence of incorrect functionality. Absence of functional test failures in these logs is not a claim that all field or security tests passed. Do not relax `-D warnings`, ignore CI jobs, or treat a cancelled job as success. No external hardware was tested in this diagnosis.

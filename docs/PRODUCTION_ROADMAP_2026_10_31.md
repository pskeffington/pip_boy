# Rust-Boy production roadmap and evidence ledger — 2026-10-31

This is the forward execution index, not a claim that open PRs are merged. Canonical detailed contracts remain in [HOST_PACKAGE_ROADMAP](HOST_PACKAGE_ROADMAP.md), [TERMINAL_AGENT_EXECUTION](TERMINAL_AGENT_EXECUTION.md), [UNIVERSAL_ADAPTER](UNIVERSAL_ADAPTER.md), [RB_SYNC_ROADMAP](RB_SYNC_ROADMAP.md), and [SECURITY](../SECURITY.md).

## Reference field hardware decision

**Raspberry Pi 5** is the primary physical acceptance platform; macOS/Windows remain development and regression targets. See [PI5-00 through PI5-08 matrix](PI5_REFERENCE_TEST_BASELINE.md). Physical testing on Pi 5 is an independent release condition; do not claim it from cross-compilation or CI.

## Baseline and classifications

- **Implemented on main:** Rust desktop and micro loopback UI, role workbenches, local Task Control, county/ZIP GIS and cache, in-process IP radio, bounded network diagnostics/UniFi adapter, manual pinned-SSH peer transport, and offline typed command catalog.
- **Accepted software gate:** TC-A01 typed command core at [seven-job exact-head Actions run 41003353102](https://github.com/pskeffington/pip_boy/actions/runs/41003353102). This is not acceptance of all CLI/TUI/GUI callers.
- **Submitted/in review, not automatically accepted:** [#88 supervised package host](https://github.com/pskeffington/pip_boy/pull/88), [#86 adapter fault recovery](https://github.com/pskeffington/pip_boy/pull/86), [#80 CLI catalog migration](https://github.com/pskeffington/pip_boy/pull/80), [#78 inert handheld MODULES selection](https://github.com/pskeffington/pip_boy/pull/78), [#87 inert SCAN registry](https://github.com/pskeffington/pip_boy/pull/87), and cleanup PRs. Verify state, changed files, exact-head jobs and conflicts before integration.
- **Design/scaffold only:** SCAN live camera/phone bridge, mini-server and actual mesh, third-party package installation, complete org-authorized RB-to-RB sync and clinical deployment.
- **Not accepted:** physical Pi, real SDR, Bluetooth fault recovery, Mac-to-Pi interoperability, high-availability network and external stream availability, accessibility/production packaging.

Status vocabulary: proposed; source_submitted; software_verified (exact SHA and executed jobs); merged; field_verified (real device); release_accepted. Never infer a later state from an earlier one.

## Critical path: ordered work packages

| Priority / gate | Owner | Deliverable | Dependencies | Exit evidence |
| --- | --- | --- | --- | --- |
| P0 / INTEG-00 | COORDINATOR | Reconcile HEAD, PR #88/#86/#80/#78/#87, component ownership and CI admission; freeze integration slices | None | PR/commit matrix with blocked overlaps and exact SHA |
| P0 / HOST-01 | HOST | Supervised first-party child process, bounded RBPK IPC, startup/shutdown, crash and timeout recovery, pinned local package roster | INTEG-00, existing host contracts | tests covering crash/reap, denial, restart, foreground preservation, no leaked child |
| P0 / HOST-02 | GUI + HOST | One real foreground adapter bound to desktop/micro; host-owned Settings always reachable; no implicit activation from MODULES | HOST-01 and approved UI contract | desktop/480x320/800x480 automated screenshots and keyboard/touch checks |
| P0 / TC-A02 | TERMINAL | Migrate both CLI and TUI to shared typed dispatcher, preserving legacy offline grammar | TC-A01 | parser compatibility, malformed-input and TUI interaction tests |
| P0 / TC-A03 | ADAPTER | Wire approved offline MAP and read-only RADIO actions through catalog; TV/COMMS refused until verified installation | HOST-01, TC-A01 | real handler and denied-capability tests |
| P1 / TC-A04/A05 | SECURITY + GUI | Negative matrix and shared desktop/micro command palette | TC-A02 and dispatch contract | denied/untrusted/revoked tests, UI viewport and CI |
| P1 / PKG-01 | PACKAGING | First-party manifest provenance, executable hash, compatibility and signed installation/rollback proposal; staged files are not approvals | HOST-01, security policy | positive and tamper/rollback/revocation tests |
| P1 / MODULE-01 | MODULE INTEGRATION | Independently owned rb_gis, rust_radio, rust_tv, rusti_rc adapters; one foreground; preserve native functionality and background declarations | HOST-02, PKG-01 | pinned integration fixtures and per-module acceptance; no dynamic repo downloads |
| P2 / SCAN-H | CAMERA/PRIVACY | Inert SCAN metadata first; later consent broker, local camera and reciprocal paired phone still capture | #87 acceptance, permission broker and paired identity | per-endpoint explicit consent, deny-by-default, no silent capture |
| P2 / ORG-3.2 | SECURITY | Protected device key custody, issuer/trust-root enrollment, persistent anti-replay and reliable offline revocation | security design and threat review | real-device key and rollback/replay/revocation tests |
| P2 / SYNC-R0–R3 | COMMS | Scoped versioned event log, authenticated paired-IP reconciliation, interrupted/unknown-outcome delivery | ORG-3.2 for org-authorized flows | two-host link-loss/rejoin, bounded queues, receipts and audit |
| P3 / SERVER-00 | DISTRIBUTED | Mini-server topology and admission-gated relay; no public listener by default | identity, scoped sync and broker security | network partition/restore, backpressure, denial and field testing |
| P3 / REL-01 | RELEASE | Mac/Pi/Windows test matrix, accessibility, stream rights, reproducible packaging, operator acceptance | relevant completed software gates | signed release evidence and explicitly scoped hardware signoff |

Parallelize only file-disjoint work; security/CI can run alongside terminal and host engineering. Do not start SYNC or server network listeners by bypassing org trust.

## Always-on engineering invariants

1. One foreground package, Settings always host-owned, no host-page scroll or obstructive map overlays; modules may use bounded interior scroll. Role/location selected in Settings.
2. Module selection, help and navigation are inert; no audio, camera, scan, network fetch, sending or hardware capture without explicit grant and action.
3. Reference/discovered/validated manifests are not installed or authorized code. Untrusted packages cannot rely solely on in-process crate boundaries.
4. Local-first/loopback by default; offline functionality and last-good map/cache survive bad updates. All remote outcomes require receipts; ambiguous sends remain UNKNOWN.
5. Keep independent module repositories independent. Do not import uncontrolled third-party GitHub actions, command-string shells or remote executable bundles.
6. Treat emergency/medical workflows as simulations until independently accepted. No PHI or clinical deployment without separate controls and review.
7. A green badge or zero-job Actions startup_failure is not a gate pass. Cite executed jobs at exact reviewed HEAD.

## CI and release decision procedure

For every code PR: `cargo fmt --all -- --check`; stable and Rust 1.90 workspace check; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --locked`; relevant JS UI tests and offline fixture validators; macOS/Windows/SDR gates where changed. Follow repository-owned workflow policy; record exact SHA, Actions URL, real job names, test totals and any failures. If integration cannot run, mark blocked, isolate the cause, and continue a truly independent work packet. After merge, revalidate main as appropriate. Field verification must have real operator/device evidence.

## Codex iteration contract

At each pass: (1) inspect HEAD/PRs/issues and queued dependencies; (2) select the highest-priority unblocked, non-overlapping task; (3) document scope, owner and negative cases; (4) implement a small reversible vertical slice; (5) run or request exact-head CI evidence; (6) record result and residual risk in its PR and task index; (7) take the next unblocked task. Never mark a task complete with only a draft PR, synthetic fixture or unexecuted test command. Coordinate the queue in [CODEX_LONG_HORIZON](CODEX_LONG_HORIZON.md).

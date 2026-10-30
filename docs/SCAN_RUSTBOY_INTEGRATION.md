# SCAN — camera-first TRANS family integration roadmap
Status: **SCAN-H01 inert registry implementation candidate; not installed or released** (2026-10-30). Exact-head CI and contract review pending.
Owner: Rust-Boy host/adapter integration. Source-of-truth TRANS processing ownership stays in its own repositories.

## Mission and unambiguous name
Add **SCAN** as a fifth Rust-Boy socket module, proposed package ID `org.rustboy.scan`. SCAN means optical/visual scanning and text recognition, **not** `pipboy packages scan DIR` (manifest discovery) or NET ENGINEER `SCAN LAN`. Preserve both existing commands and their semantics.

The operator opens a single SCAN viewport, chooses **camera on this device**, **camera on a paired phone**, **camera on a paired Rust-Boy** or **explicitly selected local image/document**. A paired phone can supply captures to Rust-Boy for TRANS processing **and vice versa**: a phone can request/review a capture from Rust-Boy's attached camera. Either endpoint can be a capture source, requestor or processing/viewing endpoint after capability negotiation. Never assume the phone can run the full TRANS stack merely because it can capture. No unsolicited reciprocal camera access.

## Existing host status
`crates/rust-boy-adapter-sdk` defines protocol v1, bounded 16-KiB envelopes and 4-KiB intents. `pipboy-core::universal_adapter` owns trusted in-process adapter lifecycle, foreground selection and Settings. The `feat/scan-h01-inert-registry` branch now adds `org.rustboy.scan` as a **fifth inert reference** to `catalog/host_adapter_contract.json`, `crates/pipboy-core/src/package_socket.rs`, the manifest validator, and a `trans_scan.example.json` design fixture. Rust and Python negative tests reject installation claims, unknown/cross-module scopes and SCAN background work. This is written source, **not an executed CI pass**, approved installation, an OS camera permission, a mounted adapter, or a working paired-phone transport. Prior original four synthetic adapter tests remain separate; do not claim they are five native adapters. The existing module UI remains separately gated by HOST-UI-1, HOST-SEC-1 and current terminal agent queue.

## Capture modes, ordered
| Mode | Direction | First operation | Release gate |
|---|---|---|---|
| A1 — attached camera | USB/UVC, embedded/CSI or explicitly supported OS camera -> Rust-Boy SCAN | user taps PREVIEW then CAPTURE | OS permission + host scope + capture-driver field evidence |
| A2 — phone supplies camera | paired phone -> Rust-Boy SCAN | paired phone operator opens SCAN and consents to local capture/send | authenticated session, receiver-bound consent, phone OS/browser acceptance |
| A3 — reciprocal camera | Rust-Boy attached camera -> paired phone SCAN view | phone requests capture; Rust-Boy camera owner independently approves | both endpoint identities, capture-specific grants, receipt verification |
| A4 — other RB device supplies camera | paired RB -> operator RB or phone | same explicit request/owner approval | scoped peer transport + two-device tests |
| A5 — local image/document import | selected local file -> SCAN | operator selects a file | file-source handle + intake rights/retention policy |
| B — live preview/video frames | source -> authorized viewport | separate later explicit preview session | frame quotas, stop/revoke, energy/thermal, platform proof |

MVP is finite still images with optional local preview; continuous network stream and background capture are **not** part of MVP. Unsupported camera backend displays `unavailable`, never fakes success.

## Authority, data boundary and security
- **Rust-Boy host** owns SCAN installation/approval, one foreground viewport, Settings, privilege broker, paired-device enrollment, session lifecycle, and permission revocation.
- **Camera-owning endpoint** owns the actual OS-level camera consent and remote request acceptance. Pairing/role membership is not a camera permission. Consent must be affirmative and specific to camera, peer, session, purpose and expiration; no auto-accept, background wake, opening from tab switch or role selection. Stop on lock, logout, app background, timeout, disconnect, revocation or package suspend unless explicitly reauthorized by separate reviewed policy.
- **Capture plane** transports image bytes through a separate authenticated, encrypted, bounded **media channel** negotiated by both endpoints. Do not put image bytes, frames, full OCR text or embedded base64 in 4-KiB intents, 16-KiB adapter replies, generic task SSH packets, registry manifests, command history, web logs, URLs, or git. Use short-lived opaque content handles; enforce max resolution/bytes/pages/time, integrity digest, content classification, local encrypted temporary storage as policy demands and deterministic purge. No unauthenticated LAN listener; do not expose loopback dashboard on the LAN. Reuse reviewed platform crypto/transport libraries and explicit trusted enrollment; never invent crypto.
- **Control plane** passes only bounded request IDs, source/page/region references, hashes, session state, capture provenance, confidence/review flags and sanitized errors. Untrusted visual/OCR results must render as text, not arbitrary HTML/script.
- **Governed TRANS chain:** `ctl-injest` admits source pointer and rights -> `Eagle-Eye` owns camera/page/region visual evidence -> `trans` owns OCR/HTR evidence and review routing -> `trans-release` alone checks release/approval -> `trans-downstream` separately authorizes output destinations. `trans-latin` and `Trans-heb` receive only bounded scholarly candidates and decide their own acceptance. A preview, machine confidence or local Save Draft does not confer scholarly or external release approval.
- Never auto-fetch models or upload images for recognition. Local/offline inference must indicate which model is actually installed, license/rights, exact revision and available compute. Model missing => explicit unavailable/manual review.

## Operator flow / UI states
`UNAVAILABLE -> READY -> SELECT SOURCE -> (PAIR/OWNER CONSENT) -> PREVIEW -> CAPTURE -> VERIFY -> ADMIT -> VISUAL -> OCR/HTR -> REVIEW -> SAVE DRAFT`. Optional approved release/export is a **separate** route. Capture and inference operations require independent explicit actions. A remote request alone cannot turn on a camera. `DENIED`, `DISCONNECTED`, `STALE`, `CANCELLED`, `UNSUPPORTED` and `UNKNOWN_OUTCOME` must never appear as `CAPTURED` or `DELIVERED`. Preserve exact hashes and source identities; review-required cannot be silently upgraded.

SCAN uses the same **single foreground package** as MAP/TV/RADIO/COMMS. Settings is always host-owned. The 480x320 micro surface has large CAPTURE, STOP, SOURCE, REVIEW controls and accessible keyboard/screen reader focus; remote approval should be unambiguous. Do not require a phone for on-device mode or a Rust-Boy camera for phone-supplied mode.

## Phased acceptance and agent order
| Gate | Agent / deliverable | Dependencies | Required evidence |
|---|---|---|---|
| SCAN-0 | Contract/index freeze, full source and camera matrix, negative fixtures (SCAN-H00) | none | reviewed docs + pinned head inventory |
| SCAN-1 | Fifth inert fixture, registry catalog and negative-test source **committed, CI pending** (SCAN-H01) | SCAN-0 review | exact-head tests reject false installation, cross-module scopes and unknown capabilities |
| SCAN-2 | Synthetic SCAN adapter, typed intents, resource/foreground/Settings behavior (SCAN-H02) | SCAN-1, HOST-ADAPTER-CORE-1 | Rust test evidence and exact CI jobs |
| SCAN-3 | Attached camera explicit still capture via host-brokered handles (SCAN-H03) | SCAN-2, HOST-SEC-1 | permissions, no select-side IO, deny/revoke + OS camera evidence |
| SCAN-4 | Paired phone -> RB still handoff + RB -> phone reciprocal capture (SCAN-H04/H05) | SCAN-3, authenticated media boundary | two-owner consent, replay/timeout/peer-offline tests |
| SCAN-5 | Governed intake -> visual -> OCR/HTR with no promotions (SCAN-H06 + upstream agents) | versioned upstream contracts | cross-repo exact-SHA fixtures, provenance, review states |
| SCAN-6 | Release, downstream, optional scholarly review wiring (SCAN-H07) | SCAN-5 | fail-closed approvals and no automatic publish |
| SCAN-7 | UI, Mac/Pi/phone field testing and production review (SCAN-H08) | HOST-UI-1, all above | 480x320/accessibility, physical camera and both directions, resource budgets, signed package/rollback |

Parallel lanes: `ctl-injest` admission, Eagle-Eye frame evidence and TRANS bounded inference may begin contract work while host registry/adapter gates advance. Serial gates must remain serial for signed pairing, real camera access, external release and authority. Do not block software-only work on manual side-by-side device testing; do not mark field gates accepted without physical evidence.

## Execution / index
- Source of machine-readable, per-repository dependency queue: [`catalog/scan_agent_tasks.json`](../catalog/scan_agent_tasks.json).
- Capture-specific control/data envelopes: [`docs/SCAN_CAPTURE_CONTRACT.md`](SCAN_CAPTURE_CONTRACT.md).
- Existing host contracts: [`docs/UNIVERSAL_ADAPTER.md`](UNIVERSAL_ADAPTER.md), [`docs/PACKAGE_SOCKET.md`](PACKAGE_SOCKET.md), [`docs/HOST_PACKAGE_ROADMAP.md`](HOST_PACKAGE_ROADMAP.md), and [`AGENTS.md`](../AGENTS.md).
- Current CLI/TUI command work remains [`catalog/terminal_agent_tasks.json`](../catalog/terminal_agent_tasks.json). Do not rewrite its acceptance statuses or bypass it.
- Family repositories: `pskeffington/ctl-injest`, `Eagle-Eye`, `trans`, `trans-release`, `trans-downstream`, `trans-latin`, `Trans-heb` (exact case for GitHub paths).
- All implementation PRs pin exact source commits and list actual `cargo fmt`, Clippy, locked tests, Python test gates, browser tests and CI job URLs. Missing CI admission = blocked, not passed. Device release is separately accepted.

# Rust-Boy — Data leakage and credential-boundary review

**Review date:** 2026-10-26  
**Baseline:** `main` at `addfeaa3176061400fb4540df15adf39c420cb17` (feature branches changed during the review)  
**Review type:** Static source, repository-tree and documented threat-boundary review. **Not a penetration test or a complete Git-history scan.**

## Executive assessment

There is **no demonstrated unauthorized external transfer of operator records or confirmed committed live credential** in the material reviewed. That is narrower than a clean security certification. The existing local dashboard binds to 127.0.0.1, requires the exact Host, enforces Origin on POST, sends no CORS allow header and uses a restrictive CSP. UniFi credentials are already omitted from browser JSON and the UniFi curl child's environment, and peer SSH uses pinned host identities.

However, local task, message, note and field-site contents can be sensitive. Most are still plaintext; the browser's local GET APIs expose them to processes with access to that localhost service; explicit online map requests share location-derived data with providers. A confirmed credential *inheritance path* existed because unrelated native commands inherited `RUSTBOY_UNIFI_API_KEY` by default. This patch removes known API/cloud credentials from those child environments.

## Evidence-ranked findings

| ID | Risk | Observation and supporting source | Status / required action |
| --- | --- | --- | --- |
| RB-SEC-01 | **Medium — credential exposure path** | `pipboy-core/src/unifi.rs` reads `RUSTBOY_UNIFI_API_KEY`; its own curl child removes it. Other native commands are created by `pipboy/src/peer.rs`, `county_cache.rs`, `endpoint_probe.rs`, `dashboard.rs` and `pipboy-core/src/network_probe.rs` without removing it. Subprocesses normally inherit the parent's environment. **No transmission or theft was observed.** | **Mitigated in this PR:** use `process_safety::isolated_command` for all built-in native subprocess launches, with unit tests and CI enforcement. The process itself still holds its environment and PATH substitution remains a separate risk. |
| RB-SEC-02 | **High for confidential use; medium for the documented development-only use** | `notes.rs`, `tasks.rs`, `reclamation.rs`, peer and PACE stores keep private data in plaintext JSON. `messenger.rs` defaults to plaintext `messages.json`; age/keyring enrollment is optional, only for fresh journals, and is **not** end-to-end encryption. Unix private modes and atomic writes help against other OS users but not same-account software or backups. | **Open:** prohibit protected/patient/classified/credential/precise-person-location content in these stores. Design per-store at-rest encryption, migrations/backups/key loss and rotation; consider encrypted messenger as a future default only with an explicit migration plan. |
| RB-SEC-03 | **Medium — local API confidentiality** | The localhost service serves `GET /api/messages`, `GET /api/tasks`, `GET /api/reclamation` and map/GeoJSON exports with local data (`pipboy/src/dashboard.rs`). The Host check, same-origin POST protection, no-CORS and loopback binding reduce browser/network attacks, but there is no per-human session authentication and a same-host process can access the service. | **Open:** use explicit short-lived local session credentials for sensitive reads/writes, enforce minimum necessary response fields, test cross-origin and same-machine access. Never proxy/port-forward the current dashboard. |
| RB-SEC-04 | **Medium — expected external map-provider disclosure** | `map.js` invokes browser geolocation only by user action; `map-online.js` transmits a selected point to Census TIGERweb for county resolution and a selected viewport to Overpass when enabled. Overpass requests set `referrerPolicy: 'origin'` for provider compatibility. Public providers also observe network origin/IP. | **Consent-dependent, not silent exfiltration:** keep independent COUNTY/WEB opt-in, show destination and data scope, never log GPS fixes in the default path, and use cached/locally imported GIS when privacy is required. Validate UX at 480×320 and ensure opt-in resets when leaving MAP. |
| RB-SEC-05 | **Medium — accidental public commit exposure** | `.gitignore` previously covered `.env` but did not explicitly exclude all Rust-Boy state records, private SSH key formats or location exports. Public tracked filenames on `main` and six open PR branches plus the index branch did not reveal obvious operator-state/credential files. | **Mitigated in this PR:** add explicit ignore entries and a read-only CI tracked-file scanner that flags sensitive paths, strong credential signatures, and unguarded process creation without printing secret values. Does **not** scan Git history. |
| RB-SEC-06 | **Medium — executable trust** | The native tools use external command names resolved through PATH (`ssh`, `curl`, `ping`, `arp`, macOS `open`). A malicious substituted binary could capture arguments, stdin, local file paths or credentials. The Unix/Windows platform trust of every executable was not independently verified. | **Open:** review fixed trusted executable resolution and binary provenance for supported deployments, especially the UniFi HTTPS credential path and SSH. The new environment scrub reduces inherited-secret exposure but does not fix executable substitution. |
| RB-SEC-07 | **Unverified — historical secrets and dependencies** | The review checked current tracked file names and selected Rust/browser/fixture source signatures; GitHub secret-scanning/Dependabot alert endpoints were not available through the connected repository tools. Full Git history and advisory databases were **not** audited. | **Open:** run an organization-approved full-history scanner; review GitHub secret-scanning, push protection, Dependabot and `cargo audit` outputs; privately rotate/revoke any real exposed secret before repository-history cleanup. |

Severity is contextual: every finding assumes a specific threat model. A compromised local account or hostile binary is outside Rust-Boy's existing OS trust boundary. The table does not establish exploitation.

## Preventive controls introduced

1. `pipboy_core::process_safety::isolated_command` creates native child processes with recognized UniFi, GitHub, OpenAI, Anthropic, AWS, Azure, Google and Hugging Face credential variables removed. Unit tests verify removals without running a child process.
2. `tools/validate_security.py` checks **only Git-tracked checkout files**. It blocks known private Rust-Boy journals, GPS trace exports, private key file types, high-confidence credential token formats, credential-bearing URLs and unreviewed `Command::new` constructors outside the reviewed helper. It reports categories and line numbers, never captured token text. An oversized text file requires explicit review.
3. `tools/test_validate_security.py` exercises synthetic detections and false-positive limits; the Ubuntu catalog CI job runs both tools.
4. `.gitignore` excludes obvious operator records and private file formats. Ignore rules **do not untrack already committed files**, and neither the ignore rules nor the scanner delete or modify user data.

## Priority follow-up gates

**P0:** Keep real secrets, patient records, protected sites, private GPS traces and operational peer rosters out of public fixtures, PR logs and screenshots. If a real credential was present in any earlier commit, rotate it and use private vulnerability reporting before discussing details publicly.

**P1:** Add local session authentication/authorization to sensitive API reads and writes; evaluate encrypting all private stores, with tested recovery, key rotation and a non-destructive migration. Preserve explicit consent for all map-provider transmissions.

**P2:** Verify operating-system trust of external executables, harden PATH resolution and child environments, and add cross-platform spoofed-PATH tests that do not use real keys. Review data retention and privacy rules for backups, exports and shared devices.

**P3:** Run full-history secret detection, dependency advisory checks, and a separate deployment/hardware threat assessment. Require independent review before any production, medical, emergency or classified use.

## Scope and limitations

- Scanned current tracked file names on main and named active feature branches, and reviewed representative source, UI, store, external transport and provider handling. Pattern matching cannot reliably identify every custom API credential, proprietary dataset, patient identifier or sensitive location.
- No live UniFi controller, GPS device, SSH peer, real patient/field record, third-party API credential, or production host was accessed or tested. CI unit/smoke tests are not proofs of hardware or deployment security.
- A public-source check cannot tell whether sensitive records remain in earlier Git objects, forks, backups, local working trees, cached artifacts or private storage. A public commit that removes a secret **does not revoke it**.

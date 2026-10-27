# Rust-Boy literature matrix — NIST / healthcare / tunnels / disruption-tolerant communication

**Reviewed:** 2026-10-27. **Scope:** research-to-requirement crosswalk for the optional mini-server and independently operating field nodes. This is not a claim of NIST validation, HIPAA compliance, responder-radio certification, or actual tunnel coverage. Citation URLs are primary references; proposed gates are Rust-Boy engineering interpretations, not requirements quoted verbatim.

| ID | Source (primary) | Domain / finding | Rust-Boy design implication | Evidence / phase |
| --- | --- | --- | --- | --- |
| L01 | [NIST SP 800-53 Rev. 5](https://csrc.nist.gov/pubs/sp/800/53/r5/final) | AC, AU, CP, IA, SC, SI controls address authorization, accountability, recovery, identity and communications protection | Trace independent device/operator grants; authenticated envelope receipt; audited enrollment/revocation; recovery and negative controls | S1 security matrix; S7 assessment |
| L02 | [NIST SP 800-207](https://csrc.nist.gov/pubs/sp/800/207/final) | Location/network membership is not an authorization boundary | A sector LAN, SSID, mDNS hint, relay or nearby radio must never confer organizational authority | S1 negative authentication tests |
| L03 | [NIST SP 800-34 Rev. 1 updated](https://csrc.nist.gov/pubs/sp/800/34/r1/upd1/final) | Business impact, recovery targets, alternate processing and contingency exercises | Define outage-state behavior, recovery objective and offline manual fallback before field claims | S2/S5 outage exercise |
| L04 | [NIST SP 800-66 Rev. 2](https://csrc.nist.gov/pubs/sp/800/66/r2/final) | Healthcare ePHI requires risk analysis and administrative, physical and technical safeguards | Public-demo payloads only in initial gates; hospital adoption needs isolated clinical scope, privacy, audit, access review and contracts | S7 independent regulated-data gate |
| L05 | [NIST CSF 2.0](https://www.nist.gov/cyberframework) | Govern/Identify/Protect/Detect/Respond/Recover integration | Maintain deployment risk register, asset inventory, event response and recovery evidence | S0–S7 documentation and evidence index |
| L06 | [CMS provider guidance](https://www.cms.gov/medicare/health-safety-standards/quality-safety-oversight-emergency-preparedness/health-care-provider-guidance) | Emergency preparedness includes risk assessment, communication plan, training and testing | A hospital communications adjunct needs primary/alternate pathways, tested downtime playbooks and human escalation outside the application | S5/S7 hospital exercise |
| L07 | [ASPR TRACIE telecommunications failures](https://asprtracie.hhs.gov/technical-resources/resource/12054/utility-failures-in-health-care-tip-sheet-telecommunications) | Power, lines, carrier/service and infrastructure dependencies can fail together | Separate failure domains and power budgets; never treat internet, VoIP and Wi-Fi as wholly independent without site survey | S5 fault tree |
| L08 | [NIOSH underground communications system installations review](https://stacks.cdc.gov/view/cdc/228109) | Underground communications systems include leaky-feeder, wired-node and wireless-node installations | Treat tunnels as site-specific RF challenge: map coverage, bends, shafts, attenuation, power and antenna placement. Do not promise a software-only mesh | S6 instrumented site survey |
| L09 | [NFPA 1225 technical material](https://docinfofiles.nfpa.org/files/AboutTheCodes/1225/A2021_Motions_Committee_Report.pdf) | Emergency-responder in-building RF systems are a distinct engineered/coded domain | Rust-Boy cannot replace approved emergency responder communication enhancement systems; consult AHJ and applicable code edition | S6 site/regulatory review |
| L10 | [RFC 9171 BPv7](https://www.rfc-editor.org/rfc/rfc9171) | Store-carry-forward tolerates partitions; protocol alone does not guarantee destination delivery | Bounded persistent queue, TTL, immutable IDs, acknowledgement distinction; evaluate BPv7 interop separately | S2–S5 deterministic stress |
| L11 | [RFC 9172 BPSec](https://www.rfc-editor.org/rfc/rfc9172) | Intermediary storage/forwarding requires integrity/confidentiality controls | End-to-end protected envelopes; relay has no plaintext or task-issuance authority; avoid inventing bespoke cryptography | S1/S4 threat tests |
| L12 | [RFC 9000 QUIC](https://www.rfc-editor.org/rfc/rfc9000) | Connection migration can help when IP path changes but not complete outages | Evaluate only after direct secure sessions exist; retain journal when disconnected | S5 roam tests |

For mining-grade physical network priorities and verified underground communications research, see [the mining communications matrix](MINING_COMMUNICATIONS_MATRIX.md).

## Scenario requirements

**Hospital:** independent local operation; unit/department-scoped grants; explicit operator and device enrollment; signage and offline contact fallback; protected clinical data disabled; separate alarms and emergency procedures, no reliance on Rust-Boy as life-safety system. Exercise power, core switch, DNS, captive network, hospital firewall, certificate revocation and shift handoff.

**Tunnel / underground:** use site-surveyed physical bearer (wired/fiber, properly engineered distributed antenna/leaky feeder, authorized Wi-Fi infrastructure or other approved radio); map coverage by distance, bends and portals; account for power, water, concrete/steel and mobility. Test no-RF zones, isolated stretches, relay outages and reconnection. Never claim mesh simply because a Rust-Boy supports multiple radio interfaces.

**Distributed / leave-the-wire:** durable local journal survives reboot; PACE selects direct approved IP, then approved relay, otherwise offline hold. Contacts and routes are untrusted until independently authenticated. Retry with jitter, duplicates are idempotent, revocation freshness fails closed. A remote durable receipt is not human acknowledgment.

## Phased evidence and gates

| Phase | Research-derived task | Gate |
| --- | --- | --- |
| S0 | Index corpus, controls crosswalk and system threat model | Each recommendation traceable to a source and test ID |
| S1 | Implement protected device keys, trust-root administration, replay/revocation evidence, per-dataset grants | Reject unknown/cross-org/stale peers and wrong trusted device |
| S2 | Persistent field queue, restart-safe journal, strict resources and local usability | Crash/partition/queue exhaustion tests |
| S3 | Mac–Pi direct authenticated task/message exchange | Two devices, disconnected and rejoined with deduplicated verified receipts |
| S4 | Optional mini-server, encrypted opaque store-forward, TTL/quotas/metrics | Relay cannot decrypt or authorize; restart and outage survive |
| S5 | PACE route hysteresis, fairness/priority, realistic congestion and hospital downtime drills | No fabricated online/human-read status; graceful degradation |
| S6 | Engineered tunnel coverage and physical bearer assessment | Survey coverage, legal RF constraints, battery/airtime budgets; no guaranteed emergency channel |
| S7 | Security assessment, signed builds, backups, hospital/HIPAA review if needed | Explicit documented sign-off, not self-certification |

## Research gaps and decision log

- Determine actual target deployment types, field hardware, power sources, expected coverage distances, and who operates each link before RF architecture selection.
- Verify applicable local building/fire codes and edition with the authority having jurisdiction; technical references are not deployment approval.
- Distinguish single-building hospital infrastructure from tunnels/underground RF and independent off-grid sectors; one physical layout cannot be presumed universal.
- Decide whether to implement BPv7 with reviewed libraries or only adopt DTN principles; present implementation is not BPv7 compatible.
- Quantify acceptable queue age, restart recovery and radio airtime with representative traces instead of inventing SLA targets.

## Source verification log (2026-10-27)

- **L03:** NIST SP 800-34 Rev. 1 final record verified; 2010 publication with November 2010 update. Treat as contingency guidance, not proof of field recovery.
- **L06–L07:** CMS emergency preparedness and ASPR TRACIE telecommunications guidance verified; stress power/network common-mode failure and training.
- **L08 corrected:** Earlier URL pointed to NIOSH publication 2012-153, *National Survey of the Mining Population Part II: Mines*, not a dedicated technical communications review. Replaced with the NIOSH *Review of Underground Coal Mine Emergency Communications and Tracking System Installations* (2014), which directly describes leaky-feeder, wired and wireless node-based systems.
- **Other rows:** Offline syntactic checks only; source-specific statement verification remains pending. The validator does not fetch websites and cannot certify references current or normatively applicable.

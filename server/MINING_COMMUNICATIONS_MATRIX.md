# Mining communications priority matrix — Rust-Boy field networking

**Research pass:** 2026-10-27. **Priority:** highest for physical-network harsh-environment validation. Mining-sector practices inform engineering; no MSHA approval, intrinsically safe certification, underground RF coverage, or emergency communication reliability is implied.

| ID | Source and publication | Evidence | Rust-Boy design action | Acceptance phase |
| --- | --- | --- | --- | --- |
| M01 | [NIOSH, A Review of Underground Coal Mine Emergency Communications and Tracking System Installations (2014)](https://stacks.cdc.gov/view/cdc/228109) | Reviews leaky-feeder, wired-node and wireless-node communications and tracking systems in underground coal mines | Maintain separate wired backbone, node network and local queues rather than assuming a single radio stack | S0, S6 |
| M02 | [NIOSH, Advanced Wireless Communication and Tracking Tutorial, chapter 2](https://archive.cdc.gov/www_cdc_gov/niosh/mining/content/emergencymanagementandresponse/commtracking/advcommtrackingtutorial2.html) | Describes leaky-feeder, UHF mesh, medium-frequency and through-the-earth communications; bridge nodes and power contingencies | Add a transport capability roster: throughput, power, coverage, backhaul independence, supported payload class | S5, S6 |
| M03 | [NIOSH, Advanced Wireless Communication and Tracking Tutorial, chapter 4](https://archive.cdc.gov/www_cdc_gov/niosh/mining/content/emergencymanagementandresponse/commtracking/advcommtrackingtutorial4.html) | Independent parallel paths and overlapping mesh coverage support survivability but underground geometry constrains redundancy | Test cable cuts, damaged nodes, portal separation, common-power outage and alternative-route recovery | S5, S6 |
| M04 | [NIST, Modeling Mine Mesh Networks](https://www.nist.gov/ctl/smart-connected-systems-division/modeling-mine-mesh-networks) | MF coupling to conductors and UHF waveguide propagation have different range/throughput characteristics; reported distances are not deployment guarantees | Do not use assumed RF radius; demand site-specific attenuation and measured coverage evidence | S6 |
| M05 | [MSHA, MINER Act implementation guidance P11-V-13](https://www.govinfo.gov/content/pkg/GOVPUB-L38-PURL-gpo36540/pdf/GOVPUB-L38-PURL-gpo36540.pdf) | Calls for redundant communication paths, status indication, maintenance and power planning | Separate link health, stored-receipt status and warning of failed alternative paths; maintain human fallback | S5, S7 |
| M06 | [30 USC 876 (MINER Act provisions)](https://www.law.cornell.edu/uscode/text/30/876) | Mine emergency response plans address redundant post-accident communications and personnel tracking | Keep safety-critical communications and tracked-person location distinct from ordinary data exchange | S7 |
| M07 | [Strata Worldwide, StrataConnect](https://www.strataworldwide.com/mining/strata-connect-mining) | Commercial offerings describe digitized leaky feeder, PoE/coax backhaul and battery-powered wireless mesh | Evaluate modular adapters for external gateway APIs; vendor capability statements require field verification | S4, S6 |
| M08 | [MSHA mine emergency checklist](https://www.msha.gov/safety-and-health/mine-emergency-operations/mine-emergency-operations/critical-item-checklist-mine-emergencies) | Command center receives, logs and relays communications | Preserve operational event provenance and operator acknowledgment as distinct journal stages | S3, S7 |

## Ranked scenarios and test cases

1. **Single cable severed:** take out one backbone segment. Accept only alternate independently authenticated transport; otherwise hold durable queue. Verify status reports do not claim connected.
2. **Power interruption:** fail both backhaul and selected repeaters. Confirm local device remains usable and persists unsent records through restart; independently evaluate battery and power isolation.
3. **Blind tunnel corner / new working face:** move peer beyond measured link coverage. No guessed position or range; test moving through actual surveyed blackspots and reconnecting.
4. **Asymmetric radio:** one-way packet reachability without durable acknowledgments. Never call a message delivered without receiver-side verified storage receipt.
5. **Mesh bridge failure:** take down a relay and force network partition. Check deduplication, bounded retries, store-and-forward route availability and no impersonation.
6. **Mass rejoin:** dozens of simulated units reconnect. Enforce quotas, jitter, backpressure, retained ordering and prioritized control data over STREAM video.
7. **Damaged command center:** optional server goes offline. Field nodes keep offline tasks; surviving approved peers remain independently usable.
8. **Wrong/counterfeit relay:** forged source labels, compromised discovery and invalid keys are rejected; no downgrade or unauthorized cross-org forwarding.

## Supplier review queue

**Strata Worldwide** is a sourced commercial reference for leaky-feeder and wireless-mesh architectural patterns. Also investigate **MST Global, PBE Group, Rajant, and Becker Mining Systems** for available offerings, technical manuals, interoperability terms and independent results. These four are research candidates, not verified interoperability partners or endorsed products.

## Boundary and validation

Mining environments may require MSHA approvals and intrinsic-safety or explosion-protection controls. Rust-Boy software cannot confer hardware certification. The mine operator, site engineer and applicable regulator must confirm the approved communications plan, emergency role, RF design and safe equipment.

Validate the physical bearer and application protocol separately: an RF link-up event is not authenticated organizational enrollment, an HTTP 200 is not decoded media, and a stored message is not evidence that a person read it. Prioritize synthesized fault injection before underground hardware acceptance.

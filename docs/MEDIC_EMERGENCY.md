# Medic, Doctor and emergency communications

The Medic profile adds **MEDICAL COMMS** to the existing common shell. Vault Doctor also pins this module. Existing saved Doctor workbenches retain their pins until the operator reloads the profile. Every role, including empty Custom, has an orange **SOS** header control. These are development features for paired Rust-Boy devices; the software does not contact a public emergency service or dispatch rescue.

## Setup and nine-line handoff

Pair the intended Doctor and emergency recipient using [the existing SSH enrollment procedure](PEER_TASK_TRANSPORT.md). Pair the reverse direction if the recipient must acknowledge an alarm. A profile or alias is not proof of clinical credentials or a person's identity. Choose the actual Doctor device explicitly in **SOS → SETUP**, then select an emergency destination, manual location and P/A route. Saving configuration never transmits. The destination is one paired device in this pass, not a broadcast/talkgroup.

Medic → MEDICAL COMMS opens a nine-line wizard. Doctor opens its received aid inbox. Nine fields are collected, each 1–48 UTF-8 bytes; UNKNOWN must be entered explicitly for unknown information. The operator selects peacetime or wartime. There is no clinical diagnosis, automatic priority assignment, invented location or default claim of no contamination.

| Line | Peacetime | Wartime |
| --- | --- | --- |
| 1 | Pickup location, including coordinate system/grid zone when applicable | Same |
| 2 | Contact frequency/call sign | Same |
| 3 | Patient counts by operator-assessed precedence | Same |
| 4 | Special equipment | Same |
| 5 | Patient counts by litter/ambulatory type | Same |
| 6 | Number/type of wounds, injury or illness | Pickup site security |
| 7 | Pickup marking method | Same |
| 8 | Patient nationality/status | Same |
| 9 | Terrain/obstacles | NBC/CBRN contamination |

The original implementation is informed by the [U.S. Army nine-line card](https://home.army.mil/stewart/2117/8170/8222/9_Line_Card_BLC_4.0.pdf) and [Army report formats, FM 6-99](https://rdl.train.army.mil/catalog-ws/view/100.ATSC/669B2F5D-20B2-4731-8176-2D432737B71F-1377692027204/fm6_99.pdf). It sends a structured handoff to the chosen Doctor device, not an aviation tasking or accepted evacuation request. Operational applicability, local clinical procedures, security and response organization require separate validation.

Review all nine fields across three pages, then confirm SEND. The typed `RB-AID/1` packet is validated in Rust, queued in the existing Messenger journal, and delivered through pinned SSH. Doctor's INBOX decodes the packet into numbered lines. Raw packets also remain visible in Messenger. Browser drafts are temporary; sent/queued reports are durable. Avoid patient identifiers unless the deployment's privacy and clinical governance explicitly permit them.

## Motorola-inspired emergency button

The design uses a distinct orange control, guarded activation, a latched emergency, explicit acknowledgment indication and deliberate exit. References: [Motorola APX 8000 Model 2 guide](https://www.motorolasolutions.com/content/dam/msi/docs/products/apx/apx8000-model2-userguide.pdf), [APX mobile O2 guide](https://www.motorolasolutions.com/content/dam/msi/docs/support/manuals/lac/astro-25/apx-mobile-o2-user-guide-en-68012006035.pdf), and [MOTOTRBO digital emergency operation](https://support.motorolasolutions.com/s/article/KB0058209). Motorola timings, retry policies and automatic radio acknowledgments vary by model and programming. The Rust-Boy timings below are original choices, not a claim of Motorola interoperability.

1. Hold the orange header SOS control for **2 seconds** to activate directly. Alternatively tap SOS to open the controls, then hold its activation control for **2 seconds**, or tap and confirm. Release, lost pointer, blur or a hidden tab aborts an incomplete hold. Enter/Space supports the same hold; a short key activation opens confirmation.
2. Activation saves a local alarm before one explicit send attempt. A missing destination still latches locally and is labeled **LOCAL ALARM ONLY**. Missing location is **UNKNOWN / NOT PROVIDED**; no GPS request is made.
3. A configured alarm snapshots its destination, selected route and manual location. Reloading or changing profiles preserves it. Editing configuration cannot retarget an addressed incident. An unaddressed active alarm may bind once to a newly configured destination; saving still does not send it. Location remains the original snapshot; cancel/re-activate to create a new incident with an updated location.
4. RETRY ALERT reuses the saved envelope where available. No automatic outbound repetition, route fallback, RF transmission or hot microphone is enabled. P uses paired SSH; A requires the previously configured private-IP alternate. **PACE E is a local hold, not an emergency beacon transmitter**, and is not offered here.
5. Receiver storage proves persistence only. A responder must select **ACKNOWLEDGE** and confirm to send a typed return acknowledgment. The origin accepts an acknowledgment only from that incident's selected peer and matching incident ID. It indicates an operator action on a paired device, not verified identity, clinical acceptance or rescue underway. ACK never cancels the originating latch.
6. Hold CANCEL for **3 seconds**, or tap and confirm. Local cancellation is saved before its own delivery attempt. An undelivered cancellation remains **NOT CONFIRMED**; RETRY CANCEL sends that incident's cancellation. A received cancellation suppresses that incident even if an older alarm arrives later. Creating another incident assigns a new ID.

The header checks the **local** journal every ten seconds while the page is visible, updating SOS WAIT (unconfirmed), SOS RCVD (peer stored), SOS ACK (operator acknowledgment), CANCEL ? (cancellation unconfirmed), or an incoming ALERT count. These reads do not contact peers, fetch remote inboxes or send retries; a running paired receiver must persist messages locally. Opening INBOX/REFRESH loads current records. There is no background OS notification, siren, radio priority/preemption, location tracking, man-down sensor or physical GPIO switch in this pass.

## Persistence and security

`aid-state.json` contains the emergency latch, monotonic local incident counter, destination preferences and manual location. `aid-state.lock` protects atomic updates and `aid-dispatch.lock` serializes aid actions. New Unix directories/files use 0700/0600; native Windows ACL acceptance is separate. Reads do not create files. Unsupported/corrupt state is preserved and activation fails visibly. Do not delete state to clear an alarm or recycle a device identity: incident IDs depend on retaining this state. Physical power-loss guarantees require hardware testing.

Aid reports, alarms, cancellation and acknowledgment packets use the existing bounded `messages.json` journal. Default history and aid configuration are plaintext; optional fresh-journal age/keyring encryption covers messages, **not** `aid-state.json`. This remains SSH encryption in transit, not E2EE. Journal corruption/key unavailability must not conceal the independently stored local latch. There is no automatic pruning: a full journal blocks additional queuing and remains an explicit unconfirmed delivery state. Local browser access and paired SSH keys remain the trust boundaries; profiles are workspace templates, not authorization roles.

All HTTP mutations require the parent loopback Host/Origin/JSON checks and `confirm:true`. Backend schemas reject unknown fields, invalid IDs, excess bytes and unsupported packet types. Malformed aid-looking messages stay ordinary Messenger records and cannot silently create local alarm state.

## Verification and remaining gates

`tools/test_aid_ui.cjs` exercises production browser code against deterministic DOM/timer fixtures. `tools/smoke_aid.py` runs the real Rust API; on POSIX, two independent homes exchange nine-line reports, alarms, human ACK and cancellations through a strictly local fake SSH relay. Offline retries, stale IDs, corrupt state/journal and restart persistence are included. Core Rust tests cover typed packets, byte bounds, frozen destinations and atomic state preservation. `tools/test_aid_viewport.cjs` uses production HTML/CSS/JS in exact-size Chromium frames to check the aid screens at 480×320, 800×480 and desktop, with 44px controls and no outbound actions. Fixtures are invented and do not certify SSH cryptography or real-world response.

Before emergency field use: independently review protocol/privacy and replay assumptions; test actual paired Mac/Pi devices, lost links and power failures; validate browser accessibility/touch layout on the selected display; specify the physical guarded switch, debounce and OS broker; establish receiver staffing/escalation, transport supervision and power monitoring. Live RF/LoRa/Meshtastic integration and periodic beacon transmission remain separately gated work.

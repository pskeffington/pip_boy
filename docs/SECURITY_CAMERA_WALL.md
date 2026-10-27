# Security | DOT-first outside-world camera views

**Status:** DOT camera snapshot candidates are wired. Per-camera live reachability and frame freshness have not been independently certified.

## DOT first, no YouTube

The shared Rust catalog now provides direct official DOT snapshots for Virginia (DC metro fringe, clearly labeled Arlington, VA) and Caltrans (San Francisco). NYC DOT and MassDOT remain access-pending until approved agency/provider feeds are available. The [feed inventory](../catalog/camera_feeders.json) lists eight agency/provider integration candidates, coverage areas, supported media and licensing gates.

| Area | Agency | Current fallback |
| --- | --- | --- |
| DC metro (Arlington, Virginia) | VDOT / 511 Virginia | HTTPS road camera PNG snapshot candidate |
| NYC | NYC DOT TMC | Developer agreement required: placeholder only |
| Boston | MassDOT / TrafficLand | Provider agreement required: placeholder only |
| San Francisco | Caltrans D04 | HTTPS road camera JPEG snapshot candidate |

## Nearest-active selection contract (October 2026)

Security uses the existing universal **Settings → Location / Map**. A manual center is exact; a selected ZIP resolves to a **county-map approximation**, not a precise home or ZIP centroid. After explicit GPS ON, a last known fix may be used for camera ranking for at most 60 seconds in volatile page memory. GPS OFF revokes that hint. Coordinates are never appended to DOT URLs, persisted to disk, sent to publishers as query parameters, or requested a second time by Cameras.

The shared `cameras.js` selector applies Haversine ground distance to the **actual DOT camera coordinates** (not the Northeast city target roster). Only fixed, approved HTTPS snapshot hosts enter its candidate queue. The closest eligible feeds are checked with a maximum of **two simultaneous image probes**, **12 attempts per tool opening**, and **8 seconds per probe**; each probe is cache-busted and then torn down on navigation. One failed source advances to the next nearest candidate. Four working tiles is the wall limit, not a network polling target. The selected primary feed is highlighted and the **FOCUS NEAREST** control opens it; explicit focus/back remains separate from automatic source selection.

Health states are intentionally distinct:

- **LIVE** requires a successful image load **and** agency-provided `active: true`, recent metadata check time, and frame capture time within 120 seconds. No currently seeded static catalog source supplies those timestamp fields, so current candidate images must not be advertised as verified live.
- **REACHABLE · FRESHNESS UNVERIFIED** means an image loaded in this session without sufficient source timestamp evidence. This is a fallback after confirmed-live candidates, not a safety claim.
- **AGENCY INACTIVE / AGENCY FRAME STALE / FEED UNAVAILABLE** are skipped for automatic primary selection. A successful validation expires after 120 seconds; the operator must explicitly refresh to recheck. A timer marks the check expired but does not fetch another image.

No endpoints are polled until Security's Cameras screen opens, and **no background refresh is enabled**. The actual user-facing UI is browser-rendered from Rust `/api/state`; native Pi media decoding and agency-authorized stream integration remain separately gated. `node tools/test_cameras_ui.cjs` covers distance ranking, live preference, offline fallback, source validation, bounded probes and teardown. `node tools/test_map_gps_permissions.cjs` covers shared location consent and revocation.

The [Northeast DOT city roster](../catalog/northeast_camera_roster.json) contains **29 discovery targets**, not 29 verified camera streams. The [provider registry](../catalog/camera_feeders.json) distinguishes public viewing from permission to ingest or redistribute feeds; no API key or subscription was bypassed. NYC, CT, PennDOT and MassDOT integrations require their documented access paths. Camera selection cannot legitimately claim nearby live coverage until individual feeds in those regions pass both access and on-device health gates.

The camera UI fetches snapshots only after its view opens, does not use iframes, and offers an explicit REFRESH control. It does not poll in the background, record, discover local IP cameras or send user location to agencies. An image load is labeled **IMAGE RECEIVED · FRESHNESS UNKNOWN**, not LIVE. Failure is labeled **FEED UNAVAILABLE**. The source URLs are HTTPS and constrained to exact agency-managed media hosts/path patterns; the HTTP CSP now allows only the two media hosts as additional image origins and disallows frames.

**Security meaning:** road activity and visible vehicles can support situational awareness, not certify that a particular person is mobile or that a route is safe. Always preserve camera identity, original agency attribution, capture freshness uncertainty and provider access requirements. Never label Virginia traffic cameras as DC city cameras.

Run `node tools/test_cameras_ui.cjs` for UI regression and `python3 tools/smoke_micro.py` with the local Rust server. These are code acceptance checks; actual provider playback remains an on-device gate.

## User task and screen state

A small, approximately 4-inch handheld has **four independently live feeds** displayed in a **2×2 grid**. Each tile labels source identity, feed age and status (connecting, live, stale, auth failed, offline). One press opens an individual feed in a main focused viewport; one press returns to the wall with the other three streams still tracked. Design target: 800×480 landscape, adaptive 480×320 fallback, and ordinary Mac window/terminal mock before hardware testing. These resolutions are reference targets, not specifications for an approved physical screen.

- **Grid:** 1/2/3/4 selects a feed directly; arrows move focus; Enter (or button/tap) opens it; Esc/Back returns to the grid; Q exits the tool. Selected pane is identified by border and label, not color only.
- **Focused feed:** Full main content area, camera name, timestamp/freshness, error badge, Return/Back hint. Do not silently start audio, PTZ or recording.
- **Bad feeds:** An offline/error tile remains visible and cannot impersonate a current live picture. Retain last frame only when explicitly marked `STALE` with age; default to a clear placeholder on connection failure.
- **Permissions:** Four manually supplied sources, independently authorized. No implicit subnet scan, SSDP, ONVIF discovery, port probing, default password attempts or network access on role selection. Optional ONVIF discovery needs separate operator confirmation and a trusted LAN scope.
- **Credentials:** macOS Keychain or equivalent secure OS credential store; never embed credentials in saved camera URLs, workbench JSON, logs, crash dumps or exports. Validate TLS certificates; do not silently downgrade HTTPS. Keep camera network access inside a reviewed, revocable connector boundary.
- **Privacy:** No background recording, microphone/audio, cloud analytics or uploads by default. Evidence capture/export, PTZ, and any storage retention need distinct UI controls and audit notices.

## Media and camera compatibility

| Phase | Source | Decoder approach | Gate |
| --- | --- | --- | --- |
| Fixture | Four independently timed deterministic color/test feeds | Memory-only renderer; no network | CAMERA-0 |
| First live | Direct HTTPS JPEG snapshots, then MJPEG if needed | Bounded image decode with content-type, payload and frame-rate limits | CAMERA-1 |
| Video | Explicit RTSP camera endpoints; support model-tested H.264/H.265 | Separate reviewed native/OS multimedia backend, not raw terminal escape sequences | CAMERA-2 |
| Interop | ONVIF Profile T manual enrollment and per-model conformance testing | Read-only capability inquiry with separate authorization for any PTZ or motion/event subscriptions | CAMERA-3 |

[ONVIF Profile T](https://www.onvif.org/profiles/profile-t/) is a modern stream/profile candidate; [Profile S](https://www.onvif.org/pressrelease/onvif-to-end-support-for-profile-s/) is being phased out. [RTSP](https://www.rfc-editor.org/rfc/rfc7826) is a session control protocol; video decoders, transport versions, authentication and camera interoperability all require separate tests. Rust-Boy must not claim to support every ONVIF or RTSP device by using those names.

## Camera wall milestone acceptance

**CAMERA-0 — interaction before network:** Four independent mock feeds at 800×480, 480×320, and desktop sizes. Repeated 1–4/Enter/Esc interactions preserve selection. Slow, missing and stale fixtures produce correct age/state. No camera credentials or network calls.

**CAMERA-1 — one authorized JPEG/MJPEG stream:** Operator enters one verified URL outside shell logs and approves a bounded connection. Block redirects beyond the allowlist, private credential leaks, unexpected content types and decompression bombs. Stop closes the handle; no write occurs. Add three more independently controlled sources only after first-source lifecycle is stable.

**CAMERA-2 — four real feeds and focus:** Simultaneous sources show actual frames with separate timestamps; focused view increases detail without disguising other feed disconnections. Test IP-camera models, H.264/H.265 on Mac, two network conditions, credential failures and lost connections. Record per-platform CPU, memory and frame-age latency rather than assuming throughput.

**CAMERA-3 — field operation:** Revoke/stop, sleep/wake, Wi-Fi loss, camera reboot, token/key expiry and hot-unplug all fail closed and recover only with appropriate authorization. Handheld contrast, target size, power and temperature are tested. Optional recording and events remain disabled until independently approved.

## Dependency and non-goals

Requires workbench/role UI, bounded media decode, independently enforced network scope/credentials, an actual GUI/handheld image-capable renderer and source lifecycle management. The existing Ratatui terminal is useful for **fixtures, menus and status**, not a guarantee of smooth four-stream compressed video on arbitrary terminals.

This feature is not a covert surveillance system; no authentication bypass, stealth discovery, hidden recording or automatic exfiltration will be developed.

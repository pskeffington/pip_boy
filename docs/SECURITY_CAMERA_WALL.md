# Security | DOT-first outside-world camera views

**Status:** Northeast camera index expanded; image playback remains access-gated or verification-pending. No Northeast source is currently certified live. The existing Caltrans sample is a direct-snapshot candidate, not a Northeast fallback.

## DOT first, no YouTube

The shared Rust catalog lists provider-owned DOT camera references. Third-party VDOT video use requires an Iteris-managed agreement, even when distribution is free, so its DC-metro candidate is **not automatically fetched**. NYC DOT, MassDOT, CTroads and 511PA also have provider access gates. Caltrans remains an out-of-region snapshot candidate, not a substitute for Northeast coverage. The [feed inventory](../catalog/camera_feeders.json) tracks **14 providers**, their coverage, metadata and media access gates.

| Area | Agency | Current fallback |
| --- | --- | --- |
| DC metro (Arlington, Virginia) | VDOT / 511 Virginia | Media gated pending third-party agreement |
| NYC | NYC DOT TMC | Developer agreement required: placeholder only |
| Boston | MassDOT / TrafficLand | Provider agreement required: placeholder only |
| San Francisco | Caltrans D04 | HTTPS road camera JPEG snapshot candidate |

## Nearest-active selection contract (October 2026)

Security uses the existing universal **Settings → Location / Map**. A manual center is exact; a selected ZIP resolves to a **county-map approximation**, not a precise home or ZIP centroid. After explicit GPS ON, a last known fix may be used for camera ranking for at most 60 seconds in volatile page memory. GPS OFF revokes that hint. Coordinates are never appended to DOT URLs, persisted to disk, sent to publishers as query parameters, or requested a second time by Cameras.

The shared `cameras.js` selector applies Haversine ground distance to **actual DOT camera coordinates**, not city centroids. It requires an existing operator-selected position; without one, it fetches no public camera images. To avoid misleading nationwide fallbacks, automatic selection is limited to **200 miles**. Only fixed, approved HTTPS snapshot hosts within that radius enter its candidate queue. The closest eligible feeds are checked with a maximum of **two simultaneous image probes**, **12 attempts per tool opening**, and **8 seconds per probe**; each probe is cache-busted and then torn down on navigation. One failed source advances to the next nearest candidate. Four working tiles is the wall limit, not a network polling target. The selected primary feed is highlighted and the **FOCUS NEAREST** control opens it; explicit focus/back remains separate from automatic source selection.

Health states are intentionally distinct:

- **LIVE** requires a successful image load **and** agency-provided `active: true`, recent metadata check time, and frame capture time within 120 seconds. No currently seeded static catalog source supplies those timestamp fields, so current candidate images must not be advertised as verified live.
- **REACHABLE · FRESHNESS UNVERIFIED** means an image loaded in this session without sufficient source timestamp evidence. This is a fallback after confirmed-live candidates, not a safety claim.
- **AGENCY INACTIVE / AGENCY FRAME STALE / FEED UNAVAILABLE** are skipped for automatic primary selection. A successful validation expires after 120 seconds; the operator must explicitly refresh to recheck. A timer marks the check expired but does not fetch another image.

No endpoints are polled until Security's Cameras screen opens, and **no background refresh is enabled**. The actual user-facing UI is browser-rendered from Rust `/api/state`; native Pi media decoding and agency-authorized stream integration remain separately gated. `node tools/test_cameras_ui.cjs` covers distance ranking, live preference, offline fallback, source validation, bounded probes and teardown. `node tools/test_map_gps_permissions.cjs` covers shared location consent and revocation.

The [Northeast DOT city roster](../catalog/northeast_camera_roster.json) contains **31 discovery cities** (including Annapolis and Arlington), and [the named-camera sites file](../catalog/northeast_camera_sites.json) now contains **53 specifically identified camera sites** (Maryland 27, Rhode Island 6, Virginia 4, NYC 16). **None of those 53 is marked as verified live or auto-licensed media.** The [provider registry](../catalog/camera_feeders.json) distinguishes public viewing from permission to ingest or redistribute feeds; no API key or subscription was bypassed. NYC, CT, PennDOT and MassDOT integrations require their documented access paths. Camera selection cannot legitimately claim nearby live coverage until individual feeds in those regions pass both access and on-device health gates.

The camera UI fetches approved snapshots only after opening Cameras **and with an eligible selected location**, never on general navigation. It does not use iframes and offers an explicit REFRESH control. It does not poll in the background, record, discover local IP cameras or send user location to agencies. An image load without trusted capture-time metadata is labeled **REACHABLE · FRESHNESS UNVERIFIED**, not LIVE. Failure is labeled **FEED UNAVAILABLE**. The source URLs are HTTPS and constrained to exact agency-managed media hosts/path patterns; the HTTP CSP now allows only the two media hosts as additional image origins and disallows frames.

**Security meaning:** road activity and visible vehicles can support situational awareness, not certify that a particular person is mobile or that a route is safe. Always preserve camera identity, original agency attribution, capture freshness uncertainty and provider access requirements. Never label Virginia traffic cameras as DC city cameras.

Run `node tools/test_cameras_ui.cjs` for UI regression and `python3 tools/smoke_micro.py` with the local Rust server. On a Mac with the dashboard already running on loopback, run `python3 tools/probe_public_cameras.py --json` to check approved direct-image HTTP responses, MIME types, image signatures and optional Last-Modified headers. The one-shot probe has an 8-second timeout, 2 MiB per-image ceiling, maximum 12 entries and no redirects, recording or automatic retries. It reports `reachable_freshness_unknown` on success: even a valid HTTPS image cannot prove live capture without agency frame timestamps. Actual provider playback remains an on-device gate.

## Northeast metadata ingestion pass (review-only)

Three additional official geospatial sources are indexed alongside VDOT, NYC DOT's official named-camera list, and other public state camera lists:

| Provider | Published camera metadata | Video integration |
| --- | --- | --- |
| Maryland iMAP / CHART | FeatureServer layer 0: ID, name, county, latitude, longitude, camera URL | Review media type and rights before enabling |
| DDOT / DC GIS | TrafficOperations FeatureServer layer 2: CameraID, Location, lat/lon, Operation_Status | **No direct video URL** in this GIS layer |
| DelDOT / FirstMap | Traffic Devices CAMERA layer 26: public name, road/intersection, status, location geometry | Test GIS host; no camera media URL in layer |
| RIDOT | Six named cameras on official traveler information page | Public browser viewing confirmed; direct media API and reuse terms not confirmed |

Run an offline importer against a previously obtained official JSON file:

```bash
python3 tools/build_northeast_camera_index.py --source md-imap --input /path/to/md.json --output /tmp/md-camera-review.json
```

Or explicitly fetch **one fixed official public metadata endpoint** (no discovery or media requests):

```bash
python3 tools/build_northeast_camera_index.py --source ddot --fetch --output /tmp/ddot-camera-review.json
```

Supported source keys: `md-imap`, `ddot`, `deldot`, `vdot`. The import caps input at 6 MiB and 1,000 features, validates state-specific camera coordinates and writes **review-only** JSON with `image_url:null`, `live_verified:false`, no retained credentials or video. Output cannot activate a camera automatically. Official metadata/terms and media access must be reviewed before promoting each device to the runtime catalog. `python3 tools/test_build_northeast_camera_index.py` exercises the normalizers offline.

## User task and screen state

A small, approximately 4-inch handheld targets **up to four independently checked feeds** in a **2×2 grid**; restricted or offline cameras display honest placeholders. Each tile labels source identity, feed age and status (connecting, live, stale, auth failed, offline). One press opens an individual feed in a main focused viewport; one press returns to the wall with the other three streams still tracked. Design target: 800×480 landscape, adaptive 480×320 fallback, and ordinary Mac window/terminal mock before hardware testing. These resolutions are reference targets, not specifications for an approved physical screen.

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

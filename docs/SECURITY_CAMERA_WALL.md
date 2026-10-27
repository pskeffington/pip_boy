# Security Chief | 4-camera handheld view specification

**Status: public publisher-player fallback implemented; private IP-camera decoding/enrollment remains planned.** Applies to the [Security Chief](ROLE_ROADMAP.md#security-security-chief) role. Use only cameras and networks the operator is authorized to view. Not a surveillance discovery product.

## Public views when private cameras are unconfigured

Security uses the shared `cameras.js` component and Rust `public_cameras::PUBLIC_CAMERA_FEEDS` catalog. Desktop opens four muted embedded publisher players only on the active Cameras view; role preview/loading does not create players. The small handheld lists four locations and opens one publisher player on selection. Leaving Cameras, opening Radio, or returning to the grid removes the old frames. No background recording, server-side video proxy, discovery, credentials, scraping, or health polling is implemented.

| City | Publisher / view | Source checked 2026-10-26 |
| --- | --- | --- |
| DC | earthTV / White House | [Publisher stream](https://www.youtube.com/watch?v=5p_KGD4fJZs) |
| NYC | EarthCam / Times Square Crossroads | [Publisher stream](https://www.youtube.com/watch?v=z-jYdOIKcTQ) |
| Boston | Boston and Maine Live / Hyatt Regency Boston Harbor | [Publisher stream](https://www.youtube.com/watch?v=3I_cIS24AfY) |
| SF | ABC7 News Bay Area / skyline | [Publisher stream](https://www.youtube.com/watch?v=G8RIAgPxaMc) |

These are visibly labeled **PUBLIC VIEWS · PRIVATE CAMERAS UNCONFIGURED**. Source discovery is not a guarantee of current live playback: publishers may stop/replace a stream, limit embedding, require interaction, or display advertising. The original publisher link always remains available. Rust-Boy does not label an iframe load as verified live or invent timestamps, connection counts or private-camera health. Fixed YouTube IDs and exact publisher watch URLs are validated before creating a frame; only `https://www.youtube-nocookie.com` is added to parent `frame-src`. Parent `connect-src` remains unchanged. Each iframe is sandboxed, starts muted, and uses its own origin referrer policy for provider playback compatibility. Third-party players can make their own network requests once opened.

`node tools/test_cameras_ui.cjs` checks four-view/focus behavior, compact opt-in, muted fixed-origin frames, malformed-source rejection and teardown with synthetic metadata. CI does not certify external playback, publisher permissions, private camera interoperability or physical handheld performance.

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

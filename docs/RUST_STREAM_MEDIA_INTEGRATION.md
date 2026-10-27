# Rust Stream / Media integration

Canonical reusable module: https://github.com/pskeffington/rust_stream

The operator's media area should be available independent of role and organize authorized feeds into SECURITY, NEWS, TRAFFIC and CUSTOM. The Security-role extension is not the permanent owner of the underlying streaming capability. Feed access is still governed by role and organizational policy.

## Existing implementation

The current `crates/pipboy/assets/cameras.js` mounts a browser quad-view with four YouTube embed players populated by `pipboy-core/src/public_cameras.rs`. This is a legacy demonstration, not the desired direct-IP video subsystem. Keep it unchanged until the native GStreamer adapter, controls and tests pass.

## Target workflow

1. Select MEDIA and then Security, News or Traffic.
2. Show up to four independently managed feed tiles, with short status and a single tap to focus.
3. Back returns to grid. No scroll bars or source entry on handheld media screen.
4. Global Settings > Media Feeds owns adding, approval, grouping, ordering, source validation and credentials.
5. Universal location service powers traffic feed selection: nearest *verified active* direct DOT source, then fallback to approved still imagery or explicit offline state.
6. Authorized vault cameras cannot be exposed to other roles; source URLs and credentials must never appear in unrestricted API state.
7. Do not launch external media players or redirect to YouTube in the final media backend.
8. Prioritize emergency communications and navigation over decoding under resource pressure.

## Integration gates

- [ ] Vendor/reference rust-stream-core and pin git commit.
- [ ] Native decoder worker + connection-time network policy enforced.
- [ ] Shared MEDIA view mounted for desktop and micro.
- [ ] Security permission checks and settings.
- [ ] News direct streaming source registry and copyright/terms review.
- [ ] Traffic direct-DOT catalog with health TTL and location ranking.
- [ ] Multi-feed acceptance tests on macOS and Raspberry Pi.
- [ ] Remove YouTube legacy embeds only after passing regression tests.

Architecture and decision matrix: https://github.com/pskeffington/rust_stream/blob/main/docs/RUST_BOY_MEDIA_INTEGRATION.md

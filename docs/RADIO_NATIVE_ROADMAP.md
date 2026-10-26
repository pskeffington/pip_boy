# Native Rust RADIO — phased build and evidence matrix

Status (2026-10-26): **R1 controller implemented in Rust, compile/CI acceptance pending; live IP playback and real SDR not yet implemented.** No external VLC, ffplay, browser or OS handler is launched from RADIO.

## Research-to-design matrix

| Source | Finding | Rust-Boy design decision | Gate |
| --- | --- | --- | --- |
| https://docs.rs/cpal/latest/cpal/ | Cross-platform native output via OS host audio APIs | CPAL owns audio device callback; no network/decoder work in callback | R3 |
| https://docs.rs/symphonia/latest/symphonia/ | Pure Rust decoding with optional MP3/AAC codecs | Enable explicitly selected codecs; decode on worker | R2–R3 |
| https://github.com/RustAudio/rodio | High-level API built atop CPAL/Symphonia; ongoing engine changes | Prefer direct CPAL ownership for long-lived device control; evaluate Rodio in separate spike | R3 |
| https://github.com/samuelb/whirr | Real internet-radio reference: HTTP + ICY + bounded queue + decoder + sink + generation cancellation | Adopt pattern independently, no third-party code imported | R2–R3 |
| https://docs.rs/soapysdr/latest/soapysdr/ | Enumerates multiple SDR receiver devices and supports IQ streams | Optional feature-gated receiver driver; never transmit | R5 |
| https://www.skylabradio.com/listen | Skylab Radio UK official listen page | Curated station with publisher provenance; recheck stream at runtime | R2 |

## Common object model

```text
Micro/Desktop UI (only commands, no decoding)
  -> RadioController (PlayInternet/PlaySdr/Stop/SetVolume)
       -> generation-tagged RadioSession (Stopped/Connecting/Buffering/Playing/Recovering/Failed)
       -> Receiver factory (IP receiver or SDR receiver)
       -> bounded PCM frames, sample clock & rate converter
       -> CPAL native output (Mac CoreAudio / Linux ALSA / Windows WASAPI)
```

Only the worker can emit playback status. A successful connection must never be mislabeled as PLAYING. Any switch/stop advances the generation number, invalidating delayed events; the worker contract must also actually cancel outstanding I/O and close audio handles.

## Phased build

**R1 / Native core — FIRST PASS**: Remove subprocess launching. Add `radio_controller` with a source-neutral session lifecycle, explicit user commands, volume bounds, and stale-worker event filtering. Unit tests require valid station selection and events before PLAYING. **Acceptance**: no external player path; cargo test/fmt/clippy pass.

**R2 / Native IP ingest**: Add optional, vetted `reqwest` TLS HTTP client; bounded redirect allowlist and response headers; official M3U parsing (small playlist limits, no localhost/private-IP redirects); ICY metadata separator; bounded byte channel; timeouts and cancellation; local HTTP fixtures. Actual published stream endpoints may change. **Acceptance**: valid/invalid/slow/endless/malformed playlist and ICY tests; no SSRF via catalog/redirects.

**R3 / Native decode and sound**: Symphonia MP3/AAC; channel/rate conversion; CPAL native output device with bounded PCM ring buffer, underflow silence, no allocation/network I/O in callback, volume limiting, user-driven Play/Stop. **Acceptance**: actual audible Skylab and SomaFM field tests on macOS; cross-platform CI and Pi audio acceptance.

**R4 / Micro RADIO**: Station list, source selector, status, Play/Stop, large touch controls, volume, no page scroll at 480×320; explicit network permission and no autoplay. Commands call local Rust radio manager, not URL navigation. **Acceptance**: browser UI tests, on-device touch, stop during reconnect.

**R5 / Hardware SDR**: Optional SoapySDR enumeration feature; explicit owned USB device selection and receive-only IQ; sample rate and frequency validation, detach/reattach. **Acceptance**: real RTL-SDR plus another supported SDR on Pi and Mac; no TX capability.

**R6 / Universal DSP & fallback**: AM/NFM/WFM demod, filtering, squelch, frequency correction, curated IP failover when no SDR, user preference persistence; preserve offline semantics. **Acceptance**: deterministic IQ fixtures + device failover and network fault injection.

**R7 / Release**: code audit, audio stream permissions/terms, battery/CPU measurements, telemetry privacy, retry budgets, output routing, CI matrix, packaging.

## Rules

- Rust-Boy owns the audio lifecycle; **no external player or browser launches**.
- No background probe or stream load on importing modules; entering RADIO may query *local* device presence; IP checks and playback need user permission/policy.
- One active session; stop/replace must cancel previous work and drop stale generations.
- Keep local PCM buffering short and bounded, with no unauthorized recording or redistribution.
- Station list is curated but not guaranteed reachable; track provenance and last actual stream health.
- Source selector must not mistake simulated hardware for a physical SDR.

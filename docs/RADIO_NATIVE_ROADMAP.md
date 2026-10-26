# Native Rust RADIO — phased build and evidence matrix

Status (2026-10-26): **R1/R2 passed five-job CI; R3 native playback code committed, pending CI and real-device acceptance. Physical SDR and micro-screen RADIO controls are not implemented.** No external VLC, ffplay, browser or OS handler is launched from RADIO.

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

**R2 / Native IP ingest (implementation under review)**: Added a pinned `reqwest` blocking Rustls client for explicit short-lived stream probes; publisher-bound HTTPS endpoint validation, redirects disabled in the client, bounded M3U parser and redirect resolution in core, and ICY audio/metadata demux. Tests use in-memory fixture responses. **Outstanding**: review/commit Cargo.lock, Cargo/CI, live endpoint checks, byte-channel backpressure, cancellation and idle-timeout semantics for long-lived streaming. The total-deadline probe client MUST NOT be reused as a continuous playback worker.

**R3 / Native decode and sound (implementation staged)**: Symphonia MP3/AAC decoding on a dedicated worker; separate no-seek ICY audio source; pure PCM channel mapping and streaming resampler; CPAL hardware sink backed by a two-second lock-free float ring. Native output callback uses no network I/O, locks or allocations; volume via atomic 0–100% control; session stop uses bounded blocking HTTP read timeout. **Acceptance pending**: all five CI jobs, actual audible Skylab/SomaFM field tests on macOS, and Raspberry Pi speaker/touch integration.

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

## R2 implementation evidence

- `crates/pipboy-core/src/radio_stream.rs`: `ApprovedEndpoint` (station origin allowlist), `RadioHttp` contract, M3U resolution bounded at 16 KiB/16 entries, maximum five request hops, ICY audio separation with at most 4080 bytes of metadata per record and maximum 8192 audio bytes per read. Strict HTTPS is intentional; HTTP-only playlist entries are rejected rather than silently downgraded.
- `crates/pipboy/src/radio_http.rs`: native Rustls HTTPS request client (`reqwest` pinned) with disabled redirects and proxies, verified TLS, 5-second connect timeout, 18-second total deadline for on-demand probes. Native HTTP handling never launches a subprocess.
- `pipboy radio probe skylab --confirm`: explicitly requests a small audio byte sample. This is **not playback**, format verification or proof of continuous reception; additional user permission is required to listen.
- `cargo test -p pipboy-core radio_stream`: offline unit fixtures cover provider allowlist, playlist limits, redirect rejection, ICY metadata and truncation.
- Future sustained audio needs a worker with read-inactivity timeout, bounded audio ring, cancellation and a CPAL sink. No audio is recorded, stored or redistributed by the probe.

## R3 native audio pass

- `crates/pipboy/src/radio_playback.rs`: `NativeRadioEngine` wraps `RadioController`, manages a single user-initiated internet station session, controls cancellation/volume, and dispatches decoding on a named worker. Emits PLAYING only after CPAL starts and consumes nonempty PCM; does not interpret a successful TCP connection as audible output.
- `crates/pipboy/src/radio_output.rs`: CPAL stream with default OS audio device, F32/I16/U16 sample format support, bounded two-second lock-free ring and silence on underrun. On Linux the build needs the ALSA development package (CI explicitly installs it).
- `crates/pipboy-core/src/radio_pcm.rs`: deterministic f32 PCM mapping and streaming linear sample-rate conversion to device rate, with packet-boundary, cancellation and non-finite fixture tests. Linear interpolation is a baseline; high-fidelity band-limited resampling is a later quality gate. Only the first two input channels are used; extra output channels receive silence.
- `crates/pipboy/src/radio_net.rs`: a dedicated Tokio/reqwest async HTTPS worker implements explicit 8-second initial connection and 3-second per-chunk inactivity deadlines, plus bounded 16 x 8-KiB compressed-byte backpressure; normal live playback has no total lifespan deadline. An OS-independent `ChunkReader` exposes bytes to Symphonia; Stop cancels both workers and the network guard joins. `radio_http.rs` remains for short-lived explicit R2 probe commands. On network failure the session reports FAILED; automatic reconnection remains R6.
- CLI: `pipboy radio listen skylab --confirm --seconds 30` starts native playback for 30 seconds; interactive `pipboy radio listen skylab --confirm` accepts `status`, `volume N`, `stop`. No VLC, FFplay, browsers, media subprocesses or durable audio storage.
- Required external acceptance: OS speaker/headphone hardware, actual station stream availability, audio routing, underruns and network-loss behavior; CI without audio hardware verifies compile and pure tests, not audibility.

## R4 / Integrated HUD radio (2026-10-26)

- Dedicated RADIO panel in micro LINK and desktop dashboard, both using the same native Rust singleton playback session.
- Same-origin `GET /api/radio` (curated directory, state, volume and errors) and `POST /api/radio` for `play` with explicit confirmation, `stop`, and bounded `volume` (0–100).
- The browser contains only UI; sound remains in Rust through Symphonia/CPAL. No `<audio>` element, stream URL navigation, or external player process.
- Polling cannot initiate playback; switching while playing requires STOP first. Session survives navigation between micro/desktop tabs until explicitly stopped or server exits.
- Accessibility: minimum 40-pixel touch targets, keyboard focus states, high-contrast status, named buttons and readable failure messages.
- Acceptance still required: CI + real Mac audio test, 480×320 screenshot and touch validation, and recovery handling for disconnected devices. The currently mounted panel is a HUD foundation, not an SDR hardware receiver.

# Native Rust RADIO — phased build and evidence matrix

Status (2026-10-26): **R1–R4 HUD integration and native Skylab playback field-validated by the operator; Play/Stop, volume and OS/Bluetooth output selection working.** CI and device-change edge cases are tracked separately. Hardware SDR, recovery hardening and final micro touch certification remain open. No external media application is launched.

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

**R3 / Native decode and sound (implemented; Mac field-confirmed)**: Symphonia MP3/AAC decoding on a dedicated worker; separate no-seek ICY audio source; pure PCM channel mapping and streaming resampler; CPAL hardware sink backed by a two-second lock-free float ring. Native output callback uses no network I/O, locks or allocations; volume via atomic 0–100% control; session stop cancels a bounded asynchronous HTTPS receiver. **Acceptance pending**: all five CI jobs, actual audible Skylab/SomaFM field tests on macOS, and Raspberry Pi speaker/touch integration.

**R4 / Micro RADIO (implemented; user field-confirmed)**: Station list, source selector, status, Play/Stop, large touch controls, volume, no page scroll at 480×320; explicit network permission and no autoplay. Commands call local Rust radio manager, not URL navigation. **Acceptance**: browser UI tests, on-device touch, stop during reconnect.

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

## Native output routing (Bluetooth-ready)

RADIO displays an **AUDIO OUTPUT** selector listing CPAL's OS-visible outputs alongside **System default**. A Bluetooth headset or speaker appears after it has been paired and connected in the operating system; Rust-Boy does not initiate Bluetooth pairing, perform unauthorized scans, or claim control of device pairing. Selecting an output explicitly restarts the current station at the prior volume on the new native output. If the selected device disconnects, the error is exposed in RADIO; choose System default or a connected device to recover. The selector is also available when playback is stopped. User has confirmed working device-output controls, including Bluetooth routing. Automatic recovery after a device disappears is a separate acceptance gate.

## Operational roadmap — following Skylab + Bluetooth acceptance

| Pass | Priority | Work package | Concrete code/actions | Release gate |
| --- | --- | --- | --- | --- |
| R4.1 | P0 | Capture working baseline | Preserve publisher allowlist, current HUD APIs, device routing, and the successful Skylab/Bluetooth operator report; run five-job CI on the exact commit | All current Rust, JS, desktop and micro tests green; no regressions |
| R4.2 | P0 | Recovery contract | Pure classified failure/retry policy with a fixed attempt budget; keep Stop and generation invalidation authoritative; deterministic tests for temporary network failure, permanent unsupported codec, Bluetooth unplug and cancel | No infinite retries, no unapproved host redirects, no reconnect after Stop |
| R4.3 | P0 | Network reconnect implementation | Worker-owned bounded retries with interruptible 1s/2s/4s delays; re-establish an approved stream only; explicitly surface RECOVERING, attempt count and final FAILED; reconnection stops instantly on user Stop | Fault-injection fixture: bounded recovery, no duplicate CPAL streams, clean Stop |
| R4.4 | P0 | Output-device resilience | Distinguish selected output disappearing vs. network outage; show DISCONNECTED and refresh output inventory; offer manual Default restore before any automatic switching | Disconnect Bluetooth during stream and recover on a selected, OS-present output |
| R4.5 | P1 | Presets and favorites | Persist up to 12 trusted station IDs (not arbitrary URLs); user ordering; startup shows last-selected station but never autoplays; stations validated against curated directory on read | Restart app: user presets persist, no untrusted endpoints/autoplay |
| R4.6 | P1 | Micro refinement | Fit RADIO at 480×320 with Play/Stop, station, output, small retry status and volume controls; keyboard and touch; avoid forced scrolling and misleading signal strength | Real hardware screenshots, mouse/touch/keyboard acceptance at 480×320 |
| R4.7 | P1 | Quality and observability | Bounded PCM underrun counters, conservative bitrate/read metrics, stream uptime and device identity; no private network addresses in logging | Offline fixtures plus CPU/memory/battery field sampling |
| R5 | P2 | Receive-only SDR adapters | SoapySDR/RTL-SDR optional backend, hardware enumeration, FM demod and source selection; IP fallback stays available | Two real receiver types and hardware detach; zero TX capabilities |
| R6 | P2 | Advanced receiver modes | AM/NFM/WFM, squelch, filtering and fully documented source selection; no silent internet usage | IQ fixtures, RF bench and poor-network tests |
| R7 | P2 | Release acceptance | Security and licensing reviews, accessibility and privacy audit, packaging, signed binaries, release checklist | All platform CI + real Mac and handheld speaker/Bluetooth acceptance |

**Ordering:** R4.1 → R4.2 → R4.3 → R4.4; station presets and layout follow only after safe recovery. User-confirmed functions must not be replaced by nonworking UI scaffolds.

**Product constraints:** Native Rust, OS-managed Bluetooth pairing, source HTTPS allowlisting, no external media application, no autoplay, bounded retry, no radio transmission, and no merge to `main` while the current PR is draft or CI has failing gates.

**Acceptance evidence:** Operator field report (Skylab Play/Stop/Volume and Bluetooth output working on Mac, 2026-10-26). This is distinct from formal fault-injection recovery tests and future physical SDR verification.

**R4.2 first implementation pass:** `pipboy-core/src/radio_recovery.rs` now provides typed failure classifications (transient network, lost output, permanent), strict retry budgeting (1s, 2s, 4s; maximum three retries) and Stop dominance. Offline tests cover exhaustion, output/manual recovery, permanent failures and Stop. This is a **policy implementation only**—the active live streaming worker does not yet schedule the retries. Wired recovery and fault-injection remain R4.3 gates. The prior CI run revealed stopped-state RADIO JSON omitted `output` and `outputs`; fixed on feature branch with regression assertion.

## R4.3 in-stream reconnect implementation pass

- The existing native decoder worker now wraps each stream in an explicit bounded reconnect loop. On an actual in-stream `symphonia::Error::IoError` it closes the prior decoder/output/HTTPS stream, moves the session to RECOVERING, and waits 1s, 2s and 4s before attempting a fresh HTTPS stream using the existing curated source allowlist.
- Each attempt builds a fresh native PCM sink and compressed-byte reader; the parent session remains live while individual network guards are dropped. Session Stop terminates retries and stale status events are rejected by the generation state machine.
- `radio-ui.js` now describes RECOVERING as a broadcaster reconnect, not as active PCM playback. Deterministic tests cover the reconnect transition, retry budget and cancelling a pending wait.
- **Remaining limits:** this pass retries only decoded in-stream I/O loss; initial HTTP connect failures and non-I/O demux/codec errors fail rather than retry. Device disappearance still requires explicit output selection. End-to-end fault injection and CI acceptance are gates before merging.

## R4.4 audio-device hotplug and manual recovery (implementation pass)

The RADIO HUD now reports `output_available` separately from `output_failure` and shows a disabled, clearly marked missing selection when Bluetooth/wired output disappears. The **USE DEFAULT OUTPUT** control only enables if the OS reports an available system default. Choosing an output explicitly restarts the previous station at its prior volume when the former session failed *because of audio output*, but changing speakers does not autoplay a stopped or unrelated codec-failed session.

CPAL output IDs are now bounded to the enumerated list index **and a deterministic fingerprint of the observed device name**. If Bluetooth reconnect or device enumeration changes the index/name, Rust-Boy refuses the old ID rather than silently routing to an unrelated speaker. These identifiers are **process-session IDs, not permanent Bluetooth identities**; reselect after hotplug. The decoder worker emits a typed output failure when the native callback fails or stops draining the bounded PCM queue; output loss is not classified as a transient broadcaster error. A five-second stalled-output guard prevents indefinite queue blockage.

**Offline acceptance automation**: `node tools/test_radio_ui.cjs` uses a simulated HUD state sequence (explicit play → Bluetooth selection → device disappearance → manual system fallback → RECOVERING → Stop → no OS speaker available). Rust unit tests cover typed network/output failure separation, stop and retry policy, stable name-bound IDs, and when output selection may resume a session. These are controlled fixtures, **not** physical Bluetooth detach tests.

### Operator fault-injection acceptance checklist (remaining physical gate)

- [ ] While Skylab plays through Bluetooth, disconnect the headset/speaker from the OS. Verify the HUD reports unavailable or failed output, does **not** silently change outputs, and STOP remains responsive.
- [ ] Reconnect it, refresh the device dropdown, explicitly reselect the connected output, and confirm restored sound at the previous radio volume.
- [ ] Disconnect Bluetooth again and choose **USE DEFAULT OUTPUT**. Verify audio returns to Mac speakers if the OS reports a valid default. If none exists, the fallback action must be disabled.
- [ ] Temporarily disable internet during an already-playing stream. Confirm RECOVERING with bounded retry, then either restored audio or FAILED after exhausting three attempts.
- [ ] Press STOP during a pending retry. Verify no later restart/audio, including after IP connectivity returns.
- [ ] Re-test no autoplay on opening the RADIO page and no outside media-player processes.

**Known limits:** CPAL may not always report device loss immediately; the PCM queue stall guard detects a non-draining callback only once queue pressure occurs. No pairing UI, universal OS device UID, automatic audio fallback, or physical SDR is claimed.

# RADIO/SDR — hardware-independent receiver foundation

## Status
**Native IP playback and the simulated receive-only SDR contract are integrated into `main` (PR #35); live physical SDR reception remains unimplemented.**
This is a receive-only Rust core intended for the existing Rust-Boy micro UI and future Raspberry Pi SDR accessory. It does not modify the LINK/SSH messenger, transmit RF, or silently discover/open devices.

## Existing API
- `RadioDevice` trait: report device capabilities, apply bounded tuning, read interleaved signed IQ.
- `RadioTuner<D>`: generic tuner object; validate frequency, exact supported sample rate and optional gain *before* invoking the driver.
- `Tuning`: frequency Hz (integer, no float rounding), sample rate Hz, demodulation metadata and optional gain in tenths of dB.
- `RadioCapabilities`: hardware-reported ranges, rates and supported manual gain span.
- `SimulatedRadio`: a fixed test fixture that emits zero-valued IQ and is always marked simulated. It does not receive radio waves.

Demodulation is a **requested mode label** at this stage, not a working AM/FM/SSB audio decoder. A future DSP component should take raw IQ and implement properly tested filtering, resampling, squelch, gain control and audio output. A successful `tune` means only that the backend accepted settings, **not** that a signal was present or intelligible.

## Command-line development checks
```bash
cargo run -p pipboy -- radio capabilities
cargo run -p pipboy -- radio simulate 100100000 wfm
cargo test -p pipboy-core radio
```

## Hardware integration pass (next)
1. Introduce an optional `SoapySDR` receive backend behind a feature flag for supported hardware (RTL-SDR, Airspy, HackRF RX, LimeSDR RX, etc., depending on installed Soapy drivers). Avoid linking mandatory native SDR libraries into the default build.
2. Enumerate device identifiers and report actual frequency ranges, sample rates, bandwidth and gain ranges. Never assume all dongles have the simulator's range or rates. Require explicit user selection and access authorization.
3. A bounded capture worker owns the device; separate it from the UI and message transport. Introduce stop/cancel, stream timeouts, bounded buffer/backpressure, unplug/replug recovery and event logs.
4. Build DSP pipeline: input samples -> complex baseband -> AM/NFM/WFM/SSB decoder -> resampler -> audio device. Bandwidth, gain and frequency correction must be device-aware.
5. Add spectrum waterfall/level meter, saved manual frequencies, band-plan presets and channel stepping for a 480×320 touch interface. Only manual tuning starts receiver I/O.
6. Validate on macOS, Linux/ARM64 Raspberry Pi and Windows with mock tests plus real-device field tests. Record hardware, driver, calibration, device access and clipping/overrun evidence.

## Permissions and safety
- The radio core is receive-only. Transmit, replay, injection and active interrogation are not in this interface.
- Device permissions are requested only by a future driver backend. Preference flags alone never grant OS permissions.
- Signal decoding/recording and storage will need per-operation user consent, bounded retention and jurisdiction-specific review. No automatic capture of private communications.
- Keep actual RF radio under the RADIO label; do not describe SSH, Meshtastic capture fixtures or LINK peer tasks as a working SDR.

## Test and acceptance gates
- Invalid frequency/rate/gain rejected before the driver is called.
- `RadioTuner` reflects new settings only after driver success.
- Invalid IQ buffer length or backend sample count rejected.
- No device opened by constructing a tuner or loading a role.
- Simulator never represented as real RF.
- Real hardware adapter, DSP, audio, waterfall and micro HUD integration remain separate gates.

## IP radio / curated directory (2026-10-26)

The native Rust/Symphonia/CPAL player is integrated in `main` (PR #35); Skylab sound, Play/Stop, volume and OS-managed Bluetooth output selection were operator-confirmed on macOS. Physical SDR enumeration, RF demodulation, Raspberry Pi hardware acceptance and full reconnect/fault validation remain unimplemented or pending. Opening the Radio HUD reads only the local state; **no stream begins until Play is pressed**.

The fixed 10-station directory contains Skylab Radio (UK), plus nine SomaFM channels: Groove Salad, Secret Agent, Lush, Drone Zone, Deep Space One, Space Station Soma, Beat Blender, DEF CON Radio and Underground 80s. Categories are CHILL, AMBIENT, ELECTRONIC and RETRO. The desktop list is genre-grouped; the handheld uses the same catalog with previous/next buttons and a station counter. The output picker is collapsible on handheld and opens automatically when audio hardware fails.

SomaFM publishes permanent MP3 M3U playlist endpoints and notes that **streams are for individual personal listening**, not relaying, rebroadcasting, embedding in video games or redistribution without authorization. Do not present these entries as licensed for a public/game distribution. The directory and license conditions must be reviewed before packaging. Publisher references are retained per entry in `radio_directory.rs`; their mere presence is not runtime stream verification.

Publisher channel pages:
- [Groove Salad](https://somafm.com/groovesalad/directstreamlinks.html), [Secret Agent](https://somafm.com/secretagent/directstreamlinks.html), [Lush](https://somafm.com/lush/directstreamlinks.html)
- [Drone Zone](https://somafm.com/dronezone/directstreamlinks.html), [Deep Space One](https://somafm.com/deepspaceone/directstreamlinks.html), [Space Station Soma](https://somafm.com/spacestation/directstreamlinks.html)
- [Beat Blender](https://somafm.com/beatblender/directstreamlinks.html), [DEF CON Radio](https://somafm.com/defcon/directstreamlinks.html), [Underground 80s](https://somafm.com/u80s/directstreamlinks.html)
- [Skylab Radio](https://www.skylabradio.com/listen)

```bash
cargo run --locked -p pipboy -- radio stations
cargo run --locked -p pipboy -- radio stations --check-ip
cargo run --locked -p pipboy -- radio probe skylab --confirm
cargo run --locked -p pipboy -- radio listen skylab --confirm --seconds 30
node tools/test_radio_ui.cjs
```

`stations` is a network-free listing. `stations --check-ip` is an explicit TCP reachability probe, not evidence of decodable audio; `probe` checks bytes at an approved HTTPS endpoint only. Live playback on new channels and actual 480x320 physical touch/scroll acceptance have not yet been proven. No arbitrary URL input or automatic connection is permitted; the runtime TLS/redirect allowlist remains authoritative.


## R4.5 / native favorites and last station (2026-10-26)

The latest RADIO branch stores preferences under `PIPBOY_HOME/radio-presets.json` (or `~/.pipboy/radio-presets.json`). **This is local preference data, not a stream cache.** It contains a schema-versioned `favorites` list and `last_selected` station ID only. No IP addresses, arbitrary URLs, browsing history, audio data, or device identifiers are persisted. The missing file means no favorites; a GET creates no file or directory.

- Tap **☆/★** to add or remove an approved station from favorites (maximum 12); **ALL / ★ ONLY** filters the list without streaming.
- **Previous/Next** cycles the current visible list. Under **SETTINGS**, the *FAVORITE ORDER* earlier/later controls update the order of curated favorite IDs; audio outputs and Bluetooth device recovery remain there.
- Selection is saved by a same-origin `POST /api/radio` command with `{"action":"select","station":"skylab"}`. Adding/removing a favorite uses `{"action":"favorite","station":"skylab","enabled":true}`; ordering uses `{"action":"move_favorite","station":"skylab","direction":"earlier"}`. None of these commands start a network stream.
- On restart the UI highlights the last explicitly selected station, **without autoplay**. The `play` command still requires `confirm:true`, independently updates the last station, and remains the only action that opens the native decoder/stream worker.
- Read/write validation rejects unknown IDs, duplicates, unknown JSON keys, malformed versions, oversize files and symlinked preference files. Updates use an OS lock, bounded JSON, restrictive Unix permissions and atomic staged replacement. A corrupt existing file is preserved and reported as a 409; it is never silently reset.
- On the 480×320 HUD the Play/Stop and volume rows share horizontal space, while **SETTINGS** opens a compact overlay; the default RADIO view does not need a list scrollbar. Physical Pi display and touch screenshots remain a separate acceptance gate.

Local no-device checks: `cargo test --locked -p pipboy-core radio_presets`, `cargo test --locked -p pipboy radio_hud_endpoints`, `node tools/test_radio_ui.cjs`.

## Native playback implementation policy

The previous external-player CLI experiment has been removed. The `radio_controller` module now models user-driven session state, volume and generation-specific events in-process. It is a control foundation, **not** a functioning player. Live HTTP download, Symphonia decode, CPAL playback and micro UI are explicit following milestones. See [research, object model and phase gates](RADIO_NATIVE_ROADMAP.md).

## R2 native stream ingress (implementation pending CI)

R2 adds `radio_stream` and `radio_http`: controlled, in-process HTTPS response acquisition, publisher-allowlisted playlist URLs, bounded M3U parsing and ICY metadata stripping. The short-lived probe reads at most 8192 audio bytes and discards them. It does not initiate a player, persist audio or declare audible output.

```bash
cargo run --locked -p pipboy -- radio probe skylab --confirm
cargo run --locked -p pipboy -- radio probe groovesalad --confirm
cargo test --locked -p pipboy-core radio_stream
```

Only `probe` contacts a published stream; `stations` remains offline. Probe uses a total timeout and is explicitly unsuitable for continuous listening. Some broadcaster playlists may list HTTP endpoints; the first-stage HTTPS-only resolver deliberately rejects those entries. Real endpoint, Cargo.lock, licensing and all-platform build verification are release gates. [Phased native roadmap](RADIO_NATIVE_ROADMAP.md).

## Native R3 playback (software implementation under CI review)

The new R3 engine decodes streaming MP3 or AAC with Symphonia and writes converted PCM into a bounded native CPAL sink, entirely within Rust-Boy. The IP receiver does not launch VLC/FFplay or a browser and does not save broadcast audio. A real sound device and broadcaster connection are required to hear sound. The 480x320 in-shell RADIO widget is an R4 gate.

```bash
# Short, bounded native output test:
cargo run --locked -p pipboy -- radio listen skylab --confirm --seconds 30
# Interactive on terminal; type status, volume 0..100, or stop:
cargo run --locked -p pipboy -- radio listen skylab --confirm
# Pure PCM conversion tests without hardware:
cargo test --locked -p pipboy-core radio_pcm
```

Playback begins only following a user command. Stop invalidates the radio session generation and requests worker cancellation; the HTTP stream has a finite per-read timeout, so stop can wait for an in-flight network read. A native ALSA audio output or equivalent macOS/Windows audio device is essential. Do not report playing until the native PCM sink consumes actual samples. Live station reliability, automatic reconnect, native device volume mixing and SDR hardware acceptance remain open. See [R1–R7 plan](RADIO_NATIVE_ROADMAP.md).

## Standalone module integration

The host now consumes the separately maintained `rust-radio` library at the Git revision pinned in the root workspace dependencies. Core compatibility modules re-export its shared control, directory, playlist, PCM, recovery and receive-only tuner APIs; native adapters re-export its transport, output and playback engine. The standalone egui panel is not linked into the HTML HUD. Host favorites and last station remain in `radio-presets.json` with the existing schema.

`radio validate STATION --confirm` invokes the module's bounded, silent MP3/AAC validator. The HUD's `play` action also accepts an optional initial `volume` from 0 through 100; omitted values retain the previous default of 60. Invalid volume is rejected before audio/network resources or preference writes. An explicit IP Play still opens the only native engine; merely opening the host or enumerating stations never plays audio.

This cutover brings the shared bounded retries, ICY cancellation fix and HTTPS-only SomaFM playlist compatibility into the host. Earlier entries describing these native implementations as pending are historical. Physical SDR reception and demodulation remain hardware acceptance gates; the receive-only tuner contract and marked simulator are shared through the module.

On 2026-10-27 the integrated loopback HUD was executed with temporary preferences and low initial volume. Skylab and Groove Salad both reached native Playing after the PCM callback consumed decoded samples. The test changed volume, rejected duplicate Play, stopped before switching, preserved favorites/last selection, and verified no autoplay on reads or output selection. The two observed Stop responses were approximately 28 ms and 21 ms; these are one-run measurements, not timing guarantees. Physical SDR and Pi remain unverified.

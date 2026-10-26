# RADIO/SDR — hardware-independent receiver foundation

## Status
**Software receiver contract and simulation implemented on feature branch; live SDR reception not implemented.**
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

## IP fallback and curated stations (2026-10-26)

When a real SDR is enumerated and authorized, prefer it. Otherwise test whether an approved publisher can be reached over IP and offer the internet station list. The resolver is interface-based (`RadioDiscovery` and `RadioConnectivity`) so the same logic can be used by the CLI or micro HUD. **The current implementation has no physical SDR enumerator, no live audio/playlist decoder, and no micro radio view.** It must not claim to have checked hardware just because `NoHardwareEnumerator` returns false.

The list contains Skylab Radio UK plus three SomaFM stations (Groove Salad, Drone Zone and Deep Space One), whose MP3 M3U playlist URLs were cross-checked against the publisher's official direct-stream documentation on 2026-10-26. Station metadata is published, but runtime availability is not certified. SomaFM states its streams are provided for individual personal listening; do not rebroadcast, relay or embed them into commercial/game distributions without permission.

- https://somafm.com/groovesalad/directstreamlinks.html
- https://somafm.com/dronezone/directstreamlinks.html
- https://somafm.com/deepspaceone/directstreamlinks.html

```bash
cargo run -p pipboy -- radio stations             # offline directory, no IP probe
cargo run -p pipboy -- radio stations --check-ip  # explicit bounded TCP check to somafm.com:443
cargo run -p pipboy -- radio source               # pure source selection, no probe
```

A successful TCP connection proves only routability to the station publisher's HTTPS server; it does not verify HTTP/TLS, playlist resolution, station audio, or decode compatibility. The future player must validate the actual HTTPS playlist/stream on demand and report `checking`, `ready`, `playing`, `unavailable`, `error` separately. Never start playing or download audio merely because the user opened RADIO. Keep playlists cached with source provenance but never cache copyrighted audio without explicit permission. Do not conflate LAN IP with internet availability.

**Next:** integrate a real hardware enumerator, bounded asynchronous HTTPS validation, a decoder with user-controlled start/stop, and the 480x320 radio station selector. On hardware disconnect, stop capture and offer IP fallback; on network loss retain SDR when available and show the last-curated directory as offline/unavailable rather than active stations.


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

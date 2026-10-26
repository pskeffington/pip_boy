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

The list currently contains three SomaFM stations (Groove Salad, Drone Zone and Deep Space One), whose MP3 M3U playlist URLs were cross-checked against the publisher's official direct-stream documentation on 2026-10-26. Station metadata is published, but runtime availability is not certified. SomaFM states its streams are provided for individual personal listening; do not rebroadcast, relay or embed them into commercial/game distributions without permission.

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

## User demo: curated IP radio (2026-10-26)

Skylab Radio (UK) is included first, followed by the three SomaFM stations. Official Skylab broadcaster page: https://www.skylabradio.com/listen . The selected direct listen address is https://uksoutha.streaming.broadcast.radio:29690/skylab-radio-limited (the broadcaster's own listen page points to the same streaming host; runtime success still needs verification). Do not confuse this UK station with skylab-radio.com in Melbourne.

```bash
# List curated stations with no network request:
cargo run -p pipboy -- radio stations

# Explicit bounded network check for Skylab's publisher streaming host:
cargo run -p pipboy -- radio check skylab --confirm

# Demo: start a preinstalled VLC player, user-triggered only:
cargo run -p pipboy -- radio listen skylab --confirm --player vlc

# Alternative: platform default application for the stream URL:
cargo run -p pipboy -- radio listen skylab --confirm

# Try FFplay if it is installed:
cargo run -p pipboy -- radio listen skylab --confirm --player ffplay
```

The `listen` command launches an external player and returns immediately; it **does not validate that audio has decoded or started playing**. VLC/FFplay must be installed and able to decode the station's format. Browser/system defaults may download an M3U or display it rather than play. Operator can stop playback in the chosen player. On a headless Pi, install a compatible local player or connect an audio frontend. No autoplay occurs on opening the radio module.

`radio stations --check-ip` retains the earlier SomaFM publisher availability check; use `radio check skylab --confirm` for Skylab-specific networking. Both checks only test TCP reachability, not TLS, HTTP playlist resolution, audio quality, format, or stream availability.

**Acceptance gates before mainline:**
- Cargo build, formatting, Clippy and test matrix
- Mac/Linux/ARM64 playback with real player, audio device and stream endpoint
- HTTPS stream health/redirect validation and resilient playback errors
- 480x320 in-app station UI with touch Play/Stop/volume and source state
- Explicit redistribution/embedding licensing review for each broadcaster (SomaFM limits use to individual personal listening)

Playback is initiated only by a user command. Never claim internet radio is a substitute for local RF emergency reception.

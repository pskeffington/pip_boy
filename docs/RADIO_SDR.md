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

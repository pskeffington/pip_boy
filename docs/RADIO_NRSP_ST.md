# nRSP-ST network receiver

The selected SDRplay receiver is an nRSP-ST. Use the standalone module's
`sdrconnect` feature, not the USB `soapy-sdr` adapter. The host forwards API calls
to the pinned module and keeps its existing HUD and preference format.

Connect SDRconnect on the Mac or Pi to the nRSP-ST and enable its WebSocket API.
Use that SDRconnect host's explicit private-LAN/loopback IP and API port (usually
5454). The radio's own network endpoint is a different protocol. No automatic
discovery, receiver configuration or endpoint persistence is performed.

```sh
cargo run --locked -p pipboy --features sdrconnect -- radio sdrconnect-status 127.0.0.1:5454 --confirm
cargo run --locked -p pipboy --features sdrconnect -- radio sdrconnect-capture 127.0.0.1:5454 iq --milliseconds 500 --confirm
cargo run --locked -p pipboy --features sdrconnect -- radio sdrconnect-capture 127.0.0.1:5454 audio --milliseconds 500 --confirm
```

Status is read-only. Capture requires an already selected/started receiver in
SDRconnect. It discards up to five seconds of primary I/Q or demodulated PCM,
then disables only that client's subscription and closes the connection. It
does not stop shared hardware, retune another listener, record or open native
audio output. Capture remains a silent ingress check; native HUD playback is
a separate explicit Play command described below.

The new commands require `--confirm`. Builds without `sdrconnect` return setup
guidance, preserving normal IP playback without any SDR SDK. The network module
has bounded messages, timeouts and cancellation; see its
[protocol and acceptance contract](https://github.com/pskeffington/rust_radio/blob/main/docs/NRSP_ST.md).

Initial API checks were refused. Later, the local SDRconnect 1.0.10 exposed API
1.0.3 and the selected nRSP-ST in IQ Lite mode. The actual Pip-Boy executable
read receiver status and consumed 26,408 stereo PCM frames in a 500 ms audio
subscription. Status before/after confirmed unsubscribe left the shared receiver
running. The test then stopped it through SDRconnect and restored the original
mute/mode settings. No PCM was saved or played by Pip-Boy.

Raw I/Q returned zero pairs in IQ Lite mode. Full IQ startup displayed an
“Update Required” device firmware notice and could not be verified. Firmware was
not modified. Raw I/Q and Pi runtime acceptance remain open.
Real-receiver native HUD playback and independent subscribers were subsequently
verified as recorded below.

Local default and all-feature configurations passed all 331 tests, formatting,
check, strict Clippy and build; RADIO UI fixtures and repository validators also
passed. An unrelated task-dashboard test failed during an earlier parallel run;
40 subsequent complete app suites and the final workspace suite passed without
reproducing it. No task-dashboard code was changed. Remote CI is a separate gate.


## Native HUD playback

Build with `--features sdrconnect` and open RADIO. The station selector includes
**nRSP-ST / SDRconnect**. Selecting it or opening Settings performs no receiver
I/O. Settings contains an IP:port field and SET button, plus the normal native
output picker. Addresses remain in process memory and never enter radio presets.
The existing IP favorites and last station keep their schema and contents.

Use **Refresh Device List** and select the nRSP-ST in IQ Lite mode in SDRconnect.
Then press PLAY in Rust-Boy. If the selected receiver is stopped and SDRconnect
grants control, PLAY sends `device_stream_enable=true` and waits for its started
status before opening audio output. If control is unavailable, PLAY reports an
error. Refresh and receiver selection remain SDRconnect actions. The shared
native engine subscribes to primary stereo PCM, converts/resamples on its worker
and feeds the bounded CPAL queue. Playing requires PCM consumed by the native
output callback. Local volume and output restart use the same engine as IP radio.
Only one source can own output; switching requires Stop and completed worker
cleanup.
Metadata shown as “CONNECTED AT” is captured at connection time; tuning remains
in SDRconnect.

The module polls startup status at most ten times per second and stops waiting
after three seconds. Missing confirmation reports a terminal startup error, with
Refresh Device List/selection/retry guidance, instead of entering network
recovery and repeating activation. After correcting setup, press PLAY again.
Transport disconnections still use bounded recovery. STOP cancels startup;
shared reception remains running if the activation already reached the receiver.

The pinned module excludes native output backpressure from the next audio
read's three-second timeout. Delivering a large PCM packet therefore cannot
trigger a false network reconnect. Secondary and empty primary packets still
cannot keep a stalled subscription alive; bounded captures keep their original
read deadline.

STOP mutes queued PCM immediately and returns while the worker unsubscribes and
releases output. The HUD shows STOPPING and blocks new playback/output changes
until cleanup completes. Stopping Rust-Boy leaves the shared SDRconnect receiver
running. Default builds retain IP playback and hide the optional receiver entry.

```sh
cargo run --locked -p pipboy --features sdrconnect -- dashboard --micro
# Manual native-output acceptance: synthetic PCM, no RF, volume zero.
python3 tools/smoke_radio_sdrconnect.py --live-ip
```

The manual smoke requires an OS audio device and an SDR-enabled binary. It is
classified as a manual hardware check and excluded from unattended CI suites.
It tests native callback evidence, source exclusion, output restart, Stop,
unconfirmed-start failure without automatic activation retries, explicit retry,
resource cleanup and optional live Skylab switching with temporary preferences.

On 2026-10-27, module `21045b3` passed the extended synthetic/native-output smoke:
a receiver that never confirmed startup received exactly one activation, opened
no audio subscription and reached Failed. After fixture setup was corrected,
explicit PLAY retried and reached Playing through native callback consumption.
Standalone playback, output restart and subscription cleanup also passed at
zero volume. This is software/OS-audio evidence, not another RF field test.
Host formatting, check, strict Clippy, build and UI/security validators passed;
all 341 tests passed with `--test-threads=1` after incorporating upstream sync
commits and repairing their formatting/lint gates. The previously recorded parallel
dashboard role-activation failure reproduced and remains an open host issue.

On 2026-10-27 the manual native-output smoke passed with synthetic SDRconnect
PCM at zero volume: standalone and host reached Playing after native callback
consumption, output restart retained the receiver source, duplicate source starts
were rejected, subscriptions closed, and switching to live Skylab succeeded.
Observed Stop response was about 50 ms in that run, not a timing guarantee.
The smoke found and verified a macOS fix: CPAL's CoreAudio stream is explicitly
paused before prebuffering. An earlier live attempt remained stopped after
UI/API start attempts; the operator supplied the missing Refresh then Start
setup step. Firmware was not changed.

On 2026-10-27, the real nRSP-ST in IQ Lite mode at its existing WFM tuning
reached Playing in the SDR-enabled Pip-Boy executable through actual native
callback consumption, using temporary preferences and low volume. The standalone
module passed five seconds of native playback and a further independent
three-second session while Pip-Boy remained Playing. Local volume adjustment,
output restart and duplicate-start rejection passed. Stop returned in 9.44 ms
in this run, completed worker cleanup and left the shared receiver running.
A fresh 500 ms subscription received 26,409 stereo frames afterward. These are
real-receiver checks, separate from the synthetic fixture; no PCM was saved or
shared tuning, firmware or receiver start/stop commands sent.

Repeating these live checks with host `f71dd2d` and module `6997dff` after an
operator Refresh/Start restored PCM ingress and passed both native players,
concurrent reception, output restart and cleanup again. Stop returned in
11.52 ms and left shared reception running; a fresh 500 ms capture received
24,008 stereo frames. This validates the module pin containing the output
backpressure timeout fix against the real receiver.

On 2026-10-27 the operator confirmed audible radio playback through the MacBook
Air speakers with the system default audio output. RF signal quality has not
been measured; Pi and physical Bluetooth recovery acceptance remain open.

Final checks passed 331 host tests in both default and all-feature builds,
strict Clippy, build, UI interaction fixtures and 11 Chromium viewport scenarios.
The known unrelated task-dashboard flake recurred once in an earlier parallel
run; the complete rerun passed. No task implementation was changed.

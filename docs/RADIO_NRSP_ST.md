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
audio output. This is API ingress, not completed RF playback in the HUD.

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
not modified. Raw I/Q, native RF audio playback in the HUD, multiple independent
listeners and Pi runtime acceptance remain open.

Local default and all-feature configurations passed all 331 tests, formatting,
check, strict Clippy and build; RADIO UI fixtures and repository validators also
passed. An unrelated task-dashboard test failed during an earlier parallel run;
40 subsequent complete app suites and the final workspace suite passed without
reproducing it. No task-dashboard code was changed. Remote CI is a separate gate.

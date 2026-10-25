# Rust-Boy | Typed mock adapter interfaces

**Gate:** G2 foundation — source-only virtual adapters. **Status:** new tests undergoing CI acceptance. Physical adapters and trusted authorization are not implemented.

## Interfaces and concrete mock implementations

The core library now exposes three object-safe, typed traits: InputSource, DisplayBackend and SerialReader. Their initial implementations are MockInputSource, MockDisplayBackend and MockSerialReader, each bound to an in-memory VirtualTransport and a single fixed virtual device. All operations are finite and synthetic.

| Trait | Fixed virtual device | Emitted record | Safety invariant |
| --- | --- | --- | --- |
| InputSource | sim:gpio / gpio.read | MockInputEvent (no_physical_input_observed) | No fabricated button press, voltage, pin state or physical input |
| DisplayBackend | sim:display / display.write | MockDisplayReceipt (mock_frame_accepted_not_rendered) | rendered_to_hardware is always false |
| SerialReader | sim:serial / serial.read | MockSerialFrame (explicitly_injected_fixture_bytes) | injected is true and all bytes originate from the caller-supplied fixture |

Display text must be 1–80 printable ASCII characters. Newlines, terminal control sequences, non-ASCII and oversized frames reject without producing a mock sample. The serial payload fixture must be 1–32 bytes long; no system serial ports, device discovery, Bluetooth or networking are involved. The CLI uses a fixed two-byte fixture (decimal [82, 66], text "RB") and a fixed eight-character display string ("RUST-BOY"). These are clearly identified test data, not observed external data.

## CLI

    pipboy simulate adapter input --confirm
    pipboy simulate adapter display --confirm
    pipboy simulate adapter serial --confirm

Each invocation emits one versioned JSON object with simulated: true, a mock source and device identifier, an explicit mock-only event label and a deterministic sequence. Display receipts contain rendered_to_hardware: false; serial frames contain injected: true. Nothing is sent to a peripheral, speaker, bus, serial interface or network endpoint.

Without the explicit --confirm flag, the operation fails even if a saved preference is requested. Any stored denied preference for gpio.read, display.write or serial.read takes precedence over confirmation. A malformed preference file prevents all mock operations. The command does not persist active grants or create a data directory.

## Library architecture and future real adapters

These traits make the eventual platform-specific implementations distinguishable from the simulator, but a Rust trait and a mock token do not create OS isolation. A real InputSource, DisplayBackend or SerialReader must be separately reviewed, bind to a known physical device, obtain an independently confirmed OS-broker-issued handle, enforce trusted revocation and timeouts and avoid automatic enumeration or communication.

Explicit revoke and cancel fixtures verify that a mock operation cannot continue after its in-memory session ends. This prototype does not provide physical hot-plug, debouncing, display refresh scheduling, actual serial framing, device drivers, peer authentication or transmission.

## Validation

Ten new core unit tests exercise trait-object dispatch, mock-only output contracts, invalid display/serial input bounds, saved preference denial, absence of implicit confirmation, revocation/cancellation and reproducible JSON. A separate tools/smoke_mock_adapters.py validates CLI schemas, denied cases and corrupt preferences. tools/smoke_native_cli.py validates cross-platform, noninteractive command output, absence of ANSI escapes and read-only filesystem behavior.

The GitHub workflow runs the new tests on Linux stable/MSRV and native macOS/Windows, plus a POSIX PTY journey on macOS using tools/smoke_runtime.py. The portable Windows CLI test is **not a Windows ConPTY interactive TUI test**; that remains an acceptance gate. Native TUI/accessibility evaluation and real devices are still outstanding.

See [virtual transport](VIRTUAL_TRANSPORT.md), [simulation](SIMULATION.md), [OS device broker requirements](OS_DEVICE_BROKER.md), [roadmap](ROADMAP.md), and [validation](VALIDATION.md).

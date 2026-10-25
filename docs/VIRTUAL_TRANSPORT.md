# Rust-Boy | Bounded virtual event transport

**Gate:** G2 prototype, not a physical device driver. **Status:** source and tests committed; native OS and new-code acceptance pending.

The new `VirtualTransport` wraps a single trusted, in-memory `SimulatedBroker` session and a strictly bounded FIFO. No OS file, network socket, serial device, GPIO pin, SDR receiver, BLE scanner, mesh transmitter or wall-clock watchdog can be opened by this code.

## CLI

```bash
pipboy simulate transport sim:sdr --count 5 --capacity 2 --confirm
```

CLI syntax requires `--count N` (1–128), `--capacity N` (1–64), and a user-issued `--confirm` for success, in that order. Omitting confirmation fails closed even when a saved preference is `requested`; a saved `denied` preference blocks the session despite confirmation. Unknown fixture IDs, duplicates, invalid options or bounds do not emit output.

One successful sample run with `--count 5 --capacity 2` returns a version-1, `simulated: true` JSON envelope including `source: "rust-boy/bounded-virtual-transport"`, device/capability, `requested: 5`, `capacity: 2`, `dropped: 3`, and only synthetic events numbered 4 and 5 under `retained`. The underlying event retains `source: "rust-boy/in-memory-simulator"` and `event: "fixture_only_no_hardware_io"`. No signal, coordinates, telemetry reading, packet contents, peer identity or hardware metric is fabricated.

## Behavior

- Capacity and pump bounds are validated before queuing; all operation counts are finite.
- When the FIFO is full, the oldest mock event is discarded and its drop counter increments. Events remain ordered.
- The queue is scoped to an explicit simulated session. `receive` checks that it is still active; `close` and `cancel` clear queued data.
- `revoke`, virtual `disconnect`, expired logical deadline, backward logical clock or denied/corrupt preference reload invalidates the mock session and clears its buffered events. Virtual reconnect never silently reauthorizes a session.
- A new session needs new explicit confirmation. The in-memory drop counter records the lifetime of the `VirtualTransport` instance, not a physical link lifetime; it does not persist.
- The mock module has no external transport backends. A Rust trait, simulated broker or event queue is not an operating-system security boundary.

## Test coverage and release gates

`virtual_transport.rs` contains eight unit tests for bounds, deterministic overflow, deny-before-sampling, revoke, disconnect/reconnect, virtual TTL and cancellation, malformed policy reload, and backward clock. `tools/smoke_transport.py` validates strict command parsing, JSON schema, sequence/drop counts, request-versus-confirmation, saved denial, future schema preservation and no filesystem side effects.

The GitHub quality workflow now adds native macOS and Windows stable builds/tests plus offline permissions/simulator/replay/transport smokes. These complement the Linux stable and Rust 1.81.0 tests; native terminal PTY/accessibility acceptance and real device tests remain separate. Do not promote G2 device, serial, radio or mesh module status based solely on this in-memory prototype.

See [SIMULATION.md](SIMULATION.md), [G1 execution plan](G1_EXECUTION_PLAN.md), [roadmap](ROADMAP.md), and [validation record](VALIDATION.md).

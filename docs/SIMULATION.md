# Rust-Boy | Deterministic device-broker simulator

**Gate:** G1-D prototype leading to G2 device broker and simulator.  
**Status:** source implemented, new tests not executed. G2 hardware adapters remain planned.

## Why this exists

Rust-Boy needs an executable, testable **authorization contract** before opening any actual GPIO, serial, GNSS, SDR, BLE, mesh or network interface. The `SimulatedBroker` is a deliberately restricted, **in-memory-only** object. It exercises denial, explicit per-session confirmation, a per-broker opaque lease, revocation, one-active-session enforcement and reloading preferences.

It never invokes an OS device API, performs device discovery, emits radio signals, accesses a network, plays audio or fabricates sensor readings. A successful simulation is **not** evidence that a physical adapter works.

## CLI examples

```bash
pipboy simulate list
pipboy simulate run sim:gpio
# denied: explicit simulation confirmation required
pipboy simulate run sim:gpio --confirm
# emits one JSON fixture event, then closes the simulated session

pipboy permissions request sdr.receive
pipboy simulate run sim:sdr
# denied: requested preference cannot substitute for confirmation
pipboy simulate run sim:sdr --confirm
# succeeds in-memory only

pipboy permissions deny sdr.receive
pipboy simulate run sim:sdr --confirm
# denied: saved explicit denial takes precedence
```

`--confirm` applies **only to this one simulation invocation**. It is not persisted, does not update `ModuleHost` grants, and cannot be used to authorize a real device. `sim:* ` identifiers are hard-coded virtual fixtures, not discovered hardware or addresses.

## Fixed fixture inventory

| ID | Declared capability | Behavior |
| --- | --- | --- |
| `sim:enumerate` | `device.enumerate` | Virtual inventory only |
| `sim:gpio` | `gpio.read` | No actual pin sampling |
| `sim:display` | `display.write` | No display access |
| `sim:serial` | `serial.read` | No serial port access |
| `sim:gnss` | `gnss.read` | No location information |
| `sim:sdr` | `sdr.receive` | No receiver or signal samples |
| `sim:ble` | `ble.discover` | No local scanning |
| `sim:mesh` | `mesh.transmit` | No radio/network transmission |
| `sim:sensor` | `sensor.read` | No fabricated measurements |
| `sim:audio` | `audio.play` | No sound |
| `sim:network` | `network.fetch` | No HTTP requests |

An emitted event is explicitly typed:

```json
{
  "schema_version": 1,
  "simulated": true,
  "source": "rust-boy/in-memory-simulator",
  "device_id": "sim:gpio",
  "capability": "gpio.read",
  "event": "fixture_only_no_hardware_io",
  "sequence": 1
}
```

There are no synthetic CPU, environmental, coordinates, RF signal levels or other numeric measurements.

## Lease and preference invariants

1. `PreferenceStore` validates and loads preferences before a simulated session begins. Unknown/future/corrupt schema prevents the CLI from running a simulation; reading never creates data files.
2. `Preference::Requested` **does not** authorize anything. `SessionConfirmation::Confirmed` is required for every `open`; `Denied` always blocks that capability.
3. A lease has private fields, a broker-instance identity and a monotonically increasing generation. It cannot be serialized as a reusable grant or applied to another broker instance.
4. At most one fixture lease can be active in one broker; explicit `revoke`, `close`, or a policy reload to `Denied` invalidates the lease before the next `sample`.
5. An invalid policy replacement clears the active lease, instead of retaining its prior permissions. Reversing a denial never restarts a previously revoked session.
6. Each successful call emits only a clearly labeled fixture event. `SimulatedBroker` owns no OS handle or background tasks, and the CLI closes its short-lived lease after one event.

A separate application or UI must notice and reload preference changes; this prototype does **not** watch filesystem events, automatically receive a write-time revocation, or supply an OS-enforced authorization service. `SessionConfirmation` is a cooperative Rust API: code linked into the same process could construct `Confirmed`. Never reuse it as the security boundary for real drivers.

## Acceptance and remaining work

New core unit tests exercise all 11 fixture mappings, saved-request denial, persisted denial precedence, absence of physical measurements, explicit close, stale-lease invalidation, policy reload/revoke, invalid updates and cross-broker replay. The new `tools/smoke_simulator.py` verifies CLI parsing, missing-confirmation rejection, fake JSON output, no default data directory, saved denial precedence and corrupt preference preservation. The stable CI workflow now invokes that script.

```bash
python3 tools/validate_catalog.py
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build -p pipboy --locked
python3 tools/smoke_simulator.py
python3 tools/smoke_permissions.py
python3 tools/smoke_notes.py
python3 tools/smoke_runtime.py
```

**No Rust compiler or executable acceptance evidence was available during this pass.** GitHub Actions currently fails before starting jobs. The real G2 broker must additionally enforce OS handle ownership, device-specific identity, a trusted authorization UX/service, cancellation and revocation even across filesystem updates, real disconnect/timeouts, platform permissions, and hardware-in-loop verification. See [G1 execution](G1_EXECUTION_PLAN.md), [permissions contract](PERMISSIONS.md), and [validation](VALIDATION.md).

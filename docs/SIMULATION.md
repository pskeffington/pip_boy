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

`--confirm` applies **only to this one simulation invocation**. It is not persisted, does not update `ModuleHost` grants, and cannot be used to authorize a real device. `sim:*` identifiers are hard-coded virtual fixtures, not discovered hardware or addresses.

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

## Deterministic logical-time replay (new, unverified)

Five **fixed source-defined scenarios** are provided by `simulator_replay.rs`. Replay does not load arbitrary user scripts, files or real device data, and never sleeps or reads a wall clock:

```bash
pipboy simulate replay sim:gpio nominal --confirm
pipboy simulate replay sim:serial disconnect-recover --confirm
pipboy simulate replay sim:sdr revoke --confirm
pipboy simulate replay sim:ble cancel --confirm
pipboy simulate replay sim:network timeout --confirm
```

| Scenario | Synthetic logical times (ms) | State-transition assertions |
| --- | --- | --- |
| `nominal` | 0, 100, 200 | Confirm/open, one fixture event, close |
| `disconnect-recover` | 0, 100, 200, 300, 400, 500, 600, 700 | Initial fixture event; virtual disconnect invalidates lease; stale reads and opens while disconnected reject; reconnect leaves the fixture inactive; missing renewed confirmation rejects |
| `revoke` | 0, 100, 200, 300, 400 | Initial fixture event; capability revoke invalidates lease; stale read and unconfirmed new open reject |
| `cancel` | 0, 100, 200, 300, 400 | Cancellation invalidates the virtual lease and blocks stale reads and implicit restart |
| `timeout` | 0, 100, 200, 300, 400 | A 200 ms injected logical TTL expires at its exact boundary; stale reads and unconfirmed restart are rejected |

Output is a version-1 `ReplayTrace` object carrying `simulated: true`, `source: "rust-boy/deterministic-simulator"`, `clock: "synthetic_logical_ms"`, scenario, fixture ID/capability and at most eight ordered steps. Each step provides `logical_ms`, `action`, `state`, `outcome` and, where applicable, an explicit error code or the original labeled `SimulationEvent`. Replays do **not** fabricate readings or GPS coordinates. The same scenario is designed to serialize identically across repeated invocations.

A single `--confirm` authorizes the **mock scenario as a whole**. The replay intentionally **does not automatically reopen** a fixture after disconnect, reconnect or revoke; any later CLI simulation requires its own explicit `--confirm`. These scenario scripts demonstrate expected policy behavior but are not fresh individual user approvals, hardware tests, or proof of OS-level isolation.

New `SimulatedBroker::disconnect/reconnect` methods only toggle virtual availability. Disconnection invalidates an active lease immediately; reconnection alone cannot restore it. The `Unavailable` error is distinct from `InvalidLease` and `ConfirmationRequired`, and all are represented by stable machine-readable replay error codes.

## Cancellation and virtual time boundaries

`SimulatedBroker` now exposes `cancel(&lease)`, `open_with_timeout(..., ttl_ms)` and `advance_to(logical_ms)`. The TTL must be 1–60,000 injected logical milliseconds; invalid bounds fail before opening a lease. An exact deadline expires the mock session. Clock rewinds fail closed by clearing the session; overflow is rejected. This is a deterministic test clock, **not** a wall-clock watchdog, physical device timeout, OS broker, or automatically scheduled task. No receiver/network/device APIs are opened. Newly authored cancellation/timeout tests require CI verification on their current commit.

## Lease and preference invariants

1. `PreferenceStore` validates and loads preferences before a simulated session begins. Unknown/future/corrupt schema prevents the CLI from running a simulation; reading never creates data files.
2. `Preference::Requested` **does not** authorize anything. `SessionConfirmation::Confirmed` is required for every `open`; `Denied` always blocks that capability.
3. A lease has private fields, a broker-instance identity and a monotonically increasing generation. It cannot be serialized as a reusable grant or applied to another broker instance.
4. At most one fixture lease can be active in one broker; explicit `revoke`, `close`, or a policy reload to `Denied` invalidates the lease before the next `sample`.
5. An invalid policy replacement clears the active lease and locks the broker against new sessions until a valid policy snapshot is loaded. Reversing a denial never restarts a previously revoked session.
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
python3 tools/smoke_replay.py
python3 tools/smoke_permissions.py
python3 tools/smoke_notes.py
python3 tools/smoke_runtime.py
```

**No Rust compiler or executable acceptance evidence was available during this pass.** GitHub Actions currently fails before starting jobs. The real G2 broker must additionally enforce OS handle ownership, device-specific identity, a trusted authorization UX/service, cancellation and revocation even across filesystem updates, real disconnect/timeouts, platform permissions, and hardware-in-loop verification. See [G1 execution](G1_EXECUTION_PLAN.md), [permissions contract](PERMISSIONS.md), and [validation](VALIDATION.md).

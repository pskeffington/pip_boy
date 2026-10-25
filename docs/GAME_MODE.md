# Rust-Boy | Fallout 4 game mode

Status: implemented **read-only companion browser**, with controlled-server protocol and terminal tests. Actual Fallout 4 builds/platforms are **not yet live-validated**. This is not a complete replacement for every official Pip-Boy app feature. Equip/use/drop, quest toggles, travel, radio control and local-map image rendering are not implemented.

## Edition compatibility target: all versions

The requested target covers all versions; **no edition is yet certified with a real running game**. A client cannot connect to a service the game does not expose.

| Installation | Connection route | Current acceptance |
| --- | --- | --- |
| PC legacy / pre-Anniversary, including differing storefronts | Built-in companion TCP service where available | Rust client implemented; actual builds untested |
| PC Anniversary Edition | Game-side service enabler or a separately validated bridge required | Native client can speak the legacy protocol; enabler integration untested |
| PS4 / Xbox One builds | Companion service only if that exact build exposes it | Not tested; no universal console support claimed |
| PS5 / Xbox Series native or backward-compatible builds | Depends on the exact installed game build/service | Not tested; cannot assume equivalence to the older build |
| Modded installations | Retain/re-enable a compatible service or supply a validated bridge | No automatic mod installation or process injection |

The [AE companion enabler author's description](https://www.nexusmods.com/fallout4/mods/108133) reports that Anniversary Edition removed the old menu option and stopped starting the service. This is a third-party game-side project, not bundled or verified by Rust-Boy. Its supported builds, prerequisites and binary trust must be checked before installation. Rust-Boy does not alter the game or install this mod automatically. Console editions without an exposed service remain blocked; PC-only service mods do not establish console support.

## Connect to your game

1. Identify the edition below. On builds exposing the legacy service, enable **Pip-Boy App Enabled** in Settings → Gameplay, restart if required, and load a save. Do not assume Anniversary Edition or a native console edition exposes this service.
2. Close other companion apps: a busy server accepts only one companion connection.
3. On the same PC, try `127.0.0.1:27000`. From another computer use the game host's LAN IP and TCP port 27000. Permit the connection through the host firewall on your trusted network if needed. Do not expose the companion port to the internet.
4. Run Rust-Boy on a computer able to reach the game host:

```bash
cargo run --locked -p pipboy -- game --connect 127.0.0.1:27000 --confirm
# A single bounded JSON view for diagnostics:
cargo run --locked -p pipboy -- game --connect 127.0.0.1:27000 --confirm --snapshot
```

`--connect` requires a literal IP:port (IPv6 uses `[address]:port`). `--confirm` explicitly authorizes this one TCP connection. Arguments and saved adapter preferences are validated before connecting. A saved `network.fetch` denial blocks the connection even with `--confirm`; explicitly clear it with `pipboy permissions clear network.fetch` to change that choice. Corrupt/future preference files fail before connection and are preserved. Reading an absent preference file creates nothing. No discovery broadcast, DNS lookup, automatic retry, background connection, game-state file or telemetry upload occurs. Normal STAT/LIBRARY/NOTES startup remains offline. The separate physical-device broker is unchanged and remains deny-only.

Game mode opens its own terminal view. Up/Down selects a game object; Enter opens it; Backspace returns to its parent. Page Up/Down scrolls, Home resets scrolling, and Q/Escape/Ctrl+C exits and restores the terminal. Browse the game's actual root categories for stats, SPECIAL, inventory, quests and map coordinate/object data. Names and availability come from the game; absent objects are not invented. No game action is transmitted: outbound bytes are heartbeat responses only. Root/selected views are generic JSON rather than fixed field mappings, so schema differences remain inspectable.

## Protocol and resource limits

The independent Rust implementation follows the published [pipboyspec communication reference](https://github.com/mattbaker/pipboyspec/blob/master/communication.md) and [pipboylib protocol reference](https://github.com/RobCoIndustries/pipboylib/blob/b1d936f0bbffb5c702a8e06faf5563b1fa109475/docs/server-msg-spec.md) / [decoder behavior](https://github.com/RobCoIndustries/pipboylib/blob/b1d936f0bbffb5c702a8e06faf5563b1fa109475/src/decoding/parseBinaryDatabase.js). These are community protocol descriptions, not a Bethesda compatibility guarantee; no upstream implementation or game assets are bundled.

Frames have a four-byte little-endian payload size and a one-byte channel. The client accumulates fragmented input, accepts coalesced frames, echoes server heartbeat frames, parses the language/version handshake, refuses busy state, and applies type-3 database updates by object ID. Dictionary removals apply before insertions so a replacement with the same key survives. Scalar, array and dictionary values are retained across incremental updates. An incomplete/invalid update never partially changes the database. Snapshot output waits until root zero is an object and all reachable references exist.

Limits: 8 MiB frame; 100,000 database objects; 16 MiB encoded retained database; 4,096-byte wire strings; 32-level/5,000-node resolved views; 32-level browser navigation; 262,144 visible text characters. Missing references and cyclic/limited views have explicit markers. Snapshot JSON is a bounded diagnostic view, not a lossless full-game export. Invalid floats/types, oversized input, connection loss and busy state stop with an error. Connect timeout is three seconds, initial complete-root timeout ten seconds, no-receive timeout fifteen seconds, write timeout two seconds. Update age is shown separately from receipt/heartbeat activity. Disconnected values are not presented as live; the view exits. Game-provided control characters are removed before terminal rendering.

## Validation and remaining acceptance

`tools/smoke_game.py` starts only a temporary loopback mock peer and verifies fragmented frames, heartbeat replies, a root whose reference arrives later, snapshot values, busy/oversize refusal, strict arguments/consent, no gameplay commands/files, and interactive browser exit/restoration. It uses POSIX PTY on Linux/macOS and native ConPTY on Windows. Core tests cover transactional updates, live references, dictionary replacement/removal, cycles, missing references and non-finite values. Native CI runs the smoke suite on all three OSs.

Before calling this fully game-compatible, record a real game session: platform, exact Fallout 4 version, App Enabled setting, successful connection, category browsing, observed inventory/stat/map changes, heartbeat continuity, disconnect/busy behavior, and terminal restoration. Neither synthetic fixtures nor CI can establish those results. Real game actions and map images need separate implementations and game-side verification.

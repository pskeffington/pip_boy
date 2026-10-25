# Rust-Boy on Mac → Fallout 4 on PlayStation

**Current development priority:** launch natively from macOS and connect to a PlayStation's exposed Fallout 4 companion service. PC/Anniversary/other-platform expansion is deferred. The native client and mock-peer launcher tests do not certify an actual PS4/PS5 game build. Game mode is read-only.

## Launch from Finder

1. Keep the entire repository folder together. Install Rust/Cargo first if not already available; the launcher builds Rust-Boy for the Mac running it. This is a source launcher, not a notarized `.app` or universal prebuilt binary.
2. Connect the Mac and console to the same home LAN. Wired/Wi-Fi combinations are fine when the router permits local traffic. A guest network, client isolation or VPN routing can block it.
3. Launch Fallout 4 on PlayStation and load a save. On builds exposing it, turn on **Pip-Boy App Enabled** in Gameplay settings and restart the game if necessary. Close another companion app before connecting.
4. Find the **console's** LAN IP. Bethesda documents PS4: Settings → Network → View Connection Status → IP Address. On PS5, inspect Network connection status for the console address. Do not enter the Mac's IP or `127.0.0.1` for a real console.
5. Double-click [`scripts/launch-playstation.command`](../scripts/launch-playstation.command). Terminal opens; enter the console IP when prompted. Q/Escape/Ctrl+C exits. The IP is not saved. If macOS refuses a downloaded script, inspect the source and use the Terminal invocation below rather than disabling system protections.

The launcher quotes the address, works when launched outside the repository and from paths containing spaces, and uses the installed native Rust toolchain. It does not download dependencies before Cargo runs, install console mods, scan the LAN or alter firewall/privacy settings. Initial Cargo builds may download locked Rust dependencies.

## Launch or diagnose from Terminal

From the repository folder, substituting your PlayStation IP:

```bash
./scripts/launch-playstation.command 192.168.1.50
./scripts/launch-playstation.command 192.168.1.50 --diagnose
# Equivalent CLI:
cargo run --locked -p pipboy -- game --playstation 192.168.1.50 --confirm
cargo run --locked -p pipboy -- game --playstation 192.168.1.50 --confirm --diagnose
```

The preset uses TCP 27000. An explicit `IP:PORT` is accepted for diagnostics/test fixtures; bare IPv6 uses port 27000, and an IPv6 address with a port must use `[address]:port`. Hostnames are not resolved. Connection consent and saved `network.fetch` denials behave as described in [game mode](GAME_MODE.md).

The diagnostic is a real protocol exchange, not just a port test. It waits for the complete reachable game root and prints JSON containing `status: ready`, endpoint, selected target profile, handshake and available root fields. `console_identity_verified: false` means the selected profile is not authentication or hardware identification; the protocol does not establish either. `game_actions_enabled: false` states that gameplay commands are absent. A diagnostic reads data and responds to heartbeats only; it does not equip/drop/travel or persist game state.

## Troubleshoot the actual connection

| Result | Check |
| --- | --- |
| Connection refused | Correct console IP, Fallout 4 running, and this exact game build exposes the companion service |
| Timeout / unreachable | Same LAN, guest/client isolation, VPN routing, network permissions if applicable; no internet port forwarding is required |
| Busy | Close the companion app/device already connected, then launch again |
| Root timeout | Load a save, check App Enabled/service availability, and inspect the actual game build |
| Saved network denial | Review the saved preference; `pipboy permissions clear network.fetch` explicitly removes it |
| Build fails / Cargo absent | Use a supported native Rust toolchain; launcher does not install one automatically |

On macOS versions with Local Network privacy controls, review System Settings → Privacy & Security → Local Network if access was denied or a relevant prompt appears. Applicability depends on the macOS release and launching application; do not assume every Terminal build needs a privacy toggle. Rust-Boy never changes privacy settings itself.

## PS4 and PS5 scope

Bethesda's original announcement explicitly lists PS4 companion support. That historical statement does not certify today's patched PS4 edition, PS4 software running on PS5, or native PS5 software. Record which edition is actually launched, not just the console model. PC F4SE/Anniversary enablers are not a PlayStation solution. If the installed build exposes no companion service, changing the Mac launcher cannot create it. No unsupported PS5 settings-transfer workaround is claimed.

The live acceptance gate requires a real Mac/console pair: record macOS/CPU architecture, console model, installed PS4/PS5 game edition/version, successful diagnostic, browsing stats/inventory/map data, observed in-game updates, heartbeat continuity, busy/disconnect behavior and terminal restoration. This environment has no such console session, so those results remain pending.

## Primary references

- [Bethesda original supported platforms](https://bethesda.net/en-US/news/fallout-4-pip-boy-app-now-available) (2015 announcement, not a current-version guarantee).
- [Bethesda console-IP instructions](https://help.bethesda.net/app/answers/detail/a_id/31479/).
- [Bethesda busy-connection explanation](https://help.bethesda.net/app/answers/detail/a_id/31478/).
- [Apple Local Network privacy controls](https://support.apple.com/en-gb/guide/mac-help/mchla4f49138/mac) and [developer guidance](https://developer.apple.com/documentation/technotes/tn3179-understanding-local-network-privacy).

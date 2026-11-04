# Rust-Boy for macOS: release path

Status: **packaging candidate; not a production release**.

The current UI is a local Rust HTTP server with browser-based desktop and micro
surfaces. The native menu-bar bundle starts that server, opens the selected
surface in the default browser, and offers explicit Stop and Quit controls. It
binds only to IPv4 loopback. It does not add a public listener, background
network service, module installation, or automatic update channel.

## Build and validate a local app

Requirements: macOS 13 or newer, Xcode Command Line Tools, Rust 1.90+, Python
3, and a default browser. The builder can create arm64 or x86_64 bundles. A
universal bundle is built by combining both Rust and Swift slices:

    RUSTBOY_TARGET=universal-apple-darwin bash scripts/build-macos-app.sh
    open target/macos/Rust-Boy.app
    bash tools/test_macos_app.sh

The universal build requires the Rust standard library for both
`aarch64-apple-darwin` and `x86_64-apple-darwin`, plus Xcode's macOS SDK and
`lipo`. Exact head `db452596e8b10a42a5fb6d6187b7b0a30c79413b` passed 7/7 jobs
in [Actions run 41778857053](https://github.com/pskeffington/pip_boy/actions/runs/41778857053).
The native macOS job installed the Intel Rust target, checked both Rust and
Swift executable slices, launched the universal bundle, and fetched desktop
and micro routes. Intel Mac hardware acceptance remains separate.

The unsigned app bundle is for local validation only. The smoke test builds the
Rust binary, assembles the app bundle, launches the native menu-bar host with
browser opening disabled, and verifies both local dashboard routes over
loopback. scripts/launch-dashboard.command remains the source/developer
launcher and continues to require Rust.

The bundle builder selects the `pipboy` package version by package name,
includes a generated multi-resolution Rust-Boy app icon and standard utility
metadata, and builds into a temporary sibling directory before replacing an
existing Rust-Boy bundle. It refuses unsafe output paths, symlinks, and
unrelated existing bundles. Hosted macOS smoke drives the actual menu item
actions for both browser surfaces, Stop, and Quit; a separate negative case
removes the bundled service and verifies the menu exposes Retry. The hosted
smoke also asks Apple's `iconutil` to decode the generated ICNS resource.
Stop and Quit send SIGTERM off the main-thread path, wait asynchronously, and
send SIGKILL after a bounded grace period if the dashboard does not exit.

## Developer ID distribution

Outside-App-Store distribution requires a Developer ID Application identity
and an App Store Connect API key configured with notarytool. Keep certificate
private keys and API credentials in the release runner's protected keychain or
secret store; never commit them or print them in logs.

1. Import the Developer ID certificate into the runner keychain and set
   RUSTBOY_SIGN_IDENTITY to its exact signing identity.
2. Store notary credentials once with xcrun notarytool store-credentials and
   set RUSTBOY_NOTARY_PROFILE to that keychain profile name.
3. Build the app with RUSTBOY_SIGN_IDENTITY set, then run
   scripts/package-macos-dmg.sh.
4. Preserve the resulting app/DMG, notarization submission ID and log,
   codesign verification, stapler validation, Gatekeeper assessment, source
   commit, and checksums as release evidence.

The package script signs and verifies the embedded executables and app, submits
both the app archive and DMG to Apple's notary service, staples accepted
tickets, and runs Gatekeeper assessment. It refuses to package without signing
and notarization configuration. This repo currently has no configured Apple
Developer ID certificate or notarization profile, so this final process has
not been run and no signed/notarized artifact is available. On a configured
release runner it also refuses a dirty Git tree, validates the universal slices,
preserves both notary submission responses and JSON logs, and writes a
source-SHA/version/identifier/architecture/checksum manifest to the private
per-attempt evidence directory without overwriting earlier attempts. The final
DMG is moved into place only after notarization,
stapling and Gatekeeper assessment succeed.

## User data and support boundaries

- Rust-Boy stores local state under ~/.pipboy by default, or under the explicit
  PIPBOY_HOME directory. Removing the app does not erase that data.
- The desktop and micro surfaces are local browser pages. Local role labels do
  not authenticate a human, and local journals are not encrypted at rest.
- Do not store credentials, regulated medical records, or other protected data
  in this development-preview build.
- Optional network and device operations retain their explicit authorization
  paths. Package selection does not grant permissions.
- MAP, RADIO, and other modules retain their separate availability and
  hardware acceptance gates. Raspberry Pi acceptance does not establish Mac
  package signing or notarization.
- Updates are manual replacement of the app. There is no in-app updater or
  automatic migration guarantee. Back up the data directory before upgrades.

## Acceptance still required

- Exact-head CI passed for the universal bundle at
  `db452596e8b10a42a5fb6d6187b7b0a30c79413b` in [run
  41778857053](https://github.com/pskeffington/pip_boy/actions/runs/41778857053)
  and for documentation head `0394056a6c3b381bf64b667ccc0ac382d90f5309` in
  [run 41779843083](https://github.com/pskeffington/pip_boy/actions/runs/41779843083).
  Follow-up head `2c6e15446c1414a42a08bc8f3c4c94d3c78d1f14` failed the native
  AppKit smoke in [run 41782734765](https://github.com/pskeffington/pip_boy/actions/runs/41782734765)
  while six other jobs passed. The failure was a stale Quit marker assertion
  after Stop began terminating directly; the current worktree corrects that
  assertion and waits for live-Quit child-shutdown evidence. At current head
  `fd41d089c6e812a5d56786619f0c937a27c05271`, six jobs passed in [run
  41783954586](https://github.com/pskeffington/pip_boy/actions/runs/41783954586),
  while the native macOS AppKit step remained in progress and its live job logs
  were unavailable. Review found an unbounded smoke-process wait and a
  redundant `waitUntilExit()` inside the termination handler; the current
  worktree removes the nested wait and bounds process waits to 30 seconds.
  Head `8f69b5a92194fe2e36b5c12b90e27adeee8942a0` failed its live-Quit smoke
  in [run 41785148903](https://github.com/pskeffington/pip_boy/actions/runs/41785148903):
  Stop and Retry passed, but Quit did not exit the app within 30 seconds. The
  current worktree reissues termination after child shutdown and requires the
  child-stopped marker before checking app exit. Exact-head CI is pending.
- Universal bundle slices compile and pass CI smoke; Intel physical-Mac
  install and runtime acceptance remain pending.
- The automated app smoke exercises the bundled Rust service in a headless
  mode. Hosted macOS CI also launches the AppKit host, confirms the dashboard
  child starts, invokes the Stop lifecycle path and checks that the child exits.
  It does not verify menu selection or use in a logged-in user session.
- Review of the app bundle identifier, copyright, icon, minimum macOS version,
  privacy wording, update and uninstall behavior.
- Developer ID signing and notarization on a protected macOS release runner.
- Gatekeeper install/launch verification on a clean supported Mac account.
- Explicit operator acceptance of browser behavior and accessibility on macOS.

Apple's distribution guidance requires Developer ID signing for direct
distribution and recommends notarization; notarization uses the hardened
runtime. See [Distributing software on macOS](https://developer.apple.com/macos/distribution/)
and [Notarizing macOS software before distribution](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution).

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
`lipo`. CI installs the Intel Rust target, checks both executable slices, and
starts the bundled dashboard on the runner's native architecture. Intel Mac
hardware acceptance remains separate.

The unsigned app bundle is for local validation only. The smoke test builds the
Rust binary, assembles the app bundle, launches the native menu-bar host with
browser opening disabled, and verifies both local dashboard routes over
loopback. scripts/launch-dashboard.command remains the source/developer
launcher and continues to require Rust.

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
not been run and no signed/notarized artifact is available.

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

- Exact-head CI for the app-bundle launch smoke and release scripts.
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

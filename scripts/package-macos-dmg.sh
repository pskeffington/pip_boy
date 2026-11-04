#!/bin/bash
set -euo pipefail
if [[ "$(uname -s)" != Darwin ]]; then echo "This packager requires macOS." >&2; exit 1; fi
if [[ -z "${RUSTBOY_SIGN_IDENTITY:-}" || -z "${RUSTBOY_NOTARY_PROFILE:-}" ]]; then
  echo "Set RUSTBOY_SIGN_IDENTITY and RUSTBOY_NOTARY_PROFILE; unsigned distribution is refused." >&2
  exit 2
fi
ROOT="$(cd -- "$(dirname -- "$0")/.." && pwd)"
APP="${RUSTBOY_APP_PATH:-$ROOT/target/macos/Rust-Boy.app}"
OUTPUT="${RUSTBOY_DMG_OUTPUT:-$ROOT/target/macos/Rust-Boy.dmg}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
STAGE="$TMP/stage"
mkdir -p "$STAGE"
[[ -d "$APP" ]] || { printf 'Missing app bundle: %s\n' "$APP" >&2; exit 1; }
codesign --verify --deep --strict --verbose=2 "$APP"
ditto -c -k --keepParent "$APP" "$TMP/Rust-Boy.zip"
xcrun notarytool submit "$TMP/Rust-Boy.zip" --keychain-profile "$RUSTBOY_NOTARY_PROFILE" --wait
xcrun stapler staple "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"
mkdir -p "$(dirname -- "$OUTPUT")"
rm -f "$OUTPUT"
ditto "$APP" "$STAGE/Rust-Boy.app"
ln -s /Applications "$STAGE/Applications"
hdiutil create -volname "Rust-Boy" -srcfolder "$STAGE" -ov -format UDZO "$OUTPUT" >/dev/null
codesign --force --timestamp --sign "$RUSTBOY_SIGN_IDENTITY" "$OUTPUT"
xcrun notarytool submit "$OUTPUT" --keychain-profile "$RUSTBOY_NOTARY_PROFILE" --wait
xcrun stapler staple "$OUTPUT"
xcrun stapler validate "$OUTPUT"
spctl --assess --type open --context context:primary-signature --verbose=2 "$OUTPUT"
printf 'Created and notarized %s\n' "$OUTPUT"

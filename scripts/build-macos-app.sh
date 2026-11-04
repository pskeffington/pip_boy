#!/bin/bash
set -euo pipefail
if [[ "$(uname -s)" != Darwin ]]; then
  echo "This builder requires macOS." >&2
  exit 1
fi
ROOT="$(cd -- "$(dirname -- "$0")/.." && pwd)"
TARGET="${RUSTBOY_TARGET:-$(rustc -vV | sed -n 's/^host: //p')}"
APP_NAME="${RUSTBOY_APP_NAME:-Rust-Boy}"
BUNDLE_ID="${RUSTBOY_BUNDLE_ID:-org.rustboy.desktop}"
APP_VERSION="${RUSTBOY_APP_VERSION:-$(cargo metadata --no-deps --format-version 1 --manifest-path "$ROOT/Cargo.toml" | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')}"
OUTPUT="${RUSTBOY_APP_OUTPUT:-$ROOT/target/macos/${APP_NAME}.app}"
BIN="${RUSTBOY_BINARY:-$ROOT/target/$TARGET/release/pipboy}"
if [[ ! "$BUNDLE_ID" =~ ^[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+$ ]]; then
  printf 'Invalid RUSTBOY_BUNDLE_ID: %s\n' "$BUNDLE_ID" >&2
  exit 2
fi
if [[ ! "$APP_VERSION" =~ ^[0-9]+(\.[0-9]+){1,2}$ ]]; then
  printf 'Invalid RUSTBOY_APP_VERSION: %s\n' "$APP_VERSION" >&2
  exit 2
fi
cd "$ROOT"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-13.0}"
case "$TARGET" in
  aarch64-apple-darwin) SWIFT_ARCH=arm64 ;;
  x86_64-apple-darwin) SWIFT_ARCH=x86_64 ;;
  *) printf 'Unsupported macOS target: %s\n' "$TARGET" >&2; exit 2 ;;
esac
if [[ ! -x "$BIN" ]]; then cargo build --release --locked --target "$TARGET" -p pipboy --bin pipboy; fi
[[ -x "$BIN" ]] || { printf 'Missing executable: %s\n' "$BIN" >&2; exit 1; }
APP="$OUTPUT"
CONTENTS="$APP/Contents"
MACOS="$CONTENTS/MacOS"
rm -rf "$APP"
mkdir -p "$MACOS"
swiftc -parse-as-library -target "$SWIFT_ARCH-apple-macosx$MACOSX_DEPLOYMENT_TARGET" \
  -module-cache-path "$ROOT/target/swift-module-cache" -O \
  -framework AppKit "$ROOT/macos/RustBoyLauncher.swift" -o "$MACOS/RustBoy"
install -m 755 "$BIN" "$MACOS/pipboy"
printf 'APPL????' > "$CONTENTS/PkgInfo"
cat > "$CONTENTS/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleDevelopmentRegion</key><string>en</string>
<key>CFBundleExecutable</key><string>RustBoy</string>
<key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
<key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
<key>CFBundleName</key><string>$APP_NAME</string>
<key>CFBundleDisplayName</key><string>$APP_NAME</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$APP_VERSION</string>
<key>CFBundleVersion</key><string>$APP_VERSION</string>
<key>LSMinimumSystemVersion</key><string>$MACOSX_DEPLOYMENT_TARGET</string>
<key>LSUIElement</key><true/>
<key>NSHighResolutionCapable</key><true/>
<key>NSLocalNetworkUsageDescription</key><string>Rust-Boy hosts its dashboard on this Mac for access from your browser.</string>
</dict></plist>
PLIST
/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$CONTENTS/Info.plist" >/dev/null
if [[ -n "${RUSTBOY_SIGN_IDENTITY:-}" ]]; then
  codesign --force --options runtime --timestamp --sign "$RUSTBOY_SIGN_IDENTITY" "$MACOS/pipboy"
  codesign --force --options runtime --timestamp --sign "$RUSTBOY_SIGN_IDENTITY" "$MACOS/RustBoy"
  codesign --force --options runtime --timestamp --sign "$RUSTBOY_SIGN_IDENTITY" "$APP"
  codesign --verify --deep --strict --verbose=2 "$APP"
fi
printf 'Built %s (%s)\n' "$APP" "$TARGET"
if [[ -n "${RUSTBOY_SIGN_IDENTITY:-}" ]]; then
  echo "Signed Developer ID app; notarization not performed."
else
  echo "Unsigned local-validation app; do not distribute."
fi

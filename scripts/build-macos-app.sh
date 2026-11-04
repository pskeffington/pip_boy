#!/bin/bash
set -euo pipefail
if [[ "$(uname -s)" != Darwin ]]; then
  echo "This builder requires macOS." >&2
  exit 1
fi
ROOT="$(cd -- "$(dirname -- "$0")/.." && pwd)"
REQUESTED_TARGET="${RUSTBOY_TARGET:-$(rustc -vV | sed -n 's/^host: //p')}"
APP_NAME="${RUSTBOY_APP_NAME:-Rust-Boy}"
BUNDLE_ID="${RUSTBOY_BUNDLE_ID:-org.rustboy.desktop}"
APP_VERSION="${RUSTBOY_APP_VERSION:-$(cargo metadata --no-deps --format-version 1 --manifest-path "$ROOT/Cargo.toml" | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')}"
OUTPUT="${RUSTBOY_APP_OUTPUT:-$ROOT/target/macos/${APP_NAME}.app}"
if [[ ! "$BUNDLE_ID" =~ ^[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+$ ]]; then
  printf 'Invalid RUSTBOY_BUNDLE_ID: %s\n' "$BUNDLE_ID" >&2
  exit 2
fi
if [[ ! "$APP_NAME" =~ ^[A-Za-z0-9][A-Za-z0-9._\ -]{0,63}$ ]]; then
  printf 'Invalid RUSTBOY_APP_NAME: %s\n' "$APP_NAME" >&2
  exit 2
fi
if [[ ! "$APP_VERSION" =~ ^[0-9]+(\.[0-9]+){1,2}$ ]]; then
  printf 'Invalid RUSTBOY_APP_VERSION: %s\n' "$APP_VERSION" >&2
  exit 2
fi
cd "$ROOT"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-13.0}"
case "$REQUESTED_TARGET" in
  aarch64-apple-darwin|arm64)
    TARGETS=(aarch64-apple-darwin)
    ARCHES=(arm64)
    ;;
  x86_64-apple-darwin|x86_64)
    TARGETS=(x86_64-apple-darwin)
    ARCHES=(x86_64)
    ;;
  universal|universal-apple-darwin)
    TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)
    ARCHES=(arm64 x86_64)
    ;;
  *) printf 'Unsupported macOS target: %s\n' "$REQUESTED_TARGET" >&2; exit 2 ;;
esac
if [[ -n "${RUSTBOY_BINARY:-}" && "${#TARGETS[@]}" -ne 1 ]]; then
  echo "RUSTBOY_BINARY override requires a single-architecture target." >&2
  exit 2
fi
STAGING="$ROOT/target/macos-build/${APP_NAME}"
rm -rf "$STAGING"
mkdir -p "$STAGING"
for index in "${!TARGETS[@]}"; do
  TARGET="${TARGETS[$index]}"
  ARCH="${ARCHES[$index]}"
  BIN="${RUSTBOY_BINARY:-$ROOT/target/$TARGET/release/pipboy}"
  if [[ ! -x "$BIN" ]]; then cargo build --release --locked --target "$TARGET" -p pipboy --bin pipboy; fi
  [[ -x "$BIN" ]] || { printf 'Missing executable: %s\n' "$BIN" >&2; exit 1; }
  install -m 755 "$BIN" "$STAGING/pipboy-$ARCH"
  swiftc -parse-as-library -target "$ARCH-apple-macosx$MACOSX_DEPLOYMENT_TARGET" \
    -module-cache-path "$ROOT/target/swift-module-cache" -O \
    -framework AppKit "$ROOT/macos/RustBoyLauncher.swift" -o "$STAGING/RustBoy-$ARCH"
done
APP="$OUTPUT"
CONTENTS="$APP/Contents"
MACOS="$CONTENTS/MacOS"
rm -rf "$APP"
mkdir -p "$MACOS"
if [[ "${#ARCHES[@]}" -eq 1 ]]; then
  install -m 755 "$STAGING/RustBoy-${ARCHES[0]}" "$MACOS/RustBoy"
  install -m 755 "$STAGING/pipboy-${ARCHES[0]}" "$MACOS/pipboy"
else
  lipo -create "$STAGING/RustBoy-arm64" "$STAGING/RustBoy-x86_64" -output "$MACOS/RustBoy"
  lipo -create "$STAGING/pipboy-arm64" "$STAGING/pipboy-x86_64" -output "$MACOS/pipboy"
fi
chmod 755 "$MACOS/RustBoy" "$MACOS/pipboy"
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
printf 'Built %s (%s)\n' "$APP" "$REQUESTED_TARGET"
if [[ -n "${RUSTBOY_SIGN_IDENTITY:-}" ]]; then
  echo "Signed Developer ID app; notarization not performed."
else
  echo "Unsigned local-validation app; do not distribute."
fi

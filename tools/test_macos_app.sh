#!/bin/bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "$0")/.." && pwd)"
TMP="$(mktemp -d)"
APP="$TMP/Rust-Boy.app"
READY="$TMP/ready"
LIFECYCLE_RESULT="$TMP/lifecycle-result"
APP_PID=""
SERVER_PID=""
LIFECYCLE_APP_PID=""
LIFECYCLE_SERVER_PID=""
BUNDLE_ID="org.rustboy.desktop.test"
cleanup() {
  [[ -n "$SERVER_PID" ]] && kill "$SERVER_PID" 2>/dev/null || true
  [[ -n "$APP_PID" ]] && kill "$APP_PID" 2>/dev/null || true
  [[ -n "$LIFECYCLE_SERVER_PID" ]] && kill "$LIFECYCLE_SERVER_PID" 2>/dev/null || true
  [[ -n "$LIFECYCLE_APP_PID" ]] && kill "$LIFECYCLE_APP_PID" 2>/dev/null || true
  [[ -n "$APP_PID" ]] && wait "$APP_PID" 2>/dev/null || true
  [[ -n "$LIFECYCLE_APP_PID" ]] && wait "$LIFECYCLE_APP_PID" 2>/dev/null || true
  rm -rf "$TMP"
}
trap cleanup EXIT

cd "$ROOT"
cargo build --locked -p pipboy --bin pipboy
RUSTBOY_BUNDLE_ID="$BUNDLE_ID" RUSTBOY_BINARY="$ROOT/target/debug/pipboy" RUSTBOY_APP_OUTPUT="$APP" \
  bash scripts/build-macos-app.sh
/usr/libexec/PlistBuddy -c 'Print :CFBundlePackageType' "$APP/Contents/Info.plist" | grep -qx APPL
/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$APP/Contents/Info.plist" | grep -qx RustBoy
test -x "$APP/Contents/MacOS/RustBoy"
test -x "$APP/Contents/MacOS/pipboy"
file "$APP/Contents/MacOS/pipboy" | grep -q 'Mach-O'
if RUSTBOY_SIGN_IDENTITY= RUSTBOY_NOTARY_PROFILE= bash scripts/package-macos-dmg.sh; then
  echo "Unsigned DMG packaging unexpectedly succeeded." >&2
  exit 1
else
  test "$?" -eq 2
fi

PIPBOY_HOME="$TMP/data" RUSTBOY_HEADLESS_SMOKE=1 RUSTBOY_APP_BUNDLE_PATH="$APP" RUSTBOY_READY_FILE="$READY" \
  "$APP/Contents/MacOS/RustBoy" &
APP_PID=$!
for _ in $(seq 1 100); do
  [[ -s "$READY" ]] && break
  sleep 0.1
done
test -s "$READY"
URL="$(sed -n '1p' "$READY")"
SERVER_PID="$(sed -n '2p' "$READY")"
[[ "$URL" =~ ^http://127\.0\.0\.1:[0-9]+/$ ]]
[[ "$SERVER_PID" =~ ^[0-9]+$ ]]
curl --fail --silent --show-error "$URL" | grep -qi 'Rust-Boy'
curl --fail --silent --show-error "${URL%/}/micro" | grep -qi 'micro'
printf 'PASS macOS app bundle build and loopback dashboard launch: %s\n' "$URL"

if [[ "${RUSTBOY_TEST_APPKIT:-0}" == "1" ]]; then
  PIPBOY_HOME="$TMP/appkit-data" RUSTBOY_LIFECYCLE_SMOKE=1 \
    RUSTBOY_LIFECYCLE_RESULT_FILE="$LIFECYCLE_RESULT" \
    "$APP/Contents/MacOS/RustBoy" &
  LIFECYCLE_APP_PID=$!
  for _ in $(seq 1 150); do
    [[ -s "$LIFECYCLE_RESULT" ]] && break
    if ! kill -0 "$LIFECYCLE_APP_PID" 2>/dev/null; then
      wait "$LIFECYCLE_APP_PID"
    fi
    sleep 0.1
  done
  test -s "$LIFECYCLE_RESULT"
  LIFECYCLE_SERVER_PID="$(sed -n '1p' "$LIFECYCLE_RESULT")"
  [[ "$LIFECYCLE_SERVER_PID" =~ ^[0-9]+$ ]]
  APPKIT_PID="$LIFECYCLE_APP_PID"
  wait "$LIFECYCLE_APP_PID"
  LIFECYCLE_APP_PID=""
  ! kill -0 "$APPKIT_PID" 2>/dev/null
  ! kill -0 "$LIFECYCLE_SERVER_PID" 2>/dev/null
  printf 'PASS AppKit lifecycle and Stop action terminated the dashboard child.\n'
fi

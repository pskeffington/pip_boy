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
FAILURE_APP_PID=""
QUIT_LIVE_APP_PID=""
BUNDLE_ID="org.rustboy.desktop.test"
cleanup() {
  for pid in "$SERVER_PID" "$APP_PID" "$LIFECYCLE_SERVER_PID" \
    "$LIFECYCLE_APP_PID" "$FAILURE_APP_PID" "$QUIT_LIVE_APP_PID"; do
    [[ -n "$pid" ]] || continue
    for child_pid in $(pgrep -P "$pid" 2>/dev/null || true); do
      kill -KILL "$child_pid" 2>/dev/null || true
    done
    kill -KILL "$pid" 2>/dev/null || true
  done
  [[ -n "$APP_PID" ]] && wait "$APP_PID" 2>/dev/null || true
  [[ -n "$LIFECYCLE_APP_PID" ]] && wait "$LIFECYCLE_APP_PID" 2>/dev/null || true
  [[ -n "$FAILURE_APP_PID" ]] && wait "$FAILURE_APP_PID" 2>/dev/null || true
  [[ -n "$QUIT_LIVE_APP_PID" ]] && wait "$QUIT_LIVE_APP_PID" 2>/dev/null || true
  rm -rf "$TMP"
}
trap cleanup EXIT

cd "$ROOT"
cargo build --locked -p pipboy --bin pipboy
RUSTBOY_BUNDLE_ID="$BUNDLE_ID" RUSTBOY_BINARY="$ROOT/target/debug/pipboy" RUSTBOY_APP_OUTPUT="$APP" \
  bash scripts/build-macos-app.sh
/usr/libexec/PlistBuddy -c 'Print :CFBundlePackageType' "$APP/Contents/Info.plist" | grep -qx APPL
/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$APP/Contents/Info.plist" | grep -qx RustBoy
/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$APP/Contents/Info.plist" | grep -qx "$BUNDLE_ID"
/usr/libexec/PlistBuddy -c 'Print :CFBundleIconFile' "$APP/Contents/Info.plist" | grep -qx RustBoy
/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist" | grep -Eq '^[0-9]+(\.[0-9]+){1,2}$'
/usr/libexec/PlistBuddy -c 'Print :NSHumanReadableCopyright' "$APP/Contents/Info.plist" | grep -q 'Rust-Boy contributors'
/usr/libexec/PlistBuddy -c 'Print :LSApplicationCategoryType' "$APP/Contents/Info.plist" | grep -qx 'public.app-category.utilities'
test -x "$APP/Contents/MacOS/RustBoy"
test -x "$APP/Contents/MacOS/pipboy"
test -s "$APP/Contents/Resources/RustBoy.icns"
file "$APP/Contents/Resources/RustBoy.icns" | grep -qi 'Mac OS X icon'
if [[ "${RUSTBOY_VERIFY_ICNS:-0}" == "1" ]]; then
  iconutil --convert iconset --output "$TMP/RustBoy.iconset" "$APP/Contents/Resources/RustBoy.icns"
  test -s "$TMP/RustBoy.iconset/icon_512x512@2x.png"
fi
file "$APP/Contents/MacOS/pipboy" | grep -q 'Mach-O'
if RUSTBOY_BUNDLE_ID="$BUNDLE_ID" RUSTBOY_APP_OUTPUT="$ROOT" bash scripts/build-macos-app.sh; then
  echo "App builder unexpectedly accepted the repository root as its output." >&2
  exit 1
else
  test "$?" -eq 2
fi
PRESERVED_APP="$TMP/Preserved.app"
cp -R "$APP" "$PRESERVED_APP"
PRESERVED_SERVICE_SHA="$(shasum -a 256 "$PRESERVED_APP/Contents/MacOS/pipboy" | awk '{print $1}')"
if MACOSX_DEPLOYMENT_TARGET=not-a-version RUSTBOY_BUNDLE_ID="$BUNDLE_ID" \
  RUSTBOY_BINARY="$ROOT/target/debug/pipboy" RUSTBOY_APP_OUTPUT="$PRESERVED_APP" \
  bash scripts/build-macos-app.sh; then
  echo "App builder unexpectedly accepted an invalid deployment target." >&2
  exit 1
fi
test "$(shasum -a 256 "$PRESERVED_APP/Contents/MacOS/pipboy" | awk '{print $1}')" = "$PRESERVED_SERVICE_SHA"
printf 'PASS failed rebuild preserved the existing app bundle.\n'
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

wait_for_app_exit() {
  local pid="$1"
  for _ in $(seq 1 300); do
    if ! kill -0 "$pid" 2>/dev/null; then
      wait "$pid"
      return
    fi
    sleep 0.1
  done
  kill -KILL "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  printf 'AppKit smoke process %s did not exit within 30 seconds.\n' "$pid" >&2
  return 1
}

if [[ "${RUSTBOY_TEST_APPKIT:-0}" == "1" ]]; then
  PIPBOY_HOME="$TMP/appkit-data" RUSTBOY_LIFECYCLE_SMOKE=1 RUSTBOY_DISABLE_BROWSER_OPEN=1 \
    RUSTBOY_LIFECYCLE_RESULT_FILE="$LIFECYCLE_RESULT" \
    "$APP/Contents/MacOS/RustBoy" &
  LIFECYCLE_APP_PID=$!
  for _ in $(seq 1 150); do
    grep -q '^stopped_child_pid=' "$LIFECYCLE_RESULT" 2>/dev/null && break
    if ! kill -0 "$LIFECYCLE_APP_PID" 2>/dev/null; then
      wait "$LIFECYCLE_APP_PID"
    fi
    sleep 0.1
  done
  test -s "$LIFECYCLE_RESULT"
  LIFECYCLE_SERVER_PID="$(sed -n 's/^stopped_child_pid=//p' "$LIFECYCLE_RESULT")"
  [[ "$LIFECYCLE_SERVER_PID" =~ ^[0-9]+$ ]]
  APPKIT_PID="$LIFECYCLE_APP_PID"
  wait_for_app_exit "$LIFECYCLE_APP_PID"
  LIFECYCLE_APP_PID=""
  ! kill -0 "$APPKIT_PID" 2>/dev/null
  ! kill -0 "$LIFECYCLE_SERVER_PID" 2>/dev/null
  grep -qx 'desktop_action=true' "$LIFECYCLE_RESULT"
  grep -qx 'micro_action=true' "$LIFECYCLE_RESULT"
  grep -qx 'stop_action=true' "$LIFECYCLE_RESULT"
  printf 'PASS AppKit menu actions opened both surfaces, stopped the child and quit.\n'

  BROKEN_APP="$TMP/Rust-Boy-broken.app"
  FAILURE_RESULT="$TMP/failure-result"
  cp -R "$APP" "$BROKEN_APP"
  rm "$BROKEN_APP/Contents/MacOS/pipboy"
  PIPBOY_HOME="$TMP/failure-data" RUSTBOY_FAILURE_SMOKE=1 RUSTBOY_DISABLE_BROWSER_OPEN=1 \
    RUSTBOY_FAILURE_RESULT_FILE="$FAILURE_RESULT" "$BROKEN_APP/Contents/MacOS/RustBoy" &
  FAILURE_APP_PID=$!
  for _ in $(seq 1 100); do
    [[ -s "$FAILURE_RESULT" ]] && break
    if ! kill -0 "$FAILURE_APP_PID" 2>/dev/null; then wait "$FAILURE_APP_PID"; fi
    sleep 0.1
  done
  test -s "$FAILURE_RESULT"
  wait_for_app_exit "$FAILURE_APP_PID"
  FAILURE_APP_PID=""
  grep -qx 'retry_dispatched=true' "$FAILURE_RESULT"
  grep -qx 'retry_available=true' "$FAILURE_RESULT"
  grep -q '^state=Dashboard unavailable' "$FAILURE_RESULT"
  printf 'PASS AppKit shows a retry action after the bundled service fails to start.\n'

  QUIT_LIVE_RESULT="$TMP/quit-live-result"
  PIPBOY_HOME="$TMP/quit-live-data" RUSTBOY_QUIT_LIVE_SMOKE=1 RUSTBOY_DISABLE_BROWSER_OPEN=1 \
    RUSTBOY_LIFECYCLE_RESULT_FILE="$QUIT_LIVE_RESULT" "$APP/Contents/MacOS/RustBoy" &
  QUIT_LIVE_APP_PID=$!
  for _ in $(seq 1 150); do
    grep -q '^quit_stopped_child_pid=' "$QUIT_LIVE_RESULT" 2>/dev/null && break
    if ! kill -0 "$QUIT_LIVE_APP_PID" 2>/dev/null; then wait "$QUIT_LIVE_APP_PID"; fi
    sleep 0.1
  done
  test -s "$QUIT_LIVE_RESULT"
  wait_for_app_exit "$QUIT_LIVE_APP_PID"
  QUIT_LIVE_APP_PID=""
  grep -qx 'quit_action=true' "$QUIT_LIVE_RESULT"
  QUIT_STOPPED_PID="$(sed -n 's/^quit_stopped_child_pid=//p' "$QUIT_LIVE_RESULT")"
  [[ "$QUIT_STOPPED_PID" =~ ^[0-9]+$ ]]
  ! kill -0 "$QUIT_STOPPED_PID" 2>/dev/null
  printf 'PASS Quit waits asynchronously for the live dashboard child to exit.\n'
fi

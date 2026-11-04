#!/bin/bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "$0")/.." && pwd)"
TMP="$(mktemp -d)"
APP="$TMP/Rust-Boy-universal.app"
READY="$TMP/ready"
APP_PID=""
SERVER_PID=""
cleanup() {
  [[ -n "$SERVER_PID" ]] && kill "$SERVER_PID" 2>/dev/null || true
  [[ -n "$APP_PID" ]] && kill "$APP_PID" 2>/dev/null || true
  [[ -n "$APP_PID" ]] && wait "$APP_PID" 2>/dev/null || true
  rm -rf "$TMP"
}
trap cleanup EXIT

cd "$ROOT"
RUSTBOY_TARGET=universal-apple-darwin RUSTBOY_APP_OUTPUT="$APP" \
  bash scripts/build-macos-app.sh
for executable in RustBoy pipboy; do
  ARCHES="$(lipo -archs "$APP/Contents/MacOS/$executable")"
  [[ " $ARCHES " == *" arm64 "* && " $ARCHES " == *" x86_64 "* ]]
done

PIPBOY_HOME="$TMP/data" RUSTBOY_HEADLESS_SMOKE=1 RUSTBOY_APP_BUNDLE_PATH="$APP" \
  RUSTBOY_READY_FILE="$READY" "$APP/Contents/MacOS/RustBoy" &
APP_PID=$!
for _ in $(seq 1 100); do
  [[ -s "$READY" ]] && break
  if ! kill -0 "$APP_PID" 2>/dev/null; then wait "$APP_PID"; fi
  sleep 0.1
done
test -s "$READY"
URL="$(sed -n '1p' "$READY")"
SERVER_PID="$(sed -n '2p' "$READY")"
[[ "$URL" =~ ^http://127\.0\.0\.1:[0-9]+/$ ]]
[[ "$SERVER_PID" =~ ^[0-9]+$ ]]
curl --fail --silent --show-error "$URL" | grep -qi 'Rust-Boy'
curl --fail --silent --show-error "${URL%/}/micro" | grep -qi 'micro'
printf 'PASS universal macOS bundle contains arm64+x86_64 host and Rust service slices.\n'

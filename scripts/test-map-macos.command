#!/bin/bash
# Mac-first Rust-Boy MAP acceptance; no Pi, SDR, GPS, or external services required.
set -euo pipefail

usage() {
  cat <<'EOF'
Rust-Boy MAP / macOS local development
Usage:
  bash scripts/test-map-macos.command [--launch]
  bash scripts/test-map-macos.command --verify [--require-chromium]
  bash scripts/test-map-macos.command --help

--launch             Run the native Rust micro HUD in the Mac's default browser.
                     Uses a separate persistent, local-only map test data home.
                     COUNTY/WEB remain OFF until deliberately selected.
--verify             Build native Rust, run county-cache Rust tests, launch a
                     temporary loopback server for API smoke checks, and run
                     offline JavaScript map tests. Never downloads map sources.
--require-chromium   Additionally require and run real Chromium viewport checks
                     at 480x320, 800x480 and 320x568. Without this flag, the
                     Chromium check runs only when a supported browser exists.
--help               Print this help without touching Rust or user data.

The browser UI is JavaScript rendered by a native Rust loopback service.
Manual Mac Safari/Chrome GPS, live Census, offline restart and touch/trackpad
acceptance remain separate from automated tests.
EOF
}

mode="${1:---launch}"
if [[ "$mode" == "--help" || "$mode" == "-h" ]]; then
  usage
  exit 0
fi
if [[ "$mode" != "--launch" && "$mode" != "--verify" ]]; then
  usage >&2
  exit 2
fi
shift "$(( $# > 0 ? 1 : 0 ))"
require_chromium=false
if [[ "$mode" == "--verify" && "${1:-}" == "--require-chromium" ]]; then
  require_chromium=true
  shift
fi
if [[ "$#" -ne 0 ]]; then
  printf '%s\n' 'Unsupported flag; run with --help.' >&2
  exit 2
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  printf '%s\n' 'This native acceptance launcher runs on macOS only.' >&2
  exit 2
fi

# Finder-launched Terminal does not always inherit a developer's shell PATH.
export PATH="${HOME}/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:${PATH}"
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd -- "$ROOT"

if ! command -v cargo >/dev/null 2>&1; then
  printf '%s\n' 'Rust/Cargo is missing. Install Rust 1.90+ from https://rustup.rs.' >&2
  exit 1
fi

if [[ "$mode" == "--launch" ]]; then
  if [[ -z "${RUSTBOY_MAP_TEST_HOME:-}" ]]; then
    RUSTBOY_MAP_TEST_HOME="${HOME}/Library/Application Support/Rust-Boy/MapTest"
  fi
  case "$RUSTBOY_MAP_TEST_HOME" in
    /*) ;;
    *) printf '%s\n' 'RUSTBOY_MAP_TEST_HOME must be an absolute directory.' >&2; exit 2 ;;
  esac
  if [[ -L "$RUSTBOY_MAP_TEST_HOME" ]]; then
    printf '%s\n' 'Refusing a symlinked map-test data directory.' >&2
    exit 2
  fi
  export PIPBOY_HOME="$RUSTBOY_MAP_TEST_HOME"
  printf '%s\n' 'Native Rust-Boy / Mac MAP test session' \
    'UI: http://127.0.0.1:<chosen-port>/micro (browser opens automatically)' \
    "Test data: $PIPBOY_HOME" \
    'COUNTY and WEB only contact a map provider after explicit operator action.' \
    'Use Ctrl+C to stop. Restart the same command to test offline cache reuse.'
  exec cargo run --locked -p pipboy -- dashboard --micro --port 0 --open
fi

if ! command -v python3 >/dev/null 2>&1; then
  printf '%s\n' 'Python 3 is required for the real local Rust HTTP smoke test.' >&2
  exit 1
fi
if ! command -v node >/dev/null 2>&1; then
  printf '%s\n' 'Node.js is required for the offline MAP JavaScript tests.' >&2
  exit 1
fi

printf '%s\n' '[1/4] Build native Rust for this Mac (no cross-compile).'
cargo build --locked -p pipboy
printf '%s\n' '[2/4] Run native Rust county-cache and boundary tests.'
cargo test --locked -p pipboy -- county_cache
printf '%s\n' '[3/4] Run native Rust loopback micro HTTP smoke (temporary test home).'
python3 tools/smoke_micro.py
printf '%s\n' '[4/4] Run offline MAP browser-logic tests (no live providers).'
for test in \
  tools/test_map_boot.cjs \
  tools/test_map_gps_permissions.cjs \
  tools/test_county_sources.cjs \
  tools/test_pointlm_ingestion.cjs \
  tools/test_county_boundary_cache.cjs \
  tools/test_map_layers.cjs \
  tools/test_rustgis_map_import.cjs \
  tools/test_rustgis_map_view.cjs
do
  node "$test"
done

# Chromium is optional for ordinary Mac smoke tests: Safari can be used for
# manual acceptance. A caller can require Chromium rather than silently skip.
browser=''
for candidate in \
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
  "/Applications/Chromium.app/Contents/MacOS/Chromium" \
  "${HOME}/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
  "${HOME}/Applications/Chromium.app/Contents/MacOS/Chromium"
do
  if [[ -f "$candidate" && -x "$candidate" ]]; then
    browser="$candidate"
    break
  fi
done
if [[ -n "${CHROME_BIN:-}" ]]; then
  browser="$CHROME_BIN"
fi
if [[ -n "$browser" ]]; then
  if [[ ! -x "$browser" ]]; then
    printf '%s\n' "CHROME_BIN is not executable: $browser" >&2
    exit 1
  fi
  printf '%s\n' 'Real headless Chromium viewport acceptance (local-only synthetic fixture).'
  CHROME_BIN="$browser" node tools/test_map_viewport.cjs
elif [[ "$require_chromium" == "true" ]]; then
  printf '%s\n' 'Chromium required but not installed. Set CHROME_BIN or install Chromium/Chrome.' >&2
  exit 1
else
  printf '%s\n' 'SKIP Chromium viewport check (no supported browser found); use Safari for manual UI.'
fi
printf '%s\n' 'PASS: macOS Rust, offline MAP parser/viewport logic and loopback API smoke tests.'
printf '%s\n' 'NEXT: run --launch, choose CENTER + COUNTY, restart offline, and record field results.'

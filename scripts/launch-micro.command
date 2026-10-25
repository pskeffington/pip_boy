#!/bin/bash
# Launch Rust-Boy's dedicated loopback micro shell on macOS.
set -eu
if [ "$(uname -s)" != Darwin ]; then
  printf '%s\n' 'Mac launcher only. Run: cargo run --locked -p pipboy -- dashboard --micro' >&2
  exit 1
fi
SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd)"
exec "$SCRIPT_DIR/launch-dashboard.command" --micro "$@"

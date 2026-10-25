#!/bin/bash
# Rust-Boy local graphical workbench on macOS.
# Runs a Rust server on 127.0.0.1; no remote service or external port is opened.
set -eu

if [ "$(uname -s)" != Darwin ]; then
  printf '%s\n' 'launch-dashboard.command requires macOS. Use: cargo run -p pipboy -- dashboard' >&2
  exit 1
fi

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd)"
cd -- "$SCRIPT_DIR/.."
printf '%s\n' 'Rust-Boy local dashboard / starting from source' \
  'The browser will open to http://127.0.0.1:8765/ unless a custom port is supplied.' \
  'Press Ctrl+C in this Terminal window to stop the dashboard.'
exec cargo run --locked -p pipboy -- dashboard --open "$@"

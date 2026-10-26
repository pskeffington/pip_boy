#!/bin/bash
# Rust-Boy local graphical workbench on macOS.
# Runs a Rust server on 127.0.0.1; no remote service or external port is opened.
set -eu

if [ "$(uname -s)" != Darwin ]; then
  printf '%s\n' 'launch-dashboard.command requires macOS. Use: cargo run -p pipboy -- dashboard' >&2
  exit 1
fi

# Finder-opened .command files often lack ~/.cargo/bin and Homebrew in PATH.
export PATH="${HOME}/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:${PATH}"
if ! command -v cargo >/dev/null 2>&1; then
  printf '%s\n' 'Rust/Cargo is not available. Install Rust 1.90+ from https://rustup.rs.' >&2
  exit 1
fi
SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd)"
cd -- "$SCRIPT_DIR/.."
printf '%s\n' 'Rust-Boy local dashboard / native macOS Rust' \
  'The browser opens the exact loopback URL printed by Rust.' \
  'Press Ctrl+C in this Terminal window to stop the dashboard.'
exec cargo run --locked -p pipboy -- dashboard --open "$@"

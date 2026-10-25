#!/bin/bash
# Native macOS Terminal launcher. No downloads, saved console addresses or scans.
set -eu
if [ "${1:-}" = "--help" ]; then
  printf '%s\n' 'Rust-Boy for macOS → Fallout 4 on PlayStation' 'usage: launch-playstation.command [CONSOLE_IP[:PORT]] [--diagnose]' 'Without an address, prompts in Terminal. Requires installed Rust/Cargo and a console companion service.'
  exit 0
fi
if [ "$(uname -s)" != Darwin ]; then
  printf '%s\n' 'This launcher requires macOS. Use pipboy game --playstation IP --confirm on other systems.' >&2
  exit 1
fi
if [ "$#" -gt 2 ] || { [ "$#" -eq 2 ] && [ "$2" != --diagnose ]; }; then
  printf '%s\n' 'usage: launch-playstation.command [CONSOLE_IP[:PORT]] [--diagnose]' >&2
  exit 1
fi
export PATH="${HOME}/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:${PATH}"
if ! command -v cargo >/dev/null 2>&1; then
  printf '%s\n' 'Rust/Cargo is required. Install it from https://rustup.rs, then launch again.' >&2
  exit 1
fi
RUSTBOY_PROJECT="$(cd -- "$(dirname -- "$0")/.." && pwd)"
RUSTBOY_CONSOLE_IP="${1:-}"
if [ -z "$RUSTBOY_CONSOLE_IP" ]; then
  printf '%s\n' 'Rust-Boy → PlayStation Fallout 4' 'Enter the console LAN IP (PS4: Settings → Network → View Connection Status).'
  read -r -p 'PlayStation IP: ' RUSTBOY_CONSOLE_IP
fi
if [ -z "$RUSTBOY_CONSOLE_IP" ]; then
  printf '%s\n' 'No console address supplied.' >&2
  exit 1
fi
cd -- "$RUSTBOY_PROJECT"
# The quoted argument is parsed as an IP by Rust, never evaluated as shell code.
if [ "${2:-}" = --diagnose ]; then
  exec cargo run --locked -p pipboy -- game --playstation "$RUSTBOY_CONSOLE_IP" --confirm --diagnose
fi
exec cargo run --locked -p pipboy -- game --playstation "$RUSTBOY_CONSOLE_IP" --confirm

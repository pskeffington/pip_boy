#!/bin/bash
# Native macOS Terminal launcher. No downloads, saved console addresses or scans.
set -eu
if [ "$#" -eq 1 ] && [ "$1" = "--help" ]; then
  printf '%s\n' 'Rust-Boy for macOS → Fallout 4 on PlayStation' 'usage: launch-playstation.command [CONSOLE_IP[:PORT]] [--diagnose | --observe-seconds N | --discover-ps4]' 'Without an address, prompts in Terminal. Requires installed Rust/Cargo and a console companion service.'
  exit 0
fi
if [ "$(uname -s)" != Darwin ]; then
  printf '%s\n' 'This launcher requires macOS. Use pipboy game --playstation IP --confirm on other systems.' >&2
  exit 1
fi
RUSTBOY_CONSOLE_IP=''
RUSTBOY_DIAGNOSE=false
RUSTBOY_DISCOVER=false
RUSTBOY_OBSERVE=''
RUSTBOY_EXPECT_SECONDS=false
for RUSTBOY_ARGUMENT in "$@"; do
  if [ "$RUSTBOY_EXPECT_SECONDS" = true ]; then
    case "$RUSTBOY_ARGUMENT" in
      ''|*[!0-9]*) printf '%s\n' 'Observation duration must be 1..120 seconds.' >&2; exit 1 ;;
    esac
    if [ "$RUSTBOY_ARGUMENT" -lt 1 ] || [ "$RUSTBOY_ARGUMENT" -gt 120 ]; then
      printf '%s\n' 'Observation duration must be 1..120 seconds.' >&2
      exit 1
    fi
    RUSTBOY_OBSERVE="$RUSTBOY_ARGUMENT"
    RUSTBOY_EXPECT_SECONDS=false
    continue
  fi
  case "$RUSTBOY_ARGUMENT" in
    --observe-seconds)
      if [ "$RUSTBOY_DISCOVER" = true ] || [ "$RUSTBOY_DIAGNOSE" = true ] || [ -n "$RUSTBOY_OBSERVE" ]; then
        printf '%s\n' 'Select only one diagnostic mode.' >&2; exit 1
      fi
      RUSTBOY_EXPECT_SECONDS=true
      ;;
    --discover-ps4)
      if [ -n "$RUSTBOY_CONSOLE_IP" ] || [ "$RUSTBOY_DIAGNOSE" = true ] || [ -n "$RUSTBOY_OBSERVE" ] || [ "$RUSTBOY_DISCOVER" = true ]; then
        printf '%s\n' 'PS4 discovery must be selected alone.' >&2; exit 1
      fi
      RUSTBOY_DISCOVER=true
      ;;
    --diagnose)
      if [ "$RUSTBOY_DIAGNOSE" = true ] || [ "$RUSTBOY_DISCOVER" = true ] || [ -n "$RUSTBOY_OBSERVE" ]; then
        printf '%s\n' 'Pass --diagnose only once.' >&2
        exit 1
      fi
      RUSTBOY_DIAGNOSE=true
      ;;
    --*|'')
      printf '%s\n' 'usage: launch-playstation.command [CONSOLE_IP[:PORT]] [--diagnose]' >&2
      exit 1
      ;;
    *)
      if [ -n "$RUSTBOY_CONSOLE_IP" ]; then
        printf '%s\n' 'Pass only one console IP address.' >&2
        exit 1
      fi
      RUSTBOY_CONSOLE_IP="$RUSTBOY_ARGUMENT"
      ;;
  esac
done
export PATH="${HOME}/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:${PATH}"
if ! command -v cargo >/dev/null 2>&1; then
  printf '%s\n' 'Rust/Cargo is required. Install it from https://rustup.rs, then launch again.' >&2
  exit 1
fi
if [ "$RUSTBOY_EXPECT_SECONDS" = true ]; then
  printf '%s\n' 'Missing observation duration.' >&2
  exit 1
fi
RUSTBOY_PROJECT="$(cd -- "$(dirname -- "$0")/.." && pwd)"
if [ "$RUSTBOY_DISCOVER" = true ]; then
  cd -- "$RUSTBOY_PROJECT"
  exec cargo run --locked -p pipboy -- game --discover-ps4 --confirm
fi
if [ -z "$RUSTBOY_CONSOLE_IP" ]; then
  printf '%s\n' 'Rust-Boy → PlayStation Fallout 4' 'Enter the console LAN IP (PS4: Settings → Network → View Connection Status).' >&2
  if ! read -r -p 'PlayStation IP: ' RUSTBOY_CONSOLE_IP; then
    printf '%s\n' 'No console address received.' >&2
    exit 1
  fi
fi
if [ -z "$RUSTBOY_CONSOLE_IP" ]; then
  printf '%s\n' 'No console address supplied.' >&2
  exit 1
fi
cd -- "$RUSTBOY_PROJECT"
# The quoted argument is parsed as an IP by Rust, never evaluated as shell code.
if [ "$RUSTBOY_DIAGNOSE" = true ]; then
  exec cargo run --locked -p pipboy -- game --ps4 "$RUSTBOY_CONSOLE_IP" --confirm --diagnose
fi
if [ -n "$RUSTBOY_OBSERVE" ]; then
  exec cargo run --locked -p pipboy -- game --ps4 "$RUSTBOY_CONSOLE_IP" --confirm --observe-seconds "$RUSTBOY_OBSERVE"
fi
exec cargo run --locked -p pipboy -- game --ps4 "$RUSTBOY_CONSOLE_IP" --confirm

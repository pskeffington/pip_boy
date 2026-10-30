#!/usr/bin/env python3
"""Build independent Rust Music in /private/tmp and test Pip-Boy supervision."""
import os
import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MUSIC = Path(os.environ.get("RUST_MUSIC_REPO", Path.home() / "GitHub" / "rust_music"))
TARGET = Path(os.environ.get("RUST_BOY_PROCESS_TARGET", "/private/tmp/rust-boy-supervised-music-target"))


def run(command: list[str], *, env: dict[str, str]) -> None:
    print("+", " ".join(command), flush=True)
    subprocess.run(command, cwd=ROOT, env=env, check=True)


def main() -> int:
    if not (MUSIC / "Cargo.toml").is_file():
        print(f"Rust Music checkout not found: {MUSIC}", file=sys.stderr)
        return 2
    system = platform.system()
    feature = {"Darwin": "macos-output", "Linux": "alsa-output"}.get(system)
    if feature is None:
        print(f"Unsupported integration host: {system}", file=sys.stderr)
        return 2
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(TARGET)
    run([
        "cargo", "build", "--manifest-path", str(MUSIC / "Cargo.toml"),
        "--locked", "--offline", "-p", "rust-music-cli", "--features", feature,
        "--bin", "rust-music-cli",
    ], env=env)
    binary = TARGET / "debug" / "rust-music-cli"
    if not binary.is_file():
        print(f"Expected module binary missing: {binary}", file=sys.stderr)
        return 1
    env["RUST_MUSIC_ADAPTER_BIN"] = str(binary)
    run([
        "cargo", "test", "--locked", "--offline", "-p", "pipboy-core",
        "package_process::tests::actual_rust_music_child_runs_under_host_supervisor_when_configured",
    ], env=env)
    print("PASS real Rust Music child under Pip-Boy supervisor")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

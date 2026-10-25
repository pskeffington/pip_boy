#!/usr/bin/env python3
"""Fail CI when Rust-Boy's supported compiler version drifts across contracts.

This policy check never accesses a network or a user workspace. Historic
test evidence can mention older Rust versions without changing current MSRV.
"""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(path):
    return (ROOT / path).read_text(encoding="utf-8")


def require(condition, message):
    if not condition:
        raise ValueError(message)


cargo = read("Cargo.toml")
require(re.search(r'(?m)^rust-version\s*=\s*"1\.90"$', cargo) is not None,
        "workspace minimum supported Rust must be 1.90")
require(re.search(r'(?m)^edition\s*=\s*"2021"$', cargo) is not None,
        "Rust edition migration is a separate change")
for path in (
    "crates/pipboy-core/Cargo.toml",
    "crates/pipboy/Cargo.toml",
    "crates/rust-boy-broker/Cargo.toml",
):
    require("rust-version.workspace = true" in read(path),
            f"workspace Rust version not inherited by {path}")

ci = read(".github/workflows/rust.yml")
require('toolchain: ["stable", "1.90.0"]' in ci,
        "CI must test latest stable and exact Rust 1.90.0")
require("RUSTUP_TOOLCHAIN: ${{ matrix.toolchain }}" in ci,
        "CI must override the stable toolchain file")
require("Assert active matrix compiler" in ci
        and r"rustc 1\.90\.0 " in ci,
        "CI must assert actual Rust 1.90.0 instead of trusting matrix job labels")
require("python3 tools/validate_rust_version.py" in ci,
        "version-policy validator must be part of CI")
toolchain = read("rust-toolchain.toml")
require('channel = "stable"' in toolchain,
        "stable is the intentional local development toolchain")
readme = read("README.md")
require("Rust: 1.90+" in readme and "Rust 1.90 or newer" in readme,
        "README compiler guidance/badge does not match Cargo MSRV")
require("minimum 1.90" in read("CONTRIBUTING.md"),
        "contributor compiler guidance does not match workspace")
guide = read("docs/SCOUT_GIS_TAK.md")
require("minimum compiler has been raised to **1.90**" in guide,
        "RustGIS integration guidance still describes the old minimum")
require("Rust 1.81" not in guide, "Scout guide incorrectly says old Rust version")
print("PASS: Rust 1.90 MSRV, 2021 edition, stable dev toolchain, "
      "1.90.0+stable CI and user documentation aligned")

#!/usr/bin/env python3
"""Offline guard for first-party workflow and validation requirements."""
import re
import sys
from pathlib import Path

from validation_common import require

ROOT = Path(__file__).resolve().parents[1]


def validate_ci_policy(workflows, bootstrap):
    require(isinstance(workflows, dict) and workflows,
            "CI policy: workflow files are missing")
    require(all(isinstance(path, str) and path.endswith((".yml", ".yaml"))
                and isinstance(text, str) for path, text in workflows.items()),
            "CI policy: invalid workflow input")
    combined = "\n".join(workflows.values())
    require(not re.search(r"^\s*(?:-\s*)?uses\s*:", combined, re.MULTILINE),
            "CI policy: external GitHub Action uses: dependency is forbidden")

    for marker, message in (
        ('git -c protocol.version=2 fetch --no-tags --depth=1 origin "$GITHUB_SHA"',
         "exact triggering SHA fetch"),
        ('test "$(git rev-parse HEAD)" = "$GITHUB_SHA"',
         "exact triggering SHA assertion"),
        ('toolchain: ["stable", "1.90.0"]', "stable and Rust 1.90 matrix"),
        ('feature: [soapy-sdr, sdrconnect]', "both optional SDR feature jobs"),
        ('os: macos-latest', "native macOS job"),
        ('os: windows-latest', "native Windows job"),
        ('cargo check --workspace --all-targets --locked', "locked workspace check"),
        ('cargo test --workspace --locked', "locked workspace tests"),
        ('cargo clippy --workspace --all-targets --locked -- -D warnings',
         "warnings-denied Clippy"),
        ('bash scripts/ci_bootstrap.sh rust "${{ matrix.toolchain }}"',
         "matrix Rust bootstrap"),
        ('bash scripts/ci_bootstrap.sh rust stable', "stable Rust bootstrap"),
        ('bash scripts/ci_bootstrap.sh node', "pinned Node bootstrap"),
    ):
        require(marker in combined, f"CI policy: missing {message}")

    require(isinstance(bootstrap, str)
            and '"$toolchain" == "stable" || "$toolchain" == "1.90.0"' in bootstrap,
            "CI policy: bootstrap allows an unapproved Rust toolchain")
    require('version="22.23.4"' in bootstrap,
            "CI policy: Node.js version is not explicitly pinned")
    for platform in ("Linux/x86_64", "Darwin/arm64", "Darwin/x86_64"):
        require(platform in bootstrap,
                f"CI policy: missing pinned Node binary for {platform}")
    require(len(re.findall(r'digest="[0-9a-f]{64}"', bootstrap)) == 3,
            "CI policy: every supported Node binary must have a SHA-256 pin")
    require("https://nodejs.org/dist/v${version}/${archive}" in bootstrap,
            "CI policy: Node binary must come from the official distribution")
    require("shasum -a 256 --check --status" in bootstrap,
            "CI policy: Node binary checksum must be verified")
    return len(workflows)


def validate(root=ROOT):
    root = Path(root).resolve()
    workflows = {
        path.relative_to(root).as_posix(): path.read_text(encoding="utf-8")
        for path in sorted((root / ".github/workflows").glob("*.y*ml"))
        if path.is_file() and not path.is_symlink()
    }
    bootstrap = (root / "scripts/ci_bootstrap.sh").read_text(encoding="utf-8")
    count = validate_ci_policy(workflows, bootstrap)
    print(f"PASS: {count} workflow file(s), zero external uses:, exact-SHA checkout, "
          "stable/1.90, SDR and native matrices, pinned first-party Node bootstrap")


if __name__ == "__main__":
    try:
        validate()
    except (ValueError, OSError, UnicodeError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc

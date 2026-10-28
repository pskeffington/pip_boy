#!/usr/bin/env python3
"""Offline, read-only schema gate for Rust-Boy's four package socket fixtures.

The fixtures advertise *design references*, not installed/runnable packages.
This script uses only Python's standard library and never executes a manifest.
"""

import json
import re
import sys
from pathlib import Path

from validation_common import require

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "catalog/host_packages"
MAX_BYTES = 16 * 1024
EXPECTED = {
    "rb_gis.example.json": ("org.rustboy.gis", "MAP", "rb_gis"),
    "rust_tv.example.json": ("org.rustboy.tv", "TV", "rust_tv"),
    "rust_radio.example.json": ("org.rustboy.radio", "RADIO", "rust_radio"),
    "rusti_rc.example.json": ("org.rustboy.comms", "COMMS", "rusti_rc"),
}
SCOPES = {
    "location.read", "map.cache.read", "map.cache.write",
    "network.fetch.explicit", "media.video", "media.audio",
    "radio.device", "network.connect.explicit", "messages.local.write",
}
TOP_KEYS = {
    "schema_version", "package_id", "name", "version", "publisher",
    "license", "source_repository", "host_api", "entrypoint", "ui",
    "capabilities", "background",
}
ID_PATTERN = re.compile(r"^[a-z][a-z0-9-]*(?:\.[a-z][a-z0-9-]*){2,}$")
VERSION_PATTERN = re.compile(r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:[-+][0-9A-Za-z.+-]+)?$")


def check_manifest(m):
    """Validate an inert manifest dict with fail-closed exact field shapes."""
    require(isinstance(m, dict), "manifest must be a JSON object")
    require(TOP_KEYS <= m.keys() and m.keys() <= TOP_KEYS | {"status"},
            "missing or unexpected manifest fields")
    for field in ("schema_version", "host_api"):
        require(type(m[field]) is int and m[field] == 1,
                f"unsupported {field}")
    identifier = m["package_id"]
    require(isinstance(identifier, str)
            and len(identifier) <= 96
            and ID_PATTERN.fullmatch(identifier)
            and all(len(s) <= 32 and not s.endswith("-") for s in identifier.split(".")),
            "unsafe package_id")
    for field, limit in (("name", 48), ("publisher", 96), ("license", 64)):
        value = m[field]
        require(isinstance(value, str) and 0 < len(value.encode("utf-8")) <= limit
                and value == value.strip()
                and not any(ord(char) < 32 or ord(char) == 127 for char in value),
                f"invalid {field}")
    version = m["version"]
    require(isinstance(version, str) and len(version) <= 64
            and VERSION_PATTERN.fullmatch(version), "invalid version")
    url = m["source_repository"]
    require(isinstance(url, str) and url.startswith("https://")
            and 4 <= len(url[8:]) <= 250 and "/" in url[8:]
            and not any(char in url for char in ("@", "#", "?"))
            and not any(char.isspace() for char in url),
            "unsafe source_repository")
    entrypoint = m["entrypoint"]
    require(isinstance(entrypoint, dict) and entrypoint.keys() == {"type"}
            and entrypoint["type"] in {"builtin_adapter", "external_process"},
            "unexpected executable entrypoint instructions")
    ui = m["ui"]
    require(isinstance(ui, dict)
            and ui.keys() == {"mode", "min_width", "min_height"}
            and ui["mode"] == "host_view"
            and all(type(ui[k]) is int and minimum <= ui[k] <= 4096
                    for k, minimum in (("min_width", 320), ("min_height", 240))),
            "invalid viewport contract")
    background = m["background"]
    require(isinstance(background, dict) and background.keys() == {"allowed"}
            and type(background["allowed"]) is bool,
            "invalid background declaration")
    caps = m["capabilities"]
    require(isinstance(caps, list) and len(caps) <= 16
            and all(isinstance(c, str) and c in SCOPES for c in caps)
            and len(caps) == len(set(caps)), "unknown or duplicate capabilities")
    require(m.get("status", "design_fixture_not_installed") == "design_fixture_not_installed",
            "manifest must not claim installed or authorized state")
    return identifier


def validate(root=ROOT):
    folder = root / "catalog/host_packages"
    require(folder.is_dir() and not folder.is_symlink(), "missing or linked fixtures directory")
    names = {p.name for p in folder.iterdir()}
    require(names == set(EXPECTED), "missing or unexpected package socket fixtures")
    seen = set()
    for filename, (expected_id, expected_name, expected_repo) in EXPECTED.items():
        path = folder / filename
        require(path.is_file() and not path.is_symlink(), f"{filename}: invalid fixture file")
        require(path.stat().st_size <= MAX_BYTES, f"{filename}: manifest too large")
        manifest = json.loads(path.read_text(encoding="utf-8"))
        identifier = check_manifest(manifest)
        require(identifier == expected_id and manifest["name"] == expected_name,
                f"{filename}: unexpected package identity")
        require(manifest["source_repository"]
                == f"https://github.com/pskeffington/{expected_repo}",
                f"{filename}: mismatched upstream repository")
        require(identifier not in seen, f"{filename}: duplicate package ID")
        seen.add(identifier)
    rust = (root / "crates/pipboy-core/src/package_socket.rs").read_text(encoding="utf-8")
    referenced = re.findall(r'include_str!\("\.\./\.\./\.\./catalog/host_packages/([^"]+)"\)', rust)
    require(set(referenced) == set(EXPECTED) and len(referenced) == len(EXPECTED),
            "Rust reference registry and checked-in fixtures drift")
    print(f"PASS: {len(seen)} inert package socket manifests; no executable package installed")
    return len(seen)


if __name__ == "__main__":
    try:
        validate()
    except (ValueError, OSError, UnicodeError, json.JSONDecodeError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc

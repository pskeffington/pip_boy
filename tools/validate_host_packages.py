#!/usr/bin/env python3
"""Offline, read-only schema gate for Rust-Boy's five package socket fixtures.

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
    "trans_scan.example.json": ("org.rustboy.scan", "SCAN", "pip_boy"),
}
SCAN_SCOPES = {
    "scan.camera.enumerate",
    "scan.camera.preview",
    "scan.camera.capture",
    "scan.media.receive",
    "scan.media.send",
    "scan.file.read",
    "scan.local.write",
    "scan.infer",
}
SCOPES = {
    "location.read", "map.cache.read", "map.cache.write",
    "network.fetch.explicit", "media.video", "media.audio",
    "radio.device", "network.connect.explicit", "messages.local.write",
} | SCAN_SCOPES
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
    if identifier == "org.rustboy.scan":
        require(background["allowed"] is False, "SCAN background capture must stay disabled")
    else:
        require(not (set(caps) & SCAN_SCOPES), "SCAN capability namespace reserved for SCAN")
    require(m.get("status", "design_fixture_not_installed") == "design_fixture_not_installed",
            "manifest must not claim installed or authorized state")
    return identifier



def check_adapter_sdk(root=ROOT):
    """Keep the portable SDK dependency-light and execution-free by contract."""
    workspace = (root / "Cargo.toml").read_text(encoding="utf-8")
    require('"crates/rust-boy-adapter-sdk"' in workspace,
            "universal adapter SDK missing from workspace")
    sdk_manifest = (
        root / "crates/rust-boy-adapter-sdk/Cargo.toml"
    ).read_text(encoding="utf-8")
    require('name = "rust-boy-adapter-sdk"' in sdk_manifest,
            "SDK package identity drift")
    require("[dependencies]" in sdk_manifest,
            "SDK dependency section missing")
    deps = sdk_manifest.split("[dependencies]", 1)[1]
    declared = set(re.findall(r"(?m)^([A-Za-z0-9_-]+)\s*=", deps))
    require(declared == {"serde", "serde_json"},
            f"unreviewed SDK dependency: {declared}")
    core_manifest = (
        root / "crates/pipboy-core/Cargo.toml"
    ).read_text(encoding="utf-8")
    require('rust-boy-adapter-sdk = { path = "../rust-boy-adapter-sdk" }'
            in core_manifest, "core must depend on portable SDK via local path")
    source = (
        root / "crates/rust-boy-adapter-sdk/src/lib.rs"
    ).read_text(encoding="utf-8")
    required = (
        "pub trait UniversalPackageAdapter",
        "pub struct AdapterEnvelope",
        "pub enum AdapterCommand",
        "pub enum AdapterReply",
        "pub struct DenyAllScopes",
        "#![forbid(unsafe_code)]",
    )
    require(all(key in source for key in required),
            "missing portable socket API or unsafe-code boundary")
    forbidden = (
        "std::net::", "std::process::", "std::fs::",
        "tokio::net::", "unsafe {", "libc::", "btleplug::",
    )
    require(not any(key in source for key in forbidden),
            "portable SDK must not contain direct hardware/network/process I/O")
    host = (
        root / "crates/pipboy-core/src/universal_adapter.rs"
    ).read_text(encoding="utf-8")
    require("pub fn mount_trusted(" in host
            and "pub fn unmount(" in host
            and "pub fn open_settings(" in host,
            "missing trusted host lifecycle contract")




def check_adapter_contract(root=ROOT):
    """Keep the machine-readable contract aligned with the actual SDK."""
    contract = json.loads(
        (root / "catalog/host_adapter_contract.json").read_text(encoding="utf-8")
    )
    require(contract.get("schema_version") == 1
            and contract.get("adapter_protocol") == 1
            and contract.get("manifest_host_api") == 1,
            "adapter protocol index version mismatch")
    require(contract.get("max_wire_bytes") == 16384
            and contract.get("max_intent_bytes") == 4096,
            "adapter protocol size limit drift")
    require(contract.get("commands") ==
            ["activate", "suspend", "resume", "resize", "intent", "shutdown"],
            "adapter command index drift")
    require(contract.get("replies") == ["ack", "view"],
            "adapter reply index drift")
    require(contract.get("host_surfaces") == ["empty", "settings", "package"],
            "adapter host surface index drift")
    require(contract.get("package_ids") == sorted(v[0] for v in EXPECTED.values()),
            "adapter target list differs from manifest fixtures")
    sdk = (
        root / "crates/rust-boy-adapter-sdk/src/lib.rs"
    ).read_text(encoding="utf-8")
    require("pub const ADAPTER_PROTOCOL: u32 = 1;" in sdk
            and "pub const MAX_WIRE_BYTES: usize = 16 * 1024;" in sdk
            and "pub const MAX_INTENT_BYTES: usize = 4 * 1024;" in sdk,
            "adapter machine index and Rust constants disagree")
    require(contract.get("third_party_execution") == "not_implemented",
            "adapter contract must not claim unreviewed external execution")



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
        require(manifest.get("status") == "design_fixture_not_installed",
                f"{filename}: explicit inert design status required")
        if identifier == "org.rustboy.scan":
            require(set(manifest["capabilities"]) == SCAN_SCOPES,
                    "SCAN fixture scope vocabulary drift")
            require(manifest["entrypoint"]["type"] == "builtin_adapter",
                    "SCAN fixture may not declare process execution")
        require(identifier not in seen, f"{filename}: duplicate package ID")
        seen.add(identifier)
    rust = (root / "crates/pipboy-core/src/package_socket.rs").read_text(encoding="utf-8")
    referenced = re.findall(r'include_str!\("\.\./\.\./\.\./catalog/host_packages/([^"]+)"\)', rust)
    require(set(referenced) == set(EXPECTED) and len(referenced) == len(EXPECTED),
            "Rust reference registry and checked-in fixtures drift")
    check_adapter_sdk(root)
    check_adapter_contract(root)
    print(f"PASS: {len(seen)} inert manifests and portable SDK boundary; no executable package installed")
    return len(seen)


if __name__ == "__main__":
    try:
        validate()
    except (ValueError, OSError, UnicodeError, json.JSONDecodeError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc

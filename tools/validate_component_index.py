#!/usr/bin/env python3
"""Offline component ownership and portability index validation.

Validates repo-relative entrypoints and acceptance labels. Never scans local
operator storage, touches hardware, or makes outbound requests.
"""

import json
import re
import sys
from pathlib import Path, PurePosixPath

from validation_common import require

ROOT = Path(__file__).resolve().parents[1]
INDEX = "catalog/component_index.json"
GATES = {"software_only", "field_pending", "deny_only"}
REQUIRED_CODE = {
    "Cargo.toml",
    "crates/pipboy-core/Cargo.toml",
    "crates/pipboy/Cargo.toml",
    "crates/rust-boy-broker/Cargo.toml",
    "server/mini-server/Cargo.toml",
    "server/mini-server/src/main.rs",
    "crates/pipboy-core/src/lib.rs",
    "crates/pipboy/src/main.rs",
    "crates/rust-boy-broker/src/main.rs",
}


def checked_file(root, value, context):
    """Reject absolute paths, traversal, symlinks and external runtime data."""
    require(isinstance(value, str) and bool(value), f"{context}: empty path")
    require("\\" not in value and ":" not in value, f"{context}: invalid path {value}")
    require(not value.startswith("/") and not value.startswith("~"),
            f"{context}: absolute path {value}")
    require(not any(part in {"", ".", ".."} for part in value.split("/")),
            f"{context}: noncanonical path {value}")
    path = PurePosixPath(value)
    require(path.as_posix() == value, f"{context}: noncanonical path {value}")
    require(path.parts[0] not in {".git", "PIPBOY_HOME", "target"},
            f"{context}: runtime or private path {value}")
    root = Path(root).resolve()
    target = root.joinpath(*path.parts)
    require(target.resolve().is_relative_to(root),
            f"{context}: path leaves repository {value}")
    require(not target.is_symlink() and target.is_file(),
            f"{context}: missing or symlinked source {value}")
    require(not any(parent.is_symlink() for parent in target.parents
                    if parent != root and parent.is_relative_to(root)),
            f"{context}: symlinked directory {value}")
    return value


def validate_index(root=ROOT, data=None):
    root = Path(root).resolve()
    if data is None:
        data = json.loads((root / INDEX).read_text(encoding="utf-8"))
    require(isinstance(data, dict) and data.get("schema_version") == 1,
            "component index: expected schema_version 1")
    for key in ("scope", "development_host", "prototype_target"):
        require(isinstance(data.get(key), str) and bool(data[key].strip()),
                f"component index: missing {key}")
    require(data["development_host"] == "macOS",
            "Mac-first development policy must be explicit")
    require("Raspberry Pi" in data["prototype_target"],
            "Raspberry Pi prototype remains separately tracked")
    areas = data.get("components")
    require(isinstance(areas, list) and bool(areas), "empty component index")
    seen_ids, code_paths = set(), set()
    for area in areas:
        require(isinstance(area, dict), "component must be an object")
        name = area.get("id")
        require(isinstance(name, str) and re.fullmatch(r"[a-z][a-z0-9-]*", name),
                f"invalid component ID: {name}")
        require(name not in seen_ids, f"duplicate component ID: {name}")
        seen_ids.add(name)
        require(isinstance(area.get("boundary"), str) and len(area["boundary"]) >= 15,
                f"{name}: missing responsibility boundary")
        require(area.get("hardware_gate") in GATES,
                f"{name}: unknown hardware acceptance gate")
        for field in ("code", "docs", "checks"):
            paths = area.get(field)
            require(isinstance(paths, list) and bool(paths),
                    f"{name}: empty {field} reference list")
            require(len(paths) == len(set(paths)), f"{name}: duplicate {field} path")
            for value in paths:
                checked_file(root, value, f"{name}/{field}")
                if field == "docs":
                    require(value.startswith("docs/") and value.endswith(".md"),
                            f"{name}: docs reference must be an authored guide")
                if field == "checks":
                    require(value.startswith("tools/") and
                            Path(value).name.startswith(("test_", "smoke_", "validate_")),
                            f"{name}: check must reference an explicit offline test")
                if field == "code":
                    require(value not in code_paths,
                            f"{name}: code entrypoint is owned twice: {value}")
                    code_paths.add(value)
    require(REQUIRED_CODE.issubset(code_paths),
            "component index misses Cargo manifests or executable crate entrypoints: "
            + ", ".join(sorted(REQUIRED_CODE - code_paths)))
    return len(areas), len(code_paths)


def validate():
    count, entrypoints = validate_index()
    workflow = (ROOT / ".github/workflows/rust.yml").read_text(encoding="utf-8")
    for command in ("python3 tools/validate_component_index.py",
                    "python3 tools/test_component_index.py"):
        require(command in workflow, f"CI missing component index gate: {command}")
    print(f"PASS: {count} owned components, {entrypoints} exact code entrypoints; "
          "Mac-first/Pi-future acceptance policy and CI registration")


if __name__ == "__main__":
    try:
        validate()
    except (ValueError, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        raise SystemExit(1) from error

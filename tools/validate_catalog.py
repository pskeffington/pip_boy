#!/usr/bin/env python3
"""Validate offline Rust-Boy catalogs using the Python standard library only.

Schema/graph checks do not prove Rust compilation, device safety, or licensing.
"""

import ast
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def get(relative):
    data = json.loads((ROOT / relative).read_text(encoding="utf-8"))
    if not isinstance(data, list):
        raise ValueError(f"{relative}: expected array")
    return data


def require(ok, message):
    if not ok:
        raise ValueError(message)


def identifiers(rows, field):
    names = [item[field] for item in rows]
    require(all(isinstance(x, str) and x.strip() for x in names),
            f"missing {field}")
    require(len(names) == len(set(names)), f"duplicate {field}")
    return set(names)


def validate():
    modules = get("catalog/module_plan.json")
    literature = get("catalog/literature.json")
    external = get("catalog/sources.json")
    mids = identifiers(modules, "id")
    lids = identifiers(literature, "id")
    identifiers(external, "id")
    identifiers(external, "repository")
    require(bool(modules and literature and external), "empty catalog")

    gates = {"G0": 0, "G1": 1, "G2": 2, "G3": 3, "G4": 4}
    by_id = {m["id"]: m for m in modules}
    md = (ROOT / "docs/MODULE_CATALOG.md").read_text(encoding="utf-8")
    lm = (ROOT / "docs/LITERATURE_MATRIX.md").read_text(encoding="utf-8")
    capability_source = (ROOT / "crates/pipboy-core/src/capability.rs").read_text(
        encoding="utf-8"
    )
    matches = re.findall(
        r'#\[serde\(rename = "([^"]+)"\)\]\s*(\w+),',
        capability_source,
    )
    vocabulary = dict(matches)
    require(len(vocabulary) >= 14 and len(matches) == len(vocabulary),
            "invalid capability vocabulary")

    # All future external capabilities must have an explicit preference key.
    # Persisted preferences are not active runtime grants.
    preferences_source = (
        ROOT / "crates/pipboy-core/src/permissions.rs"
    ).read_text(encoding="utf-8")
    declared_match = re.search(
        r"pub const ADAPTER_CAPABILITIES: &\[Capability\] = &\[(.*?)\];",
        preferences_source,
        flags=re.DOTALL,
    )
    require(declared_match is not None, "missing adapter capability roster")
    declared_adapters = re.findall(
        r"Capability::(\w+)", declared_match.group(1)
    )
    expected_adapters = set(vocabulary.values()) - {
        "HostRead", "FilesRead", "FilesWrite",
    }
    require(
        len(declared_adapters) == len(set(declared_adapters))
        and set(declared_adapters) == expected_adapters,
        "adapter preferences and runtime capability vocabulary disagree",
    )

    # The separate deny-all subprocess has an exact external capability
    # roster. Drift must be caught before accepting an unreviewed direction
    # such as a receiver becoming a transmitter.
    broker_source = (
        ROOT / "crates/rust-boy-broker/src/main.rs"
    ).read_text(encoding="utf-8")
    broker_match = re.search(
        r"const CAPABILITY_OPERATIONS: &\[\(&str, &str\)\] = &\[(.*?)\];",
        broker_source,
        flags=re.DOTALL,
    )
    require(broker_match is not None, "missing deny-all broker protocol roster")
    broker_pairs = re.findall(
        r'\("([^"]+)",\s*"([^"]+)"\)',
        broker_match.group(1),
    )
    require(
        len(broker_pairs) == len(set(broker_pairs))
        and {key for key, _ in broker_pairs}
        == {key for key, value in vocabulary.items() if value in expected_adapters},
        "deny-all broker capability roster differs from the trusted source",
    )
    broker_forbidden = (
        "std::net::", "std::fs::", "serialport::", "btleplug::",
        "rppal::", "std::process::Command", "CreateFileW",
    )
    require(
        not any(api in broker_source for api in broker_forbidden),
        "deny-all broker must not open external device/network/FS handles",
    )
    require(
        "allowed: false" in broker_source
        and '"physical_device_backend_disabled"' in broker_source,
        "deny-all broker lacks explicit policy denial",
    )

    # Fixed simulator inventory must map exactly one virtual fixture per
    # future capability. It must never fall back to hardware at runtime.
    simulator_source = (
        ROOT / "crates/pipboy-core/src/device_simulator.rs"
    ).read_text(encoding="utf-8")
    fixture_match = re.search(
        r"pub const SIMULATED_DEVICES: &\[SimulatedDevice\] = &\[(.*?)\];",
        simulator_source,
        flags=re.DOTALL,
    )
    require(fixture_match is not None, "missing simulated device inventory")
    fixture_ids = re.findall(r'id:\s*"(sim:[^"]+)"', fixture_match.group(1))
    fixture_capabilities = re.findall(
        r"capability:\s*Capability::(\w+)", fixture_match.group(1)
    )
    require(
        len(fixture_ids) == len(fixture_capabilities)
        and len(fixture_ids) == len(set(fixture_ids))
        and len(fixture_capabilities) == len(set(fixture_capabilities))
        and set(fixture_capabilities) == expected_adapters,
        "simulated device fixture roster disagrees with capability keys",
    )


    # The G2 precursor must not quietly acquire live I/O dependencies.
    # This lexical tripwire supplements (but cannot replace) OS isolation
    # and a reviewed physical-device broker.
    no_hardware_sources = (
        "device_simulator.rs",
        "simulator_replay.rs",
        "virtual_transport.rs",
        "mock_adapters.rs",
    )
    prohibited_api = (
        "std::net::", "std::fs::", "std::os::", "std::process::Command",
        "serialport::", "rppal::", "btleplug::", "tokio::net::",
        "winapi::", "windows_sys::", "CreateFileW",
    )
    for name in no_hardware_sources:
        source = (
            ROOT / "crates/pipboy-core/src" / name
        ).read_text(encoding="utf-8")
        violations = [api for api in prohibited_api if api in source]
        require(not violations, f"{name}: forbidden direct device/I-O API {violations}")

    # Validate every first-party Python CI harness before selecting a platform.
    for harness in sorted((ROOT / "tools").glob("*.py")):
        try:
            ast.parse(harness.read_text(encoding="utf-8"), filename=str(harness))
        except SyntaxError as exc:
            raise ValueError(f"{harness.name}: Python syntax error: {exc}") from exc


    for entry in literature:
        eid = entry["id"]
        require(entry["url"].startswith("https://"), f"{eid}: insecure URL")
        require(all(entry.get(k) for k in (
            "title", "contributors", "publisher", "design", "limitation", "acceptance"
        )), f"{eid}: incomplete citation")
        require(entry["modules"] and set(entry["modules"]) <= mids,
                f"{eid}: unknown mapped module")
        require(f'<a id="{eid.lower()}"></a>' in lm,
                f"{eid}: missing Markdown reference")

    for m in modules:
        mid = m["id"]
        require(m["gate"] in gates and m["priority"] in ("P0", "P1", "P2"),
                f"{mid}: invalid gate or priority")
        require(m["status"] in ("planned", "implemented_unverified"),
                f"{mid}: invalid implementation status")
        require(m["status"] != "implemented_unverified" or m["gate"] == "G0",
                f"{mid}: unverified implementation incorrectly staged")
        require(m["features"] and len(m["acceptance"]) >= 2,
                f"{mid}: missing acceptance criteria")
        require(m["evidence"] and set(m["evidence"]) <= lids,
                f"{mid}: missing literature reference")
        require(len(m["capabilities"]) == len(set(m["capabilities"]))
                and set(m["capabilities"]) <= vocabulary.keys(),
                f"{mid}: unknown/duplicate capability")
        require("### " + chr(96) + mid + chr(96) in md,
                f"{mid}: missing documentation")
        require(len(m["depends_on"]) == len(set(m["depends_on"])),
                f"{mid}: duplicate dependency")
        for dep in m["depends_on"]:
            require(dep in mids, f"{mid}: missing dependency {dep}")
            require(gates[by_id[dep]["gate"]] <= gates[m["gate"]],
                    f"{mid}: depends on later gate")

    completed = set()
    for _ in modules:
        for m in modules:
            if set(m["depends_on"]) <= completed:
                completed.add(m["id"])
    require(completed == mids, "cyclic module dependency graph")

    registry = (ROOT / "crates/pipboy-core/src/registry.rs").read_text(
        encoding="utf-8"
    )
    blocks = re.findall(r"Module\s*\{(.*?)\n\s*\},", registry, flags=re.DOTALL)
    checked = 0
    for block in blocks:
        name = re.search(r'\bid:\s*"([^"]+)"', block)
        if name is None:
            continue
        checked += 1
        mid = name.group(1)
        require(mid in by_id, f"unknown runtime module {mid}")
        require(("Readiness::Ready" in block)
                == (by_id[mid]["status"] == "implemented_unverified"),
                f"{mid}: readiness disagreement")
        declared = re.findall(r"Capability::(\w+)", block)
        expected = [vocabulary[key] for key in by_id[mid]["capabilities"]]
        require(declared == expected, f"{mid}: capability disagreement")
    require(checked >= 8, "incomplete runtime registry")

    for item in external:
        require(item["repository"].count("/") == 1,
                f"{item['id']}: malformed repository")
        require(item["disposition"] == "reference-only",
                f"{item['id']}: unreviewed source import")

    print(
        f"PASS: {len(modules)} modules, {len(literature)} evidence records, "
        f"{len(external)} external sources, {checked} registered entries"
    )


if __name__ == "__main__":
    try:
        validate()
    except (ValueError, KeyError, TypeError, OSError, json.JSONDecodeError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc

#!/usr/bin/env python3
"""Validate Rust-Boy hardware test inventory without touching physical hardware."""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = ROOT / "catalog" / "hardware_test_inventory.json"
VALID_CLASSES = {"reference", "compatibility"}
VALID_IDENTITIES = {"unverified", "verified"}


def validate(data):
    errors = []
    if not isinstance(data, dict) or data.get("schema_version") != 1:
        return ["unsupported hardware inventory schema"]
    profiles = data.get("profiles")
    if not isinstance(profiles, list) or not profiles:
        return ["profiles must be a nonempty list"]
    ids = set()
    references = []
    for index, profile in enumerate(profiles):
        if not isinstance(profile, dict):
            errors.append(f"profile {index}: invalid object")
            continue
        pid = profile.get("id")
        if not isinstance(pid, str) or not pid or pid in ids:
            errors.append(f"profile {index}: missing or duplicate id")
        else:
            ids.add(pid)
        if profile.get("class") not in VALID_CLASSES:
            errors.append(f"{pid}: invalid class")
        if profile.get("identity") not in VALID_IDENTITIES:
            errors.append(f"{pid}: invalid identity")
        if profile.get("class") == "reference":
            references.append(pid)
        elif profile.get("release_authority") != "none":
            errors.append(f"{pid}: compatibility device cannot grant release")
        if profile.get("identity") == "unverified":
            if profile.get("ram_bytes") is not None or profile.get("architecture") != "unverified":
                errors.append(f"{pid}: unverified hardware must not invent RAM/architecture")
        gates = profile.get("gates")
        if not isinstance(gates, list) or not gates or len(set(gates)) != len(gates):
            errors.append(f"{pid}: gates must be nonempty and unique")
    if references != [data.get("reference_release_target")]:
        errors.append("exactly one reference must match reference_release_target")
    safety = data.get("safety", {})
    for field in ("auto_network_probe", "auto_hardware_activation", "collect_serial_number",
                  "collect_ip_or_mac_address", "accept_simulated_result_as_hardware"):
        if not isinstance(safety, dict) or safety.get(field) is not False:
            errors.append(f"safety.{field} must be false")
    return errors


def main():
    try:
        data = json.loads(INVENTORY.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"FAIL: cannot read hardware inventory: {exc}", file=sys.stderr)
        return 1
    errors = validate(data)
    if errors:
        for error in errors:
            print(f"FAIL: {error}", file=sys.stderr)
        return 1
    print(f"PASS: {len(data['profiles'])} inventory profiles; no hardware probe performed")
    return 0


if __name__ == "__main__":
    sys.exit(main())

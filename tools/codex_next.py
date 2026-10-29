#!/usr/bin/env python3
"""Read-only, offline Codex pass handoff from the validated machine index."""
import argparse
import json
import sys

from validate_codex_passes import ROOT, validate_passes


def build_handoff(data):
    """Deterministic sorted readiness view; does not run or schedule a pass."""
    validate_passes(ROOT, data)
    states = {row["id"]: row["status"] for row in data["passes"]}
    next_pass = next(row for row in data["passes"] if row["status"] == "next")
    blocked = [
        {
            "id": row["id"],
            "waiting_for": [
                dep for dep in row["depends_on"]
                if states[dep] != "accepted"
            ],
            "blockers": row["blockers"],
        }
        for row in data["passes"]
        if row["status"] != "accepted"
        and row["id"] != next_pass["id"]
    ]
    return {
        "schema_version": 1,
        "repository": data["repository"],
        "integration_branch": data["integration_branch"],
        "next": {
            "id": next_pass["id"],
            "title": next_pass["title"],
            "priority": next_pass["priority"],
            "components": next_pass["component_ids"],
            "entrypoints": next_pass["entrypoints"],
            "docs": next_pass["docs"],
            "checks": next_pass["checks"],
            "acceptance": next_pass["acceptance"],
            "blockers": next_pass["blockers"],
        },
        "remaining": blocked,
        "accepted": [row["id"] for row in data["passes"]
                     if row["status"] == "accepted"],
        "warning": (
            "A pass marked next is not tested or accepted. "
            "Verify actual HEAD, CI results and physical gates separately."
        ),
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true",
                        help="machine-readable, read-only handoff")
    args = parser.parse_args(argv)
    try:
        data = json.loads((ROOT / "catalog/codex_passes.json").read_text(
            encoding="utf-8"))
        output = build_handoff(data)
    except (ValueError, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    if args.json:
        print(json.dumps(output, indent=2, sort_keys=True))
    else:
        current = output["next"]
        print(f"Next: {current['id']} / {current['priority']} / {current['title']}")
        print(f"Branch: {output['integration_branch']}")
        print("Components: " + ", ".join(current["components"]))
        for key in ("entrypoints", "docs", "checks", "acceptance", "blockers"):
            print(f"{key}:")
            for value in current[key]:
                print(f"  - {value}")
        print(f"Remaining passes: {len(output['remaining'])}; "
              f"accepted: {len(output['accepted'])}")
        print(output["warning"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

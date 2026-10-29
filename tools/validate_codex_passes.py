#!/usr/bin/env python3
"""Offline, non-executing validation for the Codex pass DAG and source index."""
import json
import re
import sys
from pathlib import Path

from validate_component_index import checked_file
from validation_common import require

ROOT = Path(__file__).resolve().parents[1]
INDEX = "catalog/codex_passes.json"
REPO = "pskeffington/pip_boy"
STATES = {"next", "queued", "blocked", "accepted"}
GATES = {"software_only", "field_pending"}
UPSTREAM = {
    "pskeffington/rb_gis", "pskeffington/rust_radio",
    "pskeffington/rust_tv", "pskeffington/rusti_rc",
}
AUTHORITY = {
    "agent_instructions": "AGENTS.md",
    "component_index": "catalog/component_index.json",
    "pass_guide": "docs/CODEX_PASSES.md",
    "production_checkpoint": "docs/PROD_STATUS.md",
    "host_roadmap": "docs/HOST_PACKAGE_ROADMAP.md",
}
PASS_FIELDS = {
    "id", "title", "priority", "status", "depends_on", "component_ids",
    "entrypoints", "docs", "checks", "acceptance", "blockers",
    "external_repositories", "hardware_gate", "evidence",
}


def strings(value, name, minimum=0, maximum=24):
    require(isinstance(value, list) and minimum <= len(value) <= maximum,
            f"{name}: invalid list size")
    require(all(isinstance(v, str) and v and len(v) <= 500
                and v == v.strip() for v in value),
            f"{name}: invalid list item")
    require(len(value) == len(set(value)), f"{name}: duplicate entry")
    return value


def validate_passes(root=ROOT, data=None):
    root = Path(root).resolve()
    if data is None:
        data = json.loads((root / INDEX).read_text(encoding="utf-8"))
    require(isinstance(data, dict)
            and set(data) == {
                "schema_version", "repository", "integration_branch",
                "scope", "authority", "passes",
            }
            and type(data["schema_version"]) is int and data["schema_version"] == 1,
            "Codex pass index: invalid schema")
    require(data["repository"] == REPO, "Codex pass index: wrong repository")
    branch = data["integration_branch"]
    require(isinstance(branch, str)
            and re.fullmatch(r"codex/[a-z0-9][a-z0-9-]{2,80}", branch),
            "Codex pass index: invalid integration branch")
    require(isinstance(data["scope"], str) and 30 <= len(data["scope"]) <= 500,
            "Codex pass index: missing scope")
    require(data["authority"] == AUTHORITY,
            "Codex pass index: noncanonical authority")
    for path in AUTHORITY.values():
        checked_file(root, path, "authority")

    component_data = json.loads(
        (root / AUTHORITY["component_index"]).read_text(encoding="utf-8")
    )
    components = {
        c["id"] for c in component_data["components"]
        if isinstance(c, dict) and isinstance(c.get("id"), str)
    }
    passes = data["passes"]
    require(isinstance(passes, list) and 1 <= len(passes) <= 48,
            "Codex pass index: empty or oversized pass list")
    seen = {}
    numbers = []
    next_count = 0
    for row in passes:
        require(isinstance(row, dict) and set(row) == PASS_FIELDS,
                "Codex pass: missing or unexpected fields")
        ident = row["id"]
        require(isinstance(ident, str)
                and re.fullmatch(r"RB-CX-[0-9]{2}", ident),
                "Codex pass: invalid ID")
        require(ident not in seen, f"{ident}: duplicate ID")
        numbers.append(int(ident[-2:]))
        require(isinstance(row["title"], str)
                and 12 <= len(row["title"]) <= 100
                and row["title"] == row["title"].strip(),
                f"{ident}: invalid title")
        require(row["priority"] in {"P0", "P1", "P2"}
                and row["status"] in STATES,
                f"{ident}: invalid priority/status")
        require(row["hardware_gate"] in GATES,
                f"{ident}: unknown hardware gate")
        require(set(strings(row["component_ids"], f"{ident}/components", 1))
                <= components, f"{ident}: unknown component")
        deps = strings(row["depends_on"], f"{ident}/depends_on")
        require(all(re.fullmatch(r"RB-CX-[0-9]{2}", dep)
                    and dep in seen for dep in deps),
                f"{ident}: unknown, self or forward dependency")
        if row["status"] in {"next", "accepted"}:
            require(all(seen[dep] == "accepted" for dep in deps),
                    f"{ident}: next/accepted pass has unaccepted prerequisite")
        if row["status"] == "next":
            next_count += 1

        for kind in ("entrypoints", "docs", "checks"):
            for path in strings(row[kind], f"{ident}/{kind}", 1):
                checked_file(root, path, f"{ident}/{kind}")
                if kind == "docs":
                    require(path.startswith("docs/") and path.endswith(".md"),
                            f"{ident}: invalid docs path")
                if kind == "checks":
                    require(path.startswith("tools/")
                            and Path(path).name.startswith(
                                ("test_", "validate_", "smoke_")),
                            f"{ident}: invalid test path")
        strings(row["acceptance"], f"{ident}/acceptance", 2, 12)
        strings(row["blockers"], f"{ident}/blockers", 0, 12)
        require(set(strings(row["external_repositories"],
                            f"{ident}/external_repositories", 0, 4)) <= UPSTREAM,
                f"{ident}: unreviewed upstream")
        evidence = row["evidence"]
        if row["status"] == "accepted":
            require(isinstance(evidence, dict)
                    and set(evidence) == {
                        "commit_sha", "ci_run_url", "field_report"
                    }, f"{ident}: accepted pass requires exact-head evidence")
            require(isinstance(evidence["commit_sha"], str)
                    and re.fullmatch(r"[0-9a-f]{40}", evidence["commit_sha"]),
                    f"{ident}: invalid evidence commit")
            require(isinstance(evidence["ci_run_url"], str)
                    and re.fullmatch(
                        r"https://github[.]com/pskeffington/pip_boy/actions/runs/[0-9]+",
                        evidence["ci_run_url"]),
                    f"{ident}: invalid CI run")
            if row["hardware_gate"] == "field_pending":
                path = evidence["field_report"]
                require(isinstance(path, str) and path.startswith("docs/")
                        and path.endswith(".md"),
                        f"{ident}: hardware acceptance needs field report")
                checked_file(root, path, f"{ident}/field_report")
            else:
                require(evidence["field_report"] is None,
                        f"{ident}: unexpected field report")
        else:
            require(evidence is None,
                    f"{ident}: unaccepted pass must not claim evidence")
        seen[ident] = row["status"]
    require(numbers == sorted(numbers), "Codex pass index: IDs out of order")
    require(next_count == 1, "Codex pass index: exactly one next pass required")
    return len(passes)


def validate():
    count = validate_passes()
    workflow = (ROOT / ".github/workflows/rust.yml").read_text(encoding="utf-8")
    for command in ("python3 tools/validate_codex_passes.py",
                    "python3 tools/test_validate_codex_passes.py"):
        require(command in workflow, f"CI missing Codex index gate: {command}")
    agent = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
    require("catalog/codex_passes.json" in agent
            and "docs/CODEX_PASSES.md" in agent,
            "AGENTS.md: missing canonical index navigation")
    print(f"PASS: {count} indexed passes, dependency order, "
          "one next gate, exact existing paths, CI registration")


if __name__ == "__main__":
    try:
        validate()
    except (ValueError, OSError, UnicodeError, json.JSONDecodeError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc

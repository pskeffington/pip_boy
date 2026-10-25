#!/usr/bin/env python3
"""Validate Rust-Boy role plans, source links and exact Rust preset configurations.

Planning metadata is not evidence of a passing build or working hardware.
Uses only the Python standard library; never opens network or user data files.
"""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def require(ok, message):
    if not ok:
        raise ValueError(message)


def read(path):
    return (ROOT / path).read_text(encoding="utf-8")


def roster():
    source = read("crates/pipboy-core/src/workbench.rs")
    match = re.search(r"pub const PRESETS: &\[Preset\] = &\[(.*?)\n\];", source, re.S)
    require(match is not None, "Rust preset roster not found")
    result = {}
    blocks = re.findall(r"\bPreset\s*\{(.*?)\n\s*\},", match.group(1), re.S)
    require(blocks, "Empty Rust preset roster")
    for block in blocks:
        def field(name):
            found = re.search(rf'\b{re.escape(name)}:\s*"([^"]+)"', block)
            require(found is not None, f"Missing Rust preset field {name}")
            return found.group(1)

        tools_match = re.search(r"tools:\s*&\[(.*?)\]", block, re.S)
        require(tools_match is not None, "Rust preset tools missing")
        ids = re.findall(r'"([^"]+)"', tools_match.group(1))
        rid = field("id")
        require(rid not in result, f"Duplicate Rust preset {rid}")
        result[rid] = {
            "title": field("title"),
            "fidelity": field("role_fidelity"),
            "evidence": field("lore_evidence"),
            "tools": ids,
        }
    return result


def check():
    data = json.loads(read("catalog/role_roadmap.json"))
    require(data.get("schema_version") == 1, "Unexpected role roadmap schema")
    require(data.get("designation") == "design_roadmap_not_a_feature_release",
            "Roadmap must not imply implemented hardware")
    require(data.get("gate_order") == ["R0", "R1", "R2", "R3"],
            "Role gate order changed unexpectedly")
    sources = data["sources"]
    ids = [source["id"] for source in sources]
    require(len(ids) == len(set(ids)) and len(ids) >= 8,
            "Missing/duplicate lore and technical sources")
    require(all(source["url"].startswith("https://") and source["supports"] and source["limit"]
                for source in sources), "Incomplete source evidence")
    references = set(ids)
    rust = roster()
    role_ids = [role["id"] for role in data["roles"]]
    require(set(role_ids) == set(rust) and len(role_ids) == len(rust),
            "Roadmap role IDs differ from Rust PRESETS")
    require(len(role_ids) == len(set(role_ids)), "Duplicate roadmap role IDs")
    require(role_ids[0] == "overseer" and role_ids[-1] == "custom",
            "Default role or Custom ordering drifted")
    allowed_status = {"code_present_ci_pending", "partially_available", "planned"}
    allowed_needs = {"must", "should", "later"}
    role_doc = read("docs/ROLE_ROADMAP.md")
    lore_doc = read("docs/VAULT_TEC_LORE_MATRIX.md")
    camera_doc = read("docs/SECURITY_CAMERA_WALL.md")
    for source in sources:
        require(f"{source['id']}" in lore_doc and source["url"] in lore_doc,
                f"Unlinked or uncited source {source['id']}")
    for role in data["roles"]:
        rid = role["id"]
        current = rust[rid]
        require(role["name"] == current["title"], f"Role title mismatch {rid}")
        require(role["fidelity"] == current["fidelity"], f"Fidelity mismatch {rid}")
        require(role["baseline_tools"] == current["tools"],
                f"Preset content/order drifted: {rid}")
        require(role["source_ids"] and set(role["source_ids"]) <= references,
                f"Unknown role evidence: {rid}")
        require(role["source_ids"][0] == current["evidence"],
                f"Primary lore reference drifted: {rid}")
        require(len(role["needs"]) >= 3 and len(role["gates"]) == 4,
                f"Incomplete needs or gates: {rid}")
        require(f"### {role['name']} (" in role_doc,
                f"Role missing from readable roadmap: {rid}")
        require(len(role["future_tools"]) == len(set(role["future_tools"]))
                and not set(role["future_tools"]) & set(role["baseline_tools"]),
                f"Future/available tool overlap: {rid}")
        need_ids = [need["id"] for need in role["needs"]]
        require(len(need_ids) == len(set(need_ids)), f"Duplicate needs: {rid}")
        for need in role["needs"]:
            require(need["priority"] in allowed_needs
                    and need["status"] in allowed_status
                    and len(need["need"]) >= 12,
                    f"Invalid role need: {rid}")
        for index, stage in enumerate(role["gates"]):
            require(stage["gate"] == data["gate_order"][index],
                    f"Gate sequence error: {rid}")
            require(stage["deliverables"] and stage["acceptance"],
                    f"Empty gate requirements: {rid}:{stage['gate']}")
            require(stage["status"] in allowed_status,
                    f"Unknown gate status: {rid}")
            require(all(dep in data["gate_order"][:index]
                        or dep in data["quality_prerequisites"]
                        for dep in stage["dependencies"]),
                    f"Invalid/future dependency: {rid}:{stage['gate']}")
            require(stage["title"] in role_doc,
                    f"Missing role gate in human docs: {rid}:{stage['gate']}")
    require(rust["custom"]["tools"] == [], "Custom must be empty")
    security = next(role for role in data["roles"] if role["id"] == "security-chief")
    require("camera-wall" in security["future_tools"],
            "Security camera wall missing from roadmap")
    require(all(term in camera_doc for term in ("2×2", "1/2/3/4", "Enter",
                                                 "Esc", "ONVIF", "JPEG", "RTSP",
                                                 "No background recording")),
            "Camera wall contract is incomplete")
    for link in ("ROLE_ROADMAP.md", "VAULT_TEC_LORE_MATRIX.md",
                 "SECURITY_CAMERA_WALL.md"):
        require(link in read("docs/README.md") and link in read("README.md"),
                f"Top-level documentation does not link {link}")
    require("tools/validate_roles.py" in read(".github/workflows/rust.yml"),
            "Role validation not included in CI")
    print(f"PASS: {len(role_ids)} exact Rust presets, "
          f"{sum(len(role['needs']) for role in data['roles'])} operational needs, "
          f"{sum(len(role['gates']) for role in data['roles'])} acceptance gates, "
          f"{len(sources)} cited references and security camera contract")


if __name__ == "__main__":
    check()

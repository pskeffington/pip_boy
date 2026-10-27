#!/usr/bin/env python3
"""Validate the design-only Vault reporting template against Rust PRESETS.

No networking, hardware probing, tenant creation or permissions are performed.
A reporting edge is NEVER an access-control grant.
"""
import json
from pathlib import Path
from validate_roles import roster

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "catalog/vault_org_chart.json"


def need(condition, message):
    if not condition:
        raise ValueError(message)


def check_template(data, rust_roles):
    need(data.get("schema_version") == 1, "Vault chart schema changed")
    need(data.get("designation") == "design_only_not_authorization",
         "Vault chart must remain a non-authorizing design template")
    need(data.get("root") == "organization-governance", "Unexpected root")
    need(data.get("document") == "docs/VAULT_ORG_CHART.md", "Missing canonical document")
    nodes = data.get("nodes")
    need(isinstance(nodes, list) and 10 <= len(nodes) <= 40, "Missing/oversized org chart")
    by_id = {}
    used_profiles = {}
    for node in nodes:
        need(isinstance(node, dict), "Malformed node")
        node_id = node.get("id")
        need(isinstance(node_id, str) and node_id and node_id not in by_id,
             "Duplicate/invalid node ID")
        need(isinstance(node.get("title"), str) and node["title"].strip(),
             "Missing organizational position title")
        need(isinstance(node.get("function"), str) and len(node["function"]) >= 15,
             "Missing organizational scope")
        need(node.get("kind") in {"governance", "command", "division",
                                  "field_unit", "optional_position"}, "Unexpected position kind")
        need(node.get("status") in {"organizational_policy", "existing_preset", "planned"},
             "Unknown implementation status")
        role_id = node.get("role_id")
        if role_id is not None:
            need(isinstance(role_id, str) and role_id in rust_roles,
                 "Unknown Rust preset in organizational chart")
            need(role_id not in used_profiles, "Rust role assigned to multiple chart positions")
            need(node["status"] == "existing_preset",
                 "An existing Rust role cannot be labeled planned")
            used_profiles[role_id] = node_id
        else:
            need(node["status"] != "existing_preset",
                 "An empty profile cannot be treated as implemented")
        by_id[node_id] = node

    root = data["root"]
    need(root in by_id and by_id[root].get("parent") is None and
         by_id[root].get("role_id") is None, "Governance must be the non-role root")
    for key, node in by_id.items():
        if key == root:
            continue
        parent = node.get("parent")
        need(isinstance(parent, str) and parent in by_id and parent != key,
             "Missing parent or self-reporting node")
        visited = {key}
        while parent is not None:
            need(parent not in visited, "Reporting cycle in chart")
            visited.add(parent)
            parent = by_id[parent].get("parent")
        need(root in visited, "Orphaned chart component")

    unmapped = data.get("unmapped_preset_ids")
    need(unmapped == ["custom"] and rust_roles["custom"]["tools"] == [],
         "Custom is a workspace, not an organizational rank")
    need(set(used_profiles) == set(rust_roles) - set(unmapped),
         "All built-in Rust roles must map exactly once")
    need(used_profiles.get("overseer") == "vault-command" and
         by_id["vault-command"]["parent"] == root,
         "Overseer command must report to governance")
    anchors = {
        "security": ("security-chief", "vault-command"),
        "recon": ("recon", "security"),
        "engineering": ("head-engineer", "vault-command"),
        "medical": ("vault-doctor", "vault-command"),
        "science": ("university-researcher", "vault-command"),
        "field-operations": ("reclamation-scout", "vault-command"),
        "administration": ("records-custodian", "vault-command"),
    }
    for unit, (role, parent) in anchors.items():
        need(unit in by_id and by_id[unit].get("role_id") == role and
             by_id[unit].get("parent") == parent, f"Drifted command chain: {unit}")
    future = {"deputy-overseer": "vault-command",
              "medic": "medical", "quartermaster": "administration"}
    for unit, parent in future.items():
        need(unit in by_id and by_id[unit].get("role_id") is None and
             by_id[unit].get("status") == "planned" and
             by_id[unit].get("parent") == parent,
             f"Unimplemented position was falsely enabled: {unit}")
    need(set(data.get("non_roles", [])) ==
         {node["id"] for node in nodes if node["role_id"] is None},
         "Non-role position roster disagrees with diagram")
    policy = data.get("workflow", {})
    need(policy.get("issues_missions_at") == "vault-command" and
         policy.get("device_trust", "").startswith("organization enrollment"),
         "Missing organizational trust or mission authority boundary")
    return len(nodes), len(used_profiles)


def check():
    data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    counts = check_template(data, roster())
    doc = (ROOT / "docs/VAULT_ORG_CHART.md").read_text(encoding="utf-8")
    need("mermaid" in doc and
         all(x in doc.lower() for x in ("medical", "recon", "reclamations", "overseer",
                                        "unencrypted", "authorization", "not a permission")),
         "Readable chart must show hierarchy and security boundary")
    index = (ROOT / "docs/README.md").read_text(encoding="utf-8")
    sync = (ROOT / "docs/RB_SYNC_ROADMAP.md").read_text(encoding="utf-8")
    need("VAULT_ORG_CHART.md" in index and "VAULT_ORG_CHART.md" in sync,
         "Chart not referenced from navigation and sync design")
    print(f"PASS: Vault design graph has {counts[0]} positions, "
          f"{counts[1]} stable Rust roles, one root, no cycles or inherited grants")


if __name__ == "__main__":
    check()

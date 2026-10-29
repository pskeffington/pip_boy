#!/usr/bin/env python3
"""Deterministic negative regressions for the Codex pass index."""
import copy
import tempfile
import unittest
from pathlib import Path

from validate_codex_passes import REPO, validate_passes


class PassIndexTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        for name in (
            "AGENTS.md", "catalog/component_index.json",
            "docs/CODEX_PASSES.md", "docs/PROD_STATUS.md",
            "docs/HOST_PACKAGE_ROADMAP.md", "src/example.rs",
            "tools/test_fixture.py",
        ):
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("fixture\n", encoding="utf-8")
        (self.root / "catalog/component_index.json").write_text(
            '{"components":[{"id":"host"}]}', encoding="utf-8"
        )
        first = {
            "id": "RB-CX-00", "title": "Validate checked-in host",
            "priority": "P0", "status": "next", "depends_on": [],
            "component_ids": ["host"], "entrypoints": ["src/example.rs"],
            "docs": ["docs/CODEX_PASSES.md"],
            "checks": ["tools/test_fixture.py"],
            "acceptance": ["Confirm bounded state", "Record exact evidence"],
            "blockers": ["Field evidence pending"], "external_repositories": [],
            "hardware_gate": "software_only", "evidence": None,
        }
        second = copy.deepcopy(first)
        second.update({
            "id": "RB-CX-01", "title": "Review next module gate",
            "status": "queued", "depends_on": ["RB-CX-00"],
        })
        self.data = {
            "schema_version": 1, "repository": REPO,
            "integration_branch": "codex/package-adapter-protocol",
            "scope": "Offline synthetic fixture that cannot execute any adapter or task.",
            "authority": {
                "agent_instructions": "AGENTS.md",
                "component_index": "catalog/component_index.json",
                "pass_guide": "docs/CODEX_PASSES.md",
                "production_checkpoint": "docs/PROD_STATUS.md",
                "host_roadmap": "docs/HOST_PACKAGE_ROADMAP.md",
            },
            "passes": [first, second],
        }

    def reject(self, data, message):
        with self.assertRaisesRegex(ValueError, message):
            validate_passes(self.root, data)

    def test_valid_dependency_order(self):
        self.assertEqual(validate_passes(self.root, self.data), 2)

    def test_duplicate_ids_rejected(self):
        data = copy.deepcopy(self.data)
        data["passes"][1]["id"] = "RB-CX-00"
        self.reject(data, "duplicate ID")

    def test_forward_and_unknown_dependencies_rejected(self):
        data = copy.deepcopy(self.data)
        data["passes"][0]["depends_on"] = ["RB-CX-01"]
        self.reject(data, "forward dependency")
        data = copy.deepcopy(self.data)
        data["passes"][1]["depends_on"] = ["RB-CX-55"]
        self.reject(data, "forward dependency")

    def test_unique_next_and_dependency_gate(self):
        data = copy.deepcopy(self.data)
        data["passes"][0]["status"] = "queued"
        self.reject(data, "exactly one next")
        data = copy.deepcopy(self.data)
        data["passes"][1]["status"] = "next"
        self.reject(data, "unaccepted prerequisite")

    def test_unknown_component_rejected(self):
        data = copy.deepcopy(self.data)
        data["passes"][0]["component_ids"] = ["unknown"]
        self.reject(data, "unknown component")

    def test_missing_path_and_parent_escape_rejected(self):
        data = copy.deepcopy(self.data)
        data["passes"][0]["entrypoints"] = ["src/MISSING.rs"]
        self.reject(data, "missing or symlinked")
        data = copy.deepcopy(self.data)
        data["passes"][0]["entrypoints"] = ["../secret.rs"]
        self.reject(data, "noncanonical")

    def test_unreviewed_upstream_rejected(self):
        data = copy.deepcopy(self.data)
        data["passes"][0]["external_repositories"] = ["unknown/loader"]
        self.reject(data, "unreviewed upstream")

    def test_unaccepted_cannot_claim_evidence(self):
        data = copy.deepcopy(self.data)
        data["passes"][0]["evidence"] = {"ci_run_url": "fake"}
        self.reject(data, "must not claim evidence")

    def test_accepted_requires_commit_and_real_shape_of_ci_reference(self):
        data = copy.deepcopy(self.data)
        data["passes"][0]["status"] = "accepted"
        data["passes"][1]["status"] = "next"
        self.reject(data, "exact-head evidence")
        data["passes"][0]["evidence"] = {
            "commit_sha": "bad", "ci_run_url": "https://example.invalid",
            "field_report": None,
        }
        self.reject(data, "invalid evidence commit")
        data["passes"][0]["evidence"]["commit_sha"] = "a" * 40
        self.reject(data, "invalid CI run")
        data["passes"][0]["evidence"]["ci_run_url"] = (
            "https://github.com/pskeffington/pip_boy/actions/runs/123"
        )
        self.assertEqual(validate_passes(self.root, data), 2)

    def test_hardware_gate_requires_a_checked_in_field_report(self):
        data = copy.deepcopy(self.data)
        data["passes"][0]["hardware_gate"] = "field_pending"
        data["passes"][0]["status"] = "accepted"
        data["passes"][1]["status"] = "next"
        data["passes"][0]["evidence"] = {
            "commit_sha": "a" * 40,
            "ci_run_url": "https://github.com/pskeffington/pip_boy/actions/runs/123",
            "field_report": None,
        }
        self.reject(data, "field report")
        path = self.root / "docs/FIELD_REPORT.md"
        path.write_text("field fixture\n", encoding="utf-8")
        data["passes"][0]["evidence"]["field_report"] = "docs/FIELD_REPORT.md"
        self.assertEqual(validate_passes(self.root, data), 2)

    def test_noncanonical_authority_or_extra_fields_rejected(self):
        data = copy.deepcopy(self.data)
        data["authority"]["pass_guide"] = "docs/OTHER.md"
        self.reject(data, "noncanonical authority")
        data = copy.deepcopy(self.data)
        data["passes"][0]["shell_command"] = "echo unapproved"
        self.reject(data, "unexpected fields")


if __name__ == "__main__":
    unittest.main()

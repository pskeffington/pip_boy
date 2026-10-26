#!/usr/bin/env python3
"""Offline fixture tests for the component ownership index."""

import copy
import tempfile
import unittest
from pathlib import Path

from validate_component_index import REQUIRED_CODE, validate_index


class ComponentIndexTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.refs = sorted(REQUIRED_CODE) + ["docs/REFERENCE.md", "tools/test_example.py"]
        for relative in self.refs:
            target = self.root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("fixture\n", encoding="utf-8")
        self.index = {
            "schema_version": 1,
            "scope": "fixtures: code entrypoints only, never hardware proof",
            "development_host": "macOS",
            "prototype_target": "Raspberry Pi Linux aarch64 not tested",
            "components": [
                {
                    "id": "workspace",
                    "boundary": "Cargo and Rust entrypoint ownership",
                    "hardware_gate": "software_only",
                    "code": sorted(REQUIRED_CODE),
                    "docs": ["docs/REFERENCE.md"],
                    "checks": ["tools/test_example.py"],
                }
            ],
        }

    def verify_rejected(self, data, pattern):
        with self.assertRaisesRegex(ValueError, pattern):
            validate_index(self.root, data)

    def test_valid_fixture(self):
        self.assertEqual(validate_index(self.root, self.index),
                         (1, len(REQUIRED_CODE)))

    def test_missing_path(self):
        broken = copy.deepcopy(self.index)
        broken["components"][0]["docs"] = ["docs/MISSING.md"]
        self.verify_rejected(broken, "missing or symlinked source")

    def test_parent_escape_blocked(self):
        broken = copy.deepcopy(self.index)
        broken["components"][0]["code"][0] = "../private-data.json"
        self.verify_rejected(broken, "noncanonical path")

    def test_absolute_path_blocked(self):
        broken = copy.deepcopy(self.index)
        broken["components"][0]["code"][0] = "/tmp/untrusted"
        self.verify_rejected(broken, "absolute path")

    def test_duplicate_component_id(self):
        broken = copy.deepcopy(self.index)
        broken["components"].append(copy.deepcopy(broken["components"][0]))
        self.verify_rejected(broken, "duplicate component ID")

    def test_duplicate_source_ownership(self):
        broken = copy.deepcopy(self.index)
        other = copy.deepcopy(broken["components"][0])
        other["id"] = "second"
        broken["components"].append(other)
        self.verify_rejected(broken, "owned twice")

    def test_unknown_gate_rejected(self):
        broken = copy.deepcopy(self.index)
        broken["components"][0]["hardware_gate"] = "certified_pi"
        self.verify_rejected(broken, "unknown hardware acceptance gate")

    def test_missing_test_evidence_rejected(self):
        broken = copy.deepcopy(self.index)
        broken["components"][0]["checks"] = []
        self.verify_rejected(broken, "empty checks")

    def test_missing_manifest_rejected(self):
        broken = copy.deepcopy(self.index)
        broken["components"][0]["code"].remove("crates/pipboy/Cargo.toml")
        self.verify_rejected(broken, "misses Cargo manifests")

    def test_symlinked_source_rejected(self):
        destination = self.root / "docs" / "LINKED.md"
        try:
            destination.symlink_to(self.root / "docs" / "REFERENCE.md")
        except (OSError, NotImplementedError):
            self.skipTest("symlinks unavailable on this platform")
        broken = copy.deepcopy(self.index)
        broken["components"][0]["docs"] = ["docs/LINKED.md"]
        self.verify_rejected(broken, "missing or symlinked source")

    def test_future_pi_claim_does_not_replace_mac_first_policy(self):
        broken = copy.deepcopy(self.index)
        broken["development_host"] = "Pi"
        self.verify_rejected(broken, "Mac-first")
        broken = copy.deepcopy(self.index)
        broken["prototype_target"] = "certified production hardware"
        self.verify_rejected(broken, "Raspberry Pi prototype")


if __name__ == "__main__":
    unittest.main()

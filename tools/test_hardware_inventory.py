#!/usr/bin/env python3
"""Pure validation tests; intentionally no GPIO, network or hardware probing."""
import importlib.util
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("inventory_validator", ROOT / "tools" / "validate_hardware_inventory.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class InventoryValidationTests(unittest.TestCase):
    def setUp(self):
        self.inventory = json.loads((ROOT / "catalog" / "hardware_test_inventory.json").read_text(encoding="utf-8"))

    def test_checked_in_inventory(self):
        self.assertEqual(module.validate(self.inventory), [])

    def test_compatibility_cannot_grant_release(self):
        self.inventory["profiles"][1]["release_authority"] = "requires_physical_evidence"
        self.assertTrue(any("cannot grant release" in e for e in module.validate(self.inventory)))

    def test_unverified_ram_cannot_be_fabricated(self):
        self.inventory["profiles"][2]["ram_bytes"] = 1073741824
        self.assertTrue(any("must not invent" in e for e in module.validate(self.inventory)))

    def test_no_implicit_probe(self):
        self.inventory["safety"]["auto_network_probe"] = True
        self.assertTrue(any("auto_network_probe" in e for e in module.validate(self.inventory)))

    def test_duplicate_ids(self):
        self.inventory["profiles"][2]["id"] = self.inventory["profiles"][1]["id"]
        self.assertTrue(any("duplicate id" in e for e in module.validate(self.inventory)))

    def test_reference_target_cannot_be_reassigned(self):
        self.inventory["reference_release_target"] = "rb-pi4-compat"
        self.assertTrue(any("reference_release_target" in e for e in module.validate(self.inventory)))


if __name__ == "__main__":
    unittest.main()

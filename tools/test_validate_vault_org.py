#!/usr/bin/env python3
"""Pure negative fixtures for the Vault chart; no filesystem mutation."""
import copy
import json
import unittest
from pathlib import Path
from validate_roles import roster
from validate_vault_org import check_template

ROOT = Path(__file__).resolve().parents[1]


class VaultOrgValidationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.template = json.loads((ROOT / "catalog/vault_org_chart.json").read_text(encoding="utf-8"))
        cls.rust = roster()

    def clone(self):
        return copy.deepcopy(self.template)

    @staticmethod
    def node(data, name):
        return next(entry for entry in data["nodes"] if entry["id"] == name)

    def test_canonical_roster_is_valid(self):
        self.assertEqual(check_template(self.clone(), self.rust), (12, 8))

    def test_reporting_cycle_rejected(self):
        data = self.clone()
        self.node(data, "security")["parent"] = "recon"
        with self.assertRaisesRegex(ValueError, "cycle"):
            check_template(data, self.rust)

    def test_unknown_parent_rejected(self):
        data = self.clone()
        self.node(data, "science")["parent"] = "external-tenant"
        with self.assertRaisesRegex(ValueError, "Missing parent"):
            check_template(data, self.rust)

    def test_duplicate_role_rejected(self):
        data = self.clone()
        self.node(data, "recon")["role_id"] = "security-chief"
        with self.assertRaisesRegex(ValueError, "multiple"):
            check_template(data, self.rust)

    def test_no_silent_promotion_of_medic(self):
        data = self.clone()
        self.node(data, "medic")["role_id"] = "vault-doctor"
        with self.assertRaisesRegex(ValueError, "multiple"):
            check_template(data, self.rust)

    def test_custom_cannot_inherit_command(self):
        data = self.clone()
        data["unmapped_preset_ids"] = []
        with self.assertRaisesRegex(ValueError, "Custom"):
            check_template(data, self.rust)

    def test_no_authentication_claim(self):
        data = self.clone()
        data["designation"] = "enrolled_and_authorized"
        with self.assertRaisesRegex(ValueError, "non-authorizing"):
            check_template(data, self.rust)

    def test_recon_chain_stable(self):
        data = self.clone()
        self.node(data, "recon")["parent"] = "field-operations"
        with self.assertRaisesRegex(ValueError, "Drifted command chain"):
            check_template(data, self.rust)


if __name__ == "__main__":
    unittest.main()

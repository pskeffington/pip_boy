#!/usr/bin/env python3
"""Offline handoff regressions: output stays deterministic and non-executing."""
import copy
import json
import unittest
from pathlib import Path

from codex_next import build_handoff
from validate_codex_passes import ROOT


class HandoffTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = json.loads((ROOT / "catalog/codex_passes.json").read_text(
            encoding="utf-8"))

    def test_next_gate_is_manifest_owned(self):
        result = build_handoff(copy.deepcopy(self.data))
        current = next(row for row in self.data["passes"]
                       if row["status"] == "next")
        self.assertEqual(result["next"]["id"], current["id"])
        self.assertEqual(result["next"]["checks"], current["checks"])
        self.assertEqual(result["next"]["acceptance"], current["acceptance"])

    def test_deterministic_and_explicitly_unaccepted(self):
        a = build_handoff(copy.deepcopy(self.data))
        b = build_handoff(copy.deepcopy(self.data))
        self.assertEqual(json.dumps(a, sort_keys=True),
                         json.dumps(b, sort_keys=True))
        self.assertIn("not tested or accepted", a["warning"])
        accepted = [row["id"] for row in self.data["passes"]
                    if row["status"] == "accepted"]
        self.assertEqual(a["accepted"], accepted)
        self.assertEqual(len(a["remaining"]),
                         len(self.data["passes"]) - len(accepted) - 1)

    def test_unaccepted_dependency_remains_visible(self):
        data = copy.deepcopy(self.data)
        first = data["passes"][0]
        first["status"] = "next"
        first["evidence"] = None
        second = data["passes"][1]
        second["status"] = "queued"
        result = build_handoff(data)
        second = next(item for item in result["remaining"]
                      if item["id"] == "RB-CX-01")
        self.assertEqual(second["waiting_for"], ["RB-CX-00"])

    def test_malformed_index_rejected_not_silently_repaired(self):
        bad = copy.deepcopy(self.data)
        bad["passes"][0]["status"] = "queued"
        bad["passes"][0]["evidence"] = None
        bad["passes"][1]["status"] = "queued"
        with self.assertRaisesRegex(ValueError, "exactly one next"):
            build_handoff(bad)


if __name__ == "__main__":
    unittest.main()

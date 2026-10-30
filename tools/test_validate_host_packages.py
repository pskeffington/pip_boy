#!/usr/bin/env python3
"""Negative regressions for inert socket manifests; no Git or network access."""

import copy
import json
import unittest
from pathlib import Path

from validate_host_packages import ROOT, SCAN_SCOPES, check_manifest, validate


class SocketCatalogTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.map_manifest = json.loads(
            (ROOT / "catalog/host_packages/rb_gis.example.json").read_text(encoding="utf-8")
        )
        cls.scan_manifest = json.loads(
            (ROOT / "catalog/host_packages/trans_scan.example.json").read_text(encoding="utf-8")
        )

    def fixture(self):
        return copy.deepcopy(self.map_manifest)

    def test_five_reference_fixtures_validate(self):
        self.assertEqual(validate(), 5)

    def test_scan_is_inert_reference_with_bounded_requested_scopes(self):
        scan = copy.deepcopy(self.scan_manifest)
        self.assertEqual(check_manifest(scan), "org.rustboy.scan")
        self.assertEqual(set(scan["capabilities"]), SCAN_SCOPES)
        self.assertEqual(scan["status"], "design_fixture_not_installed")
        self.assertFalse(scan["background"]["allowed"])
        self.assertEqual(scan["entrypoint"], {"type": "builtin_adapter"})

    def test_scan_cannot_claim_installation_or_background_camera(self):
        for key, value in (
            ("status", "installed"),
            ("background", {"allowed": True}),
            ("entrypoint", {"type": "external_process", "command": "/bin/sh"}),
        ):
            scan = copy.deepcopy(self.scan_manifest)
            scan[key] = value
            with self.assertRaises(ValueError):
                check_manifest(scan)

    def test_scan_capability_namespace_rejects_cross_module_or_unknown(self):
        other = self.fixture()
        other["capabilities"] = ["scan.camera.capture"]
        with self.assertRaisesRegex(ValueError, "SCAN capability namespace"):
            check_manifest(other)
        for caps in (
            ["scan.camera.capture", "scan.camera.capture"],
            ["scan.camera.remote_wake"],
            ["scan.camera.capture", "secrets.read"],
        ):
            scan = copy.deepcopy(self.scan_manifest)
            scan["capabilities"] = caps
            with self.assertRaises(ValueError):
                check_manifest(scan)

    def test_api_drift_is_rejected(self):
        m = self.fixture()
        m["host_api"] = 2
        with self.assertRaisesRegex(ValueError, "host_api"):
            check_manifest(m)

    def test_package_path_or_uppercase_rejected(self):
        for identifier in ("../../tmp", "org.RustBoy.gis", "org.rustboy.", "org.rustboy.bad/../"):
            m = self.fixture()
            m["package_id"] = identifier
            with self.assertRaises(ValueError):
                check_manifest(m)

    def test_no_shell_command_in_manifest(self):
        m = self.fixture()
        m["entrypoint"] = {"type": "external_process", "command": "/bin/sh"}
        with self.assertRaisesRegex(ValueError, "entrypoint"):
            check_manifest(m)

    def test_unknown_and_duplicate_caps_rejected(self):
        for caps in (["secrets.read"], ["location.read", "location.read"]):
            m = self.fixture()
            m["capabilities"] = caps
            with self.assertRaisesRegex(ValueError, "capabilities"):
                check_manifest(m)

    def test_manifest_cannot_self_assert_installed_state(self):
        m = self.fixture()
        m["status"] = "installed"
        with self.assertRaisesRegex(ValueError, "installed"):
            check_manifest(m)

    def test_wrong_viewport_or_unknown_top_level_key_rejected(self):
        m = self.fixture()
        m["ui"]["min_height"] = 1
        with self.assertRaisesRegex(ValueError, "viewport"):
            check_manifest(m)
        m = self.fixture()
        m["shell_override"] = True
        with self.assertRaisesRegex(ValueError, "unexpected"):
            check_manifest(m)

    def test_strict_boolean_and_required_keys(self):
        m = self.fixture()
        m["background"]["allowed"] = 1
        with self.assertRaisesRegex(ValueError, "background"):
            check_manifest(m)
        m = self.fixture()
        del m["publisher"]
        with self.assertRaisesRegex(ValueError, "missing"):
            check_manifest(m)


if __name__ == "__main__":
    unittest.main()

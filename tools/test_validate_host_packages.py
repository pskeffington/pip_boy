#!/usr/bin/env python3
"""Negative regressions for inert socket manifests; no Git or network access."""

import copy
import json
import unittest
from pathlib import Path

from validate_host_packages import ROOT, check_approved_executable_roster, check_manifest, validate


class SocketCatalogTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.map_manifest = json.loads(
            (ROOT / "catalog/host_packages/rb_gis.example.json").read_text(encoding="utf-8")
        )

    def fixture(self):
        return copy.deepcopy(self.map_manifest)

    def test_five_distinct_reference_fixtures_validate(self):
        self.assertEqual(validate(), 5)

    def test_executable_pins_are_exact_and_manifest_independent(self):
        self.assertEqual(check_approved_executable_roster(), 0)
        base = {"schema_version": 1, "executables": [{
            "package_id": "org.rustboy.music",
            "relative_executable": "bin/rust-music",
            "sha256": "a" * 64,
            "relative_manifest": "manifest.json",
            "manifest_sha256": "b" * 64,
            "arguments": ["serve"],
        }]}
        self.assertEqual(check_approved_executable_roster(data=base), 1)
        for field, value in (
            ("relative_executable", "../music"),
            ("relative_manifest", "../manifest.json"),
            ("sha256", "not-a-digest"),
            ("manifest_sha256", "not-a-digest"),
            ("arguments", ["\0bad"]),
            ("package_id", "org.rustboy.unreviewed"),
        ):
            changed = copy.deepcopy(base)
            changed["executables"][0][field] = value
            with self.assertRaises(ValueError):
                check_approved_executable_roster(data=changed)

    def test_launcher_resolves_package_scoped_install_directories(self):
        source = (ROOT / "crates/pipboy-core/src/package_process.rs").read_text(encoding="utf-8")
        self.assertIn("fn resolve_package_root", source)
        self.assertIn("resolve_package_root(install_root, &entry.package_id)", source)
        self.assertNotIn("pub fn approved_process_spec", source)

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

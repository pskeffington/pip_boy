#!/usr/bin/env python3
"""Offline regression tests for trusted package staging."""

import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from install_approved_package import data_root_from_environment, install_package

ROOT = Path(__file__).resolve().parents[1]
PACKAGE_ID = "org.rustboy.music"


class ApprovedPackageInstallTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.source = self.base / "artifact"
        (self.source / "bin").mkdir(parents=True)
        self.executable = self.source / "bin/rust-music"
        self.executable.write_bytes(b"reviewed executable bytes")
        self.executable.chmod(0o755)
        manifest = json.loads(
            (ROOT / "catalog/host_packages/rust_music.example.json").read_text(encoding="utf-8")
        )
        manifest.pop("status")
        manifest["entrypoint"]["type"] = "external_process"
        self.manifest_bytes = json.dumps(manifest, separators=(",", ":")).encode("utf-8")
        (self.source / "manifest.json").write_bytes(self.manifest_bytes)
        self.row = {
            "package_id": PACKAGE_ID,
            "relative_executable": "bin/rust-music",
            "sha256": hashlib.sha256(self.executable.read_bytes()).hexdigest(),
            "relative_manifest": "manifest.json",
            "manifest_sha256": hashlib.sha256(self.manifest_bytes).hexdigest(),
            "arguments": ["serve"],
        }
        self.roster = {"schema_version": 1, "executables": [self.row]}
        self.packages_root = self.base / "data/packages"

    def test_installs_only_pinned_files_atomically_without_mutating_roster(self):
        original_roster = copy.deepcopy(self.roster)
        installed = install_package(PACKAGE_ID, self.source, self.packages_root, self.roster)
        self.assertEqual(installed, self.packages_root.resolve() / PACKAGE_ID)
        self.assertEqual((installed / "bin/rust-music").read_bytes(), b"reviewed executable bytes")
        self.assertEqual((installed / "manifest.json").read_bytes(), self.manifest_bytes)
        self.assertEqual(sorted(path.relative_to(installed).as_posix()
                                for path in installed.rglob("*") if path.is_file()),
                         ["bin/rust-music", "manifest.json"])
        self.assertTrue(os.access(installed / "bin/rust-music", os.X_OK))
        self.assertEqual(self.roster, original_roster)

    def test_unknown_id_and_byte_tampering_are_denied(self):
        with self.assertRaisesRegex(ValueError, "not in the approved"):
            install_package("org.rustboy.tv", self.source, self.packages_root, self.roster)
        self.executable.write_bytes(b"replacement")
        with self.assertRaisesRegex(ValueError, "SHA-256"):
            install_package(PACKAGE_ID, self.source, self.packages_root, self.roster)
        self.assertFalse((self.packages_root / PACKAGE_ID).exists())

    def test_symlinks_and_existing_installs_are_rejected_without_overwrite(self):
        link_source = self.base / "symlink-artifact"
        link_source.mkdir()
        (link_source / "bin").mkdir()
        (link_source / "bin/rust-music").symlink_to(self.executable)
        (link_source / "manifest.json").write_bytes(self.manifest_bytes)
        with self.assertRaisesRegex(ValueError, "symlink"):
            install_package(PACKAGE_ID, link_source, self.packages_root, self.roster)

        installed = install_package(PACKAGE_ID, self.source, self.packages_root, self.roster)
        before = (installed / "bin/rust-music").read_bytes()
        with self.assertRaises(FileExistsError):
            install_package(PACKAGE_ID, self.source, self.packages_root, self.roster)
        self.assertEqual((installed / "bin/rust-music").read_bytes(), before)

    def test_environment_selects_default_or_explicit_data_root(self):
        home = self.base / "home"
        self.assertEqual(data_root_from_environment({}, home), home / ".pipboy")
        self.assertEqual(data_root_from_environment({"PIPBOY_HOME": str(self.base / "custom")}, home),
                         self.base / "custom")

    def test_cli_denies_all_packages_while_production_roster_is_empty(self):
        data_root = self.base / "empty-roster-data"
        env = os.environ.copy()
        env["PIPBOY_HOME"] = str(data_root)
        result = subprocess.run(
            [sys.executable, str(ROOT / "tools/install_approved_package.py"),
             PACKAGE_ID, str(self.source)],
            cwd=ROOT, env=env, capture_output=True, text=True, check=False,
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn("not in the approved host roster", result.stderr)
        self.assertFalse(data_root.exists())


if __name__ == "__main__":
    unittest.main()

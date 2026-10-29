#!/usr/bin/env python3
"""Negative regressions for inert socket manifests; no Git or network access."""

import copy
import json
import tempfile
import unittest
from pathlib import Path

from validate_host_packages import (
    ROOT, check_adapter_contract, check_manifest, check_builtin_radio_manifest,
    check_socket_admission, validate,
)


class SocketCatalogTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.map_manifest = json.loads(
            (ROOT / "catalog/host_packages/rb_gis.example.json").read_text(encoding="utf-8")
        )

    def fixture(self):
        return copy.deepcopy(self.map_manifest)

    def test_four_reference_fixtures_validate(self):
        self.assertEqual(validate(), 4)

    def test_socket_sessions_require_verified_bounded_admission(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            listener = root / "crates/pipboy-core/src/package_listener.rs"
            transport = root / "crates/pipboy-core/src/package_transport.rs"
            listener.parent.mkdir(parents=True)
            listener.write_text(
                (ROOT / "crates/pipboy-core/src/package_listener.rs").read_text(
                    encoding="utf-8"
                ),
                encoding="utf-8",
            )
            transport.write_text(
                (ROOT / "crates/pipboy-core/src/package_transport.rs").read_text(
                    encoding="utf-8"
                ),
                encoding="utf-8",
            )
            check_socket_admission(root)
            transport.write_text(
                transport.read_text(encoding="utf-8").replace(
                    "    fn from_stream(", "    pub fn from_stream(", 1
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(ValueError, "verified peer provenance"):
                check_socket_admission(root)

    def test_package_view_contract_drift_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "catalog").mkdir()
            (root / "crates/rust-boy-adapter-sdk/src").mkdir(parents=True)
            (root / "catalog/host_adapter_contract.json").write_text(
                (ROOT / "catalog/host_adapter_contract.json").read_text(encoding="utf-8"),
                encoding="utf-8",
            )
            (root / "crates/rust-boy-adapter-sdk/src/lib.rs").write_text(
                (ROOT / "crates/rust-boy-adapter-sdk/src/lib.rs").read_text(encoding="utf-8"),
                encoding="utf-8",
            )
            check_adapter_contract(root)
            contract_path = root / "catalog/host_adapter_contract.json"
            contract = json.loads(contract_path.read_text(encoding="utf-8"))
            contract["package_view"]["blocks"].append("html")
            contract_path.write_text(json.dumps(contract), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "package view schema drift"):
                check_adapter_contract(root)

    def test_builtin_radio_denies_all_capabilities(self):
        m = json.loads(
            (ROOT / "catalog/builtin_packages/rust_radio.json").read_text(encoding="utf-8")
        )
        self.assertEqual(check_builtin_radio_manifest(m), "org.rustboy.radio")
        for field in ("capabilities", "background", "status"):
            changed = copy.deepcopy(m)
            if field == "capabilities":
                changed[field] = ["media.audio"]
            elif field == "background":
                changed[field] = {"allowed": True}
            else:
                changed[field] = "installed"
            with self.assertRaises(ValueError):
                check_builtin_radio_manifest(changed)

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

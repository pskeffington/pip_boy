#!/usr/bin/env python3
"""Offline regression checks for Linux desktop launch and rootfs staging."""

import hashlib
import os
from pathlib import Path
import subprocess
import tempfile
import types
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
LAUNCHER = ROOT / "linux/rust-boy-launcher"
STAGER = ROOT / "scripts/stage-linux-native.sh"

launcher = types.ModuleType("rust_boy_launcher")
launcher.__file__ = str(LAUNCHER)
exec(compile(LAUNCHER.read_text(encoding="utf-8"), str(LAUNCHER), "exec"), launcher.__dict__)


class FakeResponse:
    status = 200

    def __init__(self, body):
        self.body = body

    def read(self, length):
        return self.body[:length]

    def __enter__(self):
        return self

    def __exit__(self, *_):
        return False


class NativeLinuxTests(unittest.TestCase):
    def test_readiness_rejects_foreign_services_and_disables_proxies(self):
        valid = b'{"schema_version":1,"service":"rust-boy","network":"loopback_only"}'
        for body, expected in ((valid, True), (b'{"service":"other"}', False),
                               (valid + b"x" * 513, False), (b"<html/>", False)):
            with self.subTest(expected=expected, body=body[:15]):
                opener = mock.Mock()
                opener.open.return_value = FakeResponse(body)
                with mock.patch.object(launcher, "build_opener", return_value=opener) as factory:
                    self.assertEqual(launcher.is_ready(), expected)
                self.assertEqual(factory.call_args.args[0].proxies, {})
                self.assertEqual(opener.open.call_args.args[0].full_url, launcher.HEALTH_URL)

    def test_no_display_and_bad_args_have_no_side_effects(self):
        with mock.patch.object(launcher.subprocess, "run") as service, \
             mock.patch.object(launcher.subprocess, "Popen") as browser:
            self.assertEqual(launcher.main(["--bad"]), 2)
            with mock.patch.dict(os.environ, {"DISPLAY": "", "WAYLAND_DISPLAY": ""}):
                self.assertEqual(launcher.main([]), 1)
            service.assert_not_called()
            browser.assert_not_called()

    def test_micro_dispatch_is_local_and_user_initiated(self):
        with mock.patch.dict(os.environ, {"WAYLAND_DISPLAY": "wayland-1"}), \
             mock.patch.object(launcher.subprocess, "run") as service, \
             mock.patch.object(launcher, "wait_until_ready", return_value=True), \
             mock.patch.object(launcher.subprocess, "Popen") as browser:
            self.assertEqual(launcher.main(["--micro"]), 0)
            self.assertEqual(service.call_args.args[0], [
                "/usr/bin/systemctl", "--user", "start", "rust-boy-dashboard.service"
            ])
            self.assertEqual(browser.call_args.args[0], [
                "/usr/bin/xdg-open", "http://127.0.0.1:8765/micro"
            ])
            self.assertNotIn("shell", browser.call_args.kwargs)

    def test_failed_start_or_readiness_never_opens_browser(self):
        with mock.patch.dict(os.environ, {"DISPLAY": ":0"}), \
             mock.patch.object(launcher.subprocess, "run") as service, \
             mock.patch.object(launcher, "wait_until_ready", return_value=False), \
             mock.patch.object(launcher.subprocess, "Popen") as browser:
            self.assertEqual(launcher.main([]), 1)
            service.side_effect = OSError("systemd unavailable")
            self.assertEqual(launcher.main([]), 1)
            browser.assert_not_called()

    def test_desktop_and_service_remain_manual_and_local(self):
        desktop = (ROOT / "linux/org.rustboy.RustBoy.desktop").read_text()
        service = (ROOT / "linux/rust-boy-dashboard.service").read_text()
        self.assertIn("Exec=/usr/local/bin/rust-boy-launcher", desktop)
        self.assertIn("TryExec=/usr/local/bin/pipboy", desktop)
        self.assertIn("Terminal=false", desktop)
        self.assertIn("ExecStart=/usr/local/bin/pipboy dashboard --port 8765", service)
        self.assertIn("NoNewPrivileges=yes", service)
        self.assertNotIn("WantedBy=default.target", service)
        self.assertNotIn("0.0.0.0", service)

    def test_staging_is_verifiable_and_non_overwriting(self):
        with tempfile.TemporaryDirectory() as folder:
            temp = Path(folder)
            binary = temp / "pipboy"
            binary.write_text("#!/bin/sh\nexit 0\n")
            binary.chmod(0o755)
            destination = temp / "rootfs"
            destination.mkdir()
            cmd = ["bash", str(STAGER), str(binary), str(destination)]
            result = subprocess.run(cmd, capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            staged = destination / "usr/local/bin/pipboy"
            self.assertEqual(staged.read_bytes(), binary.read_bytes())
            digest = hashlib.sha256(binary.read_bytes()).hexdigest()
            self.assertEqual(
                (destination / "usr/share/rust-boy/native-binary.sha256").read_text().strip(),
                digest
            )
            self.assertTrue((destination / "usr/lib/systemd/user/rust-boy-dashboard.service").is_file())
            self.assertTrue((destination / "usr/share/applications/org.rustboy.RustBoy.desktop").is_file())
            self.assertFalse((destination / "etc/systemd/user").exists())
            self.assertNotEqual(subprocess.run(cmd, capture_output=True).returncode, 0)
            self.assertEqual(staged.read_bytes(), binary.read_bytes())
            redirected = temp / "redirected"
            redirected.mkdir()
            (redirected / "usr").symlink_to(destination / "usr", target_is_directory=True)
            self.assertNotEqual(subprocess.run(
                ["bash", str(STAGER), str(binary), str(redirected)],
                capture_output=True
            ).returncode, 0)
            self.assertNotEqual(subprocess.run(
                ["bash", str(STAGER), str(binary), "/"],
                capture_output=True
            ).returncode, 0)


if __name__ == "__main__":
    unittest.main()

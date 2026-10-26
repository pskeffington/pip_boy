#!/usr/bin/env python3
"""Standard-library regression tests for the shared CI smoke roster."""

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from run_smokes import (LINUX_EXTRA, NATIVE_SMOKES, PLATFORM_SPECIFIC,
                        SUITES, main, run_suite, validate_manifest)


class SmokeRosterTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="rust-boy-ci-roster-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.tools = self.root / "tools"
        self.tools.mkdir()
        for name in (*LINUX_EXTRA, *NATIVE_SMOKES, *PLATFORM_SPECIFIC):
            (self.tools / name).write_text("# test fixture\n", encoding="utf-8")

    def test_all_known_smokes_classified_without_duplicates(self):
        self.assertEqual(validate_manifest(self.tools), 20)
        self.assertEqual(len(SUITES["linux"]), 19)
        self.assertEqual(len(SUITES["native"]), 17)
        self.assertEqual(SUITES["linux"][:2], LINUX_EXTRA)
        self.assertEqual(SUITES["linux"][2:], SUITES["native"])
        self.assertNotIn("smoke_windows_conpty.py", SUITES["native"])

    def test_new_smoke_requires_explicit_ci_classification(self):
        (self.tools / "smoke_future_adapter.py").write_text("", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "unclassified"):
            validate_manifest(self.tools)

    def test_missing_smoke_rejected(self):
        (self.tools / NATIVE_SMOKES[0]).unlink()
        with self.assertRaisesRegex(ValueError, "missing="):
            validate_manifest(self.tools)

    def test_symlinked_smoke_rejected(self):
        script = self.tools / NATIVE_SMOKES[0]
        script.unlink()
        try:
            script.symlink_to(self.root / "outside.py")
        except (NotImplementedError, OSError):
            self.skipTest("symlinks unavailable on this host")
        with self.assertRaisesRegex(ValueError, "symlinked"):
            validate_manifest(self.tools)

    def test_missing_or_symlinked_tools_directory_rejected(self):
        with self.assertRaisesRegex(ValueError, "invalid smoke-test directory"):
            validate_manifest(self.root / "missing")

    def test_native_execution_uses_current_python_and_exact_order(self):
        calls = []

        def fake_run(argv, **kwargs):
            calls.append((argv, kwargs))

        with patch("builtins.print"):
            run_suite("native", self.tools, fake_run)
        self.assertEqual(
            [Path(argv[1]).name for argv, _ in calls], list(NATIVE_SMOKES)
        )
        self.assertTrue(all(argv[0] == sys.executable for argv, _ in calls))
        self.assertTrue(all(options == {"cwd": self.root, "check": True}
                            for _, options in calls))

    def test_linux_includes_explicit_extra_checks(self):
        order = []
        with patch("builtins.print"):
            run_suite("linux", self.tools,
                      lambda argv, **kw: order.append(Path(argv[1]).name))
        self.assertEqual(order, [*LINUX_EXTRA, *NATIVE_SMOKES])

    def test_child_failure_stops_remaining_tests(self):
        attempts = []

        def fake_run(argv, **_):
            attempts.append(Path(argv[1]).name)
            if len(attempts) == 3:
                raise subprocess.CalledProcessError(7, argv)

        with patch("builtins.print"):
            with self.assertRaises(subprocess.CalledProcessError):
                run_suite("native", self.tools, fake_run)
        self.assertEqual(attempts, list(NATIVE_SMOKES[:3]))

    def test_unknown_profile_rejected_without_execution(self):
        with self.assertRaisesRegex(ValueError, "unknown smoke profile"):
            run_suite("all", self.tools, lambda *_a, **_k: self.fail("ran"))

    def test_check_only_never_executes_smoke(self):
        with patch("run_smokes.validate_manifest", return_value=20):
            with patch("run_smokes.run_suite") as execute:
                with patch("builtins.print"):
                    main(["--check"])
                execute.assert_not_called()

    def test_list_only_never_executes_smoke(self):
        with patch("run_smokes.validate_manifest", return_value=20):
            with patch("run_smokes.run_suite") as execute:
                with patch("builtins.print") as emit:
                    main(["native", "--list"])
                execute.assert_not_called()
                self.assertEqual([call.args[0] for call in emit.call_args_list],
                                 list(NATIVE_SMOKES))


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Offline unit tests for public-commit and child-environment leakage gates."""

import unittest

from validate_security import content_issues, path_issue


class PublicLeakagePolicyTests(unittest.TestCase):
    def test_runtime_records_are_never_public_fixtures(self):
        for filename in ("messages.json", "notes.json", "tasks.json",
                         "peers.json", "reclamation.json", "message-pace.json",
                         "messages.recipient"):
            self.assertEqual(path_issue(f"examples/field/{filename}"),
                             "private-operator-or-key-file")

    def test_private_key_and_location_export_paths_rejected(self):
        for filename in ("id_ed25519", "id_rsa.pub", "identity.pem",
                         "controller.key", "history.gpx", "observations.kml"):
            self.assertEqual(path_issue(f"data/{filename}"),
                             "private-operator-or-key-file")

    def test_private_directories_rejected(self):
        for path in (".pipboy/messages.json", "examples/county_cache/roads.zip",
                     "target/release/pipboy", "local-state/cache.bin"):
            self.assertEqual(path_issue(path), "runtime-data-directory")

    def test_env_files_not_templates(self):
        self.assertEqual(path_issue(".env.production"), "private-environment-file")
        self.assertIsNone(path_issue(".env.example"))
        self.assertIsNone(path_issue("data/index/manifest.v1.json"))

    def test_checkout_traversal_is_rejected(self):
        self.assertEqual(path_issue("../../private/notes"), "unsafe-tracked-path")

    def test_common_tokens_detected_without_secret_echo(self):
        fake = "ghp_" + ("Z" * 30)
        findings = content_issues("docs/guide.md", f"token={fake}".encode())
        self.assertEqual(findings, [("github-token", 1)])
        self.assertNotIn(fake, repr(findings))

    def test_private_key_block_detected(self):
        fake = "-----BEGIN " + "PRIVATE KEY-----"
        self.assertEqual(content_issues("notes.md", fake.encode()),
                         [("private-key-block", 1)])

    def test_auth_in_urls_detected(self):
        fake = "https://" + "testuser:sample-pass" + "@" + "host.invalid"
        findings = content_issues("config.md", fake.encode())
        self.assertEqual(findings, [("credentials-in-url", 1)])
        self.assertNotIn("sample-pass", repr(findings))

    def test_unprotected_command_constructor_rejected(self):
        source = 'let child = Command' + '::new("ssh");'
        self.assertEqual(content_issues("crates/pipboy/src/peer.rs", source.encode()),
                         [("unreviewed-child-environment", 1)])

    def test_isolated_command_and_its_single_constructor_allowed(self):
        safe = 'let child = isolated_command("ssh");'
        self.assertEqual(content_issues("crates/pipboy/src/peer.rs", safe.encode()), [])
        wrapper = 'let child = Command' + '::new(program);'
        self.assertEqual(
            content_issues("crates/pipboy-core/src/process_safety.rs",
                           wrapper.encode()), []
        )

    def test_binary_and_oversize_are_bounded(self):
        self.assertEqual(content_issues("fixture.png", b"\x89PNG\x00test"), [])
        self.assertEqual(
            content_issues("oversized.md", b"x" * (4 * 1024 * 1024 + 1)),
            [("oversized-file-needs-review", 0)],
        )

    def test_nonsecret_documentation_is_accepted(self):
        self.assertEqual(
            content_issues("README.md",
                           b"Use RUSTBOY_UNIFI_API_KEY from your environment.\n"),
            [],
        )


if __name__ == "__main__":
    unittest.main()

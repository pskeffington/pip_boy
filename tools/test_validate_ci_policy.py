#!/usr/bin/env python3
"""Negative regressions for Rust-Boy's first-party CI policy guard."""
import unittest

from validate_ci_policy import validate_ci_policy


WORKFLOW = '''
      - name: Checkout exact triggering commit (first-party)
        run: |
          git -c protocol.version=2 fetch --no-tags --depth=1 origin "$GITHUB_SHA"
          test "$(git rev-parse HEAD)" = "$GITHUB_SHA"
      toolchain: ["stable", "1.90.0"]
      feature: [soapy-sdr, sdrconnect]
          - os: macos-latest
          - os: windows-latest
        run: cargo check --workspace --all-targets --locked
        run: cargo test --workspace --locked
        run: cargo clippy --workspace --all-targets --locked -- -D warnings
        run: bash scripts/ci_bootstrap.sh rust "${{ matrix.toolchain }}"
        run: bash scripts/ci_bootstrap.sh rust stable
        run: bash scripts/ci_bootstrap.sh node
'''

BOOTSTRAP = '''
    [[ "$toolchain" == "stable" || "$toolchain" == "1.90.0" ]]
    version="22.23.4"
      Linux/x86_64)
        digest="1805bcd35e6815559bd2eb97db438d3fdb0eb3a5e6a977b7c1badca76337e9ce"
      Darwin/arm64)
        digest="c9e1dbc748b33ed2d14f42445191ecaf14c56259c7a9a26d0042c49ec2b66428"
      Darwin/x86_64)
        digest="efcc2b0db6f94bf53eddb038fe2b370277a042bd29e4f19b8f8e23598795b819"
        "https://nodejs.org/dist/v${version}/${archive}"
    shasum -a 256 --check --status -
'''


class CiPolicyTests(unittest.TestCase):
    def validate(self, workflow=WORKFLOW, bootstrap=BOOTSTRAP):
        return validate_ci_policy({".github/workflows/rust.yml": workflow}, bootstrap)

    def reject(self, message, workflow=WORKFLOW, bootstrap=BOOTSTRAP):
        with self.assertRaisesRegex(ValueError, message):
            self.validate(workflow, bootstrap)

    def test_complete_first_party_matrix_is_accepted(self):
        self.assertEqual(self.validate(), 1)

    def test_external_action_import_is_rejected(self):
        self.reject("external GitHub Action", WORKFLOW + "\n      - uses: actions/checkout@v4\n")

    def test_missing_exact_sha_checkout_is_rejected(self):
        self.reject("exact triggering SHA fetch", WORKFLOW.replace(
            'git -c protocol.version=2 fetch --no-tags --depth=1 origin "$GITHUB_SHA"',
            "git fetch origin main"))

    def test_missing_minimum_toolchain_is_rejected(self):
        self.reject("stable and Rust 1.90", WORKFLOW.replace('toolchain: ["stable", "1.90.0"]',
                                                              'toolchain: ["stable"]'))

    def test_missing_sdr_feature_is_rejected(self):
        self.reject("both optional SDR", WORKFLOW.replace("feature: [soapy-sdr, sdrconnect]",
                                                            "feature: [soapy-sdr]"))

    def test_missing_native_os_is_rejected(self):
        self.reject("native Windows", WORKFLOW.replace("- os: windows-latest", "- os: ubuntu-latest"))

    def test_unverified_or_unpinned_node_is_rejected(self):
        self.reject("SHA-256 pin", bootstrap=BOOTSTRAP.replace(
            'digest="efcc2b0db6f94bf53eddb038fe2b370277a042bd29e4f19b8f8e23598795b819"',
            'digest="missing"'))

    def test_unapproved_rust_toolchain_is_rejected(self):
        self.reject("unapproved Rust", bootstrap=BOOTSTRAP.replace(
            '"$toolchain" == "1.90.0"', '"$toolchain" == "1.81.0"'))


if __name__ == "__main__":
    unittest.main()

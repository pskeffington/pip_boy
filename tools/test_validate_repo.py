#!/usr/bin/env python3
"""Pure-offline regressions for the repository navigation gate."""

import tempfile
import unittest
from pathlib import Path

from validate_repo import audit_documents, audit_navigation


class NavigationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "docs").mkdir()
        self.main = self.root / "README.md"
        self.guide = self.root / "docs/README.md"
        self.guide.write_text("# Docs\n", encoding="utf-8")
        self.main.write_text("[Documentation](docs/README.md#start)\n", encoding="utf-8")
        self.navigation = {"README.md": ("docs/README.md",)}

    def test_full_document_scan_covers_non_index_docs(self):
        (self.root / "docs/EXTRA.md").write_text(
            "[Documentation](README.md)\n", encoding="utf-8"
        )
        self.assertEqual(audit_documents(self.root), (3, 2))

    def test_full_document_scan_rejects_stale_guide(self):
        (self.root / "docs/EXTRA.md").write_text(
            "[Removed map](BOSTON_BASEMAP.md)\n", encoding="utf-8"
        )
        with self.assertRaisesRegex(ValueError, "broken local link"):
            audit_documents(self.root)

    def test_full_document_scan_rejects_linked_document(self):
        (self.root / "docs/EXTRA.md").symlink_to(self.guide)
        with self.assertRaisesRegex(ValueError, "linked or invalid Markdown"):
            audit_documents(self.root)

    def test_valid_local_link_and_fragment(self):
        self.main.write_text("[Documentation](docs/README.md)\n", encoding="utf-8")
        self.assertEqual(audit_navigation(self.root, self.navigation), 1)

    def test_broken_local_link_rejected(self):
        self.main.write_text("[Documentation](docs/NOT_FOUND.md)\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "broken local link"):
            audit_navigation(self.root, self.navigation)

    def test_missing_required_cross_link_rejected(self):
        self.main.write_text("[Elsewhere](https://example.org)\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "required navigation link"):
            audit_navigation(self.root, self.navigation)

    def test_traversal_and_symlink_escape_rejected(self):
        self.main.write_text("[Private](../../secret.md)\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "leaves repository"):
            audit_navigation(self.root, self.navigation)
        self.main.write_text("[Outside](docs/external.md)\n", encoding="utf-8")
        (self.root / "docs/external.md").symlink_to(self.root.parent)
        with self.assertRaisesRegex(ValueError, "leaves repository"):
            audit_navigation(self.root, self.navigation)

    def test_fenced_examples_are_not_links(self):
        self.main.write_text(
            "[Documentation](docs/README.md)\n"
            "```markdown\n[example](docs/DOES_NOT_EXIST.md)\n```\n"
            "`[example](docs/ALSO_MISSING.md)`\n",
            encoding="utf-8",
        )
        self.assertEqual(audit_navigation(self.root, self.navigation), 1)

    def test_absolute_or_backslash_links_rejected(self):
        for path in ("/etc/passwd", r"docs\\README.md"):
            self.main.write_text(f"[Invalid]({path})\n", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "absolute/backslash"):
                audit_navigation(self.root, self.navigation)


if __name__ == "__main__":
    unittest.main()

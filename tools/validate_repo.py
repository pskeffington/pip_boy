#!/usr/bin/env python3
"""Check Rust-Boy's critical Markdown navigation without network or Git access.

This validates local link targets in the root and documentation indexes; it is
not a general-purpose Markdown parser and intentionally does not crawl user data.
"""

import re
import sys
from pathlib import Path
from urllib.parse import unquote

from validation_common import require

ROOT = Path(__file__).resolve().parents[1]
NAVIGATION = {
    "README.md": (
        "docs/README.md", "docs/REPO_STRUCTURE.md", "docs/ARCHITECTURE.md",
        "docs/VALIDATION.md", "CONTRIBUTING.md", "SECURITY.md",
    ),
    "docs/README.md": (
        "../README.md", "REPO_STRUCTURE.md", "ARCHITECTURE.md",
        "STORAGE.md", "VALIDATION.md",
    ),
    "docs/REPO_STRUCTURE.md": (
        "../README.md", "README.md", "ARCHITECTURE.md", "STORAGE.md",
    ),
}
LINK = re.compile(r"!?\[[^\]\n]+\]\(([^)\n]+)\)")
REFERENCE_DEFINITION = re.compile(r"^ {0,3}\[[^\]\n]+\]:\s*(?:<([^>]+)>|(\S+))")
FENCE = re.compile(r"^\s*(`{3,}|~{3,})")
INLINE_CODE = re.compile(r"`[^`]*`")
SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")


def markdown_links(source):
    """Yield (line, local destination) from ordinary, non-fenced Markdown."""
    fence = None
    for lineno, line in enumerate(source.splitlines(), start=1):
        marker = FENCE.match(line)
        if marker:
            current = marker.group(1)[0]
            if fence is None:
                fence = current
            elif fence == current:
                fence = None
            continue
        if fence is not None:
            continue
        line = INLINE_CODE.sub("", line)
        candidates = [match.group(1) for match in LINK.finditer(line)]
        definition = REFERENCE_DEFINITION.match(line)
        if definition and not line.lstrip().startswith("[^"):
            candidates.append(definition.group(1) or definition.group(2))
        for raw in candidates:
            parts = raw.strip().split()
            if not parts:
                continue
            candidate = parts[0].strip("<>")
            if not candidate or candidate.startswith("#"):
                continue
            if SCHEME.match(candidate) or candidate.startswith("//"):
                continue
            target = unquote(candidate.split("#", 1)[0].split("?", 1)[0])
            if target:
                yield lineno, target


def check_local_target(root, file, lineno, target):
    """Reject escape paths, absolute paths and broken source-relative links."""
    source = file.relative_to(root).as_posix()
    require("\\" not in target and not target.startswith("/"),
            f"{source}:{lineno}: unsupported absolute/backslash link: {target}")
    destination = (file.parent / target).resolve()
    require(destination.is_relative_to(root),
            f"{source}:{lineno}: link leaves repository: {target}")
    require(destination.exists(),
            f"{source}:{lineno}: broken local link: {target}")


def audit_navigation(root=ROOT, navigation=NAVIGATION):
    """Validate relative link targets and required cross-links from index pages."""
    root = Path(root).resolve()
    checked = 0
    for source, required in navigation.items():
        file = root / source
        require(file.is_file() and not file.is_symlink(),
                f"{source}: navigation file missing or symlinked")
        destinations = set()
        for lineno, target in markdown_links(file.read_text(encoding="utf-8")):
            check_local_target(root, file, lineno, target)
            destinations.add(target)
            checked += 1
        for target in required:
            require(target in destinations,
                    f"{source}: required navigation link is missing: {target}")
    return checked


def audit_documents(root=ROOT):
    """Audit only checked-in public Markdown locations, never runtime storage."""
    root = Path(root).resolve()
    scanned = links = 0
    for directory in (root, root / "catalog", root / "docs", root / "data"):
        if not directory.exists():
            continue
        require(directory.is_dir() and not directory.is_symlink(),
                f"invalid documentation directory: {directory}")
        files = directory.glob("*.md") if directory == root else directory.rglob("*.md")
        for file in sorted(files):
            require(not any(part.is_symlink() for part in file.parents
                                if part != root and part.is_relative_to(root)),
                    f"symlinked Markdown directory: {file}")
            require(file.is_file() and not file.is_symlink(),
                    f"linked or invalid Markdown file: {file}")
            for lineno, target in markdown_links(file.read_text(encoding="utf-8")):
                check_local_target(root, file, lineno, target)
                links += 1
            scanned += 1
    return scanned, links


def validate():
    links = audit_navigation()
    documents, public_links = audit_documents()
    workflow = (ROOT / ".github/workflows/rust.yml").read_text(encoding="utf-8")
    for command in ("python3 tools/validate_repo.py", "python3 tools/test_validate_repo.py"):
        require(command in workflow, f"CI missing repository hygiene gate: {command}")
    print(f"PASS: {len(NAVIGATION)} navigation pages, {links} index links; "
          f"{documents} public documents, {public_links} links; "
          "mandatory cross-links and CI gates")


if __name__ == "__main__":
    try:
        validate()
    except (ValueError, OSError, UnicodeError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        raise SystemExit(1) from error

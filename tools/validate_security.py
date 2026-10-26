#!/usr/bin/env python3
"""Fail-closed, offline source-tree leakage checks for public Rust-Boy commits.

This inspects the *tracked checkout*, not Git history or PIPBOY_HOME. Matching
content is never displayed. It is a small preventive gate, not a replacement
for full secret scanning, dependency auditing, or a professional penetration test.
"""

import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MAX_SOURCE_BYTES = 4 * 1024 * 1024
OPERATOR_FILES = frozenset({
    "notes.json", "permissions.json", "workbench.json", "tasks.json",
    "peers.json", "messages.json", "messages.recipient", "message-pace.json",
    "reclamation.json", "notes.lock", "permissions.lock", "workbench.lock",
    "tasks.lock", "peers.lock", "messages.lock", "message-dispatch.lock",
    "message-pace.lock", "reclamation.lock",
})
PRIVATE_DIRECTORIES = frozenset({
    ".pipboy", "county_cache", "local-state", "node_modules", "target",
})
PRIVATE_EXTENSIONS = frozenset({
    ".pem", ".key", ".p12", ".pfx", ".sqlite", ".sqlite3", ".db",
    ".gpx", ".kml", ".tcx",
})
PRIVATE_KEY_NAMES = ("id_rsa", "id_ed25519", "id_ecdsa", "id_dsa")
# Strong signatures only. Test fixtures construct examples at runtime rather
# than embedding plausible token strings in committed files.
CREDENTIAL_SIGNATURES = (
    ("private-key-block", re.compile(
        r"-{5}BEGIN\s+(?:(?:RSA|EC|OPENSSH|DSA|ENCRYPTED)\s+)?PRIVATE KEY-{5}"
    )),
    ("github-token", re.compile(r"\b(?:gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,})\b")),
    ("aws-access-key", re.compile(r"\bAKIA[0-9A-Z]{16}\b")),
    ("api-token", re.compile(r"\bsk-(?:proj-)?[A-Za-z0-9_-]{24,}\b")),
    ("slack-token", re.compile(r"\bxox[baprs]-[A-Za-z0-9-]{24,}\b")),
    ("google-api-key", re.compile(r"\bAIza[0-9A-Za-z_-]{35}\b")),
    ("credentials-in-url", re.compile(r"https?://[^\s/@:]+:[^\s/@]+@", re.I)),
)
UNREVIEWED_CHILD_PROCESS = re.compile(r"\bCommand\s*::\s*new\s*\(")


def path_issue(relative):
    """Return a public-commit privacy violation, or None."""
    relative = Path(relative)
    if relative.is_absolute() or ".." in relative.parts:
        return "unsafe-tracked-path"
    parts = [part.lower() for part in relative.parts]
    name = parts[-1]
    if any(part in PRIVATE_DIRECTORIES for part in parts[:-1]):
        return "runtime-data-directory"
    if name.startswith(".env") and name not in {".env.example", ".env.template"}:
        return "private-environment-file"
    if (name in OPERATOR_FILES
            or name.startswith(PRIVATE_KEY_NAMES)
            or relative.suffix.lower() in PRIVATE_EXTENSIONS):
        return "private-operator-or-key-file"
    return None


def content_issues(relative, raw):
    """Return only category/line tuples; never return matched credential text."""
    issues = []
    if len(raw) > MAX_SOURCE_BYTES:
        return [("oversized-file-needs-review", 0)]
    if b"\x00" in raw:
        return issues  # filename policy still applies to tracked binary files
    lines = raw.decode("utf-8", "replace").splitlines()
    for number, line in enumerate(lines, 1):
        for label, pattern in CREDENTIAL_SIGNATURES:
            if pattern.search(line):
                issues.append((label, number))
        if (relative.startswith("crates/")
                and relative.endswith(".rs")
                and not relative.endswith("/process_safety.rs")
                and UNREVIEWED_CHILD_PROCESS.search(line)):
            issues.append(("unreviewed-child-environment", number))
    return issues


def scan_tracked(root=ROOT):
    """Inspect Git's tracked checkout, never the user's home or runtime files."""
    root = Path(root).resolve()
    result = subprocess.run(
        ["git", "ls-files", "--cached", "-z"],
        cwd=root, check=True, capture_output=True,
    )
    paths = [os.fsdecode(value) for value in result.stdout.split(b"\0") if value]
    issues = []
    for relative in paths:
        path = root / relative
        problem = path_issue(relative)
        if problem:
            issues.append((relative, 0, problem))
        if path.is_symlink():
            issues.append((relative, 0, "tracked-symlink-needs-review"))
            continue
        if not path.is_file():
            issues.append((relative, 0, "tracked-file-unavailable"))
            continue
        for label, number in content_issues(relative, path.read_bytes()):
            issues.append((relative, number, label))
    return paths, issues


def main():
    try:
        paths, findings = scan_tracked()
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"SECURITY FAIL: tracked checkout cannot be audited ({type(error).__name__})",
              file=sys.stderr)
        return 1
    for relative, number, category in findings:
        # In particular, never copy detected secret values to public CI logs.
        print(f"SECURITY FAIL: {relative}:{number} [{category}]", file=sys.stderr)
    if findings:
        print(f"SECURITY FAIL: {len(findings)} finding(s); do not merge", file=sys.stderr)
        return 1
    print(f"SECURITY PASS: {len(paths)} tracked paths checked; no blocked operator "
          "artifacts, high-confidence token signatures or unsanitized Rust process launches")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

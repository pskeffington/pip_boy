#!/usr/bin/env python3
"""Record a failed macOS release attempt without capturing command output."""

import json
import os
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 4:
        print("usage: record_macos_release_failure.py PATH SOURCE_SHA EXIT_CODE", file=sys.stderr)
        return 2

    path = Path(sys.argv[1])
    source_sha = sys.argv[2]
    try:
        exit_code = int(sys.argv[3])
    except ValueError:
        print("exit code must be an integer", file=sys.stderr)
        return 2

    if not path.parent.is_dir() or path.is_symlink():
        print("release context must be a regular file in an existing attempt directory", file=sys.stderr)
        return 2

    record = {"schema_version": 1}
    if path.exists():
        if not path.is_file():
            print("release context must be a regular file", file=sys.stderr)
            return 2
        with path.open(encoding="utf-8") as source:
            record = json.load(source)
        if not isinstance(record, dict):
            print("release context must contain a JSON object", file=sys.stderr)
            return 2

    record.update(
        {
            "state": "failed",
            "source_sha": source_sha,
            "failed_utc": datetime.now(timezone.utc).isoformat(),
            "exit_code": exit_code,
        }
    )

    fd, temporary = tempfile.mkstemp(prefix=".release-context.", dir=path.parent)
    try:
        os.fchmod(fd, 0o600)
        with os.fdopen(fd, "w", encoding="utf-8") as output:
            json.dump(record, output, indent=2, sort_keys=True)
            output.write("\n")
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

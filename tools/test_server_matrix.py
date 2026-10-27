#!/usr/bin/env python3
"""No-network regression tests for matrix indexing and malformed source rows."""
from validate_server_matrix import check
from pathlib import Path
root = Path(__file__).resolve().parents[1]
good = (root / "server/LITERATURE_MATRIX.md").read_text(encoding="utf-8")
assert check(good) >= 12
for broken in (good.replace("| L01 |", "| L02 |", 1),
               good.replace("https://csrc.nist.gov/pubs/sp/800/53/r5/final", "http://invalid", 1),
               good.replace("| S7 |", "| S6 |", 1)):
    try:
        check(broken)
    except AssertionError:
        pass
    else:
        raise AssertionError("malformed literature matrix unexpectedly passed")
print("PASS: literature matrix validation regression fixtures")

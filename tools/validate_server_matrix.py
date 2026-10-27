#!/usr/bin/env python3
"""Offline structural validation of server literature/source and phase mapping."""
import re
from pathlib import Path
from urllib.parse import urlparse
ROOT = Path(__file__).resolve().parents[1]
MATRIX = ROOT / "server/LITERATURE_MATRIX.md"
PHASES = set(f"S{i}" for i in range(8))
SOURCES = re.compile(r"^\| (L\d{2}) \| \[([^]]+)\]\((https://[^)]+)\) \| (.+) \| (.+) \| (.+) \|$", re.M)

def check(text: str) -> int:
    rows = SOURCES.findall(text)
    ids = [r[0] for r in rows]
    assert len(rows) >= 12 and len(ids) == len(set(ids)), "missing or duplicate literature rows"
    assert ids == [f"L{i:02}" for i in range(1, len(rows)+1)], "noncanonical literature IDs"
    for ident, title, url, domain, implication, evidence in rows:
        host = urlparse(url).hostname
        assert host and not host.endswith(".invalid"), f"{ident}: invalid source URL"
        assert title.strip() and domain.strip() and implication.strip() and evidence.strip(), f"{ident}: incomplete row"
        assert re.search(r"S[0-7]", evidence), f"{ident}: missing phase"
    gates = re.findall(r"^\| (S[0-7]) \|", text, re.M)
    assert set(gates) == PHASES and len(gates) == 8, "matrix must cover S0 through S7 exactly once"
    for term in ("**Hospital:**", "**Tunnel / underground:**", "**Distributed / leave-the-wire:**"):
        assert term in text, f"missing scenario {term}"
    return len(rows)

if __name__ == "__main__":
    n = check(MATRIX.read_text(encoding="utf-8"))
    print(f"PASS: {n} unique literature sources and 8 phased gates; offline structural validation")

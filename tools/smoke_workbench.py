#!/usr/bin/env python3
"""Offline workbench acceptance: role presets, Custom, helpers, boundaries.

Uses only the built pipboy CLI and loop-free, local files. No camera or
PlayStation connection is opened; no real hardware claim is implied.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target" / "debug" / ("pipboy.exe" if os.name == "nt" else "pipboy")


def call(env, *args, ok=True):
    result = subprocess.run(
        [str(BIN), "workbench", *args],
        env=env,
        capture_output=True,
        text=True,
        cwd=ROOT,
        timeout=12,
    )
    if ok:
        assert result.returncode == 0, (args, result.returncode, result.stderr)
    else:
        assert result.returncode != 0, (args, result.stdout)
        assert result.stdout == "", (args, result.stdout)
    return result.stdout


with tempfile.TemporaryDirectory() as temporary:
    root = Path(temporary) / "dev-profile"
    env = dict(os.environ, PIPBOY_HOME=str(root))
    initial = json.loads(call(env, "list", "--json"))
    assert initial["profile"] == "overseer"
    assert initial["title"] == "Overseer Operations"
    assert [slot["tool"] for slot in initial["slots"]] == [
        "host", "notes", "library", "game"
    ]
    profiles = json.loads(call(env, "profiles", "--json"))
    assert [item["id"] for item in profiles] == [
        "overseer", "head-engineer", "security-chief", "vault-doctor",
        "university-researcher", "reclamation-scout",
        "records-custodian", "custom"
    ]
    assert profiles[-1]["tools"] == []
    palette = json.loads(call(env, "palette", "--json"))
    status = {item["id"]: item["availability"] for item in palette}
    assert status["net-lab"] == "ready"
    assert status["gpio"] == "planned"
    assert status["game"] == "explicit_connection"
    assert "Network: 192.168.1.0/24" in call(
        env, "net", "cidr", "192.168.1.50/24"
    )
    assert "Usable addresses: 2" in call(env, "net", "cidr", "10.0.0.4/31")
    assert "CRC32/IEEE: cbf43926" in call(
        env, "bytes", "crc32", "313233343536373839"
    )
    assert "Hello.." in call(
        env, "bytes", "inspect", "48656c6c6f001b"
    )
    for bad in [
        ("use", "security-chief"),
        ("use", "not-a-profile", "--replace"),
        ("add", "gpio"),
        ("net", "cidr", "192.168.1.5/35"),
        ("bytes", "crc32", "0x1"),
        ("clear",),
    ]:
        call(env, *bad, ok=False)
    assert not root.exists(), "reading tools or rejecting input wrote to disk"

    assert "APPLIED profile security-chief" in call(
        env, "use", "security-chief", "--replace"
    )
    board = json.loads(call(env, "list", "--json"))
    assert board["profile"] == "security-chief"
    assert [slot["tool"] for slot in board["slots"]] == [
        "host", "net-lab", "byte-lab", "notes"
    ]
    assert not (root / "notes.json").exists()
    assert not (root / "permissions.json").exists()
    assert "APPLIED profile custom" in call(env, "use", "custom", "--replace")
    custom = json.loads(call(env, "list", "--json"))
    assert custom["profile"] == "custom" and custom["slots"] == []
    assert "PINNED #1 byte-lab" in call(env, "add", "byte-lab")
    persisted = root / "workbench.json"
    before = persisted.read_bytes()
    call(env, "add", "byte-lab", ok=False)
    call(env, "add", "nfc", ok=False)
    assert persisted.read_bytes() == before
    assert "CLEARED" in call(env, "clear", "--confirm")
    assert json.loads(persisted.read_text())["slots"] == []
    assert not (root / "notes.json").exists()
    assert not (root / "permissions.json").exists()
    if os.name != "nt":
        assert persisted.stat().st_mode & 0o777 == 0o600
        assert root.stat().st_mode & 0o777 == 0o700

    # A version-1 board with no profile field is upgraded in memory only.
    old = '{"schema_version":1,"title":"Legacy","slots":[{"id":1,"tool":"notes"}]}'
    persisted.write_text(old)
    legacy = json.loads(call(env, "list", "--json"))
    assert legacy["profile"] == "custom" and legacy["slots"][0]["tool"] == "notes"
    assert persisted.read_text() == old
    for malformed in [
        "{invalid",
        '{"schema_version":2,"title":"Future","slots":[]}',
        '{"schema_version":1,"profile":"overseer","title":"Bad","slots":[{"id":1,"tool":"radio"}]}',
    ]:
        persisted.write_text(malformed)
        call(env, "list", "--json", ok=False)
        call(env, "use", "overseer", "--replace", ok=False)
        assert persisted.read_text() == malformed

print("PASS: eight role presets, default Overseer, empty Custom, no implicit I/O, offline CIDR/CRC32/hex, local pinning, legacy schema and fail-closed malformed manifests")

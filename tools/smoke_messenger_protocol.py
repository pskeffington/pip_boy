#!/usr/bin/env python3
"""Offline CLI acceptance: recorded USB only, no keyring/device/network required."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target/debug" / ("pipboy.exe" if os.name == "nt" else "pipboy")


def run(home, *args, ok=True):
    result = subprocess.run([str(BIN), "peer", *args], env=dict(os.environ, PIPBOY_HOME=str(home)),
                            capture_output=True, text=True, timeout=10)
    assert (result.returncode == 0) == ok, (args, result.stdout, result.stderr)
    return result


with tempfile.TemporaryDirectory(prefix="rust-boy-protocol-") as scratch:
    root = Path(scratch)
    home = root / "home"
    capture = root / "usb.bin"
    capture.write_bytes(b"boot log\n\x94\xc3\x00\x02\x08\x01\x94\xc3\x00\x01\x10")
    result = run(home, "meshtastic-capture", str(capture), "--confirm")
    assert "2 untrusted FromRadio frames" in result.stdout
    assert not home.exists(), "Reading a capture must not create local state"
    run(home, "meshtastic-capture", str(capture), ok=False)
    capture.write_bytes(b"x" * (1024 * 1024 + 1))
    run(home, "meshtastic-capture", str(capture), "--confirm", ok=False)
    for ip in ["8.8.8.8", "127.0.0.1", "scout.local", "192.168.1.2;id"]:
        run(home, "message-retry-local", "1", "scout", ip, "--confirm", ok=False)
    # Enrollment cannot replace an existing plaintext journal.
    home.mkdir()
    journal = home / "messages.json"
    book = {"schema_version": 1, "next_id": 1, "messages": []}
    journal.write_text(json.dumps(book))
    original = journal.read_bytes()
    run(home, "message-key-init", "--confirm", ok=False)
    assert journal.read_bytes() == original
    assert not (home / "messages.recipient").exists()
print("PASS: Messenger CLI capture bounds, no device access, fallback validation, enrollment preservation")

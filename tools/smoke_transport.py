#!/usr/bin/env python3
"""Strict CLI regression for bounded mock transport; no device access."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target" / "debug" / ("pipboy.exe" if os.name == "nt" else "pipboy")


def run(env, *args, success=True):
    proc = subprocess.run(
        [str(BINARY), "simulate", "transport", *args],
        text=True, capture_output=True, env=env, timeout=10, check=False,
    )
    if success:
        assert proc.returncode == 0, (args, proc.stderr)
    else:
        assert proc.returncode != 0, (args, proc.stdout)
        assert not proc.stdout.strip(), (args, "unexpected event output")
    return proc


def check_report(raw, device, capability, requested, capacity):
    doc = json.loads(raw)
    assert set(doc) == {
        "schema_version", "simulated", "source", "device_id", "capability",
        "capacity", "requested", "dropped", "retained",
    }
    assert doc["schema_version"] == 1
    assert doc["simulated"] is True
    assert doc["source"] == "rust-boy/bounded-virtual-transport"
    assert doc["device_id"] == device
    assert doc["capability"] == capability
    assert doc["requested"] == requested
    assert doc["capacity"] == capacity
    assert doc["dropped"] == max(0, requested - capacity)
    assert len(doc["retained"]) == min(requested, capacity)
    for num, event in enumerate(doc["retained"], start=requested - len(doc["retained"]) + 1):
        assert set(event) == {
            "schema_version", "simulated", "source", "device_id", "capability",
            "event", "sequence",
        }
        assert event["schema_version"] == 1
        assert event["simulated"] is True
        assert event["source"] == "rust-boy/in-memory-simulator"
        assert event["device_id"] == device
        assert event["capability"] == capability
        assert event["event"] == "fixture_only_no_hardware_io"
        assert event["sequence"] == num
    return doc


def main():
    assert BINARY.is_file(), f"build pipboy first: {BINARY}"
    with tempfile.TemporaryDirectory(prefix="rust-boy-transport-") as temporary:
        root = Path(temporary) / "uncreated"
        env = dict(os.environ, PIPBOY_HOME=str(root))
        denied = run(env, "sim:sdr", "--count", "5", "--capacity", "2", success=False)
        assert "confirmation required" in denied.stderr
        assert not root.exists()

        args = ("sim:sdr", "--count", "5", "--capacity", "2", "--confirm")
        first = run(env, *args).stdout
        check_report(first, "sim:sdr", "sdr.receive", 5, 2)
        assert first == run(env, *args).stdout, "mock transport was nondeterministic"
        check_report(
            run(env, "sim:mesh", "--count", "3", "--capacity", "4", "--confirm").stdout,
            "sim:mesh", "mesh.transmit", 3, 4,
        )
        assert not root.exists(), "mock operation wrote user data"

        for invalid in (
            ("sim:sdr", "--count", "0", "--capacity", "2", "--confirm"),
            ("sim:sdr", "--count", "129", "--capacity", "2", "--confirm"),
            ("sim:sdr", "--count", "5", "--capacity", "0", "--confirm"),
            ("sim:sdr", "--count", "5", "--capacity", "65", "--confirm"),
            ("real:sdr", "--count", "5", "--capacity", "2", "--confirm"),
            ("sim:sdr", "--capacity", "2", "--count", "5", "--confirm"),
            ("sim:sdr", "--count", "5", "--capacity", "2", "--confirm", "extra"),
        ):
            run(env, *invalid, success=False)
        assert not root.exists(), "invalid input created user data"

        def cmd(*args):
            p = subprocess.run(
                [str(BINARY), *args], capture_output=True, text=True,
                env=env, check=False, timeout=10,
            )
            assert p.returncode == 0, (args, p.stderr)

        cmd("permissions", "request", "sdr.receive")
        run(env, "sim:sdr", "--count", "2", "--capacity", "2", success=False)
        check_report(run(env, "sim:sdr", "--count", "2", "--capacity", "2", "--confirm").stdout,
                     "sim:sdr", "sdr.receive", 2, 2)
        cmd("permissions", "deny", "sdr.receive")
        blocked = run(env, *args, success=False)
        assert "denies this capability" in blocked.stderr
        path = root / "permissions.json"
        broken = b'{"schema_version":2,"entries":[]}'
        path.write_bytes(broken)
        run(env, *args, success=False)
        assert path.read_bytes() == broken

    print("PASS: bounded mock FIFO, deterministic order/drop accounting, "
          "deny-first policy, no implicit files, strict CLI and schema rejection")


if __name__ == "__main__":
    main()

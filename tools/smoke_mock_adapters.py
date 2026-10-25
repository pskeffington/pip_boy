#!/usr/bin/env python3
"""Black-box typed mock adapter regression. Never accesses OS devices."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target" / "debug" / ("pipboy.exe" if os.name == "nt" else "pipboy")

FIXTURES = {
    "input": {
        "source": "rust-boy/mock-input-adapter",
        "device_id": "sim:gpio",
        "event": "no_physical_input_observed",
        "keys": {"schema_version", "simulated", "source", "device_id", "event", "sequence"},
        "capability": "gpio.read",
    },
    "display": {
        "source": "rust-boy/mock-display-adapter",
        "device_id": "sim:display",
        "event": "mock_frame_accepted_not_rendered",
        "keys": {
            "schema_version", "simulated", "rendered_to_hardware", "source",
            "device_id", "event", "accepted_chars", "sequence",
        },
        "capability": "display.write",
    },
    "serial": {
        "source": "rust-boy/mock-serial-adapter",
        "device_id": "sim:serial",
        "event": "explicitly_injected_fixture_bytes",
        "keys": {
            "schema_version", "simulated", "injected", "source",
            "device_id", "event", "fixture_bytes", "sequence",
        },
        "capability": "serial.read",
    },
}


def run(env, *args, success=True):
    proc = subprocess.run(
        [str(BINARY), "simulate", "adapter", *args],
        capture_output=True, text=True, env=env, timeout=10, check=False,
    )
    if success:
        assert proc.returncode == 0, (args, proc.stderr)
        assert not proc.stderr.strip(), (args, proc.stderr)
    else:
        assert proc.returncode != 0, (args, proc.stdout)
        assert not proc.stdout.strip(), (args, "denied adapter emitted mock data")
    return proc


def main():
    assert BINARY.is_file(), f"build CLI before smoke tests: {BINARY}"
    with tempfile.TemporaryDirectory(prefix="rust-boy-adapters-") as temp:
        path = Path(temp) / "uncreated"
        env = dict(os.environ, PIPBOY_HOME=str(path))
        for kind, expected in FIXTURES.items():
            missing = run(env, kind, success=False)
            assert "confirmation required" in missing.stderr
            raw = run(env, kind, "--confirm").stdout
            assert raw == run(env, kind, "--confirm").stdout
            doc = json.loads(raw)
            assert set(doc) == expected["keys"], (kind, doc)
            assert doc["schema_version"] == 1 and doc["simulated"] is True
            for field in ("source", "device_id", "event"):
                assert doc[field] == expected[field]
            assert doc["sequence"] == 1
            if kind == "input":
                assert "button_pressed" not in doc and "voltage" not in doc
            elif kind == "display":
                assert doc["rendered_to_hardware"] is False
                assert doc["accepted_chars"] == 8
            else:
                assert doc["injected"] is True
                assert doc["fixture_bytes"] == [82, 66]
            assert not path.exists(), "mock adapter created user data"

        for values in (
            (),
            ("invalid", "--confirm"),
            ("display", "--bad"),
            ("serial", "--confirm", "unexpected"),
            ("input", "--confirm", "--confirm"),
        ):
            run(env, *values, success=False)
        assert not path.exists(), "invalid args created user data"

        for kind, expected in FIXTURES.items():
            result = subprocess.run(
                [str(BINARY), "permissions", "request", expected["capability"]],
                capture_output=True, text=True, check=False, env=env, timeout=10,
            )
            assert result.returncode == 0, result.stderr
            run(env, kind, success=False)
            run(env, kind, "--confirm")
            result = subprocess.run(
                [str(BINARY), "permissions", "deny", expected["capability"]],
                capture_output=True, text=True, check=False, env=env, timeout=10,
            )
            assert result.returncode == 0, result.stderr
            denied = run(env, kind, "--confirm", success=False)
            assert "denies this capability" in denied.stderr

        corrupt = b'{"schema_version":2,"entries":[]}'
        preference_path = path / "permissions.json"
        preference_path.write_bytes(corrupt)
        for kind in FIXTURES:
            run(env, kind, "--confirm", success=False)
        assert preference_path.read_bytes() == corrupt

    print(
        "PASS: typed GPIO/display/serial mock fixtures, strict JSON, "
        "explicit confirmation, saved denial, corrupt schema, zero hardware I/O"
    )


if __name__ == "__main__":
    main()

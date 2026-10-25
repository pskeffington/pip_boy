#!/usr/bin/env python3
"""Black-box Rust-Boy broker simulator checks. No physical I/O is requested."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target" / "debug" / (
    "pipboy.exe" if os.name == "nt" else "pipboy"
)

DEVICES = {
    "sim:enumerate": "device.enumerate",
    "sim:gpio": "gpio.read",
    "sim:display": "display.write",
    "sim:serial": "serial.read",
    "sim:gnss": "gnss.read",
    "sim:sdr": "sdr.receive",
    "sim:ble": "ble.discover",
    "sim:mesh": "mesh.transmit",
    "sim:sensor": "sensor.read",
    "sim:audio": "audio.play",
    "sim:network": "network.fetch",
}


def command(env, *args, allowed=True):
    result = subprocess.run(
        [str(BINARY), *args],
        env=env,
        capture_output=True,
        text=True,
        timeout=10,
        check=False,
    )
    if allowed:
        assert result.returncode == 0, (args, result.stderr)
    else:
        assert result.returncode != 0, (args, result.stdout)
    return result


def assert_simulation(result, device):
    event = json.loads(result.stdout)
    assert set(event) == {
        "schema_version", "simulated", "source",
        "device_id", "capability", "event", "sequence",
    }, event
    assert event["schema_version"] == 1
    assert event["simulated"] is True
    assert event["source"] == "rust-boy/in-memory-simulator"
    assert event["device_id"] == device
    assert event["capability"] == DEVICES[device]
    assert event["event"] == "fixture_only_no_hardware_io"
    assert event["sequence"] == 1
    assert not any(key in event for key in ("latitude", "longitude", "signal", "value"))


def main():
    assert BINARY.exists(), f"build the Rust-Boy binary first: {BINARY}"
    with tempfile.TemporaryDirectory(prefix="rust-boy-simulator-") as temporary:
        root = Path(temporary) / "uncreated"
        env = dict(os.environ, PIPBOY_HOME=str(root))
        listing = command(env, "simulate", "list").stdout
        for device, capability in DEVICES.items():
            assert device in listing and capability in listing
        assert listing.count("(SIMULATED)") == len(DEVICES)
        assert not root.exists(), "static fixture list created a data directory"

        no_confirm = command(env, "simulate", "run", "sim:sdr", allowed=False)
        assert "confirmation required" in no_confirm.stderr
        assert not root.exists(), "denied run created a data directory"

        assert_simulation(
            command(env, "simulate", "run", "sim:gnss", "--confirm"),
            "sim:gnss",
        )
        assert not root.exists(), "simulation wrote into the data directory"

        command(env, "simulate", "run", "real:gpio", "--confirm", allowed=False)
        command(env, "simulate", "run", "sim:mesh", "--invalid", allowed=False)
        command(env, "simulate", "run", "sim:mesh", "--confirm", "extra", allowed=False)
        command(env, "simulate", "list", "extra", allowed=False)
        assert not root.exists(), "invalid simulation input wrote files"

        command(env, "permissions", "request", "sdr.receive")
        command(env, "simulate", "run", "sim:sdr", allowed=False)
        assert_simulation(
            command(env, "simulate", "run", "sim:sdr", "--confirm"),
            "sim:sdr",
        )
        assert json.loads((root / "permissions.json").read_text())["entries"] == [
            {"capability": "sdr.receive", "preference": "requested"}
        ]

        command(env, "permissions", "deny", "sdr.receive")
        denied = command(
            env, "simulate", "run", "sim:sdr", "--confirm", allowed=False
        )
        assert "denies this capability" in denied.stderr
        command(env, "permissions", "clear", "sdr.receive")
        assert_simulation(
            command(env, "simulate", "run", "sim:sdr", "--confirm"),
            "sim:sdr",
        )

        preference_path = root / "permissions.json"
        original = b'{"schema_version":2,"entries":[]}'
        preference_path.write_bytes(original)
        corrupt = command(
            env, "simulate", "run", "sim:sdr", "--confirm", allowed=False
        )
        assert "unsupported preferences schema" in corrupt.stderr
        assert preference_path.read_bytes() == original
        assert "fixed in-memory fixtures" in command(
            env, "simulate", "list"
        ).stdout

    print(
        "PASS: 11 static fixtures, no implicit filesystem activity, explicit "
        "simulation confirmation, saved denial, malformed schema, truthful "
        "fixture JSON and invalid argument rejection"
    )


if __name__ == "__main__":
    main()

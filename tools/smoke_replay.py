#!/usr/bin/env python3
"""Black-box deterministic replay checks; fixed virtual fixtures only."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target" / "debug" / (
    "pipboy.exe" if os.name == "nt" else "pipboy"
)
SCENARIOS = {
    "nominal": {
        "times": [0, 100, 200],
        "actions": ["confirm_open", "sample_fixture", "close"],
        "states": ["fixture_active", "fixture_active", "inactive"],
        "rejections": {},
    },
    "disconnect-recover": {
        "times": [0, 100, 200, 300, 400, 500, 600, 700],
        "actions": [
            "confirm_open",
            "sample_fixture",
            "disconnect",
            "sample_stale_lease",
            "open_disconnected",
            "reconnect",
            "sample_stale_lease",
            "open_without_new_confirmation",
        ],
        "states": [
            "fixture_active",
            "fixture_active",
            "disconnected",
            "disconnected",
            "disconnected",
            "inactive",
            "inactive",
            "inactive",
        ],
        "rejections": {
            3: "invalid_lease",
            4: "unavailable",
            6: "invalid_lease",
            7: "confirmation_required",
        },
    },
    "revoke": {
        "times": [0, 100, 200, 300, 400],
        "actions": [
            "confirm_open",
            "sample_fixture",
            "revoke",
            "sample_stale_lease",
            "open_without_new_confirmation",
        ],
        "states": [
            "fixture_active",
            "fixture_active",
            "inactive",
            "inactive",
            "inactive",
        ],
        "rejections": {3: "invalid_lease", 4: "confirmation_required"},
    },
    "cancel": {
        "times": [0, 100, 200, 300, 400],
        "actions": [
            "confirm_open",
            "sample_fixture",
            "cancel",
            "sample_stale_lease",
            "open_without_new_confirmation",
        ],
        "states": [
            "fixture_active",
            "fixture_active",
            "inactive",
            "inactive",
            "inactive",
        ],
        "rejections": {3: "invalid_lease", 4: "confirmation_required"},
    },
    "timeout": {
        "times": [0, 100, 200, 300, 400],
        "actions": [
            "confirm_open",
            "sample_fixture",
            "advance_to_deadline",
            "sample_stale_lease",
            "open_without_new_confirmation",
        ],
        "states": [
            "fixture_active",
            "fixture_active",
            "inactive",
            "inactive",
            "inactive",
        ],
        "rejections": {3: "invalid_lease", 4: "confirmation_required"},
    },
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
        assert not result.stdout.strip(), (args, "unexpected success output")
    return result


def trace_is_valid(output, device, capability, scenario):
    trace = json.loads(output)
    expected = SCENARIOS[scenario]
    assert set(trace) == {
        "schema_version", "simulated", "source", "clock",
        "scenario", "device_id", "capability", "steps",
    }
    assert trace["schema_version"] == 1
    assert trace["simulated"] is True
    assert trace["source"] == "rust-boy/deterministic-simulator"
    assert trace["clock"] == "synthetic_logical_ms"
    assert trace["scenario"] == scenario
    assert trace["device_id"] == device
    assert trace["capability"] == capability
    steps = trace["steps"]
    assert [step["logical_ms"] for step in steps] == expected["times"]
    assert [step["action"] for step in steps] == expected["actions"]
    assert [step["state"] for step in steps] == expected["states"]
    for index, step in enumerate(steps):
        assert step["outcome"] == (
            "rejected" if index in expected["rejections"] else "ok"
        )
        assert step.get("error") == expected["rejections"].get(index)
        if index == 1:
            event = step["event"]
            assert event["simulated"] is True
            assert event["source"] == "rust-boy/in-memory-simulator"
            assert event["device_id"] == device
            assert event["capability"] == capability
            assert event["event"] == "fixture_only_no_hardware_io"
            assert event["sequence"] == 1
            assert set(event) == {
                "schema_version", "simulated", "source", "device_id",
                "capability", "event", "sequence",
            }
        else:
            assert "event" not in step
    assert all(
        key not in output.lower()
        for key in ("latitude", "longitude", "frequency_hz", "rssi", "uptime_s")
    )


def main():
    assert BINARY.exists(), f"Build Rust-Boy before running: {BINARY}"
    with tempfile.TemporaryDirectory(prefix="rust-boy-replay-") as temporary:
        home = Path(temporary) / "not-yet-created"
        env = dict(os.environ, PIPBOY_HOME=str(home))

        denied = command(
            env, "simulate", "replay", "sim:gpio", "nominal", allowed=False
        )
        assert "confirmation required" in denied.stderr
        assert not home.exists(), "denied replay created files"

        for scenario, capability, device in (
            ("nominal", "gpio.read", "sim:gpio"),
            ("disconnect-recover", "serial.read", "sim:serial"),
            ("revoke", "sdr.receive", "sim:sdr"),
            ("cancel", "ble.discover", "sim:ble"),
            ("timeout", "network.fetch", "sim:network"),
        ):
            args = (
                "simulate", "replay", device, scenario, "--confirm"
            )
            first = command(env, *args).stdout
            assert command(env, *args).stdout == first, "non-deterministic replay"
            trace_is_valid(first, device, capability, scenario)
            assert not home.exists(), "replay unexpectedly created user data"

        for args in (
            ("simulate", "replay"),
            ("simulate", "replay", "sim:gpio"),
            ("simulate", "replay", "sim:gpio", "not-a-scenario", "--confirm"),
            ("simulate", "replay", "real:gpio", "nominal", "--confirm"),
            ("simulate", "replay", "sim:gpio", "nominal", "--bad"),
            ("simulate", "replay", "sim:gpio", "nominal", "--confirm", "extra"),
        ):
            command(env, *args, allowed=False)
        assert not home.exists()

        command(env, "permissions", "request", "sdr.receive")
        command(
            env, "simulate", "replay", "sim:sdr", "revoke", allowed=False
        )
        trace_is_valid(
            command(
                env, "simulate", "replay", "sim:sdr",
                "revoke", "--confirm"
            ).stdout,
            "sim:sdr", "sdr.receive", "revoke",
        )

        command(env, "permissions", "deny", "sdr.receive")
        blocked = command(
            env, "simulate", "replay", "sim:sdr",
            "nominal", "--confirm", allowed=False
        )
        assert "denies this capability" in blocked.stderr

        preference_path = home / "permissions.json"
        broken = b'{"schema_version":2,"entries":[]}'
        preference_path.write_bytes(broken)
        malformed = command(
            env, "simulate", "replay", "sim:gpio",
            "nominal", "--confirm", allowed=False
        )
        assert "unsupported preferences schema" in malformed.stderr
        assert preference_path.read_bytes() == broken

    print(
        "PASS: deterministic synthetic traces, disconnect/recovery, revocation, "
        "bounded state transitions, cancellation, logical deadlines, denial, malformed policy and no physical I/O"
    )


if __name__ == "__main__":
    main()

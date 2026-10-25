#!/usr/bin/env python3
"""Black-box adapter preference checks; no device or network access required."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target" / "debug" / (
    "pipboy.exe" if os.name == "nt" else "pipboy"
)

ADAPTER_KEYS = (
    "device.enumerate", "gpio.read", "display.write", "serial.read",
    "gnss.read", "sdr.receive", "ble.discover", "mesh.transmit",
    "sensor.read", "audio.play", "network.fetch",
)


def command(env, *parts, succeeds=True):
    result = subprocess.run(
        [str(BINARY), *parts],
        env=env,
        text=True,
        capture_output=True,
        timeout=10,
        check=False,
    )
    if succeeds:
        assert result.returncode == 0, (parts, result.stderr)
    else:
        assert result.returncode != 0, (parts, result.stdout)
    return result


def main():
    with tempfile.TemporaryDirectory(prefix="rust-boy-permissions-") as temp:
        data = Path(temp) / "not-yet-created"
        env = dict(os.environ, PIPBOY_HOME=str(data))
        path = data / "permissions.json"

        initial = command(env, "permissions", "list")
        assert all(key in initial.stdout for key in ADAPTER_KEYS)
        assert initial.stdout.count("unset") == len(ADAPTER_KEYS)
        assert not data.exists(), "read-only list created files"

        command(env, "permissions", "request", "sdr.receive")
        first = path.read_bytes()
        document = json.loads(first)
        assert document["schema_version"] == 1
        assert document["entries"] == [
            {"capability": "sdr.receive", "preference": "requested"}
        ]
        command(env, "permissions", "request", "sdr.receive")
        assert path.read_bytes() == first, "same request rewrote preferences"
        assert "requested" in command(env, "permissions", "list").stdout
        assert "not an active permission" in command(
            env, "permissions", "request", "network.fetch"
        ).stdout

        command(env, "permissions", "deny", "sdr.receive")
        assert any(
            item["capability"] == "sdr.receive" and item["preference"] == "denied"
            for item in json.loads(path.read_text(encoding="utf-8"))["entries"]
        )
        command(env, "permissions", "clear", "sdr.receive")
        assert all(
            item["capability"] != "sdr.receive"
            for item in json.loads(path.read_text(encoding="utf-8"))["entries"]
        )

        unchanged = path.read_bytes()
        command(env, "permissions", "request", "host.read", succeeds=False)
        command(env, "permissions", "deny", "not.a.capability", succeeds=False)
        command(env, "permissions", "request", succeeds=False)
        command(env, "permissions", "list", "unexpected", succeeds=False)
        assert path.read_bytes() == unchanged

        diagnostics = command(env, "doctor")
        assert "persisted adapter preferences validated" in diagnostics.stdout

        for invalid in (
            b"{broken",
            b'{"schema_version":2,"entries":[]}',
            b'{"schema_version":1,"entries":[{"capability":"sdr.receive","preference":"requested"},{"capability":"sdr.receive","preference":"denied"}]}',
        ):
            path.write_bytes(invalid)
            command(env, "permissions", "list", succeeds=False)
            command(env, "permissions", "deny", "sdr.receive", succeeds=False)
            assert path.read_bytes() == invalid, "invalid document was replaced"

    print(
        "PASS: read-only defaults, request/deny/clear, repeat-safe writes, "
        "unsupported capability denial, doctor and corrupt schema preservation"
    )


if __name__ == "__main__":
    main()

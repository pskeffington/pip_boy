#!/usr/bin/env python3
"""Cross-platform black-box smoke for the explicit deny-all broker process.

The broker never opens a device, socket, radio or platform helper. The test
runs one bounded standard-input JSON frame in a separate process per request.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BROKER = ROOT / "target" / "debug" / (
    "rust-boy-broker.exe" if os.name == "nt" else "rust-boy-broker"
)
OUTPUT_FIELDS = {"schema_version", "allowed", "status", "reason", "source"}


def request():
    return {
        "schema_version": 1,
        "request_id": "0123456789abcdef",
        "capability": "serial.read",
        "device_id": "device:demo-serial",
        "operation": "read",
        "duration_ms": 1000,
        "purpose": "test only",
        "operator_intent": True,
    }


def run(env, frame, args=("--stdio-once",), good=True):
    result = subprocess.run(
        [str(BROKER), *args],
        input=frame, text=True, capture_output=True, check=False,
        env=env, timeout=8,
    )
    if not good:
        assert result.returncode == 2, (args, result.returncode, result.stderr)
        assert not result.stdout.strip(), "unexpected denial bypass output"
        return None
    assert result.returncode == 0, (args, result.stderr)
    assert not result.stderr.strip(), result.stderr
    assert result.stdout.endswith("\n"), "response not line framed"
    assert result.stdout.count("\n") == 1, "more than one response frame"
    doc = json.loads(result.stdout)
    assert set(doc) == OUTPUT_FIELDS, doc
    assert doc["schema_version"] == 1
    assert doc["allowed"] is False
    assert doc["status"] == "denied"
    assert doc["source"] == "rust-boy/deny-all-physical-broker"
    assert doc["reason"] in {
        "physical_device_backend_disabled", "operator_intent_required",
        "invalid_request",
    }
    assert "demo-serial" not in result.stdout, "request echoed into output"
    assert "0123456789abcdef" not in result.stdout, "caller token echoed"
    return doc


def encoded(value):
    return json.dumps(value, separators=(",", ":")) + "\n"


def main():
    assert BROKER.is_file(), f"build broker first: {BROKER}"
    with tempfile.TemporaryDirectory(prefix="rust-boy-deny-broker-") as base:
        home = Path(base) / "not-created"
        env = dict(os.environ, PIPBOY_HOME=str(home), TERM="dumb")
        valid = encoded(request())
        first = run(env, valid)
        assert first["reason"] == "physical_device_backend_disabled"
        assert first == run(env, valid), "request generated unstable output"
        assert not home.exists(), "broker touched the caller's data directory"

        missing = request()
        missing["operator_intent"] = False
        assert run(env, encoded(missing))["reason"] == "operator_intent_required"

        fixtures = [
            ("mesh.transmit", "transmit"),
            ("network.fetch", "fetch"),
            ("gpio.read", "read"),
            ("display.write", "write"),
            ("sdr.receive", "read"),
        ]
        for capability, operation in fixtures:
            case = request()
            case["capability"], case["operation"] = capability, operation
            assert run(env, encoded(case))["reason"] == "physical_device_backend_disabled"

        invalid_cases = []
        for field, value in [
            ("schema_version", 2),
            ("capability", "host.read"),
            ("capability", "serial.write"),
            ("device_id", "sim:serial"),
            ("device_id", "device:../../COM1"),
            ("device_id", "device:/dev/ttyUSB0"),
            ("operation", "transmit"),
            ("request_id", "bad"),
            ("duration_ms", 0),
            ("duration_ms", 30001),
            ("purpose", "\n"),
            ("purpose", "X" * 121),
        ]:
            case = request()
            case[field] = value
            invalid_cases.append(encoded(case))
        unknown_field = request()
        unknown_field["grant"] = True
        invalid_cases.append(encoded(unknown_field))
        invalid_cases.extend(["", "null\n", "{}\n", "{invalid\n", "X" * 4096 + "\n"])
        for frame in invalid_cases:
            assert run(env, frame)["reason"] == "invalid_request"

        # No protocol version, CLI trick, or malformed frame may spawn a
        # persistent helper, alter permissions or activate external devices.
        for args in [(), ("--help",), ("--stdio-once", "--elevated")]:
            run(env, valid, args=args, good=False)
        assert not home.exists(), "denied requests wrote user data"

    print(
        "PASS: separate deny-all broker process, bounded v1 JSON, "
        "11 capability gates, no implied consent, invalid/future input denial, "
        "one-shot process and zero device/data-directory activity"
    )


if __name__ == "__main__":
    main()

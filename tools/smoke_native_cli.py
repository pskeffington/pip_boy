#!/usr/bin/env python3
"""Portable noninteractive terminal/CLI smoke checks for Linux, macOS and Windows.

This does not claim Windows ConPTY or an interactive TUI session. The POSIX
PTY navigation and restoration tests live in smoke_runtime.py.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target" / "debug" / ("pipboy.exe" if os.name == "nt" else "pipboy")
EXPECTED_STATUS = {
    "observed_unix_s", "os", "arch", "cpu_percent", "logical_cores",
    "memory_used_mib", "memory_total_mib", "uptime_s",
}


def run(env, *arguments, success=True):
    result = subprocess.run(
        [str(BINARY), *arguments], env=env, capture_output=True,
        text=True, timeout=20, check=False,
    )
    assert "\x1b" not in result.stdout, (arguments, "unexpected terminal escape")
    assert "\x1b" not in result.stderr, (arguments, "unexpected terminal escape")
    if success:
        assert result.returncode == 0, (arguments, result.stderr)
        assert not result.stderr.strip(), (arguments, result.stderr)
    else:
        assert result.returncode != 0, (arguments, result.stdout)
        assert not result.stdout.strip(), (arguments, "failure emitted data")
    return result.stdout


def main():
    assert BINARY.is_file(), f"build the CLI first: {BINARY}"
    with tempfile.TemporaryDirectory(prefix="rust-boy-native-cli-") as temporary:
        home = Path(temporary) / "not-created"
        env = dict(os.environ, PIPBOY_HOME=str(home), TERM="dumb")

        usage = run(env, "help")
        assert "Rust-Boy" in usage and "usage:" in usage
        assert "simulate" in usage and "adapter" in usage
        assert run(env, "--help") == usage

        status = json.loads(run(env, "status", "--json"))
        assert set(status) == EXPECTED_STATUS
        assert status["observed_unix_s"] > 1_600_000_000
        assert isinstance(status["os"], str) and status["os"]
        assert isinstance(status["arch"], str) and status["arch"]
        assert status["cpu_percent"] is None or (
            isinstance(status["cpu_percent"], (int, float)) and 0 <= status["cpu_percent"] <= 100
        )
        assert status["memory_total_mib"] >= status["memory_used_mib"] >= 0
        assert status["uptime_s"] >= 0

        assert "STAT" in run(env, "status")
        details = json.loads(run(env, "status", "--details", "--json"))
        assert details["schema_version"] == 1
        assert details["source_id"] == "host.stats"
        assert details["provenance"] == "local:sysinfo"
        fields = details["value"]
        assert fields["payload_schema_version"] == 1
        assert fields["poll_interval_ms"] is None
        units = {
            "os": "text", "arch": "text", "cpu_percent": "percent",
            "logical_cores": "count", "memory_used_mib": "MiB",
            "memory_total_mib": "MiB", "uptime_s": "s",
        }
        for key, unit in units.items():
            field = fields[key]
            assert set(field) == {"value", "unit", "quality", "reason"}
            assert field["unit"] == unit
            if field["quality"] in ("unknown", "invalid"):
                assert field["value"] is None and field["reason"]
            else:
                assert field["quality"] == "valid"
                assert field["value"] is not None and field["reason"] is None
        assert "field quality" in run(env, "status", "--details")
        assert json.loads(run(env, "status", "--json", "--details"))["schema_version"] == 1
        samples = [json.loads(row) for row in run(
            env, "sample", "--details", "--count", "2", "--interval-ms", "500"
        ).splitlines()]
        assert len(samples) == 2
        assert samples[0]["value"]["poll_interval_ms"] is None
        assert samples[1]["value"]["poll_interval_ms"] >= 500
        assert all(row["value"]["cpu_percent"]["unit"] == "percent" for row in samples)
        assert "stat" in run(env, "modules")
        assert "sim:gpio" in run(env, "simulate", "list")
        assert "no OS device discovery" in run(env, "simulate", "list")
        assert "preferences do not grant" in run(env, "permissions", "list")
        assert run(env, "notes", "list") == ""
        diagnostics = run(env, "doctor")
        assert "PASS: catalog parsed" in diagnostics
        assert "no real device backend" in diagnostics
        assert "does not probe" in diagnostics
        assert not home.exists(), "read-only CLI commands created user files"

        for invalid in (
            ("status", "--json", "extra"),
            ("status", "--details", "--details"),
            ("status", "--json", "--json"),
            ("status", "--details", "extra"),
            ("sample", "--details", "--details"),
            ("modules", "extra"),
            ("simulate", "adapter", "invalid"),
            ("simulate", "adapter", "input"),
            ("permissions", "list", "extra"),
            ("notes", "list", "extra"),
            ("not-a-command",),
        ):
            run(env, *invalid, success=False)
        assert not home.exists(), "failed CLI commands created user files"

    print(
        "PASS: portable terminal output, help, host JSON, catalog, "
        "read-only notes/preferences, simulated inventory, errors and no ANSI/file side effects"
    )


if __name__ == "__main__":
    main()

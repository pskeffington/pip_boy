#!/usr/bin/env python3
"""Non-invasive acceptance for NET ENGINEER CLI and DIAG evidence parser.

All NET cases here are rejected before any network packets can be emitted.
The live loopback TCP listener fixture is covered by Rust network_probe tests.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target" / "debug" / ("pipboy.exe" if os.name == "nt" else "pipboy")


def invoke(env, *argv, success):
    result = subprocess.run([str(BINARY), *argv], cwd=ROOT, env=env,
                            capture_output=True, text=True, timeout=20, check=False)
    assert (result.returncode == 0) == success, (argv, result.stdout, result.stderr)
    if not success:
        assert not result.stdout.strip(), (argv, "invalid request emitted data")
    return result.stdout


def main():
    assert BINARY.is_file(), f"build CLI before smoke: {BINARY}"
    with tempfile.TemporaryDirectory() as temporary:
        home = Path(temporary) / "unused"
        env = dict(os.environ, PIPBOY_HOME=str(home))
        for command in [
            ("net", "scan", "192.168.1.0/24"),
            ("net", "scan", "8.8.8.8/32", "--confirm"),
            ("net", "scan", "10.0.0.0/8", "--confirm"),
            ("net", "scan", "192.168.1.0/23", "--confirm"),
            ("net", "scan", "localhost", "--confirm"),
            ("net", "watch", "192.168.1.0/24", "--confirm", "--count", "0"),
            ("net", "watch", "192.168.1.0/24", "--confirm", "--interval-sec", "1"),
            ("net", "scan", "local", "--confirm", "--bogus"),
        ]:
            invoke(env, *command, success=False)
        assert not home.exists(), "rejected scans wrote to disk"
        diag = json.loads(invoke(env, "diag", "host", "--json", success=True))
        assert diag["asset_id"] == "local-host"
        assert diag["intake"] == "local_sysinfo_one_shot"
        assert diag["record_count"] + diag["unavailable_count"] == 2
        sample = ROOT / "examples" / "diag" / "network-example.jsonl"
        imported = json.loads(invoke(
            env, "diag", "ingest", str(sample), "--asset", "lab-router",
            "--confirm-authorized", "--json", success=True
        ))
        assert imported["record_count"] == 3 and len(imported["findings"]) == 2
        assert not home.exists(), "DIAG read-only ingestion unexpectedly wrote files"
    print("PASS: NET scope/consent/limits and DIAG real local plus imported evidence")


if __name__ == "__main__":
    main()

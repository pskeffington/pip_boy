#!/usr/bin/env python3
"""Run Rust-Boy Python CLI smoke tests from one explicit, offline CI roster.

The runner does not discover executable tests dynamically: newly added smoke
scripts must be reviewed and classified. Native Mac/Windows terminal tests stay
in their existing, platform-specific CI steps.
"""

import argparse
from pathlib import Path
import subprocess
import sys

TOOLS = Path(__file__).resolve().parent

# Shared Linux/macOS/Windows CLI smoke checks in their historical CI order.
NATIVE_SMOKES = (
    "smoke_permissions.py",
    "smoke_simulator.py",
    "smoke_replay.py",
    "smoke_transport.py",
    "smoke_mock_adapters.py",
    "smoke_native_cli.py",
    "smoke_broker.py",
    "smoke_game.py",
    "smoke_workbench.py",
    "smoke_network.py",
    "smoke_network_health.py",
    "smoke_scout.py",
    "smoke_dashboard.py",
    "smoke_micro.py",
    "smoke_tasks.py",
    "smoke_peer.py",
    "smoke_messenger_protocol.py",
)

# These run in the full Linux job; smoke_runtime also has a separate native
# macOS PTY job. smoke_notes is not currently part of the native CI matrix.
LINUX_EXTRA = ("smoke_runtime.py", "smoke_notes.py")
PLATFORM_SPECIFIC = ("smoke_windows_conpty.py",)
# These require an actual native audio output and an optional feature build.
# They are explicit manual checks, never part of an unattended CI profile.
MANUAL_HARDWARE = ("smoke_radio_sdrconnect.py",)
SUITES = {"linux": LINUX_EXTRA + NATIVE_SMOKES, "native": NATIVE_SMOKES}


def validate_manifest(directory=TOOLS):
    """Fail on missing, unclassified, duplicated or symlinked smoke scripts."""
    directory = Path(directory)
    if not directory.is_dir() or directory.is_symlink():
        raise ValueError(f"invalid smoke-test directory: {directory}")
    roster = LINUX_EXTRA + NATIVE_SMOKES + PLATFORM_SPECIFIC + MANUAL_HARDWARE
    if len(roster) != len(set(roster)):
        raise ValueError("duplicate smoke script in the CI roster")
    if any(not name.startswith("smoke_") or not name.endswith(".py")
           or Path(name).name != name for name in roster):
        raise ValueError("invalid smoke script name in the CI roster")
    discovered = {path.name for path in directory.glob("smoke_*.py")}
    missing = sorted(set(roster) - discovered)
    unclassified = sorted(discovered - set(roster))
    if missing or unclassified:
        raise ValueError(
            f"smoke roster drift; missing={missing}; unclassified={unclassified}"
        )
    for name in roster:
        path = directory / name
        if not path.is_file() or path.is_symlink():
            raise ValueError(f"invalid or symlinked smoke script: {name}")
    return len(roster)


def run_suite(profile, directory=TOOLS, runner=subprocess.run):
    """Run precisely the requested CI profile, stopping at the first failure."""
    if profile not in SUITES:
        raise ValueError(f"unknown smoke profile: {profile}")
    directory = Path(directory)
    validate_manifest(directory)
    for name in SUITES[profile]:
        print(f"SMOKE [{profile}] {name}", flush=True)
        runner([sys.executable, str(directory / name)], cwd=directory.parent,
               check=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("profile", nargs="?", choices=sorted(SUITES))
    action = parser.add_mutually_exclusive_group()
    action.add_argument("--check", action="store_true",
                        help="validate the smoke roster without executing tests")
    action.add_argument("--list", action="store_true",
                        help="print the selected profile's smoke scripts")
    args = parser.parse_args(argv)
    if not args.profile and not args.check:
        parser.error("specify linux/native or use --check")
    if args.check:
        count = validate_manifest()
        print(f"PASS: {count} classified smoke scripts; no unregistered tests")
    elif args.list:
        validate_manifest()
        for name in SUITES[args.profile]:
            print(name)
    else:
        run_suite(args.profile)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        raise SystemExit(1) from error

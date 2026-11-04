#!/usr/bin/env python3
"""Stage one already-approved, hash-pinned package into the user's data root.

This copies exactly the roster-pinned executable and manifest. It does not
approve packages, install capabilities, download artifacts, or start a child.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import stat
import sys
import tempfile

from validate_host_packages import check_approved_executable_roster, check_manifest

ROOT = Path(__file__).resolve().parents[1]
EXECUTABLE_LIMIT = 512 * 1024 * 1024
MANIFEST_LIMIT = 16 * 1024


def safe_relative(value):
    if not isinstance(value, str) or not value or len(value) > 240 or "\\" in value or ":" in value:
        raise ValueError("unsafe roster path")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in {"", ".", ".."} for part in value.split("/")):
        raise ValueError("unsafe roster path")
    return path


def regular_source_file(source_root, relative, limit):
    path = source_root.joinpath(*relative.parts)
    cursor = source_root
    for part in relative.parts:
        cursor = cursor / part
        info = cursor.lstat()
        if stat.S_ISLNK(info.st_mode):
            raise ValueError("package source contains a symlink")
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_size > limit:
        raise ValueError("package source is not a bounded regular file")
    return path


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def install_package(package_id, source_directory, packages_root, roster):
    """Install one roster entry atomically; paths are relative to the package."""
    check_approved_executable_roster(data=roster)
    row = next((item for item in roster["executables"] if item["package_id"] == package_id), None)
    if row is None:
        raise ValueError("package is not in the approved host roster")

    executable_rel = safe_relative(row["relative_executable"])
    manifest_rel = safe_relative(row["relative_manifest"])
    if executable_rel == manifest_rel:
        raise ValueError("executable and manifest paths must differ")

    source_root = Path(source_directory)
    if source_root.is_symlink() or not source_root.is_dir():
        raise ValueError("package source must be a real directory")
    source_root = source_root.resolve(strict=True)
    executable = regular_source_file(source_root, executable_rel, EXECUTABLE_LIMIT)
    manifest = regular_source_file(source_root, manifest_rel, MANIFEST_LIMIT)
    if os.path.samefile(executable, manifest):
        raise ValueError("executable and manifest must be distinct files")
    if not executable.stat().st_mode & stat.S_IXUSR:
        raise ValueError("approved executable is not marked executable")
    if sha256(executable) != row["sha256"] or sha256(manifest) != row["manifest_sha256"]:
        raise ValueError("package bytes do not match the approved SHA-256 pins")

    manifest_bytes = manifest.read_bytes()
    try:
        manifest_data = json.loads(manifest_bytes.decode("utf-8"))
        manifest_id = check_manifest(manifest_data)
    except (UnicodeError, json.JSONDecodeError) as error:
        raise ValueError("approved package manifest is invalid UTF-8 JSON") from error
    if (manifest_id != package_id or manifest_data.get("status") is not None
            or manifest_data["entrypoint"]["type"] != "external_process"):
        raise ValueError("approved manifest identity or entrypoint mismatch")

    root = Path(packages_root)
    root.mkdir(mode=0o700, parents=True, exist_ok=True)
    if root.is_symlink() or not root.is_dir():
        raise ValueError("package store must be a real directory")
    root = root.resolve(strict=True)
    destination = root / package_id
    if destination.exists() or destination.is_symlink():
        raise FileExistsError("package is already installed; updates require a separate reviewed flow")

    stage = Path(tempfile.mkdtemp(prefix=f".{package_id}.", dir=root))
    try:
        for relative, source, mode in (
            (executable_rel, executable, 0o755),
            (manifest_rel, manifest, 0o644),
        ):
            target = stage.joinpath(*relative.parts)
            target.parent.mkdir(mode=0o755, parents=True, exist_ok=True)
            shutil.copyfile(source, target, follow_symlinks=False)
            target.chmod(mode)
            if sha256(target) != row["sha256" if relative == executable_rel else "manifest_sha256"]:
                raise ValueError("staged file digest changed during copy")
        stage.chmod(0o755)
        if destination.exists() or destination.is_symlink():
            raise FileExistsError("package destination appeared during staging")
        os.rename(stage, destination)
    except BaseException:
        shutil.rmtree(stage, ignore_errors=True)
        raise
    return destination


def data_root_from_environment(environ=None, home=None):
    environ = os.environ if environ is None else environ
    home = Path.home() if home is None else Path(home)
    configured = environ.get("PIPBOY_HOME")
    return Path(configured).expanduser() if configured else home / ".pipboy"


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("package_id", help="package ID already present in the host approval roster")
    parser.add_argument("source_directory", help="reviewed artifact directory with roster-relative files")
    args = parser.parse_args(argv)
    roster_path = ROOT / "catalog/approved_package_executables.json"
    roster = json.loads(roster_path.read_text(encoding="utf-8"))
    check_approved_executable_roster(data=roster)
    packages_root = data_root_from_environment() / "packages"
    try:
        destination = install_package(args.package_id, args.source_directory, packages_root, roster)
    except (OSError, ValueError, KeyError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    print(f"Installed approved package files at {destination}; no process was started and no capabilities were granted.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

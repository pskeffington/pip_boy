#!/usr/bin/env python3
"""Pack the generated PNG iconset into the documented ICNS container format."""

from __future__ import annotations

import struct
import sys
from pathlib import Path

ICON_ENTRIES = (
    (b"icp4", "icon_16x16.png", 16),
    (b"icp5", "icon_32x32.png", 32),
    (b"icp6", "icon_32x32@2x.png", 64),
    (b"ic07", "icon_128x128.png", 128),
    (b"ic08", "icon_256x256.png", 256),
    (b"ic09", "icon_512x512.png", 512),
    (b"ic10", "icon_512x512@2x.png", 1024),
)


def png_dimensions(data: bytes, path: Path) -> tuple[int, int]:
    if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
        raise ValueError(f"not a valid PNG: {path}")
    return struct.unpack(">II", data[16:24])


def build(iconset: Path, output: Path) -> None:
    chunks: list[bytes] = []
    for chunk_type, filename, expected_size in ICON_ENTRIES:
        source = iconset / filename
        data = source.read_bytes()
        width, height = png_dimensions(data, source)
        if (width, height) != (expected_size, expected_size):
            raise ValueError(
                f"{source} must be {expected_size}x{expected_size}; got {width}x{height}"
            )
        chunks.append(chunk_type + struct.pack(">I", len(data) + 8) + data)

    payload = b"".join(chunks)
    output.write_bytes(b"icns" + struct.pack(">I", len(payload) + 8) + payload)


def main() -> int:
    if len(sys.argv) != 3:
        print("Usage: build_icns.py INPUT.iconset OUTPUT.icns", file=sys.stderr)
        return 2
    try:
        build(Path(sys.argv[1]), Path(sys.argv[2]))
    except (OSError, ValueError) as error:
        print(f"Could not build app icon: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

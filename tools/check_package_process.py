#!/usr/bin/env python3
"""Exercise the RBPK v1 stdio lifecycle of a reviewed package binary."""
import argparse
import struct
import subprocess
import sys

PACKAGE_ID = b"org.rustboy.music"
MAGIC = b"RBPK"
MAX_FRAME = 16 * 1024


def request(request_id: int, tag: int, fields: bytes = b"") -> bytes:
    payload = (
        bytes((1, 1))
        + struct.pack(">QH", request_id, len(PACKAGE_ID))
        + PACKAGE_ID
        + bytes((tag,))
        + fields
    )
    if not payload or len(payload) > MAX_FRAME:
        raise ValueError("request frame outside protocol limit")
    return MAGIC + struct.pack(">I", len(payload)) + payload


def replies(output: bytes) -> list[tuple[int, bytes]]:
    result = []
    offset = 0
    while offset < len(output):
        if len(output) - offset < 8 or output[offset : offset + 4] != MAGIC:
            raise ValueError("invalid or truncated response frame header")
        size = struct.unpack_from(">I", output, offset + 4)[0]
        if not 0 < size <= MAX_FRAME or offset + 8 + size > len(output):
            raise ValueError("invalid or truncated response frame body")
        payload = output[offset + 8 : offset + 8 + size]
        offset += 8 + size
        if len(payload) < 29 or payload[:2] != bytes((1, 2)):
            raise ValueError("invalid response envelope")
        request_id = struct.unpack_from(">Q", payload, 2)[0]
        package_length = struct.unpack_from(">H", payload, 10)[0]
        if package_length != len(PACKAGE_ID) or payload[12:29] != PACKAGE_ID:
            raise ValueError("response package identity mismatch")
        result.append((request_id, payload[29:]))
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", help="reviewed package executable to exercise")
    parser.add_argument("--timeout", type=float, default=5.0)
    args = parser.parse_args()
    transcript = b"".join(
        (
            request(1, 0, struct.pack(">HH", 1, 1)),
            request(2, 1, struct.pack(">HH", 480, 320)),
            request(3, 12),
        )
    )
    try:
        completed = subprocess.run(
            [args.executable, "serve"],
            input=transcript,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=args.timeout,
            check=False,
        )
        if completed.returncode != 0:
            raise RuntimeError(
                f"package exited {completed.returncode}: "
                + completed.stderr.decode("utf-8", errors="replace")[-1000:]
            )
        observed = replies(completed.stdout)
        if len(observed) != 3 or [item[0] for item in observed] != [1, 2, 3]:
            raise ValueError("response IDs do not match hello/activate/shutdown")
        if observed[0][1] != b"\x00\x00\x01":
            raise ValueError("package did not negotiate RBPK v1")
        if not observed[1][1] or observed[1][1][0] not in (1, 2):
            raise ValueError("package did not acknowledge activation or return a view")
        if observed[2][1] != b"\x01":
            raise ValueError("package did not acknowledge shutdown")
    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
        print(f"FAIL package process smoke: {error}", file=sys.stderr)
        return 1
    print("PASS package process smoke: RBPK hello, activate, shutdown, clean exit")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

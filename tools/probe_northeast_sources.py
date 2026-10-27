#!/usr/bin/env python3
"""One-shot verification of official Northeast DOT camera *metadata* sources.

Run on Mac after pulling the branch:
    python3 tools/probe_northeast_sources.py --json

Each selected source gets one bounded HTTPS metadata GET, no redirects.
Does not fetch, play, record, scan for, or prove any camera image is LIVE.
Access-restricted publishers are deliberately excluded.
"""
import argparse
import datetime as dt
import json
import time
import urllib.error

import build_northeast_camera_index as index

# Named static allowlist: no URLs supplied by users or by untrusted catalogs.
DEFAULT_SOURCES = ("md-socrata", "md-imap", "ddot", "ridot")
OPTIONAL_SOURCES = ("deldot", "vdot")
SOURCES = DEFAULT_SOURCES + OPTIONAL_SOURCES


def check_source(source):
    if source not in SOURCES:
        raise ValueError("Source is not approved for metadata probing")
    start = time.monotonic()
    result = {"source": source, "agency": index.SOURCES[source]["agency"],
              "status": "unavailable", "camera_locations": 0,
              "image_streams_verified": 0}
    try:
        payload = index.fetch_public(source)
        records = index.normalize(source, payload)
        result["camera_locations"] = len(records)
        result["status"] = "metadata_reachable" if records else "metadata_no_valid_coordinates"
        result["upstream_may_have_more"] = (
            bool(payload.get("exceededTransferLimit"))
            or len(payload["features"]) >= 500
        )
    except (OSError, ValueError, TypeError, KeyError, urllib.error.HTTPError,
            json.JSONDecodeError) as exc:
        result["status"] = "metadata_error"
        result["error_type"] = type(exc).__name__
    result["elapsed_ms"] = round((time.monotonic() - start) * 1000)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", action="append", choices=SOURCES,
                        help="Optional source filter; can repeat. Defaults to MD, DC and RI")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    selected = list(dict.fromkeys(args.source or DEFAULT_SOURCES))
    results = [check_source(source) for source in selected]
    if args.json:
        print(json.dumps({
            "checked_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
            "results": results,
            "warning": "Metadata reachability does not verify footage or reproduction rights"
        }, indent=2))
    else:
        for item in results:
            print(f"{item['source']:<12} {item['status']:<34} "
                  f"{item['camera_locations']:>4} locations")
        print("NO IMAGE/VIDEO PLAYBACK PERFORMED. NO FEEDS VERIFIED LIVE.")
    return 0 if all(x["status"] == "metadata_reachable" for x in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())

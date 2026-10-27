#!/usr/bin/env python3
"""One-shot, read-only DOT camera snapshot connectivity diagnostic.

Run while the Rust-Boy local dashboard is running:
    python3 tools/probe_public_cameras.py
Only reads /api/state from loopback and approved HTTPS media hosts.
No camera discovery, credential collection, recordings or repeat polling.
An image response is NOT proof of a live or fresh capture.
"""
import argparse
import datetime as dt
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

MAX_BYTES = 2 * 1024 * 1024
MAX_FEEDS = 12
ALLOWED = {
    "snapshot.vdotcameras.com": re.compile(r"^/thumbs/[a-zA-Z0-9_-]+\.flv\.png$"),
    "cwwp2.dot.ca.gov": re.compile(
        r"^/data/d[0-9]{1,2}/cctv/image/[a-zA-Z0-9_-]+/[a-zA-Z0-9_-]+\.jpg$"
    ),
}


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, msg, headers, newurl):
        return None


OPENER = urllib.request.build_opener(NoRedirect)


def approved(url):
    if not isinstance(url, str) or not url:
        return False
    parts = urllib.parse.urlsplit(url)
    return (
        parts.scheme == "https"
        and parts.hostname in ALLOWED
        and not parts.username
        and not parts.password
        and parts.port is None
        and not parts.query
        and not parts.fragment
        and bool(ALLOWED[parts.hostname].fullmatch(parts.path))
    )


def load_local_state(url):
    parts = urllib.parse.urlsplit(url)
    if (
        parts.scheme != "http"
        or parts.hostname != "127.0.0.1"
        or parts.path != "/api/state"
        or parts.query
        or parts.fragment
        or parts.username
        or parts.password
    ):
        raise ValueError("State API must be http://127.0.0.1:<port>/api/state")
    request = urllib.request.Request(url, headers={"Accept": "application/json"})
    with OPENER.open(request, timeout=4) as response:
        if response.status != 200:
            raise ValueError("Local state API did not return HTTP 200")
        data = response.read(1024 * 1024 + 1)
    if len(data) > 1024 * 1024:
        raise ValueError("Local state response exceeds 1 MiB")
    document = json.loads(data)
    feeds = document.get("camera_feeds", [])
    if not isinstance(feeds, list):
        raise ValueError("Invalid camera feed list")
    return feeds[:MAX_FEEDS]


def probe(feed, now):
    result = {
        "id": str(feed.get("id", "?"))[:64],
        "city": str(feed.get("city", "?"))[:80],
        "publisher": str(feed.get("publisher", "?"))[:100],
        "status": "not_checked",
    }
    url = feed.get("image_url")
    if not url:
        result["status"] = "access_required"
        return result
    if not approved(url):
        result["status"] = "blocked_unapproved_endpoint"
        return result
    # Cache-busting only; no location, credentials or arbitrary remote URL input.
    request = urllib.request.Request(
        url + "?rb_probe=" + str(int(now * 1000)),
        headers={"Accept": "image/jpeg,image/png", "Cache-Control": "no-cache"},
    )
    try:
        with OPENER.open(request, timeout=8) as response:
            if response.status != 200:
                result["status"] = "http_" + str(response.status)
                return result
            kind = response.headers.get("Content-Type", "").split(";", 1)[0].lower()
            if kind not in {"image/jpeg", "image/png"}:
                result["status"] = "invalid_media_type"
                return result
            body = response.read(MAX_BYTES + 1)
            if len(body) > MAX_BYTES:
                result["status"] = "oversize_image"
                return result
            if not (body.startswith(b"\xff\xd8\xff") or body.startswith(b"\x89PNG\r\n\x1a\n")):
                result["status"] = "invalid_image_signature"
                return result
            result["status"] = "reachable_freshness_unknown"
            result["bytes"] = len(body)
            result["content_type"] = kind
            modified = response.headers.get("Last-Modified")
            if modified:
                result["last_modified_header"] = modified[:80]
                # A Last-Modified header is NOT treated as camera capture time.
    except urllib.error.HTTPError as exc:
        result["status"] = "http_" + str(exc.code)
    except (OSError, ValueError, TimeoutError) as exc:
        result["status"] = "request_failed"
        result["failure_type"] = type(exc).__name__
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--state-url", default="http://127.0.0.1:8765/api/state")
    parser.add_argument("--json", action="store_true", help="emit machine-readable results")
    args = parser.parse_args()
    try:
        feeds = load_local_state(args.state_url)
    except (ValueError, OSError, urllib.error.HTTPError, json.JSONDecodeError) as exc:
        print("ERROR: local Rust-Boy state unavailable: " + str(exc), file=sys.stderr)
        return 2
    now = time.time()
    results = [probe(f, now) for f in feeds]
    if args.json:
        print(json.dumps({"checked_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
                          "results": results}, indent=2))
    else:
        for item in results:
            print("{city:<10} {id:<14} {status}".format(**item))
        print("Image load = reachable only; not independently verified live.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

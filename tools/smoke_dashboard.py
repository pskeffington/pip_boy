#!/usr/bin/env python3
"""End-to-end loopback dashboard acceptance across Linux/macOS/Windows.

The only TCP address touched is 127.0.0.1:<ephemeral port>. No internet,
PlayStation, camera, radio or TAK server is opened by this test.
"""
import http.client
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target" / "debug" / ("pipboy.exe" if os.name == "nt" else "pipboy")


def send(port, method, path, data=None, origin=True, host=None, content_type="application/json"):
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=6)
    headers = {}
    if host is not None:
        headers["Host"] = host
    if method == "POST":
        if origin:
            headers["Origin"] = f"http://127.0.0.1:{port}"
        headers["Content-Type"] = content_type
        body = json.dumps(data).encode("utf-8")
    else:
        body = None
    try:
        conn.request(method, path, body=body, headers=headers)
        response = conn.getresponse()
        content = response.read()
        return response.status, dict(response.getheaders()), content
    finally:
        conn.close()


def parsed(port, method, path, data=None, **options):
    status, headers, content = send(port, method, path, data, **options)
    payload = json.loads(content)
    return status, headers, payload


with tempfile.TemporaryDirectory() as temporary:
    home = Path(temporary) / "new-install"
    env = dict(os.environ, PIPBOY_HOME=str(home))
    child = subprocess.Popen(
        [str(BINARY), "dashboard", "--port", "0"],
        cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, encoding="utf-8"
    )
    try:
        heading = child.stdout.readline().strip()
        locator = child.stdout.readline().strip()
        assert heading == "RUST-BOY DASHBOARD / LOCAL ONLY", heading
        assert locator.startswith("Open http://127.0.0.1:"), locator
        url = urlsplit(locator.split("Open ", 1)[1])
        assert url.hostname == "127.0.0.1"
        assert url.port is not None
        port = url.port

        for path, media, marker in [
            ("/", "text/html", "id=\"role-list\""),
            ("/assets/dashboard.css", "text/css", "@media"),
            ("/assets/dashboard.js", "text/javascript", "/api/state"),
        ]:
            code, headers, content = send(port, "GET", path)
            assert code == 200, (path, code, content)
            assert headers["Content-Type"].startswith(media)
            assert marker.encode() in content
            csp = headers["Content-Security-Policy"]
            assert "script-src 'self'" in csp
            assert "connect-src 'self'" in csp
            assert "frame-ancestors 'none'" in csp
            assert headers["X-Frame-Options"] == "DENY"
            assert "no-store" in headers["Cache-Control"]

        code, _, state = parsed(port, "GET", "/api/state")
        assert code == 200, state
        assert state["active"]["profile"] == "overseer"
        assert [item["id"] for item in state["profiles"]] == [
            "overseer", "head-engineer", "security-chief", "vault-doctor", "medic",
            "university-researcher", "reclamation-scout",
            "records-custodian", "custom"
        ]
        assert state["profiles"][-1]["tools"] == []
        assert state["network"] == "loopback_only"
        assert state["engine"]["rustgis"] == "not_integrated"
        assert not home.exists(), "GET dashboard unexpectedly created workbench files"

        rejected = [
            ("POST", "/api/profile", {"id": "custom", "confirm": True},
             {"origin": False}, 403),
            ("POST", "/api/profile", {"id": "custom", "confirm": True},
             {"origin": True, "host": "attacker.example"}, 403),
            ("POST", "/api/profile", {"id": "custom", "confirm": True},
             {"content_type": "application/x-www-form-urlencoded"}, 415),
            ("POST", "/api/profile", {"id": "custom"}, {}, 422),
            ("POST", "/api/profile", {"id": "does-not-exist", "confirm": True}, {}, 422),
            ("POST", "/api/profile", {"id": "custom", "confirm": True, "execute": "sh"}, {}, 400),
            ("POST", "/api/custom/add", {"id": "radio"}, {}, 409),
            ("POST", "/api/experiment", {"tool": "geo-lab", "mode": "distance"}, {}, 409),
            ("POST", "/api/shell", {"command": "ls"}, {}, 404),
        ]
        for method, path, data, options, expected in rejected:
            code, _, body = parsed(port, method, path, data, **options)
            assert code == expected, (path, expected, code, body)
        assert not home.exists(), "Rejected requests changed local data"

        code, _, response = parsed(port, "POST", "/api/profile",
                                   {"id": "reclamation-scout", "confirm": True})
        assert code == 200, response
        assert response["active"]["profile"] == "reclamation-scout"
        assert [x["tool"] for x in response["active"]["slots"]][:3] == ["reclamation", "geo-lab", "game"]
        assert not (home / "notes.json").exists()

        code, _, result = parsed(port, "POST", "/api/experiment", {
            "tool": "geo-lab", "mode": "waypoint",
            "name": "Base", "lat": "42", "lon": "-70.75"
        })
        assert code == 200, result
        feature = json.loads(result["output"])
        assert feature["geometry"]["coordinates"] == [-70.75, 42]
        assert feature["properties"]["name"] == "Base"
        code, _, result = parsed(port, "POST", "/api/experiment", {
            "tool": "geo-lab", "mode": "distance",
            "lat": "0", "lon": "179.9", "lat2": "0", "lon2": "-179.9"
        })
        assert code == 200 and "22239." in result["output"], result
        code, _, result = parsed(port, "POST", "/api/experiment", {"tool": "game"})
        assert code == 200 and result["kind"] == "information"
        assert "does not connect" in result["output"]
        assert not (home / "permissions.json").exists()

        code, _, state = parsed(port, "POST", "/api/profile",
                                {"id": "custom", "confirm": True})
        assert code == 200 and state["active"]["slots"] == []
        code, _, result = parsed(port, "POST", "/api/custom/add", {"id": "net-lab"})
        assert code == 200 and result["active"]["slots"][0]["tool"] == "net-lab"
        code, _, result = parsed(port, "POST", "/api/experiment", {
            "tool": "net-lab", "input": "192.168.1.50/24"
        })
        assert code == 200 and "192.168.1.0/24" in result["output"]
        code, _, state = parsed(port, "POST", "/api/custom/add", {"id": "byte-lab"})
        assert code == 200 and len(state["active"]["slots"]) == 2
        code, _, result = parsed(port, "POST", "/api/experiment", {
            "tool": "byte-lab", "input": "48656c6c6f"
        })
        assert code == 200 and "Hello" in result["output"]
        code, _, result = parsed(port, "POST", "/api/custom/add", {"id": "radio"})
        assert code == 422 and "error" in result
        code, _, result = parsed(port, "POST", "/api/custom/remove", {"id": 1})
        assert code == 200 and len(result["active"]["slots"]) == 1
        assert result["active"]["slots"][0]["tool"] == "byte-lab"

        code, _, state = parsed(port, "GET", "/api/state")
        assert code == 200 and state["active"]["profile"] == "custom"
        assert len(state["active"]["slots"]) == 1
        assert (home / "workbench.json").is_file()
        assert not (home / "notes.json").exists()
        assert not (home / "permissions.json").exists()
        if os.name != "nt":
            assert (home / "workbench.json").stat().st_mode & 0o777 == 0o600
        print("PASS: Rust dashboard serves responsive assets on loopback, eight real templates, "
              "persisted role switching, Custom pin/unpin, GeoJSON/CIDR/byte lab, "
              "same-origin guard, CSP, and no implicit hardware/network activity")
    finally:
        child.terminate()
        try:
            child.wait(timeout=8)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait(timeout=5)
        if child.stdout:
            child.stdout.close()
        if child.stderr:
            child.stderr.close()

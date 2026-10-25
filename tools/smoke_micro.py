#!/usr/bin/env python3
"""Micro shell local HTTP integration tests. No WAN or device connections.

Exercise the real Rust service, not a mocked browser or synthetic API. The
static UI's CSS/HTML interaction must still be visually tested on hardware.
"""
import http.client
import json
import os
from pathlib import Path
import subprocess
import tempfile
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target" / "debug" / ("pipboy.exe" if os.name == "nt" else "pipboy")


def request(port, method, path, data=None, origin=True, host=None):
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=6)
    headers = {}
    if host is not None:
        headers["Host"] = host
    if method == "POST":
        headers["Content-Type"] = "application/json"
        if origin:
            headers["Origin"] = f"http://127.0.0.1:{port}"
        data = json.dumps(data).encode("utf-8")
    try:
        conn.request(method, path, body=data, headers=headers)
        response = conn.getresponse()
        result = (response.status, dict(response.getheaders()), response.read())
        return result
    finally:
        conn.close()


def api(port, method, path, data=None, **kwargs):
    status, headers, body = request(port, method, path, data, **kwargs)
    return status, headers, json.loads(body)


with tempfile.TemporaryDirectory() as work:
    root = Path(work) / "user-workbench"
    env = dict(os.environ, PIPBOY_HOME=str(root))
    server = subprocess.Popen(
        [str(BINARY), "dashboard", "--micro", "--port", "0"],
        env=env, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, encoding="utf-8"
    )
    try:
        assert server.stdout.readline().strip() == "RUST-BOY DASHBOARD / LOCAL ONLY"
        second = server.stdout.readline().strip()
        assert second.startswith("Open http://127.0.0.1:"), second
        url = urlsplit(second.removeprefix("Open "))
        assert url.hostname == "127.0.0.1" and url.path == "/micro"
        port = url.port
        assert port and port > 0

        for path, mime, required in [
            ("/micro", "text/html", b'id="micro-rail"'),
            ("/micro/", "text/html", b'id="roles-dialog"'),
            ("/assets/micro.css", "text/css", b".micro-device"),
            ("/assets/micro.js", "text/javascript", b'"/api/state"'),
            ("/", "text/html", b'href="/micro"'),
        ]:
            status, headers, data = request(port, "GET", path)
            assert status == 200, (path, status)
            assert headers["Content-Type"].startswith(mime)
            assert required in data, path
            assert "no-store" in headers["Cache-Control"]
            assert "script-src 'self'" in headers["Content-Security-Policy"]
            assert "frame-ancestors 'none'" in headers["Content-Security-Policy"]

        _, _, html = request(port, "GET", "/micro")
        _, _, css = request(port, "GET", "/assets/micro.css")
        _, _, js = request(port, "GET", "/assets/micro.js")
        assert all(('data-tab="' + name + '"').encode() in html
                   for name in ("stat", "gear", "map", "data", "link", "conf"))
        assert b"viewport-fit=cover" in html
        assert b"@media (max-width:520px)" in css
        assert b"@media (max-height:360px)" in css
        assert b"prefers-reduced-motion:reduce" in css
        assert b"min-height:44px" in css
        assert b"/api/profile" in js and b"/api/custom/add" in js
        assert b"/api/experiment" in js and b"/api/custom/remove" in js
        assert b"eval(" not in js and b"innerHTML" not in js
        assert b"RUSTGIS NOT ATTACHED" in js and b"NO VIDEO / NO SOURCE" in js
        assert b"https://" not in js, "Micro UI must not include third-party fetch targets"

        status, headers, state = api(port, "GET", "/api/state")
        assert status == 200
        assert state["active"]["profile"] == "overseer"
        assert [entry["id"] for entry in state["profiles"]] == [
            "overseer", "head-engineer", "security-chief", "vault-doctor",
            "university-researcher", "reclamation-scout",
            "records-custodian", "custom"
        ]
        assert state["profiles"][-1]["tools"] == []
        assert state["engine"]["camera_wall"] == "not_integrated"
        assert state["engine"]["rustgis"] == "not_integrated"
        assert not root.exists(), "GET must not initialize workbench or touch notes"

        # Mutations require an explicit same-origin JSON POST to the local Rust API.
        for method, path, data, opts, code in [
            ("POST", "/api/profile", {"id": "custom", "confirm": True},
             {"origin": False}, 403),
            ("POST", "/api/profile", {"id": "custom", "confirm": True},
             {"host": "malicious.example"}, 403),
            ("POST", "/api/profile", {"id": "custom"}, {}, 422),
            ("POST", "/api/profile", {"id": "undefined", "confirm": True}, {}, 422),
            ("POST", "/api/shell", {"command": "ls"}, {}, 404),
        ]:
            actual, _, response = api(port, method, path, data, **opts)
            assert actual == code, (path, actual, response)
        assert not root.exists(), "Rejected requests must not create state"

        code, _, response = api(port, "POST", "/api/profile", {
            "id": "security-chief", "confirm": True
        })
        assert code == 200
        assert response["active"]["profile"] == "security-chief"
        assert [slot["tool"] for slot in response["active"]["slots"]] == [
            "host", "net-lab", "byte-lab", "notes"
        ]
        assert not (root / "permissions.json").exists()
        assert not (root / "notes.json").exists()

        code, _, response = api(port, "POST", "/api/profile", {
            "id": "reclamation-scout", "confirm": True
        })
        assert code == 200 and response["active"]["slots"][0]["tool"] == "geo-lab"
        code, _, result = api(port, "POST", "/api/experiment", {
            "tool": "geo-lab", "mode": "waypoint",
            "name": "Camp One", "lat": "42.0", "lon": "-70.75"
        })
        assert code == 200
        assert json.loads(result["output"])["geometry"]["coordinates"] == [-70.75, 42.0]
        code, _, result = api(port, "POST", "/api/experiment", {
            "tool": "geo-lab", "mode": "distance",
            "lat": "0", "lon": "0", "lat2": "0", "lon2": "1"
        })
        assert code == 200 and "111195." in result["output"]

        code, _, response = api(port, "POST", "/api/profile", {
            "id": "custom", "confirm": True
        })
        assert code == 200 and response["active"]["slots"] == []
        code, _, response = api(port, "POST", "/api/custom/add", {"id": "byte-lab"})
        assert code == 200 and response["active"]["slots"][0]["tool"] == "byte-lab"
        code, _, result = api(port, "POST", "/api/experiment", {
            "tool": "byte-lab", "input": "48656c6c6f"
        })
        assert code == 200 and "Hello" in result["output"]
        code, _, result = api(port, "POST", "/api/custom/remove", {"id": 1})
        assert code == 200 and result["active"]["slots"] == []
        code, _, result = api(port, "POST", "/api/custom/add", {"id": "radio"})
        assert code == 422 and "error" in result

        assert (root / "workbench.json").is_file()
        assert not (root / "permissions.json").exists()
        assert not (root / "notes.json").exists()
        if os.name != "nt":
            assert (root / "workbench.json").stat().st_mode & 0o777 == 0o600
        print("PASS: micro HTML/CSS/JS on loopback, six tabs, seven roles + Custom, "
              "scope-safe switching, Scout GIS, Custom lab, CSP, static placeholders "
              "and no implicit network/device activity")
    finally:
        server.terminate()
        try:
            server.wait(timeout=8)
        except subprocess.TimeoutExpired:
            server.kill()
            server.wait(timeout=5)
        if server.stdout:
            server.stdout.close()
        if server.stderr:
            server.stderr.close()

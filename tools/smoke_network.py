#!/usr/bin/env python3
"""Rust-Boy NET SUITE: CLI, loopback HTTP, and 13-mode engineering bench.

Nine offline calculations are deterministic. The only live probe endpoint is
127.0.0.1, plus the adapter is checked while unconfigured. API keys are never
used or injected by smoke tests. No host scan, DNS or external test traffic.
"""
import http.client
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target" / "debug" / ("pipboy.exe" if os.name == "nt" else "pipboy")


def cli(home, *arguments, success=True):
    env = {key: value for key, value in os.environ.items()
           if not key.startswith("RUSTBOY_UNIFI_")}
    env["PIPBOY_HOME"] = str(home)
    result = subprocess.run(
        [str(BIN), "workbench", "net", *arguments],
        cwd=ROOT, env=env,
        capture_output=True, text=True, encoding="utf-8", timeout=15
    )
    if success:
        assert result.returncode == 0, (arguments, result.stderr)
    else:
        assert result.returncode != 0 and not result.stdout, (arguments, result)
    return result.stdout


def api(port, method, path, payload=None, origin=True):
    headers = {}
    body = None
    if method == "POST":
        headers["Content-Type"] = "application/json"
        if origin:
            headers["Origin"] = f"http://127.0.0.1:{port}"
        body = json.dumps(payload).encode("utf-8")
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=8)
    try:
        connection.request(method, path, body=body, headers=headers)
        response = connection.getresponse()
        return response.status, json.loads(response.read())
    finally:
        connection.close()


def launch(home):
    env = {key: value for key, value in os.environ.items()
           if not key.startswith("RUSTBOY_UNIFI_")}
    env["PIPBOY_HOME"] = str(home)
    process = subprocess.Popen(
        [str(BIN), "dashboard", "--port", "0", "--micro"],
        cwd=ROOT, env=env,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, encoding="utf-8"
    )
    assert process.stdout.readline().strip() == "RUST-BOY DASHBOARD / LOCAL ONLY"
    address = process.stdout.readline().strip()
    url = urlsplit(address.removeprefix("Open "))
    assert url.hostname == "127.0.0.1" and url.port
    return process, url.port


def stop(process):
    process.terminate()
    try:
        process.wait(timeout=8)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)
    process.stdout.close()
    process.stderr.close()


cases = [
    ("cidr", "10.0.0.0/31", None, "Usable addresses: 2"),
    ("cidr4", "192.168.1.15/24", None, "Network: 192.168.1.0/24"),
    ("cidr6", "2001:db8:1::1234/64", None, "2001:db8:1::/64"),
    ("split4", "10.0.0.0/24", "26", "10.0.0.192/26"),
    ("route", "10.40.1.15", "0.0.0.0/0,10.0.0.0/8,10.40.0.0/16",
     "Longest prefix: 10.40.0.0/16"),
    ("aggregate4", "10.40.0.0/24", "10.40.1.0/24", "10.40.0.0/23"),
    ("overlap4", "10.40.0.0/16", "10.40.1.0/24", "Overlap: YES"),
    ("mac", "00:1a:2b:3c:4d:5e", None, "fe80::21a:2bff:fe3c:4d5e"),
    ("vlan", "42,5,0", None, "TCI: 0xa02a"),
    ("mtu", "1500", None, "IPv6 TCP MSS: 1440"),
]

with tempfile.TemporaryDirectory() as temp:
    root = Path(temp) / "network-suite"
    assert "NETWORK BENCH" in cli(root, "list")
    for mode, value, extra, expected in cases:
        args = [mode, value] + ([extra] if extra is not None else [])
        content = cli(root, *args)
        assert expected in content, (mode, content)
    for bad in [
        ("cidr6", "::/129"),
        ("split4", "10.0.0.0/24", "23"),
        ("aggregate4", "10.0.0.0/24", "10.0.2.0/24"),
        ("vlan", "4095,7,1"),
        ("mac", "00:11:22:33"),
        ("mtu", "576"),
        ("tcp", "127.0.0.1", "80"),
        ("tcp", "example.com", "80", "--confirm"),
        ("tcp", "127.0.0.1", "0", "--confirm"),
        ("health", "127.0.0.1", "443", "5"),
        ("health", "127.0.0.1", "443", "9", "--confirm"),
        ("health", "host.invalid", "443", "5", "--confirm"),
        ("icmp", "127.0.0.1", "5"),
        ("icmp", "not-a-host", "5", "--confirm"),
        ("unifi",),
        ("unifi", "--confirm"),
    ]:
        cli(root, *bad, success=False)
    assert not root.exists(), "Offline planning, rejection or palette reads wrote files"

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        listener.listen(32)
        port_str = str(listener.getsockname()[1])
        assert "CONNECTED" in cli(root, "tcp", "127.0.0.1", port_str, "--confirm")
        cli_health = cli(root, "health", "127.0.0.1", port_str, "5", "--confirm")
        assert "NETWORK HEALTH" in cli_health and "Failed: 0" in cli_health
        assert "NOT packet loss" in cli_health
        assert not root.exists(), "Explicit TCP diagnostic wrote workbench files"

        process, port = launch(root)
        try:
            code, state = api(port, "GET", "/api/state")
            assert code == 200 and state["active"]["profile"] == "overseer"
            ops = state["net_operations"]
            assert len(ops) == 13
            assert len([mode for mode in ops if mode["active"]]) == 4
            assert [x["id"] for x in ops[:3]] == ["icmp", "health", "unifi"]
            assert ops[-1]["id"] == "tcp"
            assert state["unifi"]["configured"] is False
            assert "api_key" not in json.dumps(state).lower()
            assert not root.exists(), "State inspection initialized a workbench"
            code, denied = api(port, "POST", "/api/experiment", {
                "tool": "net-lab", "mode": "cidr6", "input": "2001:db8::1/64"
            })
            assert code == 409, denied
            code, state = api(port, "POST", "/api/profile", {
                "id": "head-engineer", "confirm": True
            })
            assert code == 200, state
            assert state["active"]["slots"][0]["tool"] == "net-lab"
            for mode, value, extra, expected in cases:
                request = {"tool": "net-lab", "mode": mode, "input": value}
                if extra is not None:
                    request["extra"] = extra
                code, data = api(port, "POST", "/api/experiment", request)
                assert code == 200 and data["kind"] == "computed", (request, code, data)
                assert expected in data["output"], (request, data)
            code, legacy = api(port, "POST", "/api/experiment", {
                "tool": "net-lab", "input": "192.168.0.5/24"
            })
            assert code == 200 and "192.168.0.0/24" in legacy["output"]

            request = {"tool": "net-lab", "mode": "tcp",
                       "input": "127.0.0.1", "extra": port_str}
            code, error = api(port, "POST", "/api/experiment", request)
            assert code == 422 and "confirm" in error["error"].lower()
            code, error = api(port, "POST", "/api/experiment",
                              dict(request, confirm=True), origin=False)
            assert code == 403
            code, error = api(port, "POST", "/api/experiment",
                              dict(request, input="example.com", confirm=True))
            assert code == 422
            code, checked = api(port, "POST", "/api/experiment",
                                dict(request, confirm=True))
            assert code == 200 and checked["kind"] == "probe", checked
            assert "CONNECTED" in checked["output"]
            # Live health is a finite set of TCP connect timings, not packet loss.
            health_req = {"tool": "net-lab", "mode": "health", "input": "127.0.0.1",
                          "extra": port_str, "samples": "5"}
            code, error = api(port, "POST", "/api/experiment", health_req)
            assert code == 422
            code, error = api(port, "POST", "/api/experiment",
                              dict(health_req, confirm=True), origin=False)
            assert code == 403
            code, error = api(port, "POST", "/api/experiment",
                              dict(health_req, samples="9", confirm=True))
            assert code == 422
            code, data = api(port, "POST", "/api/experiment",
                             dict(health_req, confirm=True))
            assert code == 200 and data["kind"] == "health", data
            metrics = data["metrics"]
            assert metrics["attempts"] == 5 and metrics["failed"] == 0, metrics
            assert metrics["method"] == "tcp_connect"
            assert metrics["variation_ms"] is not None
            assert "NOT packet loss" in data["output"]
            code, error = api(port, "POST", "/api/experiment",
                              {"tool": "net-lab", "mode": "icmp",
                               "input": "127.0.0.1", "extra": "5"})
            assert code == 422 and "confirmation" in error["error"].lower()
            code, error = api(port, "POST", "/api/experiment",
                              {"tool": "net-lab", "mode": "icmp",
                               "input": "untrusted.example", "extra": "5", "confirm": True})
            assert code == 409
            code, error = api(port, "POST", "/api/experiment",
                              {"tool": "net-lab", "mode": "unifi"})
            assert code == 422 and "confirmation" in error["error"].lower()
            code, error = api(port, "POST", "/api/experiment",
                              {"tool": "net-lab", "mode": "unifi", "confirm": True})
            assert code == 409 and "unavailable" in error["error"].lower()
            code, error = api(port, "POST", "/api/experiment", {
                "tool": "net-lab", "mode": "other", "input": "127.0.0.1"
            })
            assert code == 422

            connection = http.client.HTTPConnection("127.0.0.1", port)
            for path, needle in [
                ("/assets/micro.js", b"showNetworkMenu"),
                ("/assets/dashboard.js", b"renderNetworkForm"),
                ("/assets/micro.css", b"net-suite-tile"),
            ]:
                connection.request("GET", path)
                response = connection.getresponse()
                assert response.status == 200 and needle in response.read()
            connection.close()
            assert not (root / "notes.json").exists()
            assert not (root / "permissions.json").exists()
            assert not (root / "peers.json").exists()
            assert not (root / "tasks.json").exists()
            print("PASS: 13-mode network suite, TCP health, consent-gated ICMP RTT, "
                  "explicit consent, UniFi fail-closed without key, no hidden files")
        finally:
            stop(process)

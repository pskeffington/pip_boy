#!/usr/bin/env python3
"""Overseer task board end-to-end test against a local Rust server.

No network transmission beyond the loopback HTTP process; handoffs are
explicitly operator-controlled and cannot be mistaken for delivered mail.
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


def query(port, method, endpoint, payload=None, **overrides):
    conn = http.client.HTTPConnection("127.0.0.1", port, timeout=7)
    headers = {}
    if method == "POST":
        headers["Content-Type"] = "application/json"
        headers["Origin"] = f"http://127.0.0.1:{port}"
        if payload is not None:
            payload = json.dumps(payload).encode("utf-8")
    headers.update(overrides)
    try:
        conn.request(method, endpoint, body=payload, headers=headers)
        response = conn.getresponse()
        return response.status, dict(response.getheaders()), response.read()
    finally:
        conn.close()


def api(port, method, path, payload=None, **overrides):
    status, headers, body = query(port, method, path, payload, **overrides)
    return status, headers, json.loads(body)


with tempfile.TemporaryDirectory() as temporary:
    state = Path(temporary) / "vault"
    env = dict(os.environ, PIPBOY_HOME=str(state))
    service = subprocess.Popen(
        [str(BINARY), "dashboard", "--micro", "--port", "0"],
        cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, encoding="utf-8"
    )
    try:
        assert service.stdout.readline().strip() == "RUST-BOY DASHBOARD / LOCAL ONLY"
        url = urlsplit(service.stdout.readline().strip().removeprefix("Open "))
        assert url.hostname == "127.0.0.1" and url.path == "/micro"
        port = url.port
        assert port

        for path, mime, marker in [
            ("/assets/tasks.js", "text/javascript", b"RustBoyTasks"),
            ("/assets/tasks.css", "text/css", b".task-control"),
            ("/micro", "text/html", b"/assets/tasks.js"),
            ("/", "text/html", b"/assets/tasks.js"),
        ]:
            status, headers, content = query(port, "GET", path)
            assert status == 200, (path, status)
            assert headers["Content-Type"].startswith(mime)
            assert marker in content
            assert "script-src 'self'" in headers["Content-Security-Policy"]

        code, _, data = api(port, "GET", "/api/tasks")
        assert code == 200
        assert data["manager"] is True
        assert data["role"] == "overseer"
        assert data["board"]["tasks"] == []
        assert not state.exists(), "Reading tasks must not initialize a data directory"
        code, _, current = api(port, "GET", "/api/state")
        assert code == 200
        assert current["active"]["slots"][0]["tool"] == "mission-board"

        created = []
        for number in range(10):
            code, _, response = api(port, "POST", "/api/tasks/create", {
                "title": f"Inspect station {number}",
                "details": "Verify and report the result",
                "assignee": f"Operator {number}",
                "role": "head-engineer" if number % 2 == 0 else "reclamation-scout",
                "priority": "critical" if number == 0 else "normal",
                "contact": f"operator{number}@example.org",
                "due": "2026-11-01"
            })
            assert code == 200, (code, response)
            created.append(response["task"])
        assert [task["id"] for task in created] == list(range(1, 11))
        assert not (state / "notes.json").exists()
        assert not (state / "permissions.json").exists()
        assert not (state / "workbench.json").exists()

        code, _, response = api(port, "POST", "/api/tasks/handoff", {"id": 1})
        assert code == 200 and response["sent"] is False
        assert response["transport"] == "operator_controlled"
        assert "TASK: RB-0001" in response["text"]
        assert "operator0@example.org" in response["text"]
        code, _, check = api(port, "GET", "/api/tasks")
        assert code == 200
        assert check["board"]["tasks"][0]["sent_at"] is None
        code, _, response = api(port, "POST", "/api/tasks/change", {
            "id": 1, "revision": 1, "operation": "acknowledge"
        })
        assert code == 422, response

        code, _, response = api(port, "POST", "/api/tasks/change", {
            "id": 1, "revision": 1, "operation": "sent"
        })
        assert code == 200 and response["task"]["sent_at"] is not None
        code, _, conflict = api(port, "POST", "/api/tasks/change", {
            "id": 1, "revision": 1, "operation": "sent"
        })
        assert code == 409, conflict
        code, _, response = api(port, "POST", "/api/tasks/change", {
            "id": 1, "revision": 2, "operation": "acknowledge"
        })
        assert code == 200 and response["task"]["acknowledged_at"] is not None
        code, _, response = api(port, "POST", "/api/tasks/change", {
            "id": 1, "revision": 3, "operation": "status",
            "status": "in_progress", "note": "Working"
        })
        assert code == 200 and response["task"]["status"] == "in_progress"

        # Old mutation intent cannot change task state under an outdated revision.
        code, _, conflict = api(port, "POST", "/api/tasks/change", {
            "id": 1, "revision": 2, "operation": "status", "status": "done"
        })
        assert code == 409
        code, _, response = api(port, "POST", "/api/tasks/change", {
            "id": 1, "revision": 4, "operation": "reassign",
            "assignee": "Security Watch", "role": "security-chief", "contact": None
        })
        assert code == 200
        assert response["task"]["sent_at"] is None
        assert response["task"]["acknowledged_at"] is None

        for payload in [
            {"title": "Invalid", "assignee": "A", "role": "head-engineer",
             "priority": "high", "due": "2026-02-30"},
            {"title": "Invalid", "assignee": "A", "role": "head-engineer",
             "priority": "unknown"},
            {"title": "Injected\nBcc:admin@example.org", "assignee": "A",
             "role": "head-engineer", "priority": "normal"},
            {"title": "Invalid", "assignee": "A", "role": "not_a_role",
             "priority": "normal"}
        ]:
            code, _, response = api(port, "POST", "/api/tasks/create", payload)
            assert code == 422, response
        code, _, response = api(port, "POST", "/api/tasks/create", {
            "title": "Bad", "assignee": "A", "role": "head-engineer",
            "priority": "normal"
        }, Origin="http://untrusted.invalid")
        assert code == 403, response

        code, _, response = api(port, "POST", "/api/profile", {
            "id": "head-engineer", "confirm": True
        })
        assert code == 200
        code, _, queue = api(port, "GET", "/api/tasks")
        assert code == 200 and queue["manager"] is False
        assert queue["role"] == "head-engineer"
        assert len(queue["board"]["tasks"]) == 4
        code, _, response = api(port, "POST", "/api/tasks/create", {
            "title": "Forbidden", "assignee": "A",
            "role": "head-engineer", "priority": "normal"
        })
        assert code == 403
        code, _, response = api(port, "POST", "/api/tasks/change", {
            "id": 3, "revision": 1, "operation": "status",
            "status": "blocked", "note": "Parts missing"
        })
        assert code == 200 and response["task"]["status"] == "blocked"
        code, _, response = api(port, "POST", "/api/tasks/change", {
            "id": 2, "revision": 1, "operation": "status", "status": "done"
        })
        assert code == 403
        code, _, response = api(port, "POST", "/api/tasks/handoff", {"id": 3})
        assert code == 403

        code, _, response = api(port, "POST", "/api/profile", {
            "id": "overseer", "confirm": True
        })
        assert code == 200
        code, _, all_tasks = api(port, "GET", "/api/tasks")
        assert code == 200 and len(all_tasks["board"]["tasks"]) == 10
        assert all_tasks["board"]["tasks"][2]["status"] == "blocked"
        assert all_tasks["board"]["tasks"][0]["role"] == "security-chief"
        assert not (state / "notes.json").exists()
        assert not (state / "permissions.json").exists()
        if os.name != "nt":
            assert (state / "tasks.json").stat().st_mode & 0o777 == 0o600
        print("PASS: Overseer persistent assignments x10, manual handoff (not sent), "
              "revision conflicts, acknowledgment/status updates, role-scoped queues, "
              "no arbitrary send or privileges, and desktop/micro UI assets")
    finally:
        service.terminate()
        try:
            service.wait(timeout=8)
        except subprocess.TimeoutExpired:
            service.kill()
            service.wait(timeout=5)
        if service.stdout: service.stdout.close()
        if service.stderr: service.stderr.close()

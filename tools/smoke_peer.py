#!/usr/bin/env python3
"""Two independent Rust-Boy home directories exercise explicit device delivery.

The OpenSSH executable is replaced by a strict *local test relay*. This tests
the typed sender/receiver protocol, failure handling, atomic inbox, role queue,
and remote status sync, NOT the security of a live SSH server or real key pair.
Real Mac/Pi host-key and key authentication require separate field acceptance.
"""
import http.client
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / "target" / "debug" / ("pipboy.exe" if os.name == "nt" else "pipboy")


def invoke(home, *args, input_text=None, ok=True, env_extra=None):
    env = dict(os.environ, PIPBOY_HOME=str(home))
    if env_extra:
        env.update(env_extra)
    result = subprocess.run(
        [str(BIN), *args], env=env, cwd=ROOT,
        input=input_text, capture_output=True, text=True, timeout=22,
        encoding="utf-8"
    )
    if ok:
        assert result.returncode == 0, (args, result.stderr)
    else:
        assert result.returncode != 0, (args, result.stdout)
    return result


def api(port, method, path, body=None):
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=22)
    headers = {}
    if method == "POST":
        headers["Content-Type"] = "application/json"
        headers["Origin"] = f"http://127.0.0.1:{port}"
        body = json.dumps(body).encode("utf-8")
    try:
        connection.request(method, path, body=body, headers=headers)
        response = connection.getresponse()
        return response.status, json.loads(response.read())
    finally:
        connection.close()


def launch(home, env_extra=None):
    env = dict(os.environ, PIPBOY_HOME=str(home))
    if env_extra:
        env.update(env_extra)
    p = subprocess.Popen([str(BIN), "dashboard", "--micro", "--port", "0"],
                         env=env, cwd=ROOT,
                         stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                         text=True, encoding="utf-8")
    assert p.stdout.readline().strip() == "RUST-BOY DASHBOARD / LOCAL ONLY"
    url = urlsplit(p.stdout.readline().strip().removeprefix("Open "))
    assert url.hostname == "127.0.0.1" and url.path == "/micro"
    return p, url.port


def finish(p):
    p.terminate()
    try:
        p.wait(timeout=8)
    except subprocess.TimeoutExpired:
        p.kill()
        p.wait(timeout=5)
    p.stdout.close()
    p.stderr.close()


with tempfile.TemporaryDirectory() as temp:
    root = Path(temp)
    manager = root / "commander"
    scout = root / "scout"
    key = root / "ssh_identity"
    known = root / "verified_hosts"
    key.write_text("CI-only SSH key fixture (not a usable private key)", encoding="utf-8")
    known.write_text("CI-only pinned host fixture (not a usable host key)", encoding="utf-8")

    invoke(manager, "peer", "init", "overseer-a", "--confirm")
    invoke(scout, "peer", "init", "scout-one", "--confirm")
    sender = {
        "schema_version": 1, "from_device": "untrusted",
        "origin_task_id": 1, "origin_revision": 1,
        "task": {
            "title": "Untrusted input", "details": "",
            "assignee": "Scout", "role": "reclamation-scout",
            "priority": "normal"
        }
    }
    invoke(scout, "peer", "receive", input_text=json.dumps(sender), ok=False)
    assert not (scout / "tasks.json").exists()
    invoke(scout, "peer", "trust", "overseer-a", "--confirm")
    # A trust label does not grant OS SSH access; actual SSH keys are mandatory.
    assert json.loads(invoke(scout, "peer", "list", "--json").stdout)["peers"] == []
    # A forced key cannot request any shell command outside typed Rust-Boy verbs.
    invoke(scout, "peer", "bridge", "overseer-a", ok=False,
           env_extra={"SSH_ORIGINAL_COMMAND": "id; touch /tmp/escape"})

    invoke(manager, "peer", "add", "scout-one", "pi@scout.local",
           str(key), str(known), "--confirm")
    assert "scout-one" in invoke(manager, "peer", "list").stdout

    if os.name == "nt":
        # Native Windows compiles/runs peer store and protocol unit tests. The
        # executable SSH relay integration is exercised on Linux and macOS.
        print("PASS: Windows peer enrollment, deny-by-default and typed CLI (SSH relay covered on POSIX)")
        sys.exit(0)

    bin_dir = root / "bin"
    bin_dir.mkdir()
    ssh = bin_dir / "ssh"
    ssh.write_text(
        "#!" + sys.executable + "\n" +
        r'''
import os, sys, subprocess
args = sys.argv[1:]
for required in [
    "StrictHostKeyChecking=yes", "BatchMode=yes", "IdentitiesOnly=yes",
    "ForwardAgent=no", "ClearAllForwardings=yes",
]:
    assert required in args, ("missing OpenSSH safety option", required, args)
assert "pi@scout.local" in args
assert "-T" in args
assert any(x.startswith("UserKnownHostsFile=") for x in args)
remote = args[-1]
assert remote == "pipboy peer receive" or remote == "pipboy peer receipts overseer-a"
command = ["peer", "bridge", "overseer-a"]
payload = sys.stdin.buffer.read(8193)
assert len(payload) <= 8192
child_env = dict(os.environ, PIPBOY_HOME=os.environ["FAKE_RB_RECEIVER_HOME"],
                 SSH_ORIGINAL_COMMAND=remote)
result = subprocess.run(
    [os.environ["FAKE_RB_BINARY"], *command],
    input=payload, capture_output=True, env=child_env, timeout=12
)
sys.stdout.buffer.write(result.stdout)
sys.stderr.buffer.write(result.stderr)
sys.exit(result.returncode)
''' ,
        encoding="utf-8"
    )
    ssh.chmod(0o755)
    sender_env = dict(
        FAKE_RB_BINARY=str(BIN),
        FAKE_RB_RECEIVER_HOME=str(scout),
        PATH=str(bin_dir) + os.pathsep + os.environ["PATH"]
    )

    service, port = launch(manager, sender_env)
    remote_service = None
    try:
        code, peers = api(port, "GET", "/api/peers")
        assert code == 200 and peers["ready"] is True
        assert peers["peers"][0]["alias"] == "scout-one"
        assert not (manager / "tasks.json").exists()
        code, created = api(port, "POST", "/api/tasks/create", {
            "title": "Survey west intake", "details": "Return site condition and water sample notes",
            "assignee": "Scout", "role": "reclamation-scout",
            "priority": "high", "due": "2026-11-05"
        })
        assert code == 200, created
        item = created["task"]
        assert item["delivery"] is None and item["origin"] is None
        code, failure = api(port, "POST", "/api/peer/send", {
            "id": item["id"], "revision": item["revision"], "alias": "scout-one"
        })
        assert code == 422 and not (scout / "tasks.json").exists()
        code, failure = api(port, "POST", "/api/peer/send", {
            "id": item["id"], "revision": item["revision"],
            "alias": "unknown-device", "confirm": True
        })
        assert code == 409 and not (scout / "tasks.json").exists()
        code, sent = api(port, "POST", "/api/peer/send", {
            "id": item["id"], "revision": item["revision"],
            "alias": "scout-one", "confirm": True
        })
        assert code == 200, sent
        assert sent["stored_on_receiver"] is True
        assert sent["human_acknowledged"] is False
        assert sent["task"]["revision"] == 2
        assert sent["task"]["delivery"]["peer"] == "scout-one"
        assert sent["task"]["acknowledged_at"] is None
        assert sent["task"]["remote_progress"] is None
        assert (scout / "tasks.json").is_file()
        code, duplicate = api(port, "POST", "/api/peer/send", {
            "id": item["id"], "revision": item["revision"],
            "alias": "scout-one", "confirm": True
        })
        assert code == 409, duplicate

        receiver_board = json.loads((scout / "tasks.json").read_text(encoding="utf-8"))
        assert len(receiver_board["tasks"]) == 1
        inbound = receiver_board["tasks"][0]
        assert inbound["origin"]["device"] == "overseer-a"
        assert inbound["origin"]["task_id"] == item["id"]
        assert inbound["assignee"] == "Scout"
        # Duplicate packet imports idempotently and never resets progress.
        repeat_packet = {
            "schema_version": 1, "from_device": "overseer-a",
            "origin_task_id": item["id"], "origin_revision": item["revision"],
            "task": {
                "title": item["title"], "details": item["details"],
                "assignee": item["assignee"], "role": item["role"],
                "priority": item["priority"], "due": item["due"]
            }
        }
        same = json.loads(invoke(scout, "peer", "receive",
                                 input_text=json.dumps(repeat_packet)).stdout)
        assert same["remote_task_id"] == inbound["id"]
        assert len(json.loads((scout / "tasks.json").read_text())["tasks"]) == 1

        invoke(scout, "workbench", "use", "reclamation-scout", "--replace")
        remote_service, remote_port = launch(scout)
        code, scoped = api(remote_port, "GET", "/api/tasks")
        assert code == 200 and scoped["manager"] is False
        assert len(scoped["board"]["tasks"]) == 1
        code, ack = api(remote_port, "POST", "/api/tasks/change", {
            "id": inbound["id"], "revision": 1, "operation": "acknowledge"
        })
        assert code == 200, ack
        code, status = api(remote_port, "POST", "/api/tasks/change", {
            "id": inbound["id"], "revision": 2, "operation": "status",
            "status": "in_progress", "note": "At station"
        })
        assert code == 200, status

        code, failure = api(port, "POST", "/api/peer/sync", {"alias": "scout-one"})
        assert code == 422
        code, sync = api(port, "POST", "/api/peer/sync", {
            "alias": "scout-one", "confirm": True
        })
        assert code == 200, sync
        assert sync["updated"] == 1
        code, board = api(port, "GET", "/api/tasks")
        assert code == 200
        original = board["board"]["tasks"][0]
        assert original["status"] == "todo", "Remote status must not silently overwrite manager status"
        assert original["remote_progress"]["status"] == "in_progress"
        assert original["remote_progress"]["acknowledged"] is True
        assert original["acknowledged_at"] is None
        code, sync = api(port, "POST", "/api/peer/sync", {
            "alias": "scout-one", "confirm": True
        })
        assert code == 200 and sync["updated"] == 0
        assert not (manager / "notes.json").exists()
        assert not (manager / "permissions.json").exists()
        print("PASS: two distinct Rust-Boy stores, explicit SSH dispatch, durable receiver ID, "
              "duplicate safety, role inbox, human acknowledgment and on-demand progress "
              "sync without automatic connections")
    finally:
        if remote_service is not None: finish(remote_service)
        finish(service)

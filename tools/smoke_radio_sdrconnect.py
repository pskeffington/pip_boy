#!/usr/bin/env python3
"""Manual native-output smoke using synthetic SDRconnect PCM, never received RF.

Requires a local audio output and a pipboy binary built with sdrconnect. Native
volume stays zero. Preferences and listener addresses live only in a temp home.
--live-ip additionally checks an explicitly requested real Skylab stream.
"""
import argparse
import base64
import hashlib
import json
import math
from pathlib import Path
import select
import socket
import struct
import subprocess
import tempfile
import threading
import time
import urllib.error
import urllib.request


def frame(opcode, payload):
    length = len(payload)
    header = bytes([0x80 | opcode, length if length < 126 else 126])
    if length >= 126:
        header += struct.pack("!H", length)
    return header + payload


class SyntheticReceiver:
    def __init__(self):
        self.listener = socket.socket()
        self.listener.bind(("127.0.0.1", 0))
        self.listener.listen()
        self.endpoint = "127.0.0.1:" + str(self.listener.getsockname()[1])
        self.running = True
        self.clients = 0
        self.errors = []
        self.subscriptions = 0
        self.unsubscribed = 0
        self.starts = 0
        self.lock = threading.Lock()
        self.listener.settimeout(0.2)
        threading.Thread(target=self.accept, daemon=True).start()

    def accept(self):
        while self.running:
            try:
                client, _ = self.listener.accept()
            except socket.timeout:
                continue
            except OSError:
                return
            threading.Thread(target=self.serve, args=(client,), daemon=True).start()

    def serve(self, client):
        with self.lock:
            self.clients += 1
        try:
            client.settimeout(3)
            request = b""
            while b"\r\n\r\n" not in request:
                part = client.recv(4096)
                if not part:
                    return
                request += part
                if len(request) > 8192:
                    raise ValueError("Oversized fixture handshake")
            headers = dict(line.split(": ", 1) for line in request.decode().split("\r\n")[1:] if ": " in line)
            key = next(value for name, value in headers.items() if name.lower() == "sec-websocket-key")
            accept = base64.b64encode(hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode()).digest()).decode()
            client.sendall(("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: " + accept + "\r\n\r\n").encode())
            properties = {
                "api_version": "1.0.3", "active_device": "SYNTHETIC FIXTURE - NO RF",
                "valid_devices": "SYNTHETIC FIXTURE - NO RF", "started": "false",
                "can_control": "true", "device_center_frequency": "99100000",
                "device_sample_rate": "2000000", "demodulator": "WFM",
                "valid_antennas": "Fixture", "active_antenna": "Fixture",
            }
            enabled = False
            pending = b""
            next_audio = time.monotonic()
            # 10 ms stereo, signed little endian. No sample or device data saved.
            pcm = b"\x01\x00" + b"".join(struct.pack("<hh", int(1600 * math.sin(i * 2 * math.pi * 440 / 48000)), 0) for i in range(480))
            while self.running:
                if enabled and time.monotonic() >= next_audio:
                    client.sendall(frame(2, pcm))
                    next_audio = time.monotonic() + 0.01
                if not select.select([client], [], [], 0.003)[0]:
                    continue
                data = client.recv(8192)
                if not data:
                    break
                pending += data
                if len(pending) > 16384:
                    raise ValueError("Oversized fixture request")
                while len(pending) >= 2:
                    opcode, size = pending[0] & 15, pending[1] & 127
                    offset = 2
                    if size == 127:
                        raise ValueError("Unexpected large client frame")
                    if size == 126:
                        if len(pending) < 4:
                            break
                        size = struct.unpack("!H", pending[2:4])[0]
                        offset = 4
                    if not pending[1] & 128:
                        raise ValueError("Client frame must be masked")
                    if len(pending) < offset + 4 + size:
                        break
                    mask = pending[offset:offset + 4]
                    payload = bytes(value ^ mask[i % 4] for i, value in enumerate(pending[offset + 4:offset + 4 + size]))
                    pending = pending[offset + 4 + size:]
                    if opcode == 8:
                        return
                    if opcode == 9:
                        client.sendall(frame(10, payload))
                        continue
                    message = json.loads(payload)
                    event = message["event_type"]
                    if event == "get_property":
                        response = {"event_type": "get_property_response", "property": message["property"], "value": properties[message["property"]], "device": "primary"}
                        client.sendall(frame(1, json.dumps(response).encode()))
                    elif event == "device_stream_enable":
                        if message["value"] != "true" or message.get("device") != "primary":
                            raise ValueError("Unexpected receiver activation request")
                        with self.lock:
                            self.starts += 1
                        properties["started"] = "true"
                    elif event == "audio_stream_enable":
                        enabled = message["value"] == "true"
                        with self.lock:
                            self.subscriptions += int(enabled)
                            self.unsubscribed += int(not enabled)
                    else:
                        raise ValueError("Unexpected shared hardware/control command: " + event)
        except (ConnectionResetError, BrokenPipeError):
            pass  # A cancelled subscription closes its owned socket promptly.
        except (OSError, ValueError, KeyError, StopIteration) as error:
            if self.running:
                self.errors.append(str(error))
        finally:
            client.close()
            with self.lock:
                self.clients -= 1

    def close(self):
        self.running = False
        self.listener.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/debug/pipboy"))
    parser.add_argument("--live-ip", action="store_true")
    parser.add_argument("--standalone", type=Path, help="Also verify rust-radio-cli native playback")
    args = parser.parse_args()
    receiver = SyntheticReceiver()
    reserve = socket.socket()
    reserve.bind(("127.0.0.1", 0))
    port = reserve.getsockname()[1]
    reserve.close()
    origin = "http://127.0.0.1:" + str(port)
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def api(body=None):
        request = urllib.request.Request(origin + "/api/radio", data=None if body is None else json.dumps(body).encode(), headers={"Origin": origin, "Content-Type": "application/json"})
        try:
            with opener.open(request, timeout=5) as response:
                return response.status, json.load(response)
        except urllib.error.HTTPError as error:
            return error.code, json.load(error)

    def wait_for(predicate, timeout=15):
        until = time.monotonic() + timeout
        while time.monotonic() < until:
            code, state = api()
            assert code == 200, state
            if predicate(state):
                return state
            assert state["status"] != "failed", state
            time.sleep(0.04)
        raise AssertionError("Timed out waiting for radio state: " + str(state))

    with tempfile.TemporaryDirectory(prefix="rust-radio-native-smoke-") as home:
        import os
        environment = dict(os.environ, PIPBOY_HOME=home)
        if args.standalone:
            result = subprocess.run([str(args.standalone.resolve()), "sdrconnect-listen", receiver.endpoint, "--seconds", "5", "--volume", "0"], env=environment, capture_output=True, text=True, timeout=15)
            assert result.returncode == 0, result.stdout + result.stderr
        process = subprocess.Popen([str(args.binary.resolve()), "dashboard", "--port", str(port)], env=environment, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            code, state = None, {"error": "Dashboard did not become ready"}
            for _ in range(100):
                try:
                    code, state = api()
                    break
                except OSError:
                    time.sleep(0.05)
            assert code == 200 and state["status"] == "stopped", state
            assert state["sdrconnect"]["available"], "Build binary with --features sdrconnect"
            assert receiver.clients == 0, "Opening HUD contacted the receiver"
            assert api({"action": "sdrconnect", "endpoint": receiver.endpoint})[0] == 200
            assert receiver.clients == 0, "Saving receiver address started I/O"
            assert api({"action": "play_sdr"})[0] == 422
            assert api({"action": "select", "station": "skylab"})[0] == 200
            assert api({"action": "play_sdr", "confirm": True, "volume": 0})[0] == 200
            playing = wait_for(lambda s: s["status"] == "playing")
            assert playing["source"] == "native_sdr" and playing["station"] is None, playing
            assert playing["receiver"]["name"] == "SYNTHETIC FIXTURE - NO RF"
            assert playing["presets"]["last_selected"] == "skylab"
            assert api({"action": "play", "station": "skylab", "confirm": True})[0] == 409
            assert api({"action": "sdrconnect", "endpoint": receiver.endpoint})[0] == 409
            assert api({"action": "volume", "volume": 0})[0] == 200
            assert api({"action": "output", "output": "system"})[0] == 200
            wait_for(lambda s: s["status"] == "playing" and s["source"] == "native_sdr")
            before = time.monotonic()
            assert api({"action": "stop"})[0] == 200
            stop_ms = (time.monotonic() - before) * 1000
            assert stop_ms < 500, "Stop blocked on worker cleanup"
            wait_for(lambda s: s["status"] == "stopped" and not s["stopping"])
            if args.live_ip:
                assert api({"action": "play", "station": "skylab", "confirm": True, "volume": 0})[0] == 200
                wait_for(lambda s: s["status"] == "playing" and s["source"] == "native_ip", 25)
                assert api({"action": "stop"})[0] == 200
                wait_for(lambda s: s["status"] == "stopped" and not s["stopping"])
            until = time.monotonic() + 3
            while receiver.clients and time.monotonic() < until:
                time.sleep(0.02)
            assert receiver.clients == 0
            assert receiver.starts >= 2, "Standalone and HUD Play must activate the stopped receiver"
            assert receiver.subscriptions >= 2 and receiver.unsubscribed >= 2
            assert not receiver.errors, receiver.errors
            print(json.dumps({"fixture": "SYNTHETIC PCM - NO RF", "native_callback_playing": True, "stopped_receiver_activation": receiver.starts, "standalone": bool(args.standalone), "output_restart": True, "stop_ms": round(stop_ms, 2), "subscriptions_closed": True, "live_ip": args.live_ip}))
        finally:
            if process.poll() is None:
                try:
                    api({"action": "stop"})
                except OSError:
                    pass
                process.terminate()
                try:
                    process.wait(5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
            receiver.close()


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Local mock peer only. Does not attest actual Fallout 4 compatibility."""
import json
import os
from pathlib import Path
import socket
import struct
import subprocess
import sys
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
BIN = ROOT / 'target' / 'debug' / ('pipboy.exe' if os.name == 'nt' else 'pipboy')
HELLO = b'{"version":"fixture","lang":"en"}'
ROOT_UPDATE = bytes([8, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0]) + b'HP\0' + bytes([0, 0])


def frame(kind, data=b''):
    return struct.pack('<IB', len(data), kind) + data


def scalar(value, identifier=1):
    return bytes([4]) + struct.pack('<II', identifier, value)


def receive_exact(client, size):
    result = b''
    while len(result) < size:
        chunk = client.recv(size - len(result))
        assert chunk, 'client closed before replying to the heartbeat'
        result += chunk
    return result


def interactive(command, env, prompt_address=None, expected_code=0):
    """Exercise the real browser, including a live HP change before exit."""
    if os.name == 'nt':
        from smoke_windows_conpty import ConPty, configure_windows_api
        host = ConPty(configure_windows_api(), Path(env['PIPBOY_HOME']), argv=command)
        try:
            host.start()
            host.wait_for(b'90', timeout=20)
            host.wait_for(b'playstation')
            if expected_code == 0:
                host.send(b'\x1b[B\r\x08\x03')
            host.finish(expected_code=expected_code)
            if expected_code != 0:
                assert b'Fallout 4 disconnected' in host.output, host.output
        finally:
            host.close()
        return

    import fcntl
    import pty
    import select
    import termios
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 24, 100, 0, 0))
    original = termios.tcgetattr(slave)
    process = subprocess.Popen(command, stdin=slave, stdout=slave, stderr=slave, env=env, cwd=Path(env['PIPBOY_HOME']).parent)
    output = b''
    sent = False
    prompted = prompt_address is None
    deadline = time.monotonic() + 30
    try:
        while process.poll() is None and time.monotonic() < deadline:
            if select.select([master], [], [], .05)[0]:
                chunk = os.read(master, 65536)
                output += chunk
                if b'\x1b[6n' in chunk:
                    os.write(master, b'\x1b[1;1R')
            if not prompted and b'PlayStation IP:' in output:
                os.write(master, (prompt_address + '\n').encode())
                prompted = True
            if b'90' in output and b'playstation' in output and not sent:
                if expected_code == 0:
                    os.write(master, b'\x1b[B\r\x7f\x03')
                sent = True
        process.wait(timeout=2)
        while select.select([master], [], [], 0)[0]:
            output += os.read(master, 65536)
        assert prompted and sent and process.returncode == expected_code, output
        if expected_code != 0:
            assert b'Fallout 4 disconnected' in output, output
        assert b'\x1b[?1049l' in output, output
        assert termios.tcgetattr(slave) == original
    finally:
        if process.poll() is None:
            process.kill()
        process.wait()
        os.close(master)
        os.close(slave)


def exercise(mode):
    errors = []
    interactive_mode = mode in ('interactive', 'late-reference', 'disconnect', 'mac-launcher-interactive')
    launcher_mode = mode.startswith('mac-launcher')
    with socket.socket() as server, tempfile.TemporaryDirectory() as temporary:
        temp = Path(temporary)
        env = dict(os.environ, PIPBOY_HOME=str(temp / 'data'), TERM='xterm-256color')
        server.bind(('127.0.0.1', 0))
        server.listen()
        server.settimeout(20)
        address = '127.0.0.1:' + str(server.getsockname()[1])

        def peer():
            try:
                with server.accept()[0] as client:
                    client.settimeout(8)
                    hello = HELLO
                    if mode == 'invalid-handshake':
                        hello = b'{"version":true,"lang":"en"}'
                    elif mode == 'oversize-handshake':
                        hello = json.dumps({'version': 'fixture', 'lang': 'en', 'extra': 'x' * 4096}).encode()
                    if mode not in ('late-handshake', 'missing-handshake'):
                        client.sendall(frame(1, hello))
                    if mode in ('invalid-handshake', 'oversize-handshake'):
                        assert client.recv(1) == b''
                        return
                    client.sendall(frame(0))
                    assert receive_exact(client, 5) == b'\0' * 5
                    if mode == 'busy':
                        client.sendall(frame(2))
                        return
                    if mode == 'oversize':
                        client.sendall(struct.pack('<IB', 9000000, 3))
                        return
                    # Root arrives before its referenced scalar, in another frame.
                    client.sendall(frame(3, ROOT_UPDATE))
                    for byte in frame(3, scalar(100)):
                        client.sendall(bytes([byte]))
                    if mode in ('late-handshake', 'missing-handshake'):
                        # A complete root alone must not produce diagnostic success.
                        client.settimeout(.3)
                        try:
                            early = client.recv(1)
                        except socket.timeout:
                            early = None
                        assert early is None, 'client declared ready before receiving the handshake'
                        client.settimeout(8)
                        if mode == 'missing-handshake':
                            return
                        client.sendall(frame(1, HELLO))
                    if mode == 'late-reference':
                        # The initial-root deadline must not reject later split updates.
                        time.sleep(10.2)
                        replacement = bytes([8, 0, 0, 0, 0, 1, 0, 2, 0, 0, 0]) + b'HP\0' + bytes([0, 0])
                        client.sendall(frame(3, replacement) + frame(0))
                        assert receive_exact(client, 5) == b'\0' * 5
                        client.sendall(frame(3, scalar(90, identifier=2)))
                    elif interactive_mode:
                        client.sendall(frame(3, scalar(90)))
                    if mode == 'disconnect':
                        # Wait until the actual view has had time to render the update.
                        time.sleep(.5)
                        return
                    assert client.recv(1) == b''  # No gameplay commands.
            except ConnectionResetError:
                # A one-shot client may close while additional peer data is queued.
                pass
            except Exception as error:
                errors.append(error)

        thread = threading.Thread(target=peer, daemon=True)
        thread.start()
        command = [str(BIN), 'game', '--connect' if mode == 'normal' else '--playstation', address, '--confirm']
        if launcher_mode:
            project = temp / 'Project With Spaces'
            project.symlink_to(ROOT, target_is_directory=True)
            command = [str(project / 'scripts' / 'launch-playstation.command')]
        try:
            if interactive_mode:
                interactive(command, env, prompt_address=address if launcher_mode else None, expected_code=1 if mode == 'disconnect' else 0)
            else:
                if launcher_mode:
                    # --diagnose without an address must prompt; stdout stays JSON.
                    invocation = [*command, '--diagnose'] if mode == 'mac-launcher-prompt' else [*command, address, '--diagnose']
                else:
                    invocation = [*command, '--snapshot' if mode == 'normal' else '--diagnose']
                result = subprocess.run(invocation, input=address + '\n', capture_output=True, text=True, timeout=30, env=env, cwd=temp)
        finally:
            thread.join(10)
        assert not thread.is_alive(), mode
        assert not errors, (mode, errors)
        assert not (temp / 'data').exists()
        assert set(p.name for p in temp.iterdir()) <= {'Project With Spaces'}
        if interactive_mode:
            return
        if mode == 'normal':
            assert result.returncode == 0, result.stderr
            report = json.loads(result.stdout)
            assert report['data'] == {'HP': 100}
            assert report['source'] == address
        elif mode in ('playstation', 'late-handshake', 'mac-launcher', 'mac-launcher-prompt'):
            assert result.returncode == 0, result.stderr
            report = json.loads(result.stdout)
            assert report['status'] == 'ready' and report['target_profile'] == 'playstation'
            assert report['handshake'] == {'version': 'fixture', 'lang': 'en'}
            assert report['root_fields'] == ['HP']
            assert report['console_identity_verified'] is False
            assert report['game_actions_enabled'] is False
            if mode == 'mac-launcher-prompt':
                assert 'Enter the console LAN IP' in result.stderr
        else:
            assert result.returncode != 0 and not result.stdout, (mode, result)
            if mode in ('invalid-handshake', 'oversize-handshake'):
                assert 'handshake' in result.stderr


for mode in ('normal', 'interactive', 'late-reference', 'disconnect', 'playstation', 'late-handshake', 'missing-handshake', 'invalid-handshake', 'oversize-handshake', 'busy', 'oversize'):
    exercise(mode)
if sys.platform == 'darwin':
    for mode in ('mac-launcher', 'mac-launcher-prompt', 'mac-launcher-interactive'):
        exercise(mode)
    launcher = ROOT / 'scripts' / 'launch-playstation.command'
    for args in (['--diagnose', '--diagnose'], ['--help', '127.0.0.1'], ['--unknown'], ['127.0.0.1', '127.0.0.2'], [''], ['--diagnose']):
        result = subprocess.run([str(launcher), *args], input='', capture_output=True, text=True, timeout=10)
        assert result.returncode != 0 and not result.stdout, (args, result)
    with tempfile.TemporaryDirectory() as temporary:
        marker = Path(temporary) / 'must-not-exist'
        result = subprocess.run([str(launcher), '$(touch ' + str(marker) + ')', '--diagnose'], capture_output=True, text=True, timeout=15)
        assert result.returncode != 0 and not result.stdout and not marker.exists()

for args in ([], ['--connect', 'localhost:27000', '--confirm'], ['--connect', '127.0.0.1:27000'], ['--connect', '127.0.0.1:27000', '--confirm', '--confirm'], ['--playstation', 'example.com', '--confirm'], ['--playstation', '127.0.0.1', '--connect', '127.0.0.1:1', '--confirm'], ['--playstation', '127.0.0.1', '--confirm', '--snapshot', '--diagnose']):
    result = subprocess.run([str(BIN), 'game', *args], capture_output=True, timeout=5)
    assert result.returncode != 0 and not result.stdout

with tempfile.TemporaryDirectory() as temporary:
    env = dict(os.environ, PIPBOY_HOME=temporary)
    subprocess.run([str(BIN), 'permissions', 'deny', 'network.fetch'], env=env, capture_output=True, check=True)
    policy = Path(temporary) / 'permissions.json'
    original = policy.read_bytes()
    commands = [[str(BIN), 'game', '--playstation', '127.0.0.1:1', '--confirm', '--diagnose']]
    if sys.platform == 'darwin':
        commands.append([str(ROOT / 'scripts' / 'launch-playstation.command'), '127.0.0.1:1', '--diagnose'])
    for command in commands:
        result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=15)
        assert result.returncode != 0 and 'denied by saved' in result.stderr
        assert policy.read_bytes() == original
    policy.write_text('{invalid')
    for command in commands:
        result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=15)
        assert result.returncode != 0 and 'cannot reach' not in result.stderr
        assert policy.read_text() == '{invalid'

print('PASS: game framing, handshake readiness/failures, heartbeat, live updates, disconnect/terminal restoration, PlayStation diagnostics, strict consent, no gameplay commands or saved data')
if sys.platform == 'darwin':
    print('PASS: actual Mac launcher, path with spaces, prompted/explicit IP, clean diagnostic JSON, interactive launch/exit, invalid arguments, literal address handling and saved denial')

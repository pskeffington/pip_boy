#!/usr/bin/env python3
"""Aid API + two real Rust homes joined by an invented local SSH relay.

No real SSH server, usable keys, RF, GPS, medical records or outside network.
The relay checks interoperability, not SSH cryptography or emergency readiness.
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
BIN = ROOT / 'target/debug' / ('pipboy.exe' if os.name == 'nt' else 'pipboy')


def cli(home, *args):
    result = subprocess.run([str(BIN), *args], env=dict(os.environ, PIPBOY_HOME=str(home)),
                            capture_output=True, text=True, timeout=20)
    assert result.returncode == 0, result.stderr


def launch(home, extra=None):
    process = subprocess.Popen([str(BIN), 'dashboard', '--port', '0'], cwd=ROOT,
                               env=dict(os.environ, PIPBOY_HOME=str(home), **(extra or {})),
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    assert process.stdout.readline().strip() == 'RUST-BOY DASHBOARD / LOCAL ONLY'
    port = urlsplit(process.stdout.readline().strip().removeprefix('Open ')).port
    return process, port


def stop(process):
    process.terminate()
    try:
        process.wait(timeout=8)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)
    process.stdout.close()
    process.stderr.close()


def api(port, method, path, body=None, origin=True):
    connection = http.client.HTTPConnection('127.0.0.1', port, timeout=25)
    headers = {}
    if method == 'POST':
        headers['Content-Type'] = 'application/json'
        headers['Origin'] = f'http://127.0.0.1:{port}' if origin else 'https://invalid.example'
        body = json.dumps(body).encode()
    try:
        connection.request(method, path, body=body, headers=headers)
        response = connection.getresponse()
        return response.status, json.loads(response.read())
    finally:
        connection.close()


def post(port, path, body=None, expected=200):
    code, result = api(port, 'POST', '/api/aid/' + path, dict(body or {}, confirm=True))
    assert code == expected, (path, code, result)
    return result


with tempfile.TemporaryDirectory(prefix='rust-boy-aid-') as temporary:
    root = Path(temporary)
    medic, doctor = root / 'medic', root / 'doctor'
    service, port = launch(medic)
    try:
        code, data = api(port, 'GET', '/api/aid')
        assert code == 200 and data['state']['beacon'] is None and not medic.exists()
        assert api(port, 'POST', '/api/aid/activate', {})[0] == 422
        assert api(port, 'POST', '/api/aid/activate', {'confirm': True}, origin=False)[0] == 403
        config = dict(doctor_peer='', emergency_peer='', location='Training location / manual', tier='primary')
        post(port, 'config', {'config': config})
        alarm = post(port, 'activate')['beacon']
        assert alarm['active'] and alarm['peer'] == ''
        assert post(port, 'activate')['beacon']['id'] == alarm['id']
        assert post(port, 'transmit', {'id': alarm['id']})['delivery']['status'] == 'local_only'
        post(port, 'cancel', {'id': alarm['id'] + 1}, expected=409)
        post(port, 'cancel', {'id': alarm['id']})
        packet = dict(kind='nine_line', mode='peacetime', lines=['UNKNOWN'] * 9)
        post(port, 'nine-line', {'packet': packet}, expected=403)
        assert api(port, 'POST', '/api/profile', {'id': 'medic', 'confirm': True})[0] == 200
        post(port, 'nine-line', {'packet': packet}, expected=422)
        bad = dict(config, tier='emergency')
        post(port, 'config', {'config': bad}, expected=422)
        # Journal corruption never hides the independent local emergency latch.
        post(port, 'activate')
        journal = medic / 'messages.json'
        journal.write_text('invalid', encoding='utf-8')
        view = api(port, 'GET', '/api/aid')[1]
        assert view['state']['beacon']['active'] and view['journal_available'] is False
        journal.unlink()
    finally:
        stop(service)
    # Restart preserves latch and cancellation remains explicit.
    service, port = launch(medic)
    try:
        view = api(port, 'GET', '/api/aid')[1]
        assert view['state']['beacon']['active']
        post(port, 'cancel', {'id': view['state']['beacon']['id']})
    finally:
        stop(service)
    if os.name == 'nt':
        print('PASS aid: Windows API validation, persistence, local-only latch and corrupt journal recovery; relay on POSIX')
        sys.exit(0)
    key, known = root / 'fixture-identity', root / 'fixture-known-hosts'
    key.write_text('Invented unusable test identity')
    known.write_text('Invented unusable test host fixture')
    for home, own, other in [(medic, 'medic-one', 'doctor-one'), (doctor, 'doctor-one', 'medic-one')]:
        cli(home, 'peer', 'init', own, '--confirm')
        cli(home, 'peer', 'trust', other, '--confirm')
        cli(home, 'peer', 'add', other, 'unit@' + other + '.invalid', str(key), str(known), '--confirm')
    bindir = root / 'bin'
    bindir.mkdir()
    mode = root / 'relay-mode'
    mode.write_text('online')
    relay = bindir / 'ssh'
    relay.write_text('#!' + sys.executable + '\n' + r'''
import json, os, subprocess, sys
args = sys.argv[1:]
assert args[-1] == 'pipboy peer message-receive'
for option in ['StrictHostKeyChecking=yes', 'BatchMode=yes', 'IdentitiesOnly=yes']:
    assert option in args
if open(os.environ['FAKE_RB_MODE_FILE']).read() == 'offline':
    sys.exit(255)
target = args[-2].split('@')[1].removesuffix('.invalid')
homes = json.loads(os.environ['FAKE_RB_HOME_MAP'])
assert target in homes
result = subprocess.run([os.environ['FAKE_RB_BINARY'], 'peer', 'bridge', os.environ['FAKE_RB_SOURCE']],
    input=sys.stdin.buffer.read(8193), capture_output=True, timeout=15,
    env=dict(os.environ, PIPBOY_HOME=homes[target], SSH_ORIGINAL_COMMAND=args[-1]))
sys.stdout.buffer.write(result.stdout)
sys.exit(result.returncode)
''', encoding='utf-8')
    relay.chmod(0o755)
    extra = dict(PATH=str(bindir) + os.pathsep + os.environ['PATH'], FAKE_RB_BINARY=str(BIN),
                 FAKE_RB_HOME_MAP=json.dumps({'medic-one': str(medic), 'doctor-one': str(doctor)}),
                 FAKE_RB_MODE_FILE=str(mode))
    sender, port = launch(medic, dict(extra, FAKE_RB_SOURCE='medic-one'))
    receiver, remote = launch(doctor, dict(extra, FAKE_RB_SOURCE='doctor-one'))
    try:
        config.update(doctor_peer='doctor-one', emergency_peer='doctor-one')
        post(port, 'config', {'config': config})
        # An existing network dispatch cannot block local activation or cancellation.
        import fcntl
        with (medic / 'aid-dispatch.lock').open('a+b') as busy_dispatch:
            fcntl.flock(busy_dispatch, fcntl.LOCK_EX | fcntl.LOCK_NB)
            busy_alarm = post(port, 'activate')
            assert busy_alarm['beacon']['active'] and busy_alarm['delivery']['status'] == 'unconfirmed'
            cancelled = post(port, 'cancel', {'id': busy_alarm['beacon']['id']})
            assert cancelled['beacon']['active'] is False
        sent = post(port, 'nine-line', {'packet': packet})
        assert sent['stored_on_receiver'] and sent['read_by_doctor'] is False
        records = api(remote, 'GET', '/api/aid')[1]['records']
        assert records[0]['packet'] == packet
        assert api(remote, 'POST', '/api/profile', {'id': 'vault-doctor', 'confirm': True})[0] == 200
        assert api(remote, 'GET', '/api/aid')[1]['medical_allowed']
        mode.write_text('offline')
        alarm = post(port, 'activate')
        assert alarm['beacon']['active'] and alarm['delivery']['status'] == 'queued'
        incident = alarm['beacon']['id']
        pending = alarm['delivery']['message_id']
        assert post(port, 'transmit', {'id': incident})['delivery']['message_id'] == pending
        mode.write_text('online')
        assert post(port, 'transmit', {'id': incident})['delivery']['stored_on_receiver']
        view = api(remote, 'GET', '/api/aid')[1]
        incoming = next(r for r in view['records'] if r['packet']['kind'] == 'alarm')
        # A matching incident number from a different device is not this responder's ACK.
        journal_path = medic / 'messages.json'
        book = json.loads(journal_path.read_text())
        book['messages'].append(dict(id=book['next_id'], peer='wrong-responder', direction='incoming',
                                     body='RB-AID/1 ' + json.dumps(dict(kind='ack', incident=incident)),
                                     created_at=1, delivery='received', remote_origin_id=9901,
                                     remote_stored_id=None))
        book['next_id'] += 1
        journal_path.write_text(json.dumps(book))
        assert api(port, 'GET', '/api/aid')[1]['human_acknowledged'] is False
        assert post(remote, 'ack', {'message_id': incoming['id']})['stored_on_receiver']
        assert api(port, 'GET', '/api/aid')[1]['human_acknowledged'] is True
        assert api(port, 'GET', '/api/aid')[1]['state']['beacon']['active']
        assert post(port, 'cancel', {'id': incident})['delivery']['stored_on_receiver']
        post(remote, 'ack', {'message_id': incoming['id']}, expected=409)
        assert any(r['packet']['kind'] == 'cancel' for r in api(remote, 'GET', '/api/aid')[1]['records'])
        original = (medic / 'aid-state.json').read_bytes()
        (medic / 'aid-state.json').write_text('invalid')
        post(port, 'activate', expected=409)
        assert (medic / 'aid-state.json').read_text() == 'invalid'
        (medic / 'aid-state.json').write_bytes(original)
    finally:
        stop(sender)
        stop(receiver)
print('PASS aid: Medic-to-Doctor nine-line relay, offline queue/retry, responder ACK, cancellation, restart and corruption')

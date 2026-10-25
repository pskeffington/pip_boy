#!/usr/bin/env python3
"""Linux/macOS PTY smoke checks; run after cargo build -p pipboy --locked."""
import json
import os
import re
from pathlib import Path
import select
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / 'target/debug/pipboy'


def terminal_journey(env, malformed_notes=False):
    import fcntl
    import pty
    import struct
    import termios

    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 24, 100, 0, 0))
    original = termios.tcgetattr(slave)
    process = subprocess.Popen([str(BINARY), 'tui'], stdin=slave,
                               stdout=slave, stderr=slave, env=env)
    stage, stage_output = 0, b''
    output = b''
    paused_at = None
    deadline = time.monotonic() + 12
    try:
        while process.poll() is None and time.monotonic() < deadline:
            if select.select([master], [], [], 0.05)[0]:
                chunk = os.read(master, 65536)
                output += chunk
                stage_output += chunk
                if b'\x1b[6n' in chunk:  # Terminal cursor-position query.
                    os.write(master, b'\x1b[1;1R')
            if stage == 0 and b'HISTORY:' in stage_output:
                os.write(master, b'\t\t' if malformed_notes else b'p')
                stage, stage_output = 1, b''
            elif not malformed_notes and stage == 1 and b'PAUSED' in stage_output:
                paused_at = time.monotonic()
                stage, stage_output = 2, b''
            elif stage == 2 and time.monotonic() - paused_at >= 2.3:
                os.write(master, b'p')
                stage, stage_output = 3, b''
            elif stage == 3 and b'1retained/0dropped' in re.sub(rb'\s+', b'', re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]', b'', stage_output)):
                os.write(master, b'\t\t\t\t\tq')
                stage = 4
        if process.poll() is None:
            raise AssertionError(f'TUI timed out at stage {stage}: {stage_output[:1400]!r}')
        while select.select([master], [], [], 0)[0]:
            output += os.read(master, 65536)
        assert process.returncode == (1 if malformed_notes else 0)
        assert malformed_notes or stage == 4
        assert b'\x1b[?1049l' in output, 'alternate screen not restored'
        assert termios.tcgetattr(slave) == original, 'terminal attributes not restored'
    finally:
        if process.poll() is None:
            process.kill()
        process.wait()
        os.close(master)
        os.close(slave)


def resize_and_reload_journey(env):
    import fcntl
    import pty
    import signal
    import struct
    import termios

    path = Path(env['PIPBOY_HOME'], 'notes.json')
    document = {'schema_version': 1, 'notes': [
        {'id': index, 'created_unix_s': 1234, 'text': 'OLDEST_FIXTURE' if index == 1 else f'note {index}'}
        for index in range(1, 21)
    ]}
    path.write_text(json.dumps(document), encoding='utf-8')
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', 24, 100, 0, 0))
    original = termios.tcgetattr(slave)
    process = subprocess.Popen([str(BINARY), 'tui'], stdin=slave, stdout=slave, stderr=slave, env=env)

    def pump(duration):
        chunks = b''
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            if select.select([master], [], [], 0.05)[0]:
                chunk = os.read(master, 65536)
                chunks += chunk
                if b'\x1b[6n' in chunk:
                    os.write(master, b'\x1b[1;1R')
        return re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]', b'', chunks)

    def resize(width, height):
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        process.send_signal(signal.SIGWINCH)
        return pump(0.5)

    try:
        assert b'HISTORY:' in pump(1)
        os.write(master, b'\t\t')
        pump(0.3)
        document['notes'].append({'id': 21, 'created_unix_s': 1234, 'text': 'RELOADED_FIXTURE'})
        path.write_text(json.dumps(document), encoding='utf-8')
        os.write(master, b'r')
        pump(0.3)
        assert b'RELOADED_FIXTURE' in resize(101, 25), 'R did not refresh cached notes'
        os.write(master, b'\x1b[F')  # End selects the oldest note.
        pump(0.3)
        assert b'OLDEST_FIXTURE' in resize(100, 24), 'older notes are unreachable'
        fallback = resize(20, 5)
        assert b'Resizeto40x12' in re.sub(rb'\s+', b'', fallback), repr(fallback)
        assert b'OLDEST_FIXTURE' in resize(80, 24), 'selection lost after resize'
        os.write(master, b'q')
        process.wait(timeout=5)
        assert process.returncode == 0
        assert termios.tcgetattr(slave) == original
        assert json.loads(path.read_text(encoding='utf-8')) == document, 'TUI changed notes'
    finally:
        if process.poll() is None:
            process.kill()
        process.wait()
        os.close(master)
        os.close(slave)


def main():
    with tempfile.TemporaryDirectory(prefix='rust-boy-runtime-') as directory:
        env = dict(os.environ, PIPBOY_HOME=directory, TERM='xterm-256color')
        result = subprocess.run([str(BINARY), 'sample', '--count', '2', '--interval-ms', '500'],
                                env=env, capture_output=True, text=True, check=True, timeout=10)
        rows = [json.loads(line) for line in result.stdout.splitlines()]
        assert len(rows) == 2
        assert rows[1]['captured_session_ms'] - rows[0]['captured_session_ms'] >= 500
        assert all(row['source_id'] == 'host.stats' and row['quality'] == 'valid' for row in rows)
        assert not list(Path(directory).iterdir()), 'telemetry wrote user data'

        process = subprocess.Popen([str(BINARY), 'sample', '--count', '30', '--interval-ms', '30000'],
                                   env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        process.stdout.close()
        try:
            assert process.wait(timeout=5) == 0, process.stderr.read()
        finally:
            if process.poll() is None:
                process.kill()
            process.wait()
            process.stderr.close()
        assert not list(Path(directory).iterdir())
        if os.name == 'posix':
            terminal_journey(env)
            resize_and_reload_journey(env)
            Path(directory, 'notes.json').write_text('{broken', encoding='utf-8')
            terminal_journey(env, malformed_notes=True)
        else:
            print('SKIP: PTY checks require POSIX')
    print('PASS: timed NDJSON, closed pipe, no telemetry files, TUI pause/resume, note reload/oldest selection, resize fallback/recovery and error restoration')


if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""CLI notes persistence and concurrent-writer checks using temporary data only."""
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / 'target/debug' / ('pipboy.exe' if os.name == 'nt' else 'pipboy')


def main():
    with tempfile.TemporaryDirectory(prefix='rust-boy-notes-') as temporary:
        directory = Path(temporary, 'notes')
        env = dict(os.environ, PIPBOY_HOME=str(directory))

        def run(*args):
            return subprocess.run([str(BINARY), 'notes', *args], env=env,
                                  capture_output=True, text=True, timeout=10)

        assert run('list').returncode == 0
        assert run('search', 'battery').returncode == 0
        assert not directory.exists(), 'reads must not create user data'
        assert run('add', 'Battery baseline').returncode == 0

        def add(index):
            deadline = time.monotonic() + 15
            while True:
                result = run('add', f'concurrent fixture {index}')
                if result.returncode == 0:
                    return
                assert 'another writer; retry' in result.stderr, result.stderr
                if time.monotonic() >= deadline:
                    raise AssertionError('concurrent note writer timed out')
                time.sleep(0.01)

        with ThreadPoolExecutor(max_workers=8) as executor:
            list(executor.map(add, range(24)))
        path = directory / 'notes.json'
        before = path.read_bytes()
        document = json.loads(before)
        assert document['schema_version'] == 1
        assert sorted(note['id'] for note in document['notes']) == list(range(1, 26))
        assert {note['text'] for note in document['notes']} == {
            'Battery baseline', *(f'concurrent fixture {index}' for index in range(24))
        }
        result = run('search', 'BATTERY')
        assert result.returncode == 0 and '#1' in result.stdout
        assert 'concurrent fixture' not in result.stdout
        assert path.read_bytes() == before
        assert run('search').returncode != 0
        assert run('search', 'query', 'extra').returncode != 0
        assert run('add', 'embedded\nnewline').returncode != 0
        assert path.read_bytes() == before
        assert {item.name for item in directory.iterdir()} == {'notes.json', 'notes.lock'}
        path.write_text('{"schema_version":2,"notes":[]}', encoding='utf-8')
        unknown_schema = path.read_bytes()
        assert run('list').returncode != 0
        assert run('add', 'must not overwrite').returncode != 0
        assert path.read_bytes() == unknown_schema
    print('PASS: notes read-only access, 24 concurrent processes with retry/no lost notes, unique IDs, search, invalid input and schema preservation')


if __name__ == '__main__':
    main()

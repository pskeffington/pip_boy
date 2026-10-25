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

        def run(*args, command_env=None):
            return subprocess.run([str(BINARY), 'notes', *args], env=command_env or env,
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
        exported = run('export')
        assert exported.returncode == 0
        assert json.loads(exported.stdout) == document
        assert path.read_bytes() == before
        assert run('export', 'extra').returncode != 0
        backup = Path(temporary, 'backup.json')
        backup.write_text(exported.stdout, encoding='utf-8')
        target = Path(temporary, 'restored')
        target_env = dict(env, PIPBOY_HOME=str(target))
        assert run('add', 'existing target note', command_env=target_env).returncode == 0
        restored = run('import', str(backup), command_env=target_env)
        assert restored.returncode == 0 and 'Imported 25 notes' in restored.stdout
        target_path = target / 'notes.json'
        restored_bytes = target_path.read_bytes()
        restored_document = json.loads(restored_bytes)
        assert len(restored_document['notes']) == 26
        assert sorted(note['id'] for note in restored_document['notes']) == list(range(1, 27))
        repeat = run('import', str(backup), command_env=target_env)
        assert repeat.returncode == 0 and 'Imported 0 notes' in repeat.stdout
        assert target_path.read_bytes() == restored_bytes
        assert run('import', command_env=target_env).returncode != 0
        assert run('import', str(backup), 'extra', command_env=target_env).returncode != 0
        backup.write_text('{"schema_version":2,"notes":[]}', encoding='utf-8')
        assert run('import', str(backup), command_env=target_env).returncode != 0
        assert target_path.read_bytes() == restored_bytes
        path.write_text('{"schema_version":2,"notes":[]}', encoding='utf-8')
        unknown_schema = path.read_bytes()
        assert run('list').returncode != 0
        invalid_export = run('export')
        assert invalid_export.returncode != 0 and invalid_export.stdout == ''
        assert run('add', 'must not overwrite').returncode != 0
        assert path.read_bytes() == unknown_schema
    print('PASS: notes read-only access, 24 concurrent processes with retry/no lost notes, unique IDs, search, export/import/re-import, invalid input and schema preservation')


if __name__ == '__main__':
    main()

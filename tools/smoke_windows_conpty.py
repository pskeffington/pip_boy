#!/usr/bin/env python3
"""Windows ConPTY interaction test for Rust-Boy's real TUI.

Uses Windows APIs through Python ctypes; no third-party Python package or
hardware access. Tests normal keyboard flow, virtual-terminal restoration and
the malformed-notes error path. Fails (never silently skips) on CI Windows.
References: Microsoft Windows Console ConPTY API and pseudoconsole walkthrough.
"""
import ctypes
from ctypes import wintypes
import os
from pathlib import Path
import queue
import subprocess
import sys
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "target" / "debug" / "pipboy.exe"
MAX_OUTPUT = 2_000_000
ATTRIBUTE_PSEUDOCONSOLE = 0x00020016
EXTENDED_STARTUPINFO_PRESENT = 0x00080000
CREATE_UNICODE_ENVIRONMENT = 0x00000400
STARTF_USESTDHANDLES = 0x00000100
WAIT_OBJECT_0 = 0
WAIT_TIMEOUT = 258


class Coord(ctypes.Structure):
    _fields_ = [("X", ctypes.c_short), ("Y", ctypes.c_short)]


class StartupInfoW(ctypes.Structure):
    _fields_ = [
        ("cb", wintypes.DWORD),
        ("lpReserved", wintypes.LPWSTR),
        ("lpDesktop", wintypes.LPWSTR),
        ("lpTitle", wintypes.LPWSTR),
        ("dwX", wintypes.DWORD),
        ("dwY", wintypes.DWORD),
        ("dwXSize", wintypes.DWORD),
        ("dwYSize", wintypes.DWORD),
        ("dwXCountChars", wintypes.DWORD),
        ("dwYCountChars", wintypes.DWORD),
        ("dwFillAttribute", wintypes.DWORD),
        ("dwFlags", wintypes.DWORD),
        ("wShowWindow", wintypes.WORD),
        ("cbReserved2", wintypes.WORD),
        ("lpReserved2", ctypes.POINTER(ctypes.c_ubyte)),
        ("hStdInput", wintypes.HANDLE),
        ("hStdOutput", wintypes.HANDLE),
        ("hStdError", wintypes.HANDLE),
    ]


class StartupInfoExW(ctypes.Structure):
    _fields_ = [
        ("StartupInfo", StartupInfoW),
        ("lpAttributeList", ctypes.c_void_p),
    ]


class ProcessInformation(ctypes.Structure):
    _fields_ = [
        ("hProcess", wintypes.HANDLE),
        ("hThread", wintypes.HANDLE),
        ("dwProcessId", wintypes.DWORD),
        ("dwThreadId", wintypes.DWORD),
    ]


def configure_windows_api():
    if os.name != "nt":
        raise RuntimeError("Windows ConPTY test must run on Windows, not as a skip")
    dll = ctypes.WinDLL("kernel32", use_last_error=True)
    ptr = ctypes.c_void_p
    size_t = ctypes.c_size_t
    calls = {
        "CreatePipe": ([ctypes.POINTER(wintypes.HANDLE), ctypes.POINTER(wintypes.HANDLE), ptr, wintypes.DWORD], wintypes.BOOL),
        "CreatePseudoConsole": ([Coord, wintypes.HANDLE, wintypes.HANDLE, wintypes.DWORD, ctypes.POINTER(wintypes.HANDLE)], ctypes.c_long),
        "ResizePseudoConsole": ([wintypes.HANDLE, Coord], ctypes.c_long),
        "ClosePseudoConsole": ([wintypes.HANDLE], None),
        "InitializeProcThreadAttributeList": ([ptr, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(size_t)], wintypes.BOOL),
        "UpdateProcThreadAttribute": ([ptr, wintypes.DWORD, size_t, ptr, size_t, ptr, ptr], wintypes.BOOL),
        "DeleteProcThreadAttributeList": ([ptr], None),
        "CreateProcessW": ([wintypes.LPCWSTR, wintypes.LPWSTR, ptr, ptr, wintypes.BOOL, wintypes.DWORD, ptr, wintypes.LPCWSTR, ctypes.POINTER(StartupInfoW), ctypes.POINTER(ProcessInformation)], wintypes.BOOL),
        "ReadFile": ([wintypes.HANDLE, ptr, wintypes.DWORD, ctypes.POINTER(wintypes.DWORD), ptr], wintypes.BOOL),
        "WriteFile": ([wintypes.HANDLE, ptr, wintypes.DWORD, ctypes.POINTER(wintypes.DWORD), ptr], wintypes.BOOL),
        "CloseHandle": ([wintypes.HANDLE], wintypes.BOOL),
        "WaitForSingleObject": ([wintypes.HANDLE, wintypes.DWORD], wintypes.DWORD),
        "GetExitCodeProcess": ([wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)], wintypes.BOOL),
        "TerminateProcess": ([wintypes.HANDLE, wintypes.UINT], wintypes.BOOL),
    }
    for name, (args, result) in calls.items():
        function = getattr(dll, name)
        function.argtypes = args
        function.restype = result
    return dll


def check(ok, operation):
    if not ok:
        raise OSError(ctypes.get_last_error(), operation)


def check_hr(value, operation):
    if value < 0:
        raise RuntimeError(f"{operation} failed with HRESULT 0x{value & 0xffffffff:08x}")


class ConPty:
    def __init__(self, api, home, initial_size=(100, 24), argv=None):
        self.api = api
        self.home = home
        self.input_read = wintypes.HANDLE()
        self.input_write = wintypes.HANDLE()
        self.output_read = wintypes.HANDLE()
        self.output_write = wintypes.HANDLE()
        self.pseudoconsole = wintypes.HANDLE()
        self.process = ProcessInformation()
        self.attribute_buffer = None
        self.attr_ptr = None
        self.attr_initialized = False
        self.started = False
        self.messages = queue.SimpleQueue()
        self.output = bytearray()
        self.reader = None
        self.initial_size = initial_size
        self.argv = list(argv) if argv is not None else [str(BINARY), "tui"]

    def start(self):
        api = self.api
        check(
            api.CreatePipe(ctypes.byref(self.input_read), ctypes.byref(self.input_write), None, 0),
            "CreatePipe input",
        )
        check(
            api.CreatePipe(ctypes.byref(self.output_read), ctypes.byref(self.output_write), None, 0),
            "CreatePipe output",
        )
        check_hr(
            api.CreatePseudoConsole(
                Coord(*self.initial_size), self.input_read, self.output_write,
                0, ctypes.byref(self.pseudoconsole),
            ),
            "CreatePseudoConsole",
        )

        size = ctypes.c_size_t()
        api.InitializeProcThreadAttributeList(None, 1, 0, ctypes.byref(size))
        check(size.value > 0, "InitializeProcThreadAttributeList sizing")
        self.attribute_buffer = ctypes.create_string_buffer(size.value)
        self.attr_ptr = ctypes.c_void_p(ctypes.addressof(self.attribute_buffer))
        check(
            api.InitializeProcThreadAttributeList(
                self.attr_ptr, 1, 0, ctypes.byref(size)
            ),
            "InitializeProcThreadAttributeList",
        )
        self.attr_initialized = True
        # PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE takes the HPCON value as
        # lpValue (an opaque pointer), not the address of a HANDLE variable.
        # Passing byref(HPCON) creates a different invalid pointer and leaves
        # the child disconnected even when process creation reports success.
        check(
            api.UpdateProcThreadAttribute(
                self.attr_ptr, 0, ATTRIBUTE_PSEUDOCONSOLE,
                self.pseudoconsole, ctypes.sizeof(wintypes.HANDLE), None, None,
            ),
            "UpdateProcThreadAttribute pseudoconsole",
        )

        startup = StartupInfoExW()
        startup.StartupInfo.cb = ctypes.sizeof(StartupInfoExW)
        # The Actions runner has no parent interactive console; force the
        # child to use the new ConPTY's standard handles instead of the
        # CI process's inherited redirected stdout/stderr pipes.
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES
        startup.lpAttributeList = self.attr_ptr
        command = ctypes.create_unicode_buffer(
            subprocess.list2cmdline(self.argv)
        )
        env = dict(os.environ, PIPBOY_HOME=str(self.home), TERM="xterm-256color")
        env_text = "".join(
            f"{key}={value}\0" for key, value in
            sorted(env.items(), key=lambda pair: pair[0].upper())
        ) + "\0"
        env_block = ctypes.create_unicode_buffer(env_text)
        check(
            api.CreateProcessW(
                self.argv[0], command, None, None, False,
                EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT,
                ctypes.cast(env_block, ctypes.c_void_p), None,
                ctypes.byref(startup.StartupInfo),
                ctypes.byref(self.process),
            ),
            "CreateProcessW with ConPTY",
        )
        self.started = True
        # The pseudoconsole and child retain their own references to pipe ends.
        self._close_handle(self.input_read)
        self._close_handle(self.output_write)

        def consume():
            while True:
                data = ctypes.create_string_buffer(4096)
                amount = wintypes.DWORD()
                ok = api.ReadFile(
                    self.output_read, data, 4096, ctypes.byref(amount), None
                )
                if not ok:
                    self.messages.put(("read_error", ctypes.get_last_error()))
                    break
                if amount.value == 0:
                    self.messages.put(None)
                    break
                self.messages.put(data.raw[:amount.value])

        self.reader = threading.Thread(target=consume, daemon=True)
        self.reader.start()

    def _close_handle(self, handle):
        # ctypes Structure HANDLE fields may be exposed as plain Python ints.
        value = handle.value if isinstance(handle, ctypes.c_void_p) else handle
        if value:
            self.api.CloseHandle(value)
            if isinstance(handle, ctypes.c_void_p):
                handle.value = None

    def _drain(self, duration=0.05):
        limit = time.monotonic() + duration
        while time.monotonic() < limit:
            try:
                chunk = self.messages.get(timeout=min(0.05, max(0.001, limit - time.monotonic())))
            except queue.Empty:
                continue
            if chunk is None:
                break
            if isinstance(chunk, tuple):
                raise AssertionError(f"ConPTY read failed: win32={chunk[1]}")
            self.output.extend(chunk)
            if len(self.output) > MAX_OUTPUT:
                raise AssertionError("ConPTY produced excessive terminal output")
            # Cursor-position queries must be answered by the virtual host.
            if b"\x1b[6n" in chunk:
                self.send(b"\x1b[1;1R")

    def wait_for(self, marker, *, since=0, timeout=10):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            self._drain(0.1)
            if marker in self.output[since:]:
                return
            # The child can exit before ConHost flushes pending pipe output.
            # Keep draining until the deadline instead of racing process exit.
        alive = self.api.WaitForSingleObject(self.process.hProcess, 0) == WAIT_TIMEOUT
        raise AssertionError(
            f"ConPTY never displayed {marker!r}, child_alive={alive}, "
            f"reader_alive={self.reader.is_alive() if self.reader else False}: "
            f"{self.output[-1200:]!r}"
        )

    def send(self, data):
        written = wintypes.DWORD()
        blob = ctypes.create_string_buffer(data)
        check(
            self.api.WriteFile(
                self.input_write, blob, len(data), ctypes.byref(written), None
            ),
            "WriteFile ConPTY input",
        )
        assert written.value == len(data), "partial ConPTY input"

    def resize(self, width, height):
        check_hr(
            self.api.ResizePseudoConsole(self.pseudoconsole, Coord(width, height)),
            "ResizePseudoConsole",
        )

    def finish(self, expected_code=0, timeout=10, require_screen=True):
        done = self.api.WaitForSingleObject(self.process.hProcess, int(timeout * 1000))
        assert done == WAIT_OBJECT_0, f"ConPTY child timed out; tail={self.output[-900:]!r}"
        code = wintypes.DWORD()
        check(
            self.api.GetExitCodeProcess(self.process.hProcess, ctypes.byref(code)),
            "GetExitCodeProcess",
        )
        self._drain(0.2)
        assert code.value == expected_code, (
            f"ConPTY exit code {code.value}, expected {expected_code}; "
            f"tail={self.output[-900:]!r}"
        )
        if require_screen:
            assert b"\x1b[?1049h" in self.output, "alternate screen was never entered"
            assert b"\x1b[?1049l" in self.output, "alternate screen was not restored"

    def close(self):
        # Always reap the child and close ConPTY, even when a test assertion fails.
        if self.started and self.process.hProcess:
            if self.api.WaitForSingleObject(self.process.hProcess, 0) == WAIT_TIMEOUT:
                self.api.TerminateProcess(self.process.hProcess, 1)
                self.api.WaitForSingleObject(self.process.hProcess, 3000)
        if self.attr_initialized:
            self.api.DeleteProcThreadAttributeList(self.attr_ptr)
            self.attr_initialized = False
        if self.pseudoconsole.value:
            self.api.ClosePseudoConsole(self.pseudoconsole)
            self.pseudoconsole.value = None
        for handle in (
            self.input_write, self.input_read, self.output_write,
            self.output_read, self.process.hThread, self.process.hProcess,
        ):
            self._close_handle(handle)
        if self.reader is not None:
            self.reader.join(timeout=2)


def exercise(api, home, *, corrupt_notes):
    if corrupt_notes:
        home.mkdir(parents=True, exist_ok=True)
        (home / "notes.json").write_text("{invalid", encoding="utf-8")
    host = ConPty(api, home)
    try:
        host.start()
        host.wait_for(b"HISTORY:")
        if corrupt_notes:
            host.send(b"\t\t")
            host.finish(expected_code=1)
            assert (home / "notes.json").read_text(encoding="utf-8") == "{invalid"
        else:
            offset = len(host.output)
            host.send(b"p")
            host.wait_for(b"PAUSED", since=offset)
            offset = len(host.output)
            host.send(b"p")
            host.wait_for(b"HISTORY:", since=offset)
            offset = len(host.output)
            host.resize(20, 5)
            host.wait_for(b"Resize to 40x12", since=offset)
            offset = len(host.output)
            host.resize(100, 24)
            host.wait_for(b"HISTORY:", since=offset)
            host.send(b"q")
            host.finish(expected_code=0)
            assert not home.exists(), "normal TUI created or modified user files"
    finally:
        host.close()


def main():
    if os.name != "nt":
        raise RuntimeError("ConPTY validation requires a Windows runner")
    assert BINARY.is_file(), f"build the native Windows executable first: {BINARY}"
    api = configure_windows_api()
    with tempfile.TemporaryDirectory(prefix="rust-boy-conpty-") as temp:
        root = Path(temp)
        command = [
            os.environ.get("COMSPEC", r"C:\Windows\System32\cmd.exe"),
            "/d", "/c", "echo RUSTBOY_CONPTY_PROBE",
        ]
        probe = ConPty(api, root / "probe", argv=command)
        try:
            probe.start()
            probe.wait_for(b"RUSTBOY_CONPTY_PROBE", timeout=5)
            probe.finish(expected_code=0, require_screen=False)
            print("PASS: ConPTY Windows cmd echo and output pipe")
        finally:
            probe.close()
        exercise(api, root / "normal", corrupt_notes=False)
        exercise(api, root / "corrupt", corrupt_notes=True)
    print(
        "PASS: Windows ConPTY TUI launch, pause/resume, resize/fallback, "
        "clean exit, error restoration, no hidden writes, no hardware IO"
    )


if __name__ == "__main__":
    main()

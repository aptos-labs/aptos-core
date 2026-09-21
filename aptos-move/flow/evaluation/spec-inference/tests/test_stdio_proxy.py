from __future__ import annotations

import asyncio
import os
from pathlib import Path
import signal
import sys
import tempfile
import unittest

from harness.stdio_proxy import StdioProxy


class StdioProxyTest(unittest.TestCase):
    def test_bridges_one_client_to_controller_side_process(self) -> None:
        async def exercise(root: Path) -> tuple[bytes, list[str]]:
            socket_path = root / "mcp.sock"
            errors: list[str] = []
            program = (
                "import sys\n"
                "for line in sys.stdin.buffer:\n"
                " sys.stdout.buffer.write(line.upper()); sys.stdout.buffer.flush()\n"
            )
            async with StdioProxy(
                socket_path,
                [sys.executable, "-c", program],
                dict(os.environ),
                root,
                errors.append,
            ):
                reader, writer = await asyncio.open_unix_connection(str(socket_path))
                writer.write(b"hello\n")
                await writer.drain()
                response = await reader.readline()
                writer.close()
                await writer.wait_closed()
            return response, errors

        with tempfile.TemporaryDirectory() as temporary:
            response, errors = asyncio.run(exercise(Path(temporary)))
        self.assertEqual(b"HELLO\n", response)
        self.assertEqual([], errors)

    def test_checked_in_stdio_client_bridges_to_proxy(self) -> None:
        async def exercise(root: Path) -> tuple[bytes, bytes, int]:
            socket_path = root / "mcp.sock"
            program = (
                "import sys\n"
                "for line in sys.stdin.buffer:\n"
                " sys.stdout.buffer.write(line.upper()); sys.stdout.buffer.flush()\n"
            )
            async with StdioProxy(
                socket_path,
                [sys.executable, "-c", program],
                dict(os.environ),
                root,
                lambda _: None,
            ):
                process = await asyncio.create_subprocess_exec(
                    str(Path(__file__).resolve().parent.parent / "sandbox/stdio-proxy-client.py"),
                    str(socket_path),
                    stdin=asyncio.subprocess.PIPE,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE,
                )
                stdout, stderr = await process.communicate(b"hello\n")
            return stdout, stderr, int(process.returncode or 0)

        with tempfile.TemporaryDirectory() as temporary:
            stdout, stderr, returncode = asyncio.run(exercise(Path(temporary)))
        self.assertEqual(b"HELLO\n", stdout)
        self.assertEqual(b"", stderr)
        self.assertEqual(0, returncode)

    def test_child_exit_closes_a_client_that_keeps_stdin_open(self) -> None:
        async def exercise(root: Path) -> bytes:
            socket_path = root / "mcp.sock"
            async with StdioProxy(
                socket_path,
                [sys.executable, "-c", "pass"],
                dict(os.environ),
                root,
                lambda _: None,
            ):
                reader, writer = await asyncio.open_unix_connection(str(socket_path))
                try:
                    return await asyncio.wait_for(reader.read(), timeout=2)
                finally:
                    writer.close()
                    await writer.wait_closed()

        with tempfile.TemporaryDirectory() as temporary:
            response = asyncio.run(exercise(Path(temporary)))
        self.assertEqual(b"", response)

    def test_teardown_terminates_the_child_process_group(self) -> None:
        async def exercise(root: Path) -> tuple[int, Path]:
            socket_path = root / "mcp.sock"
            pid_path = root / "child.pid"
            stopped_path = root / "child.stopped"
            child = (
                "import os, pathlib, signal, sys, time\n"
                "pid, stopped = map(pathlib.Path, sys.argv[1:])\n"
                "pid.write_text(str(os.getpid()))\n"
                "def stop(*_):\n"
                " stopped.write_text('stopped')\n"
                " raise SystemExit(0)\n"
                "signal.signal(signal.SIGTERM, stop)\n"
                "while True: time.sleep(1)\n"
            )
            parent = (
                "import subprocess, sys, time\n"
                "subprocess.Popen([sys.executable, '-c', sys.argv[1], *sys.argv[2:]])\n"
                "while True: time.sleep(1)\n"
            )
            async with StdioProxy(
                socket_path,
                [
                    sys.executable,
                    "-c",
                    parent,
                    child,
                    str(pid_path),
                    str(stopped_path),
                ],
                dict(os.environ),
                root,
                lambda _: None,
            ):
                reader, writer = await asyncio.open_unix_connection(str(socket_path))
                del reader
                for _ in range(200):
                    if pid_path.is_file():
                        break
                    await asyncio.sleep(0.01)
                self.assertTrue(pid_path.is_file(), "child process did not start")
                child_pid = int(pid_path.read_text())
                writer.close()
            return child_pid, stopped_path

        child_pid = -1
        try:
            with tempfile.TemporaryDirectory() as temporary:
                child_pid, stopped_path = asyncio.run(exercise(Path(temporary)))
                self.assertTrue(stopped_path.is_file())
        finally:
            if child_pid > 0:
                try:
                    os.kill(child_pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass

if __name__ == "__main__":
    unittest.main()

from __future__ import annotations

import asyncio
import os
from pathlib import Path
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

if __name__ == "__main__":
    unittest.main()

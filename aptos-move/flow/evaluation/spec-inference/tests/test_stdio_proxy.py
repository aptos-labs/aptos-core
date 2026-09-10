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


if __name__ == "__main__":
    unittest.main()

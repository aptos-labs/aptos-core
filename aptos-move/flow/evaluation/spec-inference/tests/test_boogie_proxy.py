from __future__ import annotations

import asyncio
import json
import shutil
import tempfile
import unittest
from pathlib import Path

from harness.boogie_proxy import BoogieProxy


class BoogieProxyTest(unittest.TestCase):
    def test_executes_only_from_allowed_working_root(self) -> None:
        async def exercise(root: Path) -> None:
            socket = root / "boogie.sock"
            async with BoogieProxy(socket, Path("/usr/bin/printf"), (root,)):
                reader, writer = await asyncio.open_unix_connection(socket)
                writer.write(
                    json.dumps({"arguments": ["proxy-ok"], "cwd": str(root)}).encode()
                    + b"\n"
                )
                await writer.drain()
                response = json.loads(await reader.readline())
                writer.close()
                await writer.wait_closed()
                self.assertEqual(0, response["returncode"])
                self.assertNotEqual("", response["stdout_base64"])

        with tempfile.TemporaryDirectory() as temporary:
            asyncio.run(exercise(Path(temporary)))

    def test_boogie_is_killed_when_the_client_disconnects(self) -> None:
        # The prover's watchdog kills the client; the controller-side Boogie
        # it asked for must go with it, not run on for its own time limit.
        async def exercise(root: Path) -> None:
            socket = root / "boogie.sock"
            sleep = Path(shutil.which("sleep") or "/usr/bin/sleep")
            async with BoogieProxy(socket, sleep, (root,)) as proxy:
                reader, writer = await asyncio.open_unix_connection(socket)
                writer.write(
                    json.dumps({"arguments": ["30"], "cwd": str(root)}).encode() + b"\n"
                )
                await writer.drain()
                for _ in range(100):
                    if proxy.processes:
                        break
                    await asyncio.sleep(0.05)
                self.assertTrue(proxy.processes, "Boogie was not started")
                process = proxy.processes[-1]
                writer.close()
                await writer.wait_closed()
                for _ in range(100):
                    if process.returncode is not None:
                        break
                    await asyncio.sleep(0.05)
                self.assertIsNotNone(process.returncode, "Boogie outlived its client")

        with tempfile.TemporaryDirectory() as temporary:
            asyncio.run(exercise(Path(temporary)))


if __name__ == "__main__":
    unittest.main()

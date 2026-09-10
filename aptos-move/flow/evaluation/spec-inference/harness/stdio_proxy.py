"""Controller-side stdio process exposed through one Unix-socket byte stream."""

from __future__ import annotations

import asyncio
import contextlib
from pathlib import Path
from typing import Callable


class StdioProxy:
    """Launch one process outside the agent domain per active client."""

    def __init__(
        self,
        socket_path: Path,
        command: list[str],
        environment: dict[str, str],
        cwd: Path,
        stderr_sink: Callable[[str], None],
    ):
        self.socket_path = socket_path
        self.command = command
        self.environment = environment
        self.cwd = cwd
        self.stderr_sink = stderr_sink
        self._server: asyncio.AbstractServer | None = None
        self._connection: asyncio.Task[None] | None = None
        self._connected = False

    async def __aenter__(self) -> "StdioProxy":
        self.socket_path.unlink(missing_ok=True)
        self._server = await asyncio.start_unix_server(
            self._accept, path=str(self.socket_path)
        )
        return self

    async def __aexit__(self, *args: object) -> None:
        if self._server is not None:
            self._server.close()
            await self._server.wait_closed()
        if self._connection is not None and not self._connection.done():
            self._connection.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await self._connection
        self.socket_path.unlink(missing_ok=True)

    async def _accept(
        self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter
    ) -> None:
        if self._connected:
            writer.close()
            await writer.wait_closed()
            return
        self._connected = True
        self._connection = asyncio.current_task()
        process = await asyncio.create_subprocess_exec(
            *self.command,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            cwd=self.cwd,
            env=self.environment,
        )
        assert process.stdin is not None
        assert process.stdout is not None
        assert process.stderr is not None

        async def client_to_process() -> None:
            try:
                while chunk := await reader.read(65536):
                    process.stdin.write(chunk)
                    await process.stdin.drain()
            finally:
                process.stdin.close()

        async def process_to_client() -> None:
            while chunk := await process.stdout.read(65536):
                writer.write(chunk)
                await writer.drain()

        async def collect_stderr() -> None:
            while chunk := await process.stderr.read(65536):
                self.stderr_sink(chunk.decode(errors="replace"))

        try:
            await asyncio.gather(
                client_to_process(), process_to_client(), collect_stderr()
            )
            await process.wait()
        finally:
            if process.returncode is None:
                process.terminate()
                await process.wait()
            writer.close()
            with contextlib.suppress(ConnectionError):
                await writer.wait_closed()
            self._connected = False
            self._connection = None

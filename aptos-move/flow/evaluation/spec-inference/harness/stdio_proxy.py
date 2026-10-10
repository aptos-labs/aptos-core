"""Controller-side stdio process exposed through one Unix-socket byte stream."""

from __future__ import annotations

import asyncio
import contextlib
import os
from pathlib import Path
import signal
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
        self._process: asyncio.subprocess.Process | None = None
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
        if self._process is not None:
            await self._stop_process_group(self._process)
        if self._connection is not None and not self._connection.done():
            self._connection.cancel()
            with contextlib.suppress(asyncio.CancelledError):
                await self._connection
        if self._server is not None:
            await self._server.wait_closed()
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
            # The MCP server launches Boogie and Z3. Give the whole tree a
            # private process group so cancellation cannot orphan solvers.
            start_new_session=True,
        )
        self._process = process
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

        client_task = asyncio.create_task(client_to_process())
        output_task = asyncio.create_task(process_to_client())
        stderr_task = asyncio.create_task(collect_stderr())
        wait_task = asyncio.create_task(process.wait())
        tasks = (client_task, output_task, stderr_task, wait_task)
        cancellation: asyncio.CancelledError | None = None
        try:
            # Waiting for all three copy loops deadlocks when the child exits:
            # stdout and stderr reach EOF, but the client input remains open.
            # The process watcher closes that direction explicitly.
            done, _ = await asyncio.wait(
                (client_task, output_task, wait_task),
                return_when=asyncio.FIRST_COMPLETED,
            )
            for task in done:
                await task
            if wait_task in done:
                client_task.cancel()
                with contextlib.suppress(asyncio.CancelledError):
                    await client_task
            elif output_task in done:
                # An MCP process without stdout can no longer serve requests.
                await self._stop_process_group(process)
            else:
                # Client EOF closes the child's stdin; let it flush its final
                # response and stderr before collecting its exit status.
                await wait_task
            await asyncio.gather(output_task, stderr_task)
        except asyncio.CancelledError as error:
            # Consume cancellation while the process group and socket are
            # synchronously cleaned up, then preserve it for the caller.
            cancellation = error
        finally:
            for task in tasks:
                if not task.done():
                    task.cancel()
            await asyncio.gather(*tasks, return_exceptions=True)
            await self._stop_process_group(process)
            writer.close()
            with contextlib.suppress(ConnectionError):
                await writer.wait_closed()
            self._connected = False
            self._connection = None
            self._process = None
        if cancellation is not None:
            raise cancellation

    @staticmethod
    async def _stop_process_group(process: asyncio.subprocess.Process) -> None:
        """Stop the MCP server and every solver descendant it launched."""

        def signal_group(number: signal.Signals) -> None:
            with contextlib.suppress(ProcessLookupError):
                os.killpg(process.pid, number)

        signal_group(signal.SIGTERM)
        if process.returncode is None:
            try:
                await asyncio.wait_for(process.wait(), timeout=1)
            except asyncio.TimeoutError:
                signal_group(signal.SIGKILL)
                await process.wait()

        # The group can outlive its leader when a solver child survives a
        # clean or abrupt MCP exit. Give TERM a short grace period, then make
        # the teardown deterministic.
        for _ in range(20):
            try:
                os.killpg(process.pid, 0)
            except ProcessLookupError:
                return
            await asyncio.sleep(0.01)
        signal_group(signal.SIGKILL)

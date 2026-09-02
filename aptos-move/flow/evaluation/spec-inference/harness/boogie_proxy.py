"""Run Boogie outside the agent's inherited Landlock domain.

Boogie is a self-contained .NET bundle, and CoreCLR reads ``/proc/self/maps``
and ``/proc/self/mountinfo`` at startup. Landlock resolves the ``/proc/self``
symlink once, when the agent wrapper builds its ruleset, so the rule names the
wrapper's own PID directory and no other. Every Boogie the agent's ``move-flow``
spawns is a different process, its ``/proc/self`` is refused, and CoreCLR
aborts with ``0x8007000E`` -- while a Boogie exec'd directly by the wrapper
works, which is how a preflight that only exec's Boogie misses it. A fresh
``/proc`` per agent is not available on a host whose procfs carries locked
over-mounts, and admitting all of ``/proc`` would expose other processes.

So the agent never runs Boogie itself. Its ``BOOGIE_EXE`` is a small client
that hands the argument list and working directory to the controller over a
run-local Unix socket; the controller, which is the trusted judge boundary
anyway, runs the real executable and returns its streams and exit status. The
working directory is checked to lie inside the run, so the proxy verifies only
what the agent could have verified itself.
"""

from __future__ import annotations

import asyncio
import base64
import json
import os
from contextlib import suppress
from pathlib import Path
from typing import Any


MAX_REQUEST_BYTES = 1024 * 1024


async def _wait_for_eof(reader: asyncio.StreamReader) -> None:
    """Return once the peer has closed its end; the request was one line."""
    while await reader.read(65536):
        pass


class BoogieProxy:
    def __init__(
        self,
        socket_path: Path,
        executable: Path,
        allowed_working_roots: tuple[Path, ...],
    ) -> None:
        self.socket_path = socket_path
        self.executable = executable
        self.allowed_working_roots = tuple(path.resolve() for path in allowed_working_roots)
        self._server: asyncio.AbstractServer | None = None
        #: Every Boogie process this proxy started, for tests and diagnostics.
        self.processes: list[asyncio.subprocess.Process] = []

    async def __aenter__(self) -> "BoogieProxy":
        if not self.executable.is_file() or not os.access(self.executable, os.X_OK):
            raise RuntimeError(f"Boogie proxy executable is not executable: {self.executable}")
        self.socket_path.unlink(missing_ok=True)
        self._server = await asyncio.start_unix_server(
            self._handle_client, path=self.socket_path
        )
        self.socket_path.chmod(0o600)
        return self

    async def __aexit__(self, *args: object) -> None:
        if self._server is not None:
            self._server.close()
            await self._server.wait_closed()
        self.socket_path.unlink(missing_ok=True)

    async def _handle_client(
        self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter
    ) -> None:
        try:
            raw = await reader.readline()
            if not raw or len(raw) > MAX_REQUEST_BYTES:
                raise ValueError("invalid Boogie proxy request size")
            request = json.loads(raw)
            arguments, cwd = self._validate_request(request)
            process = await asyncio.create_subprocess_exec(
                str(self.executable),
                *arguments,
                cwd=cwd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            self.processes.append(process)
            # The client holds its connection open until it has the answer, so
            # its end closing means it is gone -- killed by the prover's
            # watchdog, or its session over. Boogie must not outlive it.
            communicate = asyncio.ensure_future(process.communicate())
            gone = asyncio.ensure_future(_wait_for_eof(reader))
            done, _ = await asyncio.wait({communicate, gone}, return_when=asyncio.FIRST_COMPLETED)
            if communicate not in done:
                process.kill()
                await communicate
                # The peer is gone, but this end still has to close, or the
                # server's shutdown waits for a connection that never ends.
                writer.close()
                with suppress(Exception):
                    await writer.wait_closed()
                return
            gone.cancel()
            stdout, stderr = communicate.result()
            response: dict[str, Any] = {
                "returncode": process.returncode,
                "stdout_base64": base64.b64encode(stdout).decode("ascii"),
                "stderr_base64": base64.b64encode(stderr).decode("ascii"),
            }
        except Exception as error:
            response = {
                "returncode": 125,
                "stdout_base64": "",
                "stderr_base64": base64.b64encode(
                    f"boogie proxy: {error}\n".encode()
                ).decode("ascii"),
            }
        writer.write(json.dumps(response, sort_keys=True).encode() + b"\n")
        await writer.drain()
        writer.close()
        await writer.wait_closed()

    def _validate_request(self, request: Any) -> tuple[list[str], Path]:
        if not isinstance(request, dict):
            raise ValueError("request is not an object")
        arguments = request.get("arguments")
        if not isinstance(arguments, list) or not all(
            isinstance(argument, str) for argument in arguments
        ):
            raise ValueError("arguments must be a string list")
        cwd_value = request.get("cwd")
        if not isinstance(cwd_value, str):
            raise ValueError("cwd must be a string")
        cwd = Path(cwd_value).resolve()
        if not any(cwd == root or cwd.is_relative_to(root) for root in self.allowed_working_roots):
            raise ValueError(f"working directory is outside the run sandbox: {cwd}")
        return arguments, cwd

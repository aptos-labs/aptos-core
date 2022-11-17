from __future__ import annotations

import asyncio
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from typing import Sequence


@dataclass
class RunResult:
    exit_code: int
    output: bytes

    def unwrap(self) -> bytes:
        if not self.succeeded():
            raise Exception(self.output.decode("utf-8"))
        return self.output

    def succeeded(self) -> bool:
        return self.exit_code == 0


class Shell:
    def run(self, command: Sequence[str], stream_output: bool = False) -> RunResult:
        raise NotImplementedError()

    async def gen_run(
        self, command: Sequence[str], stream_output: bool = False
    ) -> RunResult:
        raise NotImplementedError()


@dataclass
class LocalShell(Shell):
    verbose: bool = False

    def run(self, command: Sequence[str], stream_output: bool = False) -> RunResult:
        # Write to a temp file, stream to stdout
        tmpname = tempfile.mkstemp()[1]
        with open(tmpname, "wb") as writer, open(tmpname, "rb") as reader:
            if self.verbose:
                print(f"+ {' '.join(command)}")
            process = subprocess.Popen(command, stdout=writer, stderr=writer)
            output = b""
            while process.poll() is None:
                chunk = reader.read()
                output += chunk
                if stream_output:
                    sys.stdout.write(chunk.decode("utf-8"))
                time.sleep(0.1)
            output += reader.read()
        return RunResult(process.returncode, output)

    async def gen_run(
        self, command: Sequence[str], stream_output: bool = False
    ) -> RunResult:
        # Write to a temp file, stream to stdout
        tmpname = tempfile.mkstemp()[1]
        with open(tmpname, "wb") as writer, open(tmpname, "rb") as reader:
            if self.verbose:
                print(f"+ {' '.join(command)}")
            try:
                process = await asyncio.create_subprocess_exec(
                    command[0], *command[1:], stdout=writer, stderr=writer
                )
            except Exception as e:
                raise Exception(f"Failed running {command}") from e
            output = b""
            while True:
                wait_task = asyncio.create_task(process.wait())
                finished, running = await asyncio.wait({wait_task}, timeout=1)
                assert bool(finished) ^ bool(
                    running
                ), "Cannot have both finished and running"
                if finished:
                    break
                chunk = reader.read()
                output += chunk
                if stream_output:
                    sys.stdout.write(chunk.decode("utf-8"))
                await asyncio.sleep(1)
            output += reader.read()
        exit_code = process.returncode
        assert exit_code is not None, "Process must have exited"
        return RunResult(exit_code, output)

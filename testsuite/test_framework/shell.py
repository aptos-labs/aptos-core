# A wrapper around shell operations

from __future__ import annotations

import asyncio
import logging
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from typing import Optional, Sequence, Union

from test_framework.logging import log


@dataclass
class RunResult:
    exit_code: int
    output: bytes

    def output_str(self) -> str:
        return self.output.decode("utf-8")

    def unwrap(self) -> bytes:
        if not self.succeeded():
            raise Exception(self.output_str())
        return self.output

    def succeeded(self) -> bool:
        return self.exit_code == 0


class Shell:
    def run(
        self,
        command: Sequence[str],
        stream_output: bool = False,
        timeout_secs: Optional[float] = None,
    ) -> RunResult:
        raise NotImplementedError()

    async def gen_run(
        self, command: Sequence[str], stream_output: bool = False
    ) -> RunResult:
        raise NotImplementedError()


@dataclass
class LocalShell(Shell):
    logger: logging.Logger = log

    def run(
        self,
        command: Sequence[str],
        stream_output: bool = False,
        timeout_secs: Optional[float] = None,
    ) -> RunResult:
        self.logger.debug(f"+ {' '.join(command)}")

        process = subprocess.Popen(
            command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT
        )
        if process.stdout is None:
            raise Exception(f"Could not get stdout for command: {command}")

        output = b""
        start_time = time.time()
        for line in iter(process.stdout.readline, b""):
            if stream_output:
                sys.stdout.buffer.write(line)
            output += line
            if timeout_secs and time.time() - start_time > timeout_secs:
                process.kill()
                raise subprocess.TimeoutExpired(" ".join(command), timeout_secs)

        process.communicate()  # ensure process is done
        return RunResult(process.returncode, output)

    async def gen_run(
        self,
        command: Sequence[str],
        stream_output: bool = False,
        timeout_secs: Optional[float] = None,
    ) -> RunResult:
        self.logger.debug(f"+ {' '.join(command)}")

        process = await asyncio.create_subprocess_exec(
            *command, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.STDOUT
        )

        output = b""
        while True:
            if process.stdout is None:
                break
            chunk = await asyncio.wait_for(
                process.stdout.read(4096), timeout=timeout_secs
            )
            if not chunk:  # process finished and no more data
                break

            output += chunk
            if stream_output:
                sys.stdout.buffer.write(chunk)

        await asyncio.wait_for(process.wait(), timeout=timeout_secs)

        exit_code = process.returncode
        assert exit_code is not None, "Process must have exited"
        return RunResult(exit_code, output)


class FakeShell(Shell):
    def run(self, command: Sequence[str], stream_output: bool = False) -> RunResult:
        return RunResult(0, b"output")

    async def gen_run(
        self, command: Sequence[str], stream_output: bool = False
    ) -> RunResult:
        return RunResult(0, b"async output")


class FakeCommand:
    def __init__(
        self, command: str, result_or_exception: Union[RunResult, Exception]
    ) -> None:
        self.command = command
        self.result_or_exception = result_or_exception

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, FakeCommand):
            return False
        return self.command == other.command

    def __hash__(self) -> int:
        return hash(self.command)

    def __repr__(self) -> str:
        return f"FakeCommand({self.command})"

    def __str__(self) -> str:
        return self.command


class SpyShell(FakeShell):
    logger: logging.Logger = log

    def __init__(
        self,
        expected_command_list: Sequence[FakeCommand],
        strict: bool = False,
    ) -> None:
        self.expected_command_list = expected_command_list
        self.commands = []
        self.strict = strict

    def get_fake_commands(self) -> Sequence[str]:
        """Get the list of commands that are expected to be run"""
        return [fakecommand.command for fakecommand in self.expected_command_list]

    def run(
        self,
        command: Sequence[str],
        stream_output: bool = False,
        timeout_secs: Optional[float] = None,
    ) -> RunResult:
        """Mock a command run by adding it to a list of commands and returning the result"""
        rendered_command = " ".join(command)
        default = (
            Exception(f"Command not mocked: {rendered_command}")
            if self.strict
            else super().run(command)
        )
        # get how many times it's been called before, and use that to index into the expected command list
        # XXX: could be optimized, since it does N^2 scans of the command list
        times_called_before = self.commands.count(rendered_command)
        if rendered_command in self.get_fake_commands():
            try:
                command_index = [
                    i
                    for i, fakecommand in enumerate(self.expected_command_list)
                    if fakecommand.command == rendered_command
                ][times_called_before]
            except IndexError:
                pretty_fake_cmds = "\n".join(
                    [f"$ {c}" for c in self.get_fake_commands()]
                )
                raise Exception(
                    f"Did not find command {times_called_before + 1} times in expected command list: $ {rendered_command}\n\nExpected command list:\n{pretty_fake_cmds}"
                )
            result = self.expected_command_list[command_index].result_or_exception
        else:
            raise Exception(
                f"Did not find command '{rendered_command}' in expected command list: {self.get_fake_commands()}"
            )
        self.commands.append(rendered_command)
        if isinstance(result, Exception):
            raise result
        return result

    async def gen_run(
        self, command: Sequence[str], stream_output: bool = False
    ) -> RunResult:
        return self.run(command, stream_output)

    def assert_commands(self, testcase) -> None:
        """Compare the list of commands that were run to the list of expected commands"""
        testcase.assertEqual(self.get_fake_commands(), self.commands)

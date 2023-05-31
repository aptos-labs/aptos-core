# A wrapper around operating system processes

from __future__ import annotations

import atexit
import os
import psutil
import pwd
from dataclasses import dataclass
from typing import (
    Callable,
    Generator,
)


class Process:
    def name(self) -> str:
        raise NotImplementedError()

    def ppid(self) -> int:
        raise NotImplementedError()


class Processes:
    def processes(self) -> Generator[Process, None, None]:
        raise NotImplementedError()

    def get_pid(self) -> int:
        raise NotImplementedError()

    def atexit(self, callback: Callable[[], None]) -> None:
        raise NotImplementedError()

    def user(self) -> str:
        raise NotImplementedError()


@dataclass
class SystemProcess(Process):
    process: psutil.Process

    def name(self) -> str:
        return self.process.name()

    def ppid(self) -> int:
        return self.process.ppid()


class SystemProcesses(Processes):
    def processes(self) -> Generator[Process, None, None]:
        for process in psutil.process_iter():
            yield SystemProcess(process)

    def get_pid(self) -> int:
        return os.getpid()

    def atexit(self, callback: Callable[[], None]) -> None:
        atexit.register(callback)

    def user(self) -> str:
        return pwd.getpwuid(os.getuid())[0]


@dataclass
class FakeProcess(Process):
    _name: str
    _ppid: int

    def name(self) -> str:
        return self._name

    def ppid(self) -> int:
        return self._ppid


class FakeProcesses(Processes):
    def __init__(self) -> None:
        self.exit_callbacks = []

    def processes(self) -> Generator[Process, None, None]:
        yield FakeProcess("concensus", 1)

    def get_pid(self) -> int:
        return 2

    def spawn(self, target: Callable[[], None]) -> Process:
        return FakeProcess("child", 2)

    def atexit(self, callback: Callable[[], None]) -> None:
        return self.exit_callbacks.append(callback)

    def user(self) -> str:
        return "perry"


class SpyProcesses(FakeProcesses):
    def run_atexit(self) -> None:
        for callback in self.exit_callbacks:
            callback()

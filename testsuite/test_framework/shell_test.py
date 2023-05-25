import unittest
from .shell import SpyShell, FakeCommand, RunResult


class SpyTests(unittest.TestCase):
    def testSpyShell(self) -> None:
        shell = SpyShell(
            [
                FakeCommand(
                    "echo hello",
                    RunResult(0, b""),
                ),
                FakeCommand(
                    "echo hello_banana",
                    RunResult(0, b""),
                ),
            ]
        )
        shell.run(["echo", "hello"])
        shell.run(["echo", "hello_banana"])
        shell.assert_commands(self)

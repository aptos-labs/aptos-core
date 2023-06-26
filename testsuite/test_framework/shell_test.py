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
                FakeCommand(
                    "echo hello_banana",  # test duplicate commands in order too
                    RunResult(1, b""),
                ),
                FakeCommand(
                    "echo hello_banana",
                    RunResult(2, b""),
                ),
                FakeCommand(
                    "echo hello_banana",
                    RunResult(3, b""),
                ),
            ]
        )
        shell.run(["echo", "hello"])
        ret = shell.run(["echo", "hello_banana"]).exit_code
        self.assertEqual(ret, 0)
        ret = shell.run(["echo", "hello_banana"]).exit_code
        self.assertEqual(ret, 1)
        ret = shell.run(["echo", "hello_banana"]).exit_code
        self.assertEqual(ret, 2)
        ret = shell.run(["echo", "hello_banana"]).exit_code
        self.assertEqual(ret, 3)
        shell.assert_commands(self)

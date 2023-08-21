import unittest
from .git import Git
from .shell import FakeCommand, RunResult, Shell, SpyShell


class SpyTests(unittest.TestCase):
    def test_get_repo_from_remote_git(self):
        shell: Shell = SpyShell(
            [
                FakeCommand(
                    "git remote get-url origin",
                    RunResult(0, b"git@github.com:banana-corp/aptos-core.git"),
                )
            ]
        )
        git: Git = Git(shell)
        self.assertEqual(git.get_repo_from_remote("origin"), "banana-corp/aptos-core")

    def test_get_repo_from_remote_http(self):
        shell: Shell = SpyShell(
            [
                FakeCommand(
                    "git remote get-url origin",
                    RunResult(0, b"https://github.com/kiwi-corp/aptos-core.git"),
                )
            ]
        )
        git: Git = Git(shell)
        self.assertEqual(git.get_repo_from_remote("origin"), "kiwi-corp/aptos-core")

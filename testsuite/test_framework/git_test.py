import unittest
from .git import Git
from .shell import FakeCommand, RunResult, Shell, SpyShell
from datetime import datetime, timezone

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
    
    def test_get_tags(self):
        shell: Shell = SpyShell([
            FakeCommand(
                "git tag --sort=-committerdate --points-at HEAD",
                RunResult(0, b"v1.2.3\nv1.2.0\nv1.1.0")
            )
        ])
        git: Git = Git(shell)
        self.assertEqual(git.get_tags(), ["v1.2.3", "v1.2.0", "v1.1.0"])

    def test_get_tags_with_pattern(self):
        shell: Shell = SpyShell([
            FakeCommand(
                "git tag --sort=-committerdate --points-at HEAD --list v1.2.*",
                RunResult(0, b"v1.2.3\nv1.2.0")
            )
        ])
        git: Git = Git(shell)
        self.assertEqual(git.get_tags(pattern="v1.2.*"), ["v1.2.3", "v1.2.0"])

    def test_get_remote_branches_matching_pattern(self):
        shell: Shell = SpyShell([
            FakeCommand(
                "git ls-remote --heads origin aptos-release-v*",
                RunResult(0, b"ref1 refs/heads/aptos-release-v1.0\nref2 refs/heads/aptos-release-v1.1")
            )
        ])
        git: Git = Git(shell)
        branches = git.get_remote_branches_matching_pattern("origin", "aptos-release-v*", r"refs/heads/(aptos-release-v\d+\.\d+)")
        self.assertEqual(branches, ["aptos-release-v1.0", "aptos-release-v1.1"])

    def test_get_commit_hashes(self):
        shell: Shell = SpyShell([
            FakeCommand(
                "git log -n 3 --format=%H main",
                RunResult(0, b"hash1\nhash2\nhash3")
            )
        ])
        git: Git = Git(shell)
        hashes = git.get_commit_hashes("main", max_commits=3)
        self.assertEqual(hashes, ["hash1", "hash2", "hash3"])

    def test_get_branch_creation_time(self):
        shell: Shell = SpyShell([
            FakeCommand(
                "git rev-parse --verify main",
                RunResult(0, b"")
            ),
            FakeCommand(
                "git rev-list --first-parent --max-count=1 main",
                RunResult(0, b"first_commit_hash")
            ),
            FakeCommand(
                "git show -s --format=%ci first_commit_hash",
                RunResult(0, b"2023-04-01 12:00:00 +0000")
            )
        ])
        git: Git = Git(shell)
        creation_time = git.get_branch_creation_time("main")
        expected_time = datetime(2023, 4, 1, 12, 0, 0, tzinfo=timezone.utc)
        self.assertEqual(creation_time, expected_time)

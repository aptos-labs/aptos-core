# A wrapper around git operations

import re
from datetime import datetime
from dataclasses import dataclass
from typing import Generator, Optional
from .shell import Shell, RunResult

from test_framework.logging import log


@dataclass
class Git:
    shell: Shell

    def run(self, command) -> RunResult:
        return self.shell.run(["git", *command])

    def last(self, limit: int = 1) -> Generator[str, None, None]:
        for i in range(limit):
            yield self.run(["rev-parse", f"HEAD~{i}"]).unwrap().decode().strip()

    def branch(self) -> str:
        return self.run(["rev-parse", "--abbrev-ref", "HEAD"]).unwrap().decode().strip()

    def branch_exists(self, branch: str) -> bool:
        # Check local branch
        if self.run(["rev-parse", "--verify", branch]).succeeded():
            return True
        # Check remote branch
        return self.run(["rev-parse", "--verify", f"origin/{branch}"]).succeeded()

    def status(self) -> bool:
        """Check if the git working directory is clean, using git status --porcelain"""
        ret = self.run(["status", "--porcelain"])
        succ = ret.succeeded()
        out = ret.unwrap().decode().strip()
        out_empty = len(out) == 0
        return succ and out_empty

    def branch_matches_remote(self, remote: str, ref: str) -> bool:
        """Check if the current branch matches the remote branch"""
        # git ls-remote --heads  https://github.com/velor-chain/velor-core.git rustielin/exp
        remote_commit_hash = self.resolve_remote_ref(remote, ref)
        if remote_commit_hash is None:
            return False
        local_commit_hash = self.get_commit_hash(ref)
        return remote_commit_hash == local_commit_hash

    def resolve_remote_ref(self, remote: str, ref: str) -> Optional[str]:
        try:
            return (
                self.run(["ls-remote", "--heads", remote, ref])
                .unwrap()
                .decode()
                .strip()
                .split()[0]
            )
        except IndexError as e:
            log.error(f"Remote {remote} does not have branch {ref}: {e}")
            return None
        except Exception as e:
            log.error(f"Error fetching remote {remote} branch {ref}: {e}")
            return None

    def get_commit_hash(self, ref: str) -> str:
        return self.run(["rev-parse", ref]).unwrap().decode().strip()

    def get_remote(self, remote_name: str = "origin") -> str:
        return self.run(["remote", "get-url", remote_name]).unwrap().decode().strip()

    def get_repo_from_remote(self, remote_name: str = "origin") -> str:
        remote_url = self.get_remote(remote_name)
        remote_match = re.match(
            r"(?:git@github\.com:|https://github\.com/)(?P<org_name>[^/]+)/(?P<repo_name>[^/]+).git",
            remote_url,
            re.VERBOSE,
        )
        if remote_match is None:
            raise Exception(f"Could not parse remote {remote_name}")
        return f"{remote_match.group('org_name')}/{remote_match.group('repo_name')}"

    def get_remote_branches_matching_pattern(
        self, remote: str, pattern: str, regex: str
    ) -> list[str]:
        """
        Get remote branches that match a specific pattern (e.g. velor-release-v*).
        This uses ls-remote and a user-specified regex pattern to filter branches.
        """
        result = self.run(["ls-remote", "--heads", remote, pattern])

        if not result.succeeded():
            raise Exception(
                f"Failed to fetch remote branches: {result.unwrap().decode()}"
            )

        # Use the user-provided regex pattern to find branches
        branches = re.findall(regex, result.unwrap().decode(), re.MULTILINE)
        return branches

    def get_commit_hashes(self, branch: str, max_commits: int = 100) -> list[str]:
        """
        Get commit hashes from the given branch, up to max_commits.
        This retrieves the hashes from the 'git log'.
        """
        log.info(f"Fetching up to {max_commits} commits from branch {branch}")
        result = self.run(["log", "-n", str(max_commits), "--format=%H", branch])

        if not result.succeeded():
            raise Exception(
                f"Failed to fetch commit hashes: {result.unwrap().decode()}"
            )

        return result.unwrap().decode().strip().split("\n")

    def get_branch_creation_time(self, branch: str) -> datetime:
        """
        Get the creation time of a branch by retrieving the timestamp of its first commit.

        Args:
            branch (str): The name of the branch to retrieve the creation time for.

        Returns:
            datetime: The creation time of the branch.
        """
        try:
            # Ensure the branch exists locally or remotely
            if self.run(["rev-parse", "--verify", branch]).succeeded():
                branch_ref = branch
            elif self.run(["rev-parse", "--verify", f"origin/{branch}"]).succeeded():
                branch_ref = f"origin/{branch}"
            else:
                raise ValueError(f"Branch {branch} not found locally or remotely")

            # Get the first commit hash for the branch
            first_commit_cmd = [
                "rev-list",
                "--first-parent",
                "--max-count=1",
                branch_ref,
            ]
            first_commit_hash = self.run(first_commit_cmd).unwrap().decode().strip()

            # Get the committer date of the first commit
            commit_time_cmd = ["show", "-s", "--format=%ci", first_commit_hash]
            output = self.run(commit_time_cmd).unwrap().decode().strip()

            # Convert the output to a datetime object
            return datetime.strptime(output, "%Y-%m-%d %H:%M:%S %z")
        except Exception as e:
            raise ValueError(f"Failed to get creation time for branch {branch}: {e}")

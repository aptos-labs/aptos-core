# A wrapper around git operations

import re
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
        return self.run(["rev-parse", "--verify", branch]).succeeded()

    def status(self) -> bool:
        """Check if the git working directory is clean, using git status --porcelain"""
        ret = self.run(["status", "--porcelain"])
        succ = ret.succeeded()
        out = ret.unwrap().decode().strip()
        out_empty = len(out) == 0
        return succ and out_empty

    def branch_matches_remote(self, remote: str, ref: str) -> bool:
        """Check if the current branch matches the remote branch"""
        # git ls-remote --heads  https://github.com/aptos-labs/aptos-core.git rustielin/exp
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

import sys
import re

from test_framework.logging import init_logging, log
from test_framework.git import Git
from test_framework.shell import LocalShell

# GitHub output logic from determinator
from determinator import GithubOutput, write_github_output

# Initialize logging and the shell/git instances
shell = LocalShell()
git = Git(shell)

GH_OUTPUT_KEY = "TARGET_BRANCH"


def get_all_release_branches():
    """Get all velor-release-vX.Y branches"""
    pattern = "velor-release-v*"
    regex = r"refs/heads/(velor-release-v\d+\.\d+)$"
    branches = git.get_remote_branches_matching_pattern("origin", pattern, regex)
    return sorted(branches, key=lambda x: [int(n) for n in x.split("v")[1].split(".")])


def get_all_release_branches_with_times():
    """Get all velor-release-vX.Y branches with their creation times"""
    branches = get_all_release_branches()
    return [(branch, git.get_branch_creation_time(branch)) for branch in branches]


def get_latest_branch_for_previous_major(major):
    """Get the latest velor-release-v(previous_major).Y branch"""
    prev_major = int(major) - 1
    pattern = f"velor-release-v{prev_major}.*"
    regex = rf"refs/heads/(velor-release-v{prev_major}\.\d+)$"

    branches = git.get_remote_branches_matching_pattern("origin", pattern, regex)

    if branches:
        return max(branches, key=lambda x: int(x.split(".")[-1]))
    return None


def determine_target_branch(base_branch):
    """Determine the appropriate target branch based on the base branch"""
    all_release_branches = get_all_release_branches_with_times()
    if not all_release_branches:
        raise ValueError("No release branches found")

    if base_branch == "main":
        # Sort by version numbers for 'main' branch
        sorted_branches = sorted(
            all_release_branches,
            key=lambda x: [int(n) for n in x[0].split("v")[1].split(".")],
            reverse=True,
        )
        return sorted_branches[0][0]

    all_release_branches.sort(
        key=lambda x: x[1], reverse=True
    )  # Sort by creation time, newest first

    # If the base branch is a release branch, find the previous release branch
    match = re.match(r"^velor-release-v(\d+)\.(\d+)", base_branch)
    if match:
        major, minor = match.groups()
        if int(minor) == 0:
            return get_latest_branch_for_previous_major(major)
        else:
            return f"velor-release-v{major}.{int(minor) - 1}"

    # For other personal branches, find the latest release branch earlier than the current branch
    base_branch_time = git.get_branch_creation_time(base_branch)
    for branch, time in all_release_branches:
        if time < base_branch_time:
            return branch

    # If no suitable branch found, return the earliest release branch
    return all_release_branches[-1][0]


def main() -> None:
    if len(sys.argv) != 2:
        log.error("Usage: python determine_target_branch.py <base_branch>")
        sys.exit(1)

    base_branch = sys.argv[1]
    try:
        # Determine the target branch
        target_branch = determine_target_branch(base_branch)
        if target_branch is not None:
            # Write the target branch to GitHub output
            write_github_output(GithubOutput(GH_OUTPUT_KEY, str(target_branch)))
            log.info(
                f"Successfully wrote target branch to GitHub output: {target_branch}"
            )
    except Exception as e:
        log.error(
            f"Error determining target branch and writing to GitHub output: {e}\n"
            "This may be an indication that you're running locally or the environment is not configured correctly."
        )
        sys.exit(1)


if __name__ == "__main__":
    main()

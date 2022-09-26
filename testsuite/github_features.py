from __future__ import annotations

import click
import json
import re
from typing import Dict, Optional

from forge import LocalShell, Git
from determinator import GithubOutput


@click.group()
def main() -> None:
    pass


def parse_tags(commit_message: str) -> Dict[str, str]:
    # Find key-value pairs in the commit message
    # Find flag tags
    tags = {}
    pairs = re.findall(r"^@(\w+):(\w+)$", commit_message, re.MULTILINE)
    flags = re.findall(r"^@(\w+)$", commit_message, re.MULTILINE)
    for key, val in pairs:
        tags[key] = val
    for flag in flags:
        tags[flag] = "true"
    return tags


@main.command("commit-info")
@click.option(
    "--base-ref",
    default="main",
    help="Base branch to compare against",
    show_default=True,
)
@click.option(
    "--head-ref",
    default="HEAD",
    help="Head ref to compare against",
    show_default=True,
)
@click.option(
    "--github-output-key",
    help="Key to use for the output",
)
@click.option(
    "--verbose",
    is_flag=True,
)
def commit_info(
    base_ref: str,
    head_ref: str,
    github_output_key: Optional[str],
    verbose: Optional[bool],
) -> None:
    shell = LocalShell(verbose)
    git = Git(shell)

    commits = git.log((base_ref, head_ref), format="%H")
    commit_tags = {}
    for commit in commits.splitlines():
        # Get the commit info, parse the commit info
        commit_message = git.log(commit, format="%s%n%n%b")
        commit_tags.update(parse_tags(commit_message))

    if github_output_key:
        serialized_commit_data = json.dumps(commit_tags)
        print(GithubOutput(
            github_output_key,
            serialized_commit_data,
        ).format())


if __name__ == "__main__":
    main()
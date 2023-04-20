from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Generic, Optional, Sequence, Tuple, TypeVar, TypedDict

import click
import os


@dataclass
class Verdict:
    verdict: bool
    reason: str
    subverdicts: Sequence[Verdict]

    def format(self, indent=0) -> str:
        return "\n".join(
            [
                (indent * "  ")
                + ("PASS" if self.verdict else "FAIL")
                + f"ED because {self.reason}",
            ]
            + [verdict.format(indent + 1) for verdict in self.subverdicts]
        )


TEvaluationContext = TypeVar("TEvaluationContext")


class Predicate(Generic[TEvaluationContext]):
    def evaluate(self, context: TEvaluationContext) -> Verdict:
        raise NotImplementedError()


@dataclass
class OrPredicate(Predicate[TEvaluationContext]):
    left: Predicate[TEvaluationContext]
    right: Predicate[TEvaluationContext]

    def evaluate(self, context: TEvaluationContext) -> Verdict:
        leftval = self.left.evaluate(context)
        rightval = self.right.evaluate(context)
        return Verdict(
            leftval.verdict or rightval.verdict,
            f"({leftval.reason}) or ({rightval.reason})",
            [leftval, rightval],
        )


class ChangedFilesContext(TypedDict):
    changed_files: Sequence[str]


@dataclass
class ChangedFilesPredicate(Predicate[ChangedFilesContext]):
    patterns: Sequence[str]

    def evaluate(self, context: ChangedFilesContext) -> Verdict:
        matches = []
        for path in context["changed_files"]:
            for pattern in self.patterns:
                if re.match(pattern, path):
                    matches.append(path)
        return Verdict(
            bool(matches),
            f"Matched files: {matches}",
            [],
        )


@click.group()
def main() -> None:
    pass


@dataclass
class GithubOutput:
    """
    Represents a Github Output string
    It should be written separately to $GITHUB_OUTPUT in the action: https://github.blog/changelog/2022-10-11-github-actions-deprecating-save-state-and-set-output-commands/
    """

    key: str
    value: str

    def format(self) -> str:
        return f"{self.key}={self.value}"


def write_github_output(output: GithubOutput) -> None:
    try:
        with open(os.environ["GITHUB_OUTPUT"], "a") as f:
            f.write(f"{output.format()}\n")
    except KeyError:
        raise Exception(
            "GITHUB_OUTPUT not set, not writing output. This may be an error with the action setup."
        )


@main.command()
@click.option("--pattern", "patterns", multiple=True)
@click.option("--github-output-key")
@click.argument("changed_files", nargs=-1)
def changed_files(
    patterns: Tuple[str],
    github_output_key: Optional[str],
    changed_files: Tuple[str],
) -> None:
    context: ChangedFilesContext = {
        "changed_files": changed_files,
    }
    predicate = ChangedFilesPredicate(patterns)
    verdict = predicate.evaluate(context)

    print(verdict.format())

    if github_output_key:
        output = GithubOutput(github_output_key, "true" if verdict.verdict else "false")
        write_github_output(output)
        print(output.format())
    else:
        if not verdict.verdict:
            raise SystemExit(1)


if __name__ == "__main__":
    main()

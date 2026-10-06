"""Bind a completed pull_request_target workflow run to exactly one pull request."""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Any, Iterable

from ci_actions.github import ActionError, GitHubClient, paginate
from ci_actions.validation import REPOSITORY, SHA40, is_safe_positive_int

JOB_CONCLUSIONS = frozenset({
    "action_required", "cancelled", "failure", "neutral", "skipped",
    "stale", "startup_failure", "success", "timed_out",
})
PAGE_SIZE = 100
MAX_PAGES = 10
_CONTROL = re.compile(r"[\x00-\x1f\x7f]")


@dataclass(frozen=True)
class Producer:
    key: str            # see pr-ci-report-producers.json and forge_report.PRODUCER
    workflow_path: str  # ".github/workflows/<file>.yaml"
    workflow_name: str  # must equal both workflow.name and run.name


@dataclass(frozen=True)
class RunOrigin:
    producer: Producer
    run_id: int
    head_repository: str
    head_branch: str
    head_sha: str


@dataclass(frozen=True)
class RunBinding:
    producer: Producer
    run_id: int
    pr_number: int
    head_sha: str
    head_repository: str
    head_branch: str


def is_head_sha(value: Any) -> bool:
    return isinstance(value, str) and SHA40.fullmatch(value) is not None


def _full_name(value: Any) -> Any:
    return value.get("full_name") if isinstance(value, dict) else None


def check_origin(run: Any, workflow: Any, expected_repository: str, run_id: int,
                 producers: Iterable[Producer]) -> RunOrigin:
    """Validate run and workflow metadata already fetched from the API."""
    if not is_safe_positive_int(run_id):
        raise ActionError("expected run ID must be a positive safe integer")
    if not isinstance(run, dict) or run.get("id") != run_id or type(run.get("id")) is not int:
        raise ActionError("originating run ID does not match the requested run ID")
    if _full_name(run.get("repository")) != expected_repository:
        raise ActionError("originating run repository does not match the trusted repository")
    if run.get("event") != "pull_request_target":
        raise ActionError("originating run event must be pull_request_target")
    if run.get("status") != "completed":
        raise ActionError("originating run must be completed")
    if not is_safe_positive_int(run.get("workflow_id")):
        raise ActionError("originating run workflow ID is invalid")
    if (not isinstance(workflow, dict) or not is_safe_positive_int(workflow.get("id"))
            or workflow.get("id") != run["workflow_id"]):
        raise ActionError("workflow ID does not match the originating run")
    matching = [producer for producer in producers if producer.workflow_path == workflow.get("path")]
    if len(matching) != 1:
        raise ActionError("workflow path is not an approved producer path")
    producer = matching[0]
    if workflow.get("name") != producer.workflow_name or run.get("name") != producer.workflow_name:
        raise ActionError("workflow display name does not match its stable path")
    head_repository = _full_name(run.get("head_repository"))
    if not isinstance(head_repository, str) or REPOSITORY.fullmatch(head_repository) is None:
        raise ActionError("originating run head repository is invalid")
    head_branch = run.get("head_branch")
    if not isinstance(head_branch, str) or not 0 < len(head_branch) <= 255 or _CONTROL.search(head_branch):
        raise ActionError("originating run head branch is invalid")
    if not is_head_sha(run.get("head_sha")):
        raise ActionError("originating run head SHA is invalid")
    return RunOrigin(producer, run_id, head_repository, head_branch, run["head_sha"])


def inspect_run(client: GitHubClient, run_id: int, producers: Iterable[Producer]) -> RunOrigin:
    run = client.get_json(f"/actions/runs/{run_id}")
    workflow_id = run.get("workflow_id") if isinstance(run, dict) else None
    if not is_safe_positive_int(workflow_id):
        raise ActionError("originating run workflow ID is invalid")
    workflow = client.get_json(f"/actions/workflows/{workflow_id}")
    return check_origin(run, workflow, client.repository, run_id, producers)


def _exact_match(pull_request: Any, number: int, origin: RunOrigin, expected_repository: str) -> bool:
    if (not isinstance(pull_request, dict) or not is_safe_positive_int(pull_request.get("number"))
            or pull_request.get("number") != number):
        raise ActionError("GitHub API returned a mismatched pull request")
    if pull_request.get("state") not in ("open", "closed"):
        raise ActionError("GitHub API returned an invalid pull request state")
    base = pull_request.get("base")
    if not isinstance(base, dict):
        raise ActionError("pull request base metadata is missing")
    if _full_name(base.get("repo")) != expected_repository:
        raise ActionError("pull request base repository does not match the trusted repository")
    head = pull_request.get("head")
    if not isinstance(head, dict) or not is_head_sha(head.get("sha")):
        raise ActionError("pull request head metadata is invalid")
    if not isinstance(head.get("repo"), dict):
        return False
    return (
        _full_name(head["repo"]) == origin.head_repository
        and head.get("ref") == origin.head_branch
        and head["sha"] == origin.head_sha
    )


def list_pull_request_candidates(client: GitHubClient, origin: RunOrigin) -> list[int]:
    head_owner = origin.head_repository.split("/", 1)[0]
    candidates = paginate(
        client, "/pulls", items_key=None, max_pages=MAX_PAGES, per_page=PAGE_SIZE,
        params={"state": "all", "head": f"{head_owner}:{origin.head_branch}"},
    )
    numbers: list[int] = []
    for candidate in candidates:
        number = candidate.get("number") if isinstance(candidate, dict) else None
        if not is_safe_positive_int(number):
            raise ActionError("GitHub API returned a malformed pull request candidate")
        numbers.append(number)
    if len(set(numbers)) != len(numbers):
        raise ActionError("GitHub API repeated a pull request candidate")
    return numbers


def bind_pull_request(client: GitHubClient, origin: RunOrigin) -> RunBinding:
    matches = [
        number for number in list_pull_request_candidates(client, origin)
        if _exact_match(client.get_json(f"/pulls/{number}"), number, origin, client.repository)
    ]
    if len(matches) != 1:
        raise ActionError("expected exactly one pull request matching the originating run head")
    # Re-read the matched pull request so a head change during candidate lookup fails closed.
    if not _exact_match(client.get_json(f"/pulls/{matches[0]}"), matches[0], origin, client.repository):
        raise ActionError("stale workflow run: the pull request head has changed")
    return RunBinding(origin.producer, origin.run_id, matches[0], origin.head_sha,
                      origin.head_repository, origin.head_branch)


def bind_run_to_pull_request(client: GitHubClient, run_id: int, producers: Iterable[Producer]) -> RunBinding:
    return bind_pull_request(client, inspect_run(client, run_id, producers))


def list_run_jobs(client: GitHubClient, run_id: int) -> list[dict]:
    if not is_safe_positive_int(run_id):
        raise ActionError("workflow run ID is invalid")
    return paginate(
        client, f"/actions/runs/{run_id}/jobs", items_key="jobs", total_count_key="total_count",
        max_pages=MAX_PAGES, per_page=PAGE_SIZE, params={"filter": "latest"},
    )

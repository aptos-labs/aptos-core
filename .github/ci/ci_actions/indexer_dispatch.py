"""indexer-processor-dispatch: send an exact-source repository_dispatch and wait
for the one downstream run that echoes a fresh correlation UUID."""

from __future__ import annotations

import os
import re
import time
import urllib.parse
import uuid
from dataclasses import dataclass
from typing import Any, Callable, Mapping

from ci_actions.github import MAX_SAFE_INTEGER, ActionError, GitHubClient, parse_repository, write_outputs

_SHA = re.compile(r"[0-9a-f]{40}")
_SAFE_TEXT = re.compile(r"[A-Za-z0-9._:/-]+")
_UUID = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}", re.IGNORECASE)
_INTEGER = re.compile(r"0|[1-9][0-9]*")
RUN_STATUSES = frozenset({"queued", "in_progress", "completed", "waiting", "requested", "pending"})
USER_AGENT = "aptos-indexer-processor-dispatch"


@dataclass(frozen=True)
class DispatchInputs:
    repository: str
    event_type: str
    approved_sha: str
    pr_number: int
    source_run_id: int
    source_run_attempt: int
    branch: str
    discovery_attempts: int
    completion_attempts: int
    poll_interval_seconds: int


@dataclass(frozen=True)
class DispatchResult:
    correlation: str
    run_id: int
    run_url: str


def _single_line(value: Any, name: str) -> str:
    if not isinstance(value, str) or not value or value.strip() != value or "\r" in value or "\n" in value:
        raise ActionError(f"{name} must be a non-empty single-line string")
    return value


def _safe_text(value: Any, name: str) -> str:
    if _SAFE_TEXT.fullmatch(_single_line(value, name)) is None:
        raise ActionError(f"{name} contains unsupported characters")
    return value


def _integer(value: Any, name: str, *, allow_zero: bool = False, maximum: int = MAX_SAFE_INTEGER) -> int:
    if not isinstance(value, str) or _INTEGER.fullmatch(value) is None:
        raise ActionError(f"{name} must be an integer")
    result = int(value)
    if result > maximum or (result == 0 and not allow_zero):
        raise ActionError(f"{name} is outside the allowed range")
    return result


def parse_inputs(env: Mapping[str, str]) -> DispatchInputs:
    approved_sha = _single_line(env.get("INPUT_APPROVED_SHA"), "approved_sha")
    if _SHA.fullmatch(approved_sha) is None:
        raise ActionError("approved_sha must be a lowercase full 40-character commit SHA")
    repository = _safe_text(env.get("INPUT_DOWNSTREAM_REPOSITORY"), "downstream_repository")
    parse_repository(repository, "downstream_repository")
    return DispatchInputs(
        repository=repository,
        event_type=_safe_text(env.get("INPUT_EVENT_TYPE"), "event_type"),
        approved_sha=approved_sha,
        pr_number=_integer(env.get("INPUT_PR_NUMBER"), "pr_number", allow_zero=True),
        source_run_id=_integer(env.get("INPUT_SOURCE_RUN_ID"), "source_run_id"),
        source_run_attempt=_integer(env.get("INPUT_SOURCE_RUN_ATTEMPT"), "source_run_attempt"),
        branch=_safe_text(env.get("INPUT_DOWNSTREAM_BRANCH"), "downstream_branch"),
        discovery_attempts=_integer(env.get("INPUT_DISCOVERY_ATTEMPTS"), "discovery_attempts", maximum=100),
        completion_attempts=_integer(env.get("INPUT_COMPLETION_ATTEMPTS"), "completion_attempts", maximum=300),
        poll_interval_seconds=_integer(env.get("INPUT_POLL_INTERVAL_SECONDS"), "poll_interval_seconds",
                                       allow_zero=True, maximum=3600),
    )


def expected_run_name(inputs: DispatchInputs, correlation: str) -> str:
    return (
        f"indexer-pr-ci:{inputs.approved_sha}:pr:{inputs.pr_number}:source:{inputs.source_run_id}:"
        f"{inputs.source_run_attempt}:correlation:{correlation}"
    )


def _validate_run(run: Any, inputs: DispatchInputs, run_name: str) -> dict[str, Any]:
    if not isinstance(run, dict):
        raise ActionError("Downstream workflow run is malformed")
    if type(run.get("id")) is not int or not 0 < run["id"] <= MAX_SAFE_INTEGER:
        raise ActionError("Downstream workflow run ID is malformed")
    if (
        run.get("event") != "repository_dispatch"
        or run.get("head_branch") != inputs.branch
        or run.get("display_title") != run_name
    ):
        raise ActionError("Downstream workflow run lost exact source correlation")
    if not isinstance(run.get("status"), str) or run["status"] not in RUN_STATUSES:
        raise ActionError("Downstream workflow run status is malformed")
    if run["status"] == "completed" and not isinstance(run.get("conclusion"), str):
        raise ActionError("Completed downstream workflow run has no conclusion")
    if "html_url" in run:
        url = _single_line(run["html_url"], "downstream run URL")
        try:
            parts = urllib.parse.urlsplit(url)
            parts.port  # Raises ValueError for malformed or out-of-range ports.
        except ValueError as error:
            raise ActionError("Downstream workflow run URL is malformed") from error
        if (
            not url.startswith("https://") or not parts.hostname
            or parts.username is not None or parts.password is not None
            or any(ord(character) <= 0x20 or ord(character) == 0x7F for character in url)
        ):
            raise ActionError("Downstream workflow run URL is malformed")
    return run


def _dispatch(client: GitHubClient, inputs: DispatchInputs, correlation: str, run_name: str) -> None:
    client.post_json("/dispatches", {
        "event_type": inputs.event_type,
        "client_payload": {
            "commit_hash": inputs.approved_sha,
            "pr_number": inputs.pr_number,
            "source_run_id": inputs.source_run_id,
            "source_run_attempt": inputs.source_run_attempt,
            "correlation": correlation,
            "expected_run_name": run_name,
        },
    })


def _discover_run(
    client: GitHubClient, inputs: DispatchInputs, run_name: str, sleep: Callable[[float], None]
) -> dict[str, Any]:
    for _ in range(inputs.discovery_attempts):
        # Newest runs come first; the correlation run is new, so one page is enough.
        response = client.get_json(
            "/actions/runs", {"event": "repository_dispatch", "branch": inputs.branch, "per_page": 100}
        )
        runs = response.get("workflow_runs") if isinstance(response, dict) else None
        if not isinstance(runs, list):
            raise ActionError("Downstream workflow run list is malformed")
        matches = [run for run in runs if isinstance(run, dict) and run.get("display_title") == run_name]
        if len(matches) > 1:
            raise ActionError("Multiple downstream runs echoed the exact correlation")
        if matches:
            return _validate_run(matches[0], inputs, run_name)
        sleep(inputs.poll_interval_seconds)
    raise ActionError("Downstream repository did not echo the exact correlation")


def _await_completion(
    client: GitHubClient, inputs: DispatchInputs, run_id: int, run_name: str, sleep: Callable[[float], None]
) -> dict[str, Any]:
    for _ in range(inputs.completion_attempts):
        run = _validate_run(client.get_json(f"/actions/runs/{run_id}"), inputs, run_name)
        if run["id"] != run_id:
            raise ActionError("Downstream workflow run ID changed after discovery")
        if run["status"] == "completed":
            if run["conclusion"] != "success":
                raise ActionError(f"Downstream workflow run completed with conclusion {run['conclusion']}")
            return run
        sleep(inputs.poll_interval_seconds)
    raise ActionError("Downstream workflow run did not complete within the polling limit")


def dispatch_and_poll(
    client: GitHubClient,
    inputs: DispatchInputs,
    *,
    new_correlation: Callable[[], str] = lambda: str(uuid.uuid4()),
    sleep: Callable[[float], None] = time.sleep,
) -> DispatchResult:
    correlation = new_correlation()
    if not isinstance(correlation, str) or _UUID.fullmatch(correlation) is None:
        raise ActionError("The correlation generator did not return a UUID")
    run_name = expected_run_name(inputs, correlation)
    _dispatch(client, inputs, correlation, run_name)
    selected = _discover_run(client, inputs, run_name, sleep)
    run = _await_completion(client, inputs, selected["id"], run_name, sleep)
    return DispatchResult(correlation, run["id"], run.get("html_url", ""))


def main() -> None:
    inputs = parse_inputs(os.environ)
    client = GitHubClient.from_env(user_agent=USER_AGENT, repository=inputs.repository, token_env="INPUT_TOKEN")
    result = dispatch_and_poll(client, inputs)
    write_outputs({
        "correlation": result.correlation,
        "downstream_run_id": str(result.run_id),
        "downstream_run_url": result.run_url,
    })
    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_path:
        with open(summary_path, "a", encoding="utf-8") as summary:
            summary.write(
                f"Downstream correlation: `{result.correlation}`\n\n"
                f"Downstream run: {result.run_url or result.run_id}\n"
            )

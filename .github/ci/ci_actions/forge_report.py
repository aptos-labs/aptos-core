"""docker-forge-pr-report: turn a completed Docker run's Forge markers into PR comments.

Marker job names, comment headers and titles come from the Docker manifest.
The run-to-PR binding is the shared `run_binding` module.
"""

from __future__ import annotations

import json
import urllib.parse
from typing import Any, Iterable

from ci_actions.docker_plan import Marker, load_manifest
from ci_actions.github import (
    ActionError,
    GitHubClient,
    Transport,
    parse_positive_int,
    require_env,
    write_outputs,
)
from ci_actions.run_binding import (
    JOB_CONCLUSIONS,
    Producer,
    RunBinding,
    bind_run_to_pull_request,
    list_run_jobs,
)

PRODUCER = Producer(
    key="docker-build-test",
    workflow_path=".github/workflows/docker-build-test.yaml",
    workflow_name="Build+Test Docker Images",
)


def report_comments(markers: Iterable[Marker], jobs: list[Any], binding: RunBinding) -> list[dict]:
    """Return one comment record per marker job that ran.

    The marker matrix holds only active markers, so an absent marker means the
    workload did not run. A duplicate marker name fails closed.
    """
    comments = []
    for marker in markers:
        matching = [job for job in jobs if isinstance(job, dict) and job.get("name") == marker.id]
        if len(matching) > 1:
            raise ActionError(f"expected at most one {marker.id} marker job")
        if not matching:
            continue
        job = matching[0]
        job_id = job.get("id")
        if (
            type(job_id) is not int
            or job_id <= 0
            or job.get("run_id") != binding.run_id
            or job.get("head_sha") != binding.head_sha
            or job.get("status") != "completed"
        ):
            raise ActionError(f"{marker.id} marker metadata is invalid")
        conclusion = job.get("conclusion")
        if not isinstance(conclusion, str) or conclusion not in JOB_CONCLUSIONS:
            raise ActionError(f"{marker.id} marker conclusion is invalid")
        if conclusion == "skipped":
            continue
        comments.append({
            "key": marker.id,
            "header": marker.comment_header,
            "title": marker.title,
            "result": conclusion,
        })
    return comments


def run_url(server_url: str, repository: str, run_id: int) -> str:
    parts = urllib.parse.urlsplit(server_url)
    if (
        parts.scheme != "https"
        or not parts.hostname
        or parts.username
        or parts.password
        or parts.query
        or parts.fragment
    ):
        raise ActionError("server URL must be HTTPS without credentials, query, or fragment")
    return f"{server_url.rstrip('/')}/{repository}/actions/runs/{run_id}"


def main(transport: Transport | None = None) -> None:
    run_id = parse_positive_int(require_env("INPUT_RUN_ID"), "run_id")
    client = GitHubClient.from_env(user_agent="aptos-docker-forge-pr-report", transport=transport)
    binding = bind_run_to_pull_request(client, run_id, [PRODUCER])
    comments = report_comments(load_manifest().markers(), list_run_jobs(client, run_id), binding)
    write_outputs({
        "pr_number": str(binding.pr_number),
        "head_sha": binding.head_sha,
        "run_url": run_url(require_env("GITHUB_SERVER_URL"), client.repository, run_id),
        "report_matrix": json.dumps({"include": comments}, separators=(",", ":")),
    })

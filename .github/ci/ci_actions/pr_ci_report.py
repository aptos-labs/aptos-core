"""pr-ci-report: bind a completed report producer run to its pull request,
download its report artifact, validate it and render the sticky comment."""

from __future__ import annotations

import io
import stat
import tempfile
import zipfile
import zlib
from pathlib import Path
from typing import Any, Optional

from ci_actions.github import (
    ActionError, GitHubClient, paginate, parse_positive_int, parse_utc_timestamp, require_env, write_outputs,
)
from ci_actions.report_producers import load_report_producers
from ci_actions.report_schema import MAX_REPORT_BYTES, SCHEMA, missing_report, parse_and_validate_report, render_report
from ci_actions.run_binding import JOB_CONCLUSIONS, RunBinding, bind_pull_request, inspect_run, list_run_jobs
from ci_actions.validation import is_safe_positive_int

REPORT_PRODUCERS = load_report_producers()
PRODUCERS = tuple(producer.binding() for producer in REPORT_PRODUCERS.values())
EXECUTED_STEP_CONCLUSIONS = frozenset({"cancelled", "failure", "neutral", "success", "timed_out"})
REPORT_ARTIFACT_NAME = SCHEMA
REPORT_MEMBER_NAME = f"{SCHEMA}.json"
MAX_ARCHIVE_BYTES = 2 * 1024 * 1024
MAX_ARTIFACT_PAGES = 100
USER_AGENT = "aptos-pr-ci-report-action"


def benchmark_job_disposition(jobs: list[Any], producer: str, run_id: int) -> str:
    """Return "skip" when the approval gate skipped the benchmark job, "report"
    when its benchmark step ran. Anything else fails closed."""
    report_producer = REPORT_PRODUCERS.get(producer)
    if report_producer is None:
        raise ActionError("benchmark step metadata is invalid")
    matching = [job for job in jobs if isinstance(job, dict) and job.get("name") == report_producer.job]
    if len(matching) != 1:
        raise ActionError("expected exactly one benchmark job for the producer")
    job = matching[0]
    if (not is_safe_positive_int(job.get("id")) or not is_safe_positive_int(job.get("run_id"))
            or job.get("run_id") != run_id or job.get("status") != "completed"):
        raise ActionError("benchmark job metadata is invalid")
    if not isinstance(job.get("conclusion"), str) or job.get("conclusion") not in JOB_CONCLUSIONS:
        raise ActionError("benchmark job conclusion is invalid")
    if job["conclusion"] == "skipped":
        return "skip"
    steps = job.get("steps")
    if not isinstance(steps, list):
        raise ActionError("benchmark step metadata is invalid")
    matching_steps = [
        step for step in steps if isinstance(step, dict) and step.get("name") == report_producer.benchmark_step
    ]
    if len(matching_steps) != 1:
        raise ActionError("expected exactly one producer benchmark step")
    step = matching_steps[0]
    started_at = parse_utc_timestamp(step.get("started_at"))
    completed_at = parse_utc_timestamp(step.get("completed_at"))
    if (
        not is_safe_positive_int(step.get("number"))
        or step.get("status") != "completed"
        or not isinstance(step.get("conclusion"), str)
        or step.get("conclusion") not in EXECUTED_STEP_CONCLUSIONS
        or started_at is None
        or completed_at is None
        or completed_at < started_at
    ):
        raise ActionError("benchmark step did not have valid completed execution metadata")
    return "report"


def select_report_artifact(artifacts: list[Any], run_id: int) -> Optional[dict[str, Any]]:
    for artifact in artifacts:
        if not isinstance(artifact, dict) or not isinstance(artifact.get("name"), str) or not artifact["name"]:
            raise ActionError("artifact list contains a malformed artifact")
    matching = [artifact for artifact in artifacts if artifact["name"] == REPORT_ARTIFACT_NAME]
    if not matching:
        return None
    if len(matching) != 1:
        raise ActionError(f"expected exactly one {REPORT_ARTIFACT_NAME} artifact")
    artifact = matching[0]
    if not is_safe_positive_int(artifact.get("id")):
        raise ActionError("report artifact ID is invalid")
    if artifact.get("expired") is not False:
        raise ActionError("report artifact is expired")
    if not is_safe_positive_int(artifact.get("size_in_bytes")) or artifact["size_in_bytes"] > MAX_ARCHIVE_BYTES:
        raise ActionError("report artifact API archive size is invalid or too large")
    workflow_run = artifact.get("workflow_run")
    if (not isinstance(workflow_run, dict) or not is_safe_positive_int(workflow_run.get("id"))
            or workflow_run.get("id") != run_id):
        raise ActionError("report artifact is not bound to the originating run")
    return artifact


def read_report_archive(archive: bytes) -> bytes:
    """Return the single report member without extracting anything to disk."""
    try:
        with zipfile.ZipFile(io.BytesIO(archive)) as zipped:
            members = zipped.infolist()
            if len(members) != 1:
                raise ActionError("artifact must contain exactly one file")
            member = members[0]
            if member.filename != REPORT_MEMBER_NAME:
                raise ActionError("artifact member name is invalid")
            if member.create_system != 3:
                raise ActionError("artifact member must carry Unix file metadata")
            if not stat.S_ISREG(member.external_attr >> 16):
                raise ActionError("artifact member must be a regular file")
            if member.file_size > MAX_REPORT_BYTES:
                raise ActionError("report exceeds one MiB")
            with zipped.open(member) as report:
                payload = report.read(MAX_REPORT_BYTES + 1)
    except (OSError, zipfile.BadZipFile, RuntimeError, EOFError, zlib.error) as error:
        raise ActionError(f"malformed artifact ZIP: {error}") from error
    if len(payload) > MAX_REPORT_BYTES or len(payload) != member.file_size:
        raise ActionError("report actual size exceeds one MiB or does not match metadata")
    return payload


def _binding_fields(binding: RunBinding) -> dict[str, Any]:
    return {"producer": binding.producer.key, "run_id": binding.run_id,
            "pr_number": binding.pr_number, "head_sha": binding.head_sha}


def build_comment(client: GitHubClient, run_id: int) -> Optional[dict[str, Any]]:
    """Return the validated report, or None when the benchmark job was skipped."""
    origin = inspect_run(client, run_id, PRODUCERS)
    if benchmark_job_disposition(list_run_jobs(client, run_id), origin.producer.key, run_id) == "skip":
        return None
    binding = _binding_fields(bind_pull_request(client, origin))
    artifacts = paginate(client, f"/actions/runs/{run_id}/artifacts", items_key="artifacts",
                         total_count_key="total_count", max_pages=MAX_ARTIFACT_PAGES)
    artifact = select_report_artifact(artifacts, run_id)
    if artifact is None:
        return missing_report(binding)
    # The archive endpoint redirects to blob storage; the token is not forwarded.
    archive = client.get_bytes(f"/actions/artifacts/{artifact['id']}/zip",
                               max_bytes=MAX_ARCHIVE_BYTES, follow_redirect=True)
    return parse_and_validate_report(read_report_archive(archive), binding)


def main() -> None:
    run_id = parse_positive_int(require_env("INPUT_RUN_ID"), "run_id")
    report = build_comment(GitHubClient.from_env(user_agent=USER_AGENT), run_id)
    if report is None:
        write_outputs({"should_comment": "false"})
        return
    comment_path = Path(tempfile.mkdtemp(prefix="pr-ci-report-", dir=require_env("RUNNER_TEMP"))) / "comment.md"
    comment_path.write_text(render_report(report), encoding="utf-8")
    write_outputs({
        "should_comment": "true",
        "comment_path": str(comment_path),
        "comment_header": report["producer"],
        "pr_number": str(report["pr_number"]),
    })

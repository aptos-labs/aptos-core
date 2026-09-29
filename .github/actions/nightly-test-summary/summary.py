#!/usr/bin/env python3
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

"""Render a nightly result for GitHub and Slack; never sends notifications itself."""

import html
import json
import os
import subprocess


HISTORY_NIGHTS = 7
GREEN, YELLOW, RED, GREY = "\U0001f7e9", "\U0001f7e8", "\U0001f7e5", "\u2b1c"
CANCELLED = "\u274c"
FAILED = ("failure", "timed_out", "cancelled")


def square(conclusion, retried=False, run_cancelled=False):
    if conclusion == "success":
        # Passing only after the retry mixes green and red.
        return YELLOW if retried else GREEN
    # A job that times out is also `cancelled`; only a cancelled run crosses out.
    if conclusion == "cancelled" and run_cancelled:
        return CANCELLED
    if conclusion in (*FAILED, "startup_failure"):
        return RED
    return GREY


def failed_names(jobs):
    return {job["name"] for job in jobs or [] if job.get("conclusion") in FAILED}


def recent_nights(previous):
    """The completed nights before this run, oldest first."""
    runs = [run for run in previous or [] if run.get("status") == "completed"]
    return sorted(runs, key=lambda run: run["createdAt"])[-(HISTORY_NIGHTS - 1) :]


def night_square(run):
    conclusion = run.get("conclusion")
    return square(conclusion, run.get("attempt", 1) > 1, conclusion == "cancelled")


def build_history(previous, run_url, failed, attempt=1, cancelled=False):
    """One linked square per night, oldest first, ending with this run."""
    cells = [f"<{run['url']}|{night_square(run)}>" for run in recent_nights(previous)]
    if cancelled:
        tonight = CANCELLED
    else:
        tonight = square("failure" if failed else "success", attempt > 1)
    cells.append(f"<{run_url}|{tonight}>")
    return f"Last {len(cells)} nights: " + "".join(cells)


def job_rows(jobs, first_attempt_jobs, previous, cancelled=False):
    """One row per failed job: its square on each night of the bar, then its failed steps."""
    nights = recent_nights(previous)
    rows = []
    for job in jobs or []:
        if job.get("conclusion") not in FAILED:
            continue
        cells = []
        for run in nights:
            past = {past["name"]: past.get("conclusion") for past in run.get("jobs") or []}
            retried = job["name"] in failed_names(run.get("first_attempt_jobs"))
            run_cancelled = run.get("conclusion") == "cancelled"
            cells.append(square(past.get(job["name"]), retried, run_cancelled))
        tonight = square(job["conclusion"], run_cancelled=cancelled)
        cells.append(f"<{job['html_url']}|{tonight}>" if job.get("html_url") else tonight)
        steps = [
            step["name"] for step in job.get("steps", []) if step.get("conclusion") in FAILED
        ]
        text = job["name"] + (" \u2014 " + ", ".join(steps) if steps else "")
        rows.append("".join(cells) + "  " + html.escape(text, quote=False))
    return rows


def build_summary(
    needs,
    branch,
    sha,
    run_url,
    jobs=None,
    previous_sha=None,
    previous_runs=None,
    attempt=1,
    first_attempt_jobs=None,
    cancelled=False,
):
    incomplete = sorted(
        f"{name}: {job['result']}"
        for name, job in needs.items()
        if job["result"] != "success"
    )
    if not needs:
        incomplete = ["No required suite results received"]
    if cancelled:
        verdict = "CANCELLED"
    elif incomplete:
        verdict = "FAILED / INCOMPLETE"
    elif attempt > 1:
        verdict = "passed after retry"
    else:
        verdict = "passed"
    lines = [
        f"Nightly full-suite {verdict}",
        build_history(previous_runs, run_url, bool(incomplete), attempt, cancelled),
        f"Branch: {html.escape(branch, quote=False)}; commit: {sha}",
        f"<{run_url}|Run, logs, and artifacts>",
    ]
    if incomplete:
        rows = job_rows(jobs, first_attempt_jobs, previous_runs, cancelled)
        skipped = sorted(name for name, job in needs.items() if job["result"] == "skipped")
        if rows:
            lines.extend(rows)
            if skipped:
                lines.append("Skipped suites: " + ", ".join(skipped))
        else:
            # Without job details, name the incomplete suites themselves.
            lines.append("Required suites: " + "; ".join(incomplete))
    # Jobs still running, such as this one, are neither failed nor passed.
    passed = {job["name"] for job in jobs or [] if job.get("conclusion") == "success"}
    recovered = sorted(failed_names(first_attempt_jobs) & passed)
    if recovered:
        lines.append(html.escape("Passed on retry: " + ", ".join(recovered), quote=False))
    if previous_sha:
        repo_url = run_url.split("/actions/runs/")[0]
        lines.append(
            f"<{repo_url}/compare/{previous_sha}...{sha}|Changes since last successful nightly>"
        )
    # Slack message limits: full details remain available through the run link.
    return bool(incomplete), {"text": "\n".join(lines)[:12000]}


def gh_json(args):
    try:
        return json.loads(subprocess.check_output(["gh", *args], text=True, timeout=30))
    except (subprocess.SubprocessError, OSError, ValueError):
        # Optional enrichment must never prevent the primary failure alert.
        return None


def main():
    needs = json.loads(os.environ["NIGHTLY_NEEDS"])
    run_id = os.environ["GITHUB_RUN_ID"]
    run_url = f"{os.environ['GITHUB_SERVER_URL']}/{os.environ['GITHUB_REPOSITORY']}/actions/runs/{run_id}"
    attempt = int(os.environ["GITHUB_RUN_ATTEMPT"])
    repo = os.environ["GITHUB_REPOSITORY"]

    def run_jobs(run, number=None):
        """Jobs of one attempt, or the latest execution of every job."""
        runs = f"repos/{repo}/actions/runs/{run}"
        path = f"attempts/{number}/jobs?" if number else "jobs?filter=latest&"
        return (gh_json(["api", f"{runs}/{path}per_page=100"]) or {}).get("jobs")

    jobs = run_jobs(run_id)
    previous = gh_json(
        [
            "run",
            "list",
            "--workflow",
            "nightly-full-suite.yaml",
            "--branch",
            os.environ["GITHUB_REF_NAME"],
            "--status",
            "success",
            "--limit",
            "1",
            "--json",
            "headSha",
        ]
    )
    history = gh_json(
        [
            "run",
            "list",
            "--workflow",
            "nightly-full-suite.yaml",
            "--branch",
            os.environ["GITHUB_REF_NAME"],
            "--limit",
            str(HISTORY_NIGHTS),
            "--json",
            "databaseId,status,conclusion,attempt,url,createdAt",
        ]
    )
    nights = recent_nights(
        [run for run in history or [] if str(run.get("databaseId")) != run_id]
    )
    for run in nights:
        run["jobs"] = run_jobs(run["databaseId"])
        if run.get("attempt", 1) > 1:
            run["first_attempt_jobs"] = run_jobs(run["databaseId"], 1)
    failed, payload = build_summary(
        needs,
        os.environ["GITHUB_REF_NAME"],
        os.environ["GITHUB_SHA"],
        run_url,
        jobs,
        previous[0]["headSha"] if previous else None,
        nights,
        attempt,
        run_jobs(run_id, 1) if attempt > 1 else None,
        os.environ.get("RUN_CANCELLED") == "true",
    )
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"failed={str(failed).lower()}\n")
        output.write("payload=" + json.dumps(payload) + "\n")
    with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as summary:
        summary.write(payload["text"] + "\n")
        summary.write(
            "\nCoverage inventory: devtools/aptos-cargo-cli/README.md#nightly-backstop\n"
        )


if __name__ == "__main__":
    main()

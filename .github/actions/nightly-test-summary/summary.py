#!/usr/bin/env python3
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

"""Render a nightly result for GitHub and Slack; never sends notifications itself."""

import html
import json
import os
import subprocess


HISTORY_NIGHTS = 7
GREEN, RED, GREY = "\U0001f7e9", "\U0001f7e5", "\u2b1c"


def square(conclusion):
    if conclusion == "success":
        return GREEN
    if conclusion in ("failure", "timed_out", "cancelled", "startup_failure"):
        return RED
    return GREY


def build_history(previous, run_url, failed):
    """One linked square per night, oldest first, ending with this run."""
    runs = [run for run in previous or [] if run.get("status") == "completed"]
    runs = sorted(runs, key=lambda run: run["createdAt"])[-(HISTORY_NIGHTS - 1) :]
    cells = [f"<{run['url']}|{square(run.get('conclusion'))}>" for run in runs]
    cells.append(f"<{run_url}|{RED if failed else GREEN}>")
    return f"Last {len(cells)} nights: " + "".join(cells)


def build_summary(
    needs, branch, sha, run_url, jobs=None, previous_sha=None, previous_runs=None
):
    incomplete = sorted(
        f"{name}: {job['result']}"
        for name, job in needs.items()
        if job["result"] != "success"
    )
    if not needs:
        incomplete = ["No required suite results received"]
    lines = [
        f"Nightly full-suite {'FAILED / INCOMPLETE' if incomplete else 'passed'}",
        build_history(previous_runs, run_url, bool(incomplete)),
        f"Branch: {html.escape(branch, quote=False)}; commit: {sha}",
        f"<{run_url}|Run, logs, and artifacts>",
    ]
    if incomplete:
        lines.append("Required suites: " + "; ".join(incomplete))
        for job in jobs or []:
            if job.get("conclusion") not in (
                "failure",
                "timed_out",
                "cancelled",
            ):
                continue
            failed_steps = [
                step["name"]
                for step in job.get("steps", [])
                if step.get("conclusion") in ("failure", "timed_out", "cancelled")
            ]
            lines.append(
                html.escape(job["name"] + ": " + ", ".join(failed_steps), quote=False)
            )
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
    jobs = gh_json(
        [
            "run",
            "view",
            run_id,
            "--attempt",
            os.environ["GITHUB_RUN_ATTEMPT"],
            "--json",
            "jobs",
        ]
    )
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
            "databaseId,status,conclusion,url,createdAt",
        ]
    )
    failed, payload = build_summary(
        needs,
        os.environ["GITHUB_REF_NAME"],
        os.environ["GITHUB_SHA"],
        run_url,
        (jobs or {}).get("jobs"),
        previous[0]["headSha"] if previous else None,
        [run for run in history or [] if str(run.get("databaseId")) != run_id],
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

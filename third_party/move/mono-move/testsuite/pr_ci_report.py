#!/usr/bin/env python3

"""Deterministic writer for the pr-ci-report-v1 artifact.

The trusted reporter (.github/ci/ci_actions/report_schema.py) owns the schema
and validates every report. This module only serializes, so the producer
runtime never imports code from .github."""

import json
from pathlib import Path


def build_report(*, producer, run_id, pr_number, head_sha, status, metrics):
    return {
        "schema": "pr-ci-report-v1",
        "producer": producer,
        "run_id": run_id,
        "pr_number": pr_number,
        "head_sha": head_sha,
        "status": status,
        "metrics": metrics,
    }


def write_report(path, report):
    # allow_nan=False rejects NaN and infinity with ValueError.
    payload = json.dumps(report, allow_nan=False, separators=(",", ":"), sort_keys=True)
    Path(path).write_text(payload + "\n", encoding="utf-8")

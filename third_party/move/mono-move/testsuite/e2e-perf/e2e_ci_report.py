#!/usr/bin/env python3

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""Build and write the end-to-end performance PR CI report."""

import os

from pr_ci_report import build_report as build_report_envelope
from pr_ci_report import write_report as write_pr_ci_report


def build_pr_ci_report(
    results,
    failures,
    *,
    pr_number,
    head_sha,
    run_id,
    status,
    summarize,
    verdict_metrics,
):
    """Build the bounded report consumed by the trusted reporter."""
    metrics = []
    for result in results:
        v1_tps, _ = summarize(result.v1_runs, "execution")
        mono_tps, _ = summarize(result.mono_runs, "execution")
        metrics.append(
            {
                "name": result.workload.name,
                "verdict": result.verdict,
                "v1_tps": v1_tps,
                "mono_tps": mono_tps,
                "execution_speedup": result.speedup["execution"],
                "max_execution_spread": max(
                    result.spread[metric] for metric in verdict_metrics
                ),
            }
        )
    for name, _reason in failures:
        metrics.append(
            {
                "name": name,
                "verdict": "failed",
                "v1_tps": None,
                "mono_tps": None,
                "execution_speedup": None,
                "max_execution_spread": None,
            }
        )
    return build_report_envelope(
        producer="mono-move-e2e-perf",
        run_id=run_id,
        pr_number=pr_number,
        head_sha=head_sha,
        status=status,
        metrics=metrics,
    )


def write_ci_report_if_requested(
    results, failures, status, *, report_path, summarize, verdict_metrics
):
    """Write the CI report when a path is configured by the caller."""
    if not report_path:
        return
    write_pr_ci_report(
        report_path,
        build_pr_ci_report(
            results,
            failures,
            pr_number=int(os.environ["PR_CI_PR_NUMBER"]),
            head_sha=os.environ["PR_CI_HEAD_SHA"],
            run_id=int(os.environ["PR_CI_RUN_ID"]),
            status=status,
            summarize=summarize,
            verdict_metrics=verdict_metrics,
        ),
    )

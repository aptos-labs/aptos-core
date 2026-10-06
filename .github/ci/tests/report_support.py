"""Test-side pr-ci-report-v1 contract and fixed reports shared by the report suites."""

from tests.helpers import SHA

# Test-side protocol specification; these constants are independent of the validator.
CONTRACTS = {
    "mono-move-e2e-perf": {
        "verdicts": ("ok", "improvement", "regression", "noisy", "uncalibrated", "failed", "self-compare"),
        "numbers": {"v1_tps": (0, 1e12), "mono_tps": (0, 1e12),
                    "execution_speedup": (0, 1e6), "max_execution_spread": (0, 1e6)},
    },
    "mono-move-micro-bench": {
        "verdicts": ("regression", "improvement", "ok", "notable", "noise", "new", "absent",
                     "workload changed", "incomplete"),
        "numbers": {"mean_percent": (-1e6, 1e6), "ci_low_percent": (-1e6, 1e6),
                    "ci_high_percent": (-1e6, 1e6), "base_median_ns": (0, 1e18), "pr_median_ns": (0, 1e18)},
    },
}


def trusted_binding(report):
    return {key: report[key] for key in ("producer", "run_id", "pr_number", "head_sha")}


def e2e_report(**overrides):
    return {
        "schema": "pr-ci-report-v1", "producer": "mono-move-e2e-perf", "run_id": 1234, "pr_number": 99,
        "head_sha": SHA, "status": "passed",
        "metrics": [{"name": "apt-fa-transfer", "verdict": "ok", "v1_tps": 100.0, "mono_tps": 125.0,
                     "execution_speedup": 1.25, "max_execution_spread": 0.02}],
        **overrides,
    }


def micro_report(**overrides):
    return {
        "schema": "pr-ci-report-v1", "producer": "mono-move-micro-bench", "run_id": 1234, "pr_number": 99,
        "head_sha": SHA, "status": "failed",
        "metrics": [{"name": "fib/mono", "verdict": "regression", "mean_percent": 4.2, "ci_low_percent": 3.1,
                     "ci_high_percent": 5.3, "base_median_ns": 1000.0, "pr_median_ns": 1042.0}],
        **overrides,
    }

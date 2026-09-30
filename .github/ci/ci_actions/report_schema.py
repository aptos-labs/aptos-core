"""pr-ci-report-v1: the only report shape the trusted reporter accepts and renders.
Producers under third_party/move/mono-move/testsuite write it; their tests
validate their output with this module."""

from __future__ import annotations

import json
import math
import re
from typing import Any, Mapping

SCHEMA = "pr-ci-report-v1"
MAX_REPORT_BYTES = 1024 * 1024
MAX_ROWS = 100
MAX_STRING_LENGTH = 128
MAX_SAFE_INTEGER = 2**53 - 1
BINDING_KEYS = ("producer", "run_id", "pr_number", "head_sha")
ROOT_KEYS = frozenset({*BINDING_KEYS, "schema", "status", "metrics"})
STATUSES = frozenset({"passed", "failed"})
PRODUCER_ROWS = {
    "mono-move-e2e-perf": {
        "verdicts": frozenset({"ok", "improvement", "regression", "noisy", "uncalibrated", "failed", "self-compare"}),
        "numbers": {
            "v1_tps": (0, 1e12),
            "mono_tps": (0, 1e12),
            "execution_speedup": (0, 1e6),
            "max_execution_spread": (0, 1e6),
        },
    },
    "mono-move-micro-bench": {
        "verdicts": frozenset({
            "regression", "improvement", "ok", "notable", "noise", "new", "absent",
            "workload changed", "incomplete",
        }),
        "numbers": {
            "mean_percent": (-1e6, 1e6),
            "ci_low_percent": (-1e6, 1e6),
            "ci_high_percent": (-1e6, 1e6),
            "base_median_ns": (0, 1e18),
            "pr_median_ns": (0, 1e18),
        },
    },
}
SAFE_TEXT = re.compile(r"[A-Za-z0-9][A-Za-z0-9._/+ :\-]*")
HEAD_SHA = re.compile(r"[0-9a-f]{40}")


def _reject_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate key: {key}")
        result[key] = value
    return result


def _reject_constant(value: str) -> Any:
    raise ValueError(f"JSON number must be finite: {value}")


def _require_exact_keys(value: Any, expected: frozenset[str], context: str) -> None:
    if not isinstance(value, dict):
        raise ValueError(f"{context} must be an object")
    unknown = set(value) - expected
    missing = expected - set(value)
    if unknown:
        raise ValueError(f"{context} has unknown key: {sorted(unknown)[0]}")
    if missing:
        raise ValueError(f"{context} is missing key: {sorted(missing)[0]}")


def _positive_integer(value: Any, context: str) -> None:
    if type(value) is not int or not 0 < value <= MAX_SAFE_INTEGER:
        raise ValueError(f"{context} must be a positive safe integer")


def _bounded_nullable_number(value: Any, context: str, lower: float, upper: float) -> None:
    if value is None:
        return
    if type(value) not in (int, float) or (type(value) is float and not math.isfinite(value)):
        raise ValueError(f"{context} must be a finite JSON number or null")
    if not lower <= value <= upper:
        raise ValueError(f"{context} is outside its bounded range")


def _safe_text(value: Any, context: str) -> None:
    if not isinstance(value, str) or not value or len(value) > MAX_STRING_LENGTH:
        raise ValueError(f"{context} must be safe plain text at most 128 characters long")
    if SAFE_TEXT.fullmatch(value) is None:
        raise ValueError(f"{context} must be safe plain text")


def validate_report(report: Any, binding: Mapping[str, Any]) -> dict[str, Any]:
    """Validate `report` against the schema and the trusted run binding
    ({"producer", "run_id", "pr_number", "head_sha"})."""
    _require_exact_keys(report, ROOT_KEYS, "report")
    if report["schema"] != SCHEMA:
        raise ValueError("invalid report schema")
    if not isinstance(report["producer"], str) or report["producer"] not in PRODUCER_ROWS:
        raise ValueError("invalid report producer")
    _positive_integer(report["run_id"], "run_id")
    _positive_integer(report["pr_number"], "pr_number")
    if not isinstance(report["head_sha"], str) or HEAD_SHA.fullmatch(report["head_sha"]) is None:
        raise ValueError("head_sha must be an exact lowercase 40-hex SHA")
    if not isinstance(report["status"], str) or report["status"] not in STATUSES:
        raise ValueError("invalid report status")
    for key in BINDING_KEYS:
        if report[key] != binding.get(key):
            raise ValueError(f"report {key} does not match trusted workflow run metadata")
    rows = report["metrics"]
    if not isinstance(rows, list):
        raise ValueError("metrics must be an array")
    if len(rows) > MAX_ROWS:
        raise ValueError("metrics must contain at most 100 rows")
    contract = PRODUCER_ROWS[report["producer"]]
    row_keys = frozenset({"name", "verdict", *contract["numbers"]})
    for index, row in enumerate(rows):
        _require_exact_keys(row, row_keys, f"metric row {index}")
        _safe_text(row["name"], f"metric row {index} name")
        if not isinstance(row["verdict"], str) or row["verdict"] not in contract["verdicts"]:
            raise ValueError(f"metric row {index} has an invalid verdict")
        for key, (lower, upper) in contract["numbers"].items():
            _bounded_nullable_number(row[key], f"metric row {index} {key}", lower, upper)
    return report


def parse_and_validate_report(payload: bytes, binding: Mapping[str, Any]) -> dict[str, Any]:
    if len(payload) > MAX_REPORT_BYTES:
        raise ValueError("report exceeds one MiB")
    try:
        report = json.loads(payload.decode("utf-8"), object_pairs_hook=_reject_duplicates,
                            parse_constant=_reject_constant)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError(f"report contains malformed JSON: {error}") from error
    return validate_report(report, binding)


def missing_report(binding: Mapping[str, Any]) -> dict[str, Any]:
    """The only allowed fallback when the producer uploaded no report."""
    report = {key: binding.get(key) for key in BINDING_KEYS}
    return validate_report({**report, "schema": SCHEMA, "status": "failed", "metrics": []}, binding)


def _number(value: Any, suffix: str = "") -> str:
    return "n/a" if value is None else f"{value:.2f}{suffix}"


def _render_e2e_perf(validated: str, metrics: list[Any]) -> list[str]:
    lines = [
        "### MonoMove E2E performance", "", validated, "",
        "| Workload | Verdict | V1 exec TPS | MonoMove exec TPS | Exec speedup | Max range |",
        "| --- | --- | ---: | ---: | ---: | ---: |",
    ]
    for row in metrics:
        spread = row["max_execution_spread"]
        lines.append(
            f"| `{row['name']}` | {row['verdict']} | {_number(row['v1_tps'])} | "
            f"{_number(row['mono_tps'])} | {_number(row['execution_speedup'], 'x')} | "
            f"{_number(None if spread is None else spread * 100, '%')} |"
        )
    return lines


def _render_micro_bench(validated: str, metrics: list[Any]) -> list[str]:
    lines = [
        "### MonoMove micro-benchmark", "", validated, "",
        "| Benchmark | Verdict | Mean change | 95% CI | Main median | PR median |",
        "| --- | --- | ---: | :---: | ---: | ---: |",
    ]
    for row in metrics:
        interval = (
            "n/a" if row["ci_low_percent"] is None or row["ci_high_percent"] is None
            else f"[{_number(row['ci_low_percent'], '%')}, {_number(row['ci_high_percent'], '%')}]"
        )
        lines.append(
            f"| `{row['name']}` | {row['verdict']} | {_number(row['mean_percent'], '%')} | "
            f"{interval} | {_number(row['base_median_ns'], 'ns')} | {_number(row['pr_median_ns'], 'ns')} |"
        )
    return lines


RENDERERS = {"mono-move-e2e-perf": _render_e2e_perf, "mono-move-micro-bench": _render_micro_bench}


def render_report(report: Mapping[str, Any]) -> str:
    summary = "passed" if report["status"] == "passed" else "failed"
    validated = (
        f"Validated result: **{summary}**. PR #{report['pr_number']} at `{report['head_sha']}` "
        f"(originating run {report['run_id']})."
    )
    lines = RENDERERS[report["producer"]](validated, report["metrics"])
    if not report["metrics"]:
        lines.extend(["", "No metric rows were produced. See the originating run logs."])
    return "\n".join(lines) + "\n"

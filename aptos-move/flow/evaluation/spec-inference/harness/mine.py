"""Machine-labelled analysis of archived controller and Claude transcripts.

The miner turns a round's raw telemetry into per-run counts that explain where
model turns went: which tool calls were made, which verifier failures occurred
and of what kind, how long the model deliberated after each failure, and how
many turns were spent on activities that better feedback could remove.

Labels here are mechanical and auditable. They are the input to a failure
taxonomy, not the taxonomy itself: a category is only adopted after the labelled
transcripts have been read.
"""

from __future__ import annotations

import argparse
import json
import os
import statistics
from collections import Counter
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Iterator

from .artifacts import write_json
from .config import RunSpec


# Diagnostic wording, taken from the prover, translator, and compiler
# themselves. Classification reads only diagnostic headlines so that a word
# quoted from a source snippet cannot decide the label.
FAILURE_KINDS: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("solver_timeout", ("out of resources/timeout",)),
    # A process killed at its wall-clock limit names no condition at all, which
    # is a different repair problem from one condition exceeding its budget.
    ("prover_process_timeout", ("exceeded hard timeout",)),
    ("postcondition", ("post-condition does not hold",)),
    ("abort_not_covered", ("abort not covered",)),
    ("abort_never_happens", ("function does not abort under this condition",)),
    # The prover already separates the two loop obligations, and they want
    # different repairs: a base-case failure means the invariant is not
    # established on entry, an induction-case failure means the body does
    # not preserve it. Collapsing them loses the repair signal.
    ("loop_invariant_base", ("base case of the loop invariant",)),
    ("loop_invariant_induction", ("induction case of the loop invariant",)),
    ("loop_invariant", ("invariant does not hold", "loop invariant")),
    ("global_invariant", ("global memory invariant does not hold",)),
    ("frame", ("caller does not have permission", "modifies clause")),
    ("translator_defect", ("boogie exited with compilation errors", "more than one declaration")),
    (
        "unsupported_construct",
        (
            "unsupported specification construct",
            "current restriction",
            "must range over",
        ),
    ),
    (
        "spec_context_error",
        (
            "is not valid in this context",
            "not allowed in this context",
            "applied to expression which does not depend on state",
        ),
    ),
    # The agent asked a tool for a path the session does not have.
    ("tool_usage_error", ("MCP error", "Unable to find package manifest")),
    ("syntax_error", ("unexpected token", "expected identifier", "invalid character")),
    ("declaration_error", (
        "invalid struct declaration",
        "invalid function declaration",
        "duplicate declaration, item, or annotation",
        "must match function declaration",
    )),
    ("name_resolution", ("unbound", "no function named", "undeclared", "not declared")),
    (
        "type_error",
        (
            "missing required abilities",
            "a reference is expected",
            "but found a value of type",
            "expected a value of type",
            "is not a",
            "incompatible",
            "mismatched",
        ),
    ),
    ("verify_on_broken_package", ("package has compilation errors",)),
    ("tool_infrastructure", ("sent no response or progress", "tool timeout")),
)

HEADLINE_PREFIXES = ("error:", "bug:", "warning:", "verification failed")

EDIT_TOOLS = frozenset({"Edit", "Write", "NotebookEdit"})
VERIFY_TOOLS = frozenset(
    {
        "mcp__move-flow__move_package_verify",
        "mcp__move-flow__move_spec_check",
    }
)
INFER_TOOLS = frozenset({"mcp__move-flow__move_package_wp"})
PLAN_TOOLS = frozenset({"TaskCreate", "TaskUpdate", "TaskList", "TaskGet"})
SEARCH_TOOLS = frozenset({"Glob", "Grep", "Read"})


@dataclass
class ToolCall:
    sequence: int
    utc_ms: int
    name: str
    identifier: str
    arguments: dict[str, Any]
    result: str = ""
    failed: bool = False


@dataclass
class RunAnalysis:
    run_id: str
    task_id: str
    arm: str
    replicate: int
    terminal_status: str
    attempts: int
    model_turns: int
    controller_turns: int
    api_seconds: float
    wall_seconds: float
    output_tokens: int
    input_tokens: int
    cache_read_tokens: int
    cost_usd: float
    tool_calls: int
    tool_call_counts: dict[str, int]
    verifier_calls: int
    verifier_failures: int
    compiler_failures: int
    failure_kinds: dict[str, int]
    out_of_workspace_searches: int
    whole_file_rewrites: int
    targeted_edits: int
    reverted_edit_pairs: int
    repair_iterations: int
    post_failure_seconds: float
    longest_gap_seconds: float
    gap_after_failure_seconds: list[float] = field(default_factory=list)


def analyze_round(runs_dir: Path, schedule_dir: Path | None = None) -> dict[str, Any]:
    # A round can be stopped while runs are in flight. Such a run has a
    # manifest but no judge result, and is reported rather than dropped
    # silently, so a partial round never reads as a complete one.
    directories = sorted(_run_directories(runs_dir))
    complete = [path for path in directories if (path / "judge.json").is_file()]
    incomplete = [path.name for path in directories if path not in complete]
    # A cell the round aborted before starting has no directory at all, so
    # only the schedule can say it is missing; without one the count of
    # cells that were expected is unknown and the report says so.
    missing: list[str] | None = None
    if schedule_dir is not None:
        expected = {
            RunSpec.load(path).run_id for path in sorted((schedule_dir / "runs").glob("*.json"))
        }
        present = {path.name for path in directories}
        missing = sorted(expected - present)
    analyses = [analyze_run(path) for path in complete]
    return {
        "schema_version": 1,
        "runs": len(analyses),
        "incomplete_runs": incomplete,
        "missing_runs": missing,
        "per_run": [asdict(analysis) for analysis in analyses],
        "by_arm": _aggregate(analyses, lambda item: item.arm),
        "by_task": _aggregate(analyses, lambda item: item.task_id),
        "failure_kinds": dict(
            sum((Counter(item.failure_kinds) for item in analyses), Counter())
        ),
    }


def analyze_run(run_dir: Path) -> RunAnalysis:
    run = json.loads((run_dir / "run.json").read_text(encoding="utf-8"))
    judge = json.loads((run_dir / "judge.json").read_text(encoding="utf-8"))
    controller = list(_read_jsonl(run_dir / "controller-events.jsonl"))
    calls = list(_tool_calls(run_dir / "claude-events.jsonl"))

    totals, api_ms, model_turns, cost = _session_totals(controller)

    verifier = [call for call in calls if call.name in VERIFY_TOOLS]
    failures = [call for call in verifier if call.failed]
    failure_kinds = Counter()
    for call in failures:
        failure_kinds[_failure_kind(call.result)] += 1
    compiler_failures = [
        call
        for call in calls
        if call.name.endswith("move_package_status") and call.failed
    ]
    for call in compiler_failures:
        failure_kinds[_failure_kind(call.result)] += 1

    gaps = _gaps(calls)
    after_failure = [seconds for seconds, failed in gaps if failed]
    return RunAnalysis(
        run_id=run["run_id"],
        task_id=run["task_id"],
        arm=run["arm"],
        replicate=int(run.get("replicate", 1)),
        terminal_status=judge.get("terminal_status", "unknown"),
        attempts=int(judge.get("attempts", 1)),
        model_turns=model_turns,
        controller_turns=sum(1 for e in controller if e.get("event") == "prompt"),
        api_seconds=round(api_ms / 1000, 3),
        wall_seconds=round(int(judge.get("controller_wall_ms", 0)) / 1000, 3),
        output_tokens=totals["output_tokens"],
        input_tokens=totals["input_tokens"],
        cache_read_tokens=totals["cache_read_input_tokens"],
        cost_usd=round(cost, 6),
        tool_calls=len(calls),
        tool_call_counts=dict(sorted(Counter(call.name for call in calls).items())),
        verifier_calls=len(verifier),
        verifier_failures=len(failures),
        compiler_failures=len(compiler_failures),
        failure_kinds=dict(sorted(failure_kinds.items())),
        out_of_workspace_searches=_out_of_workspace_searches(calls, run_dir),
        whole_file_rewrites=sum(1 for call in calls if call.name == "Write"),
        targeted_edits=sum(1 for call in calls if call.name == "Edit"),
        reverted_edit_pairs=_reverted_edit_pairs(calls),
        repair_iterations=_repair_iterations(calls),
        post_failure_seconds=round(sum(after_failure), 3),
        longest_gap_seconds=round(max((seconds for seconds, _ in gaps), default=0.0), 3),
        gap_after_failure_seconds=[round(value, 3) for value in after_failure],
    )


def _session_totals(controller: list[dict[str, Any]]) -> tuple[Counter, int, int, float]:
    """Total a run's usage, honouring which telemetry fields accumulate.

    A turn's `usage` and `duration_ms` describe that turn alone, while
    `model_usage`, `total_cost_usd`, and `duration_api_ms` are session totals
    that already include every earlier turn. Summing the latter counts the same
    inference several times over, and the error grows with the number of
    controller turns. An infrastructure retry starts a fresh session, so its
    totals restart and the per-session values are added.
    """
    totals: Counter = Counter()
    model_turns = 0
    session_api_ms = 0
    session_cost = 0.0
    completed_api_ms = 0
    completed_cost = 0.0
    for event in controller:
        if event.get("event") == "infrastructure_retry":
            completed_api_ms += session_api_ms
            completed_cost += session_cost
            session_api_ms = 0
            session_cost = 0.0
            continue
        if event.get("event") != "agent_result":
            continue
        result = event.get("result") or {}
        model_turns += int(result.get("num_turns") or 0)
        session_api_ms = int(result.get("duration_api_ms") or 0)
        session_cost = sum(
            float(model.get("costUSD") or 0.0)
            for model in (result.get("model_usage") or {}).values()
        )
        usage = result.get("usage") or {}
        for field_name in (
            "input_tokens",
            "output_tokens",
            "cache_read_input_tokens",
            "cache_creation_input_tokens",
        ):
            totals[field_name] += int(usage.get(field_name) or 0)
    return totals, completed_api_ms + session_api_ms, model_turns, completed_cost + session_cost


def _run_directories(runs_dir: Path) -> Iterator[Path]:
    for candidate in sorted(runs_dir.iterdir()):
        if not candidate.is_dir():
            continue
        # A dispatched round nests the artifact under its own run id.
        nested = candidate / candidate.name
        directory = nested if (nested / "run.json").is_file() else candidate
        if (directory / "run.json").is_file():
            yield directory


def _read_jsonl(path: Path) -> Iterator[dict[str, Any]]:
    if not path.is_file():
        return
    for line in path.read_text(encoding="utf-8").splitlines():
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            yield value


def _tool_calls(path: Path) -> Iterator[ToolCall]:
    calls: dict[str, ToolCall] = {}
    ordered: list[ToolCall] = []
    for event in _read_jsonl(path):
        message = event.get("message")
        if not isinstance(message, dict):
            continue
        kind = message.get("type")
        if kind == "AssistantMessage":
            for block in message.get("content") or []:
                if isinstance(block, dict) and {"id", "name", "input"} <= set(block):
                    call = ToolCall(
                        sequence=int(event.get("sequence", 0)),
                        utc_ms=int(event.get("utc_ms", 0)),
                        name=str(block["name"]),
                        identifier=str(block["id"]),
                        arguments=block["input"] if isinstance(block["input"], dict) else {},
                    )
                    calls[call.identifier] = call
                    ordered.append(call)
        elif kind == "UserMessage":
            content = message.get("content")
            if not isinstance(content, list):
                continue
            for block in content:
                if not isinstance(block, dict) or "tool_use_id" not in block:
                    continue
                call = calls.get(str(block["tool_use_id"]))
                if call is None:
                    continue
                call.result = _result_text(block.get("content"))
                call.failed = bool(block.get("is_error")) or _looks_failed(call.result)
    yield from ordered


def _result_text(content: Any) -> str:
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        parts = []
        for block in content:
            if isinstance(block, dict) and isinstance(block.get("text"), str):
                parts.append(block["text"])
            elif isinstance(block, str):
                parts.append(block)
        return "\n".join(parts)
    return ""


def _looks_failed(result: str) -> bool:
    lowered = result.lower()
    return (
        "verification failed" in lowered
        or "candidate_rejected" in lowered
        or lowered.startswith("error:")
        or "\nerror:" in lowered
        or "<tool_use_error>" in lowered
    )


def _failure_kind(result: str) -> str:
    headlines = _headlines(result)
    for kind, needles in FAILURE_KINDS:
        if any(needle in headlines for needle in needles):
            return kind
    return "unclassified"


def _headlines(result: str) -> str:
    """Diagnostic headline text, excluding quoted source and location frames."""
    lines = []
    for line in result.splitlines():
        stripped = line.strip()
        if any(stripped.startswith(prefix) for prefix in HEADLINE_PREFIXES):
            lines.append(stripped)
    return "\n".join(lines) if lines else result


def _out_of_workspace_searches(calls: list[ToolCall], run_dir: Path) -> int:
    """Count searches aimed outside the task workspace.

    These are attempts to find documentation or examples the session does not
    contain, and are the signature of missing reference material rather than of
    a hard proof.
    """
    workspace = os.path.normpath(str((run_dir / "workspace").resolve()))
    count = 0
    for call in calls:
        if call.name not in SEARCH_TOOLS:
            continue
        target = call.arguments.get("path") or call.arguments.get("file_path")
        if not isinstance(target, str) or not target:
            continue
        # Lexically normalised, and a path-component prefix: `workspace/../x`
        # is outside whatever its string starts with. The path need not exist
        # any more, so it is not resolved.
        normalized = os.path.normpath(target)
        if normalized != workspace and not normalized.startswith(workspace + os.sep):
            count += 1
    return count


def _reverted_edit_pairs(calls: list[ToolCall]) -> int:
    """Count edits later undone by their exact inverse.

    A deliberate mutation followed by its inverse is the agent testing its own
    contract for exactness, which an authoritative acceptance check makes
    unnecessary.
    """
    pending: list[tuple[str, str, str]] = []
    reverted = 0
    for call in calls:
        if call.name != "Edit":
            continue
        path = str(call.arguments.get("file_path", ""))
        old = str(call.arguments.get("old_string", ""))
        new = str(call.arguments.get("new_string", ""))
        inverse = (path, new, old)
        if inverse in pending:
            pending.remove(inverse)
            reverted += 1
        else:
            pending.append((path, old, new))
    return reverted


def _repair_iterations(calls: list[ToolCall]) -> int:
    """Count edits made while the last verifier answer was a failure."""
    failing = False
    repairs = 0
    for call in calls:
        if call.name in VERIFY_TOOLS or call.name.endswith("move_package_status"):
            failing = call.failed
        elif call.name in EDIT_TOOLS and failing:
            repairs += 1
    return repairs


def _gaps(calls: list[ToolCall]) -> list[tuple[float, bool]]:
    """Seconds between consecutive tool calls, tagged by the preceding outcome."""
    gaps: list[tuple[float, bool]] = []
    for previous, current in zip(calls, calls[1:]):
        if current.utc_ms and previous.utc_ms:
            gaps.append(((current.utc_ms - previous.utc_ms) / 1000, previous.failed))
    return gaps


def _aggregate(analyses: list[RunAnalysis], key) -> dict[str, Any]:
    grouped: dict[str, list[RunAnalysis]] = {}
    for analysis in analyses:
        grouped.setdefault(key(analysis), []).append(analysis)
    summary = {}
    for name, group in sorted(grouped.items()):
        summary[name] = {
            "runs": len(group),
            "operational_success": sum(
                1 for item in group if item.terminal_status == "operational_success"
            ),
            "model_turns": _distribution([item.model_turns for item in group]),
            "api_seconds": _distribution([item.api_seconds for item in group]),
            "output_tokens": _distribution([item.output_tokens for item in group]),
            "cache_read_tokens": _distribution([item.cache_read_tokens for item in group]),
            "cost_usd": round(sum(item.cost_usd for item in group), 6),
            "verifier_failures": sum(item.verifier_failures for item in group),
            "out_of_workspace_searches": sum(item.out_of_workspace_searches for item in group),
            "reverted_edit_pairs": sum(item.reverted_edit_pairs for item in group),
            "whole_file_rewrites": sum(item.whole_file_rewrites for item in group),
        }
    return summary


def _distribution(values: list[float]) -> dict[str, float]:
    if not values:
        return {"total": 0, "median": 0, "maximum": 0}
    return {
        "total": round(sum(values), 3),
        "median": round(statistics.median(values), 3),
        "maximum": round(max(values), 3),
    }


def render_markdown(report: dict[str, Any]) -> str:
    lines = [
        "| run | arm | status | turns | API s | out tok | cache-read | verify fail | probes | reverts | rewrites |",
        "|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for item in report["per_run"]:
        lines.append(
            "| {run_id} | {arm} | {terminal_status} | {model_turns} | {api_seconds:.0f} | "
            "{output_tokens} | {cache_read_tokens} | {verifier_failures} | "
            "{out_of_workspace_searches} | {reverted_edit_pairs} | {whole_file_rewrites} |".format(**item)
        )
    lines.append("")
    lines.append("| arm | runs | success | turns (total) | API s | out tok | cache-read | cost USD |")
    lines.append("|---|---:|---:|---:|---:|---:|---:|---:|")
    for arm, summary in report["by_arm"].items():
        lines.append(
            f"| {arm} | {summary['runs']} | {summary['operational_success']} | "
            f"{summary['model_turns']['total']:.0f} | {summary['api_seconds']['total']:.0f} | "
            f"{summary['output_tokens']['total']:.0f} | {summary['cache_read_tokens']['total']:.0f} | "
            f"{summary['cost_usd']:.2f} |"
        )
    if report["failure_kinds"]:
        lines.append("")
        lines.append("| failure kind | count |")
        lines.append("|---|---:|")
        for kind, count in sorted(report["failure_kinds"].items(), key=lambda item: -item[1]):
            lines.append(f"| {kind} | {count} |")
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs-dir", type=Path, required=True)
    parser.add_argument(
        "--schedule-dir",
        type=Path,
        help="the round's schedule, so cells that never produced an artifact are reported",
    )
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--markdown", type=Path)
    args = parser.parse_args()
    report = analyze_round(
        args.runs_dir.resolve(),
        args.schedule_dir.resolve() if args.schedule_dir else None,
    )
    write_json(args.output, report)
    if args.markdown:
        args.markdown.parent.mkdir(parents=True, exist_ok=True)
        args.markdown.write_text(render_markdown(report), encoding="utf-8")
    print(json.dumps({"runs": report["runs"]}))


if __name__ == "__main__":
    main()

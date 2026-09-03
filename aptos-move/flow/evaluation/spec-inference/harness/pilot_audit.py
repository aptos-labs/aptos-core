"""Audit Phase 4 artifacts without inspecting treatment outcomes by arm."""

from __future__ import annotations

import argparse
import json
import math
import statistics
from pathlib import Path
from typing import Any

from .artifacts import load_object, sha256_file, tree_hash, write_json
from .config import ARM_TO_TACTIC, ExperimentConfig, RunSpec
from .pilot import load_round_shape


REQUIRED_FILES = (
    "run.json",
    "judge.json",
    "controller-events.jsonl",
    "claude-events.jsonl",
    "flow-events.jsonl",
    "stdout.log",
    "stderr.log",
    "workspace.diff",
)
TERMINAL_STATUSES = {
    "operational_success",
    "repeated_forbidden_weakening",
    "wall_budget_exhausted",
    "output_token_budget_exhausted",
    "controller_turn_budget_exhausted",
    "invalid_infrastructure_failure",
}
USAGE_FIELDS = (
    "input_tokens",
    "output_tokens",
    "cache_creation_input_tokens",
    "cache_read_input_tokens",
)


def audit_pilot(
    schedule_dir: Path,
    artifacts_dir: Path,
    config_path: Path,
    forbidden_paths: tuple[Path, ...] = (),
) -> dict[str, Any]:
    config = ExperimentConfig.load(config_path)
    run_paths = sorted((schedule_dir / "runs").glob("*.json"))
    issues: list[dict[str, str]] = []
    infrastructure_invalid: list[dict[str, str]] = []
    pilot_manifest: dict[str, Any] = {}
    expected_runs = len(run_paths)
    try:
        pilot_manifest = load_object(schedule_dir / "pilot-manifest.json")
        expected_runs = load_round_shape(schedule_dir).runs
    except Exception as error:
        issues.append(_issue("schedule", f"missing or invalid pilot manifest: {error}"))
    if len(run_paths) != expected_runs:
        issues.append(
            _issue("schedule", f"expected {expected_runs} run manifests, found {len(run_paths)}")
        )
    expected_harness_sha256 = pilot_manifest.get("controller_harness_sha256")
    expected_prompts_sha256 = pilot_manifest.get("controller_prompts_sha256")
    expected_config_sha256 = pilot_manifest.get("experiment_config_sha256")
    for name, value in (
        ("controller_harness_sha256", expected_harness_sha256),
        ("controller_prompts_sha256", expected_prompts_sha256),
        ("experiment_config_sha256", expected_config_sha256),
    ):
        if not isinstance(value, str) or len(value) != 64:
            issues.append(_issue("schedule", f"pilot manifest lacks {name}"))

    session_owner: dict[str, str] = {}
    flow_session_owner: dict[str, str] = {}
    termination_counts: dict[str, int] = {}
    feedback_levels: dict[str, int] = {}
    totals = {field: 0 for field in USAGE_FIELDS}
    run_wall_ms: list[int] = []
    run_tokens: list[int] = []
    audited_runs = 0

    for manifest_path in run_paths:
        try:
            resolved = RunSpec.load(manifest_path).resolve_paths(manifest_path)
        except Exception as error:
            issues.append(_issue(manifest_path.name, f"invalid run manifest: {error}"))
            continue
        spec = resolved.spec
        artifact = artifacts_dir / spec.run_id
        if not artifact.is_dir():
            issues.append(_issue(spec.run_id, "missing run artifact directory"))
            continue
        missing = [name for name in REQUIRED_FILES if not (artifact / name).is_file()]
        if not (artifact / "final").is_dir():
            missing.append("final/")
        if missing:
            issues.append(_issue(spec.run_id, f"missing artifacts: {', '.join(missing)}"))
            continue

        try:
            run = load_object(artifact / "run.json")
            judge = load_object(artifact / "judge.json")
            controller = _load_jsonl(artifact / "controller-events.jsonl")
            claude = _load_jsonl(artifact / "claude-events.jsonl")
            flow = _load_jsonl(artifact / "flow-events.jsonl")
        except Exception as error:
            issues.append(_issue(spec.run_id, f"malformed telemetry: {error}"))
            continue

        audited_runs += 1
        if run.get("run_id") != spec.run_id or judge.get("run_id") != spec.run_id:
            issues.append(_issue(spec.run_id, "run identity disagreement"))
        if run.get("arm") != spec.arm or run.get("config", {}).get("model") != config.model:
            issues.append(_issue(spec.run_id, "arm or configured model identity disagreement"))
        # A round may compare feedback levels, so the level is a property of the
        # cell rather than of the round, and the plugin must match its cell.
        plugin_level = (run.get("plugin_manifest") or {}).get("feedback_level")
        if spec.feedback_level is not None and plugin_level != spec.feedback_level:
            issues.append(
                _issue(
                    spec.run_id,
                    f"plugin feedback level {plugin_level!r} does not match the "
                    f"scheduled {spec.feedback_level!r}",
                )
            )
        if run.get("config_sha256") != expected_config_sha256:
            issues.append(_issue(spec.run_id, "experiment configuration identity disagreement"))
        if run.get("controller_harness_sha256") != expected_harness_sha256:
            issues.append(_issue(spec.run_id, "controller harness identity disagreement"))
        if run.get("controller_prompts_sha256") != expected_prompts_sha256:
            issues.append(_issue(spec.run_id, "controller prompt identity disagreement"))
        message_types = {
            event.get("message", {}).get("type")
            for event in claude
            if event.get("event") == "claude_message"
            and isinstance(event.get("message"), dict)
        }
        for required_type in ("SystemMessage", "ResultMessage"):
            if required_type not in message_types:
                issues.append(
                    _issue(spec.run_id, f"raw Claude telemetry lacks {required_type}")
                )
        _audit_flow_telemetry(
            flow,
            spec.run_id,
            spec.arm,
            config.source_commit,
            flow_session_owner,
            issues,
            infrastructure_invalid,
        )
        if tree_hash(artifact / "baseline") != spec.initial_tree_sha256:
            issues.append(_issue(spec.run_id, "baseline tree differs from round input"))
        plugin_manifest = artifact / "plugin" / "move-flow-manifest.json"
        if not plugin_manifest.is_file() or sha256_file(plugin_manifest) != spec.plugin_manifest_sha256:
            issues.append(_issue(spec.run_id, "run-local plugin differs from round plugin"))
        elif tree_hash(artifact / "plugin") != spec.plugin_tree_sha256:
            issues.append(
                _issue(spec.run_id, "run-local plugin tree differs from round plugin")
            )

        observed_level = spec.feedback_level or plugin_level or "unknown"
        feedback_levels[observed_level] = feedback_levels.get(observed_level, 0) + 1
        terminal = judge.get("terminal_status")
        if terminal not in TERMINAL_STATUSES:
            issues.append(_issue(spec.run_id, f"unclassified termination: {terminal!r}"))
        else:
            termination_counts[terminal] = termination_counts.get(terminal, 0) + 1
        if (
            any(entry["run_id"] == spec.run_id for entry in infrastructure_invalid)
            and terminal != "invalid_infrastructure_failure"
        ):
            # The controller watches the same telemetry live and must abandon a
            # restarted session itself. Reaching the audit means the observation
            # was scored under a silently replaced apparatus.
            issues.append(
                _issue(
                    spec.run_id,
                    f"Flow apparatus failure was scored as {terminal!r}",
                )
            )
        # Inference cannot outlast the run that contains it. This catches a
        # cumulative telemetry field being summed as if it were per-turn.
        session_api_ms = max(
            (
                int(result.get("duration_api_ms") or 0)
                for result in agent_results_of(controller)
            ),
            default=0,
        )
        wall_ms = judge.get("controller_wall_ms")
        if isinstance(wall_ms, int) and session_api_ms > wall_ms:
            issues.append(
                _issue(
                    spec.run_id,
                    f"inference time {session_api_ms} ms exceeds the run's "
                    f"{wall_ms} ms wall time",
                )
            )
        wall = judge.get("controller_wall_ms")
        if not isinstance(wall, int) or wall < 0:
            issues.append(_issue(spec.run_id, "missing or invalid controller timing"))
        else:
            run_wall_ms.append(wall)

        agent_results = [event.get("result") for event in controller if event.get("event") == "agent_result"]
        if not agent_results:
            issues.append(_issue(spec.run_id, "no agent-result telemetry"))
            continue
        per_run = {field: 0 for field in USAGE_FIELDS}
        session_ids: set[str] = set()
        for result in agent_results:
            if not isinstance(result, dict):
                issues.append(_issue(spec.run_id, "invalid agent-result payload"))
                continue
            system = result.get("system_init", {}).get("system", {})
            if system.get("model") != config.model:
                issues.append(_issue(spec.run_id, f"runtime model mismatch: {system.get('model')!r}"))
            if system.get("claude_code_version") != config.claude_code_version:
                issues.append(_issue(spec.run_id, "runtime Claude Code version mismatch"))
            for timing in ("duration_ms", "duration_api_ms", "num_turns"):
                if not isinstance(result.get(timing), int) or result[timing] < 0:
                    issues.append(_issue(spec.run_id, f"missing or invalid {timing}"))
            usage = result.get("usage")
            if not isinstance(usage, dict):
                issues.append(_issue(spec.run_id, "missing raw usage object"))
                continue
            for field in USAGE_FIELDS:
                value = usage.get(field)
                if not isinstance(value, int) or value < 0:
                    issues.append(_issue(spec.run_id, f"missing or invalid usage.{field}"))
                else:
                    per_run[field] += value
            session_id = result.get("session_id")
            if not isinstance(session_id, str) or not session_id:
                issues.append(_issue(spec.run_id, "missing session ID"))
            else:
                session_ids.add(session_id)
        # An infrastructure retry starts a fresh session, so a recovered run
        # legitimately carries one session ID per attempt.
        attempts = _attempt_count(judge)
        if not 1 <= len(session_ids) <= max(attempts, 1):
            issues.append(
                _issue(
                    spec.run_id,
                    f"expected one session ID per attempt ({attempts}), found {len(session_ids)}",
                )
            )
        for session_id in session_ids:
            owner = session_owner.setdefault(session_id, spec.run_id)
            if owner != spec.run_id:
                issues.append(_issue(spec.run_id, f"session ID reused from {owner}"))
        if judge.get("total_output_tokens") != per_run["output_tokens"]:
            issues.append(_issue(spec.run_id, "controller/output-token reconciliation failed"))
        for field, value in per_run.items():
            totals[field] += value
        run_tokens.append(sum(per_run.values()))

        for forbidden in forbidden_paths:
            needle = str(forbidden.resolve()).encode()
            for path in artifact.rglob("*"):
                if path.is_file() and path.stat().st_size <= 64 * 1024 * 1024:
                    if needle in path.read_bytes():
                        issues.append(_issue(spec.run_id, f"forbidden path disclosed in {path.name}"))
                        break

    return {
        "schema_version": 1,
        "phase": "unscored_pilot",
        "expected_runs": expected_runs,
        "scheduled_runs": len(run_paths),
        "audited_runs": audited_runs,
        "complete": (
            len(run_paths) == expected_runs
            and audited_runs == expected_runs
            and not issues
            and not infrastructure_invalid
        ),
        "issues": issues,
        "infrastructure_invalid_runs": infrastructure_invalid,
        "termination_counts": dict(sorted(termination_counts.items())),
        "feedback_levels": sorted(feedback_levels.items()),
        "raw_usage_totals": totals,
        "run_total_tokens": _distribution(run_tokens),
        "controller_wall_ms": _distribution(run_wall_ms),
        "apparatus": {
            "controller_harness_sha256": expected_harness_sha256,
            "controller_prompts_sha256": expected_prompts_sha256,
            "experiment_config_sha256": expected_config_sha256,
        },
    }


def _distribution(values: list[int]) -> dict[str, int | None]:
    if not values:
        return {"median": None, "p95": None, "maximum": None}
    ordered = sorted(values)
    p95 = ordered[max(0, math.ceil(0.95 * len(ordered)) - 1)]
    return {
        "median": int(statistics.median(ordered)),
        "p95": p95,
        "maximum": ordered[-1],
    }


def _audit_flow_telemetry(
    records: list[dict[str, Any]],
    run_id: str,
    arm: str,
    source_commit: str,
    session_owner: dict[str, str],
    issues: list[dict[str, str]],
    infrastructure_invalid: list[dict[str, str]],
) -> None:
    starts = [record for record in records if record.get("event") == "session_start"]
    ends = [record for record in records if record.get("event") == "session_end"]
    if not starts:
        issues.append(_issue(run_id, "Flow telemetry lacks session_start"))
    # A supervisor restart replaces the MCP child mid-session, discarding its
    # package cache and any tool call in flight. The remaining turns no longer
    # measure the intended apparatus, so the observation is invalid rather than
    # a study outcome.
    for start in starts:
        if start.get("restart") is True:
            infrastructure_invalid.append(
                _issue(
                    run_id,
                    f"Flow MCP supervisor restarted session {start.get('session_id')!r}",
                )
            )
    if len(starts) != len(ends):
        infrastructure_invalid.append(
            _issue(
                run_id,
                f"Flow session start/end mismatch: {len(starts)} starts, {len(ends)} ends",
            )
        )
    # Flow serializes the tactic enum in snake_case, while controller CLI
    # arguments use the corresponding kebab-case spelling.
    expected_tactic = ARM_TO_TACTIC[arm].replace("-", "_")
    for record in records:
        if record.get("flow_source_commit") != source_commit:
            issues.append(_issue(run_id, "Flow source commit mismatch"))
            break
        if record.get("inference_tactic") != expected_tactic:
            issues.append(_issue(run_id, "Flow inference tactic mismatch"))
            break
        if record.get("evaluation_mode") is not True:
            issues.append(_issue(run_id, "Flow evaluation mode is not enabled"))
            break
    for start in starts:
        session_id = start.get("session_id")
        if not isinstance(session_id, str) or not session_id:
            issues.append(_issue(run_id, "Flow session_start lacks a session ID"))
            continue
        owner = session_owner.setdefault(session_id, run_id)
        if owner != run_id:
            issues.append(_issue(run_id, f"Flow session ID reused from {owner}"))


def agent_results_of(controller: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """The agent results a controller ledger recorded, in order."""
    return [
        event["result"]
        for event in controller
        if event.get("event") == "agent_result" and isinstance(event.get("result"), dict)
    ]



def _load_jsonl(path: Path, allow_empty: bool = False) -> list[dict[str, Any]]:
    records = []
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        value = json.loads(line)
        if not isinstance(value, dict):
            raise ValueError(f"{path.name}:{number} is not an object")
        records.append(value)
    if not records and not allow_empty:
        raise ValueError(f"{path.name} is empty")
    return records


def _attempt_count(result: dict) -> int:
    """How many attempts the controller made, from its own result record.

    The count belongs to `judge.json` (and `run.json`'s `result` member),
    written when the run ends. It is never at the top level of `run.json`,
    which is the scheduler's manifest and predates the run -- reading it there
    silently yields 1, which turns a legitimately recovered retry into an
    audit failure.
    """
    attempts = result.get("attempts")
    return attempts if isinstance(attempts, int) and attempts >= 1 else 1


def _issue(run_id: str, detail: str) -> dict[str, str]:
    return {"run_id": run_id, "detail": detail}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--schedule-dir", type=Path, required=True)
    parser.add_argument("--artifacts-dir", type=Path, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--forbidden-path", type=Path, action="append", default=[])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = audit_pilot(
        args.schedule_dir.resolve(),
        args.artifacts_dir.resolve(),
        args.config.resolve(),
        tuple(args.forbidden_path),
    )
    write_json(args.output, result)
    print(
        json.dumps(
            {
                "audited_runs": result["audited_runs"],
                "infrastructure_invalid_runs": len(result["infrastructure_invalid_runs"]),
                "complete": result["complete"],
            }
        )
    )
    if not result["complete"]:
        raise SystemExit("pilot audit is incomplete or found integrity failures")


if __name__ == "__main__":
    main()

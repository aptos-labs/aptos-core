"""Collect one round's per-run facts into a single JSON document.

Reads only what the round recorded: the run manifest, the controller event
stream, the agent transcript, and — when the round was scored — the mutation
result written beside each run. Nothing here recomputes an outcome; a report
built from this file says what the round said.
"""

from __future__ import annotations

import argparse
import collections
import json
from pathlib import Path
from typing import Any

FLOW_TOOL_PREFIX = "mcp__move-flow__move_package_"


def collect_run(artifact: Path) -> dict[str, Any] | None:
    record_path = artifact / "run.json"
    if not record_path.is_file():
        return None
    record = json.loads(record_path.read_text(encoding="utf-8"))
    result = record.get("result") or {}

    turns = 0
    controller_turns = 0
    report = ""
    # `model_usage` is a session total that already includes every earlier turn,
    # so it is kept per session and summed only across sessions. Adding it once
    # per controller turn counts the same inference repeatedly, and the error
    # grows with turn count. See DESIGN.md section 5, "Measuring cost".
    session_usage: dict[str, dict[str, Any]] = {}
    events = artifact / "controller-events.jsonl"
    if events.is_file():
        for line in events.read_text(encoding="utf-8").splitlines():
            event = json.loads(line)
            if event.get("event") != "agent_result":
                continue
            controller_turns += 1
            agent = event["result"]
            turns += agent.get("num_turns") or 0
            # An infrastructure retry starts a fresh session, and those do add.
            session = agent.get("session_id") or ""
            for model, usage in (agent.get("model_usage") or {}).items():
                session_usage[f"{session}\0{model}"] = usage
            report = agent.get("result") or report
    output_tokens = sum(
        usage.get("outputTokens", 0) for usage in session_usage.values()
    )
    cost = sum(usage.get("costUSD", 0.0) for usage in session_usage.values())

    tools: collections.Counter[str] = collections.Counter()
    transcript = artifact / "claude-events.jsonl"
    if transcript.is_file():
        for line in transcript.read_text(encoding="utf-8").splitlines():
            event = json.loads(line)
            if event.get("event") != "claude_message":
                continue
            for item in event["message"].get("content") or []:
                if isinstance(item, dict) and item.get("name"):
                    tools[item["name"]] += 1

    mutation = None
    score_path = artifact / "mutation-score.json"
    if score_path.is_file():
        score = json.loads(score_path.read_text(encoding="utf-8"))
        mutation = {
            "essential": score["essential_mutants"],
            "killed": score["killed"],
            "adequacy": score["mutation_adequacy"],
            # `not killed` is not the same as survived. A timeout, an
            # infrastructure failure or an unclassified prover failure
            # establishes nothing about the mutant, and listing those as
            # survivors asserts the opposite of what happened: that the
            # contract verified against code it should have rejected.
            "survived": [
                r["mutant_id"] for r in score["results"] if r["outcome"] == "survived"
            ],
            "inconclusive": {
                r["mutant_id"]: r["outcome"]
                for r in score["results"]
                if r["outcome"] not in ("killed", "survived")
            },
            "outcomes": {r["mutant_id"]: r["outcome"] for r in score["results"]},
        }

    return {
        "run_id": record["run_id"],
        "task_id": record["task_id"],
        "arm": record["arm"],
        "target": record["target"],
        # The controller always records how the run ended; the eventual judge
        # exists only for a run that succeeded, so deriving the status from it
        # collapsed every compile failure, timeout and exhausted budget to
        # null. Both are reported: they differ only in that the judge state
        # describes the tree, and the terminal status describes the run.
        "terminal_status": result.get("terminal_status"),
        "eventual_judge_state": (result.get("eventual_judge") or {}).get("state"),
        "operational_success": result.get("operational_success"),
        "attempts": result.get("attempts"),
        "controller_turns": controller_turns,
        "turns": turns,
        "output_tokens": output_tokens,
        "cost_usd": round(cost, 4),
        "wall_seconds": round(result.get("controller_wall_ms", 0) / 1000, 1),
        "tools": {
            name.removeprefix(FLOW_TOOL_PREFIX) if name.startswith(FLOW_TOOL_PREFIX) else name: count
            for name, count in sorted(tools.items())
        },
        "final_report": report,
        "mutation": mutation,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--round-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    runs_dir = args.round_dir.resolve() / "runs"
    runs = [
        collected
        for artifact in sorted(p for p in runs_dir.iterdir() if p.is_dir())
        if (collected := collect_run(artifact)) is not None
    ]
    summary = {
        "schema_version": 1,
        "round_id": args.round_dir.resolve().name,
        "runs": len(runs),
        "scored": sum(1 for r in runs if r["mutation"]),
        "detail": runs,
    }
    args.output.write_text(json.dumps(summary, indent=1) + "\n", encoding="utf-8")
    print(json.dumps({k: v for k, v in summary.items() if k != "detail"}, sort_keys=True))


if __name__ == "__main__":
    main()

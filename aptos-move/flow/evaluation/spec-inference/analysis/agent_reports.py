"""Collect each cell's final agent report from a recorded round.

The skill ends every session with a short report of the result, the strategy,
and the pivotal decisions. The report is the last agent result the controller
recorded for the cell. This writes them, with the cell's identity and scored
outcome, as JSON lines and as a Markdown file grouped by target and tactic.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from analysis.codex_round_report import ARMS

ARM_NAMES = {"agent_only": "agent-only", "hybrid_flexible": "hybrid flexible",
             "hybrid_guided": "hybrid guided"}


def final_report(run_dir: Path) -> str | None:
    report = None
    events = run_dir / "controller-events.jsonl"
    if events.is_file():
        for line in events.read_text(encoding="utf-8").splitlines():
            event = json.loads(line)
            if event.get("event") == "agent_result":
                report = (event.get("result") or {}).get("result") or report
    return report


def collect(round_dir: Path, manifest: Path) -> list[dict[str, Any]]:
    targets = {
        record["task_id"]: record
        for record in json.loads(manifest.read_text(encoding="utf-8"))["records"]
    }
    summary = round_dir / "mutation-summary.json"
    scored = (
        {entry["run_id"]: entry for entry in json.loads(summary.read_text(encoding="utf-8"))["runs"]}
        if summary.is_file() else {}
    )
    rows = []
    for path in sorted((round_dir / "schedule" / "runs").glob("*.json")):
        spec = json.loads(path.read_text(encoding="utf-8"))
        record = targets[spec["task_id"]]
        entry = scored.get(spec["run_id"], {})
        rows.append({
            "run_id": spec["run_id"],
            "task_id": spec["task_id"],
            "target": record["function"],
            "module": record["module"],
            "arm": spec["arm"],
            "replicate": spec["replicate"],
            "outcome": entry.get("outcome"),
            "strict_success": entry.get("strict_success"),
            "report": final_report(round_dir / "runs" / spec["run_id"]),
        })
    return rows


def render(round_id: str, rows: list[dict[str, Any]]) -> str:
    lines = [f"# Agent reports: {round_id}", "",
             "The final report of every cell, as the agent wrote it, grouped by "
             "target and tactic.", ""]
    for target in sorted({row["target"] for row in rows}):
        cells = [row for row in rows if row["target"] == target]
        lines += [f"## `{target}`", "", f"`{cells[0]['module']}`, task id `{cells[0]['task_id']}`", ""]
        for arm in ARMS:
            for row in sorted((r for r in cells if r["arm"] == arm), key=lambda r: r["replicate"]):
                outcome = "strict success" if row["strict_success"] else (row["outcome"] or "unscored")
                lines += [f"### {ARM_NAMES[arm]}, replicate {row['replicate']}: {outcome}", ""]
                lines += [(row["report"] or "(no report recorded)").strip(), ""]
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--round-dir", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, default=Path("corpus-v4/manifest.json"))
    parser.add_argument("--output", type=Path, required=True,
                        help="output stem: writes <stem>.jsonl and <stem>.md")
    args = parser.parse_args()
    round_dir = args.round_dir.resolve()
    rows = collect(round_dir, args.manifest)
    args.output.with_suffix(".jsonl").write_text(
        "".join(json.dumps(row, ensure_ascii=False) + "\n" for row in rows), encoding="utf-8")
    args.output.with_suffix(".md").write_text(render(round_dir.name, rows) + "\n", encoding="utf-8")
    missing = sum(row["report"] is None for row in rows)
    print(f"{len(rows)} cells, {missing} without a report")


if __name__ == "__main__":
    main()

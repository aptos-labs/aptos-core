"""Compare tactics within each model and models within each tactic.

Reads only the aggregate archives `harness.publication` builds from
`analysis.codex_round_report` output, so a reader can recompute the summary
from published material. Each archive is one model's round over the same
corpus and schedule shape.

The tactics view repeats each round's preregistered contrasts. The models view
pairs cells by `(task, replicate)` within an arm and compares the second model
with the first, as the equal-weight mean over tasks of within-task differences,
with percentile task-bootstrap intervals. The model comparison is exploratory:
it is not one of the preregistered contrasts.
"""

from __future__ import annotations

import argparse
import collections
import csv
import io
import json
import statistics
import tarfile
from pathlib import Path
from typing import Any

from analysis.codex_round_report import ARMS, percentile_interval

METRICS = ("strict_success", "cost_usd", "wall_seconds")


def read_archive(path: Path) -> dict[str, Any]:
    """The round's analysis and per-cell rows from its aggregate archive."""
    with tarfile.open(path, "r:gz") as archive:
        members = {Path(member.name).name: member for member in archive.getmembers()}
        def text(name: str) -> str:
            return archive.extractfile(members[name]).read().decode("utf-8")
        analysis = json.loads(text("analysis.json"))
        cells = list(csv.DictReader(io.StringIO(text("cells.csv"))))
    for cell in cells:
        strict = cell["strict_success"]
        cell["strict_success"] = None if strict == "" else strict == "True"
        cell["cost_usd"] = float(cell["cost_usd"])
        cell["wall_seconds"] = float(cell["wall_seconds"])
    return {"model": analysis["model"], "round_id": analysis["round_id"], "analysis": analysis, "cells": cells}


def model_differences(
    first: list[dict[str, Any]], second: list[dict[str, Any]], arm: str, metric: str
) -> dict[str, Any]:
    """Second model minus first, paired by (task, replicate) within `arm`."""
    def keyed(cells: list[dict[str, Any]]) -> dict[tuple[str, str], Any]:
        return {
            (cell["task_id"], cell["replicate"]): cell[metric]
            for cell in cells
            if cell["arm"] == arm and cell[metric] is not None
        }
    a, b = keyed(first), keyed(second)
    by_task: dict[str, list[float]] = collections.defaultdict(list)
    for key in sorted(set(a) & set(b)):
        by_task[key[0]].append(float(b[key]) - float(a[key]))
    interval = percentile_interval([statistics.fmean(v) for v in by_task.values()])
    interval["pairs"] = sum(len(v) for v in by_task.values())
    return interval


def compare(rounds: list[dict[str, Any]]) -> dict[str, Any]:
    first, second = rounds
    tactics = {
        r["model"]: {
            "round_id": r["round_id"],
            "arms": {
                arm: {k: e[k] for k in (
                    "cells", "measured", "unmeasured", "strict_successes", "disqualified",
                    "cost_usd", "mean_cost_usd", "mean_wall_seconds",
                )}
                for arm, e in r["analysis"]["arms"].items()
            },
            "contrasts": r["analysis"]["contrasts"],
        }
        for r in rounds
    }
    models = {
        arm: {
            metric: model_differences(first["cells"], second["cells"], arm, metric)
            for metric in METRICS
        }
        for arm in ARMS
    }
    tasks: dict[str, dict[str, dict[str, list[int]]]] = collections.defaultdict(
        lambda: collections.defaultdict(dict)
    )
    for r in rounds:
        for cell in r["cells"]:
            entry = tasks[cell["task_id"]][r["model"]].setdefault(cell["arm"], [0, 0])
            entry[0] += bool(cell["strict_success"])
            entry[1] += 1
    return {
        "models": [r["model"] for r in rounds],
        "difference": f"{second['model']} minus {first['model']}",
        "tactics": tactics,
        "model_differences": models,
        "tasks": {t: dict(v) for t, v in sorted(tasks.items())},
        "scope": "model differences are exploratory; the preregistered contrasts are C1-C3 within a model",
    }


def interval_text(interval: dict[str, Any], scale: float = 1.0, unit: str = "", digits: int = 3) -> str:
    if interval.get("mean") is None:
        return "–"
    return (
        f"{interval['mean'] * scale:+.{digits}f}{unit} "
        f"[{interval['low95'] * scale:+.{digits}f}, {interval['high95'] * scale:+.{digits}f}]"
    )


def render(data: dict[str, Any]) -> str:
    first, second = data["models"]
    lines = ["## Tactics within each model", ""]
    lines += [
        "| model | arm | strict | unmeasured | disqualified | mean cost / cell | vs agent-only | mean wall |",
        "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for model in data["models"]:
        arms = data["tactics"][model]["arms"]
        base = arms["agent_only"]["mean_cost_usd"]
        for arm, e in arms.items():
            change = "" if arm == "agent_only" else f"{100 * (e['mean_cost_usd'] / base - 1):+.1f}%"
            lines.append(
                f"| `{model}` | {arm} | {e['strict_successes']}/{e['measured']} | {e['unmeasured']} "
                f"| {e['disqualified']} | ${e['mean_cost_usd']:.3f} | {change} | {e['mean_wall_seconds']:.1f} s |"
            )
    lines += [
        "",
        "| model | contrast | strict success | Holm p | cost / cell |",
        "| --- | --- | ---: | ---: | ---: |",
    ]
    for model in data["models"]:
        for name, contrast in data["tactics"][model]["contrasts"].items():
            metrics = contrast["metrics"]
            holm = metrics["strict_success"].get("holm_p")
            lines.append(
                f"| `{model}` | {name}: {contrast['definition']} "
                f"| {interval_text(metrics['strict_success'], 100, ' pp', 1)} "
                f"| {'–' if holm is None else f'{holm:.2f}'} | {interval_text(metrics['cost_usd'])} |"
            )
    lines += [
        "",
        f"## Models within each tactic: `{second}` minus `{first}`",
        "",
        "| arm | strict success | cost / cell | wall / cell |",
        "| --- | ---: | ---: | ---: |",
    ]
    for arm, metrics in data["model_differences"].items():
        lines.append(
            f"| {arm} | {interval_text(metrics['strict_success'], 100, ' pp', 1)} "
            f"| {interval_text(metrics['cost_usd'])} | {interval_text(metrics['wall_seconds'], unit=' s', digits=1)} |"
        )
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("archives", type=Path, nargs=2, help="the two rounds' aggregate archives")
    parser.add_argument("--json", type=Path, help="write the comparison as JSON")
    args = parser.parse_args()
    data = compare([read_archive(path) for path in args.archives])
    if args.json:
        args.json.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    print(render(data), end="")


if __name__ == "__main__":
    main()

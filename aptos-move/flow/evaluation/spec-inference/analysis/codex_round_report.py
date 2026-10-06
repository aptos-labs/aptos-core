"""Task-clustered report of a Codex round, priced from its recorded requests.

Reads only what the round recorded: the schedule, each run's request usage,
Flow events, judge record and workspace diff, and the round's mutation
summary. Cost is derived from the token counts with the archived price table
in `pricing.json`. Writes `cells.csv`, `requests.csv`, `analysis.json`,
`pricing.json` and `REPORT.md`, all admitted by `harness.publication`.

The estimands follow DESIGN.md section 6. The task is the unit: a contrast is
the equal-weight mean over tasks of the within-task mean of the differences
in `(task, replicate)` blocks holding both arms. Intervals are percentile
bootstraps over tasks; `C1` and `C2` are also tested with a blocked
randomization test, Holm-adjusted across the two.
"""

from __future__ import annotations

import argparse
import collections
import csv
import json
import random
import statistics
from pathlib import Path
from typing import Any, Iterable

ARMS = ("agent_only", "hybrid_flexible", "hybrid_guided")
CONTRASTS = {
    "C1": ("hybrid_flexible", "agent_only"),
    "C2": ("hybrid_guided", "hybrid_flexible"),
    "C3": ("hybrid_guided", "agent_only"),
}
TESTED_CONTRASTS = ("C1", "C2")
SEED = 20260907
RESAMPLES = 10_000
PRICING_FILE = Path(__file__).with_name("pricing.json")
# Outcomes of `harness.score_round` for which strict success is decided. The
# others (`inconclusive`, `not_scorable`, `no_mutant_set`) leave it unmeasured.
MEASURED_OUTCOMES = frozenset({"scored", "disqualified", "not_operationally_successful"})
METRICS = (
    "strict_success",
    "operational_success",
    "mutation_adequacy",
    "cost_usd",
    "wall_seconds",
    "restricted_time_to_success_seconds",
)
TOKEN_FIELDS = (
    "input_tokens",
    "fresh_input_tokens",
    "cached_input_tokens",
    "cache_write_input_tokens",
    "output_tokens",
    "reasoning_output_tokens",
)
COST_FIELDS = (
    "fresh_input_cost_usd",
    "cached_input_cost_usd",
    "cache_write_cost_usd",
    "output_cost_usd",
    "cost_usd",
)


def model_price(pricing: dict[str, Any], model: str) -> dict[str, Any]:
    if model not in pricing["models"]:
        raise ValueError(f"no price table for model `{model}` in {PRICING_FILE.name}")
    return pricing["models"][model]


def price_request(usage: dict[str, int], price: dict[str, Any]) -> dict[str, Any]:
    """Price one model request.

    The long-context tier is decided by this request's own input alone. A
    per-turn total, which sums many requests, is never a request size.
    """
    input_tokens = usage["input_tokens"]
    cached = usage["cached_input_tokens"]
    cache_write = usage.get("cache_write_input_tokens", 0)
    output = usage["output_tokens"]
    fresh = input_tokens - cached - cache_write
    if fresh < 0:
        raise ValueError(f"cached and cache-write tokens exceed input: {usage}")
    long_context = input_tokens > price["long_context_threshold_input_tokens"]
    input_rate = price["long_context_input_multiplier"] if long_context else 1
    output_rate = price["long_context_output_multiplier"] if long_context else 1
    costs = {
        "fresh_input_cost_usd": fresh * price["fresh_input_per_million"] * input_rate / 1e6,
        "cached_input_cost_usd": cached * price["cached_input_per_million"] * input_rate / 1e6,
        "cache_write_cost_usd": cache_write
        * price["cache_write_per_million"]
        * input_rate
        / 1e6,
        "output_cost_usd": output * price["output_per_million"] * output_rate / 1e6,
    }
    return {
        "input_tokens": input_tokens,
        "fresh_input_tokens": fresh,
        "cached_input_tokens": cached,
        "cache_write_input_tokens": cache_write,
        "output_tokens": output,
        "reasoning_output_tokens": usage["reasoning_output_tokens"],
        "long_context": long_context,
        **costs,
        "cost_usd": sum(costs.values()),
    }


def read_jsonl(path: Path) -> Iterable[dict[str, Any]]:
    if not path.is_file():
        return
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            yield json.loads(line)


def collect_requests(
    run_dir: Path, base: dict[str, Any], price: dict[str, Any], audit: list[dict[str, Any]]
) -> list[dict[str, Any]]:
    requests = []
    for event in read_jsonl(run_dir / "codex-request-usage.jsonl"):
        if event.get("event") == "codex_usage_reconciliation":
            if not event.get("matched"):
                audit.append(
                    {"run_id": base["run_id"], "issue": "usage_reconciliation_mismatch",
                     "controller_turn": event.get("controller_turn")}
                )
            continue
        if event.get("event") != "codex_response_usage":
            continue
        requests.append(
            {
                **base,
                "attempt": event.get("attempt"),
                "controller_turn": event.get("controller_turn"),
                "sequence": event.get("sequence"),
                "classification": event.get("classification"),
                "model": event.get("model"),
                **price_request(event["usage"], price),
            }
        )
    return requests


def flow_counts(run_dir: Path) -> dict[str, Any]:
    checks = failed = wp = prover = 0
    first_accepted = None
    for event in read_jsonl(run_dir / "flow-events.jsonl"):
        kind = event.get("event")
        if kind == "candidate_check":
            checks += 1
            accepted = bool(event.get("accepted"))
            if first_accepted is None:
                first_accepted = accepted
            failed += not accepted
        elif kind == "wp_engine":
            wp += 1
        elif kind == "tool_start" and event.get("tool_name") == "move_package_verify":
            prover += 1
    return {
        "candidate_checks": checks,
        "first_check_accepted": bool(first_accepted),
        "failed_candidate_checks": failed,
        "wp_calls": wp,
        "prover_calls": prover,
    }


def diff_lines(run_dir: Path) -> dict[str, int]:
    added = removed = 0
    diff = run_dir / "workspace.diff"
    if diff.is_file():
        for line in diff.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.startswith("+") and not line.startswith("+++"):
                added += 1
            elif line.startswith("-") and not line.startswith("---"):
                removed += 1
    return {"added_lines": added, "removed_lines": removed}


def collect_cell(
    round_dir: Path,
    spec: dict[str, Any],
    scored: dict[str, Any] | None,
    price: dict[str, Any],
    wall_cap_seconds: float,
    audit: list[dict[str, Any]],
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    base = {key: spec[key] for key in ("run_id", "task_id", "arm", "replicate", "block", "order")}
    run_dir = round_dir / "runs" / spec["run_id"]
    if not (run_dir / "run.json").is_file():
        audit.append({"run_id": spec["run_id"], "issue": "not_recorded"})
        return {**base, "recorded": False}, []
    result = json.loads((run_dir / "run.json").read_text(encoding="utf-8"))["result"]
    requests = collect_requests(run_dir, base, price, audit)
    canonical = [r for r in requests if r["classification"] == "turn_request"]
    warmup = [r for r in requests if r["classification"] == "startup_warmup"]
    unknown = [r for r in requests if r["classification"] not in ("turn_request", "startup_warmup")]
    if unknown:
        audit.append({"run_id": spec["run_id"], "issue": "unclassified_requests", "count": len(unknown)})
    outcome = (scored or {}).get("outcome")
    if scored is None:
        audit.append({"run_id": spec["run_id"], "issue": "not_in_mutation_summary"})
    strict = bool(scored.get("strict_success")) if outcome in MEASURED_OUTCOMES else None
    wall = result["controller_wall_ms"] / 1000
    row = {
        **base,
        "recorded": True,
        "terminal_status": result.get("terminal_status"),
        "operational_success": bool(result.get("operational_success")),
        "outcome": outcome,
        "strict_success": strict,
        "mutation_adequacy": (scored or {}).get("mutation_adequacy"),
        "controller_attempts": result.get("attempts"),
        "wall_seconds": wall,
        # Time to strict success under the common wall cap; a failure counts
        # as censored at the cap.
        "restricted_time_to_success_seconds": (
            None if strict is None else min(wall, wall_cap_seconds) if strict else wall_cap_seconds
        ),
        "canonical_requests": len(canonical),
        "warmup_requests": len(warmup),
        "long_context_requests": sum(r["long_context"] for r in canonical),
        **{field: sum(r[field] for r in canonical) for field in TOKEN_FIELDS},
        **{field: sum(r[field] for r in canonical) for field in COST_FIELDS},
        "warmup_cost_usd": sum(r["cost_usd"] for r in warmup),
        **flow_counts(run_dir),
        **diff_lines(run_dir),
    }
    return row, requests


def percentile_interval(values: list[float]) -> dict[str, Any]:
    """Equal-weight mean of per-task values, with task-bootstrap intervals."""
    if not values:
        return {"mean": None, "tasks": 0}
    rng = random.Random(SEED)
    n = len(values)
    draws = sorted(statistics.fmean(rng.choices(values, k=n)) for _ in range(RESAMPLES))
    return {
        "mean": statistics.fmean(values),
        "low95": draws[int(0.025 * RESAMPLES)],
        "high95": draws[int(0.975 * RESAMPLES) - 1],
        "low97_5": draws[int(0.0125 * RESAMPLES)],
        "high97_5": draws[int(0.9875 * RESAMPLES) - 1],
        "tasks": n,
    }


def per_task_means(cells: list[dict[str, Any]], metric: str) -> list[float]:
    by_task: dict[str, list[float]] = collections.defaultdict(list)
    for cell in cells:
        if cell.get(metric) is not None:
            by_task[cell["task_id"]].append(float(cell[metric]))
    return [statistics.fmean(values) for values in by_task.values()]


def block_differences(
    cells: list[dict[str, Any]], metric: str, left: str, right: str
) -> dict[str, list[float]]:
    blocks: dict[tuple[str, Any], dict[str, Any]] = collections.defaultdict(dict)
    for cell in cells:
        if cell.get(metric) is not None:
            blocks[(cell["task_id"], cell["replicate"])][cell["arm"]] = float(cell[metric])
    differences: dict[str, list[float]] = collections.defaultdict(list)
    for (task, _), arms in sorted(blocks.items(), key=lambda item: str(item[0])):
        if left in arms and right in arms:
            differences[task].append(arms[left] - arms[right])
    return dict(differences)


def clustered_mean(differences: dict[str, list[float]]) -> float:
    return statistics.fmean(statistics.fmean(values) for values in differences.values())


def randomization_p_value(differences: dict[str, list[float]]) -> float | None:
    """Two-sided blocked randomization test: within each block the two arm
    labels are exchangeable under the null, which flips the difference."""
    if not differences:
        return None
    observed = abs(clustered_mean(differences))
    rng = random.Random(SEED)
    tasks = [values for _, values in sorted(differences.items())]
    extreme = 0
    for _ in range(RESAMPLES):
        statistic = statistics.fmean(
            statistics.fmean(value if rng.random() < 0.5 else -value for value in values)
            for values in tasks
        )
        extreme += abs(statistic) >= observed - 1e-12
    return (extreme + 1) / (RESAMPLES + 1)


def holm(p_values: dict[str, float | None]) -> dict[str, float | None]:
    present = sorted((p, name) for name, p in p_values.items() if p is not None)
    adjusted: dict[str, float | None] = {name: None for name in p_values}
    running = 0.0
    for rank, (p, name) in enumerate(present):
        running = max(running, min(1.0, (len(present) - rank) * p))
        adjusted[name] = running
    return adjusted


def analyze(cells: list[dict[str, Any]]) -> dict[str, Any]:
    recorded = [cell for cell in cells if cell.get("recorded")]
    arms = {}
    for arm in ARMS:
        rows = [cell for cell in recorded if cell["arm"] == arm]
        if not rows:
            continue
        measured = [cell for cell in rows if cell["strict_success"] is not None]
        strict = sum(cell["strict_success"] for cell in measured)
        cost = sum(cell["cost_usd"] for cell in rows)
        arms[arm] = {
            "cells": len(rows),
            "measured": len(measured),
            "unmeasured": len(rows) - len(measured),
            "strict_successes": strict,
            "operational_successes": sum(cell["operational_success"] for cell in rows),
            "disqualified": sum(cell["outcome"] == "disqualified" for cell in rows),
            "cost_usd": cost,
            "warmup_cost_usd": sum(cell["warmup_cost_usd"] for cell in rows),
            "mean_cost_usd": cost / len(rows),
            "cost_per_strict_success_usd": cost / strict if strict else None,
            "canonical_requests": sum(cell["canonical_requests"] for cell in rows),
            "long_context_requests": sum(cell["long_context_requests"] for cell in rows),
            **{field: sum(cell[field] for cell in rows) for field in TOKEN_FIELDS},
            **{field: sum(cell[field] for cell in rows) for field in COST_FIELDS},
            "mean_wall_seconds": statistics.fmean(cell["wall_seconds"] for cell in rows),
            "task_clustered": {
                metric: percentile_interval(per_task_means(rows, metric)) for metric in METRICS
            },
        }
    contrasts = {}
    for name, (left, right) in CONTRASTS.items():
        if left not in arms or right not in arms:
            continue
        metrics = {}
        for metric in METRICS:
            differences = block_differences(recorded, metric, left, right)
            interval = percentile_interval(
                [statistics.fmean(values) for values in differences.values()]
            )
            interval["blocks"] = sum(len(values) for values in differences.values())
            if name in TESTED_CONTRASTS:
                interval["randomization_p"] = randomization_p_value(differences)
            metrics[metric] = interval
        contrasts[name] = {"definition": f"{left} minus {right}", "metrics": metrics}
    for metric in METRICS:
        adjusted = holm(
            {
                name: contrasts[name]["metrics"][metric].get("randomization_p")
                for name in TESTED_CONTRASTS
                if name in contrasts
            }
        )
        for name, p in adjusted.items():
            contrasts[name]["metrics"][metric]["holm_p"] = p
    tasks: dict[str, dict[str, Any]] = collections.defaultdict(dict)
    for cell in recorded:
        entry = tasks[cell["task_id"]].setdefault(
            cell["arm"], {"cells": 0, "strict_successes": 0, "cost_usd": 0.0, "wall_seconds": 0.0}
        )
        entry["cells"] += 1
        entry["strict_successes"] += bool(cell["strict_success"])
        entry["cost_usd"] += cell["cost_usd"]
        entry["wall_seconds"] += cell["wall_seconds"]
    return {
        "scheduled_cells": len(cells),
        "recorded_cells": len(recorded),
        "arms": arms,
        "contrasts": contrasts,
        "tasks": {task: dict(sorted(arms.items())) for task, arms in sorted(tasks.items())},
        "bootstrap": {
            "seed": SEED,
            "resamples": RESAMPLES,
            "unit": "task",
            "estimand": "equal-weight mean over tasks of within-task means",
            "interval": "percentile; 97.5% bounds are Bonferroni for the C1/C2 family",
            "randomization_test": "two-sided sign flip within (task, replicate) blocks, Holm across C1 and C2",
        },
    }


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    fields = list(dict.fromkeys(key for row in rows for key in row))
    with path.open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)


def money(value: float | None) -> str:
    return "–" if value is None else f"${value:,.4f}"


def interval_text(
    interval: dict[str, Any], scale: float = 1.0, unit: str = "", digits: int = 3
) -> str:
    if interval.get("mean") is None:
        return "–"
    return (
        f"{interval['mean'] * scale:+.{digits}f}{unit} "
        f"[{interval['low95'] * scale:+.{digits}f}, {interval['high95'] * scale:+.{digits}f}]"
    )


def render_report(round_id: str, model: str, price: dict[str, Any], data: dict[str, Any]) -> str:
    intro = (
        f"Model `{model}`; {data['recorded_cells']} of {data['scheduled_cells']} scheduled cells "
        "recorded. Cost is API-equivalent, priced per request from the recorded token counts "
        f"(${price['fresh_input_per_million']}/M input, ${price['cached_input_per_million']}/M "
        f"cached, ${price['output_per_million']}/M output, reasoning included in output; a request "
        f"above {price['long_context_threshold_input_tokens']:,} input tokens is long-context)."
    )
    arm_header = (
        "| arm | strict | unmeasured | disqualified | operational | cost | mean / cell "
        "| cost / strict success | mean wall | long-context requests |"
    )
    lines = [
        f"# {round_id}",
        "",
        intro,
        "",
        arm_header,
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for arm, entry in data["arms"].items():
        lines.append(
            f"| {arm} | {entry['strict_successes']}/{entry['measured']} | {entry['unmeasured']} "
            f"| {entry['disqualified']} | {entry['operational_successes']}/{entry['cells']} "
            f"| {money(entry['cost_usd'])} | {money(entry['mean_cost_usd'])} "
            f"| {money(entry['cost_per_strict_success_usd'])} | {entry['mean_wall_seconds']:.1f} s "
            f"| {entry['long_context_requests']} |"
        )
    contrast_note = (
        "Contrasts are equal-weight means over tasks of within-task block differences, with 95% "
        "task-bootstrap intervals; p-values are blocked randomization tests, Holm-adjusted across "
        "C1 and C2."
    )
    lines += [
        "",
        contrast_note,
        "",
        "| contrast | strict success | Holm p | cost / cell | wall / cell | restricted time to success |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for name, contrast in data["contrasts"].items():
        metrics = contrast["metrics"]
        holm_p = metrics["strict_success"].get("holm_p")
        lines.append(
            f"| {name}: {contrast['definition']} "
            f"| {interval_text(metrics['strict_success'], 100, ' pp', 1)} "
            f"| {'–' if holm_p is None else f'{holm_p:.4f}'} "
            f"| {interval_text(metrics['cost_usd'])} "
            f"| {interval_text(metrics['wall_seconds'], unit=' s', digits=1)} "
            f"| {interval_text(metrics['restricted_time_to_success_seconds'], unit=' s', digits=1)} |"
        )
    arms = list(data["arms"])
    lines += [
        "",
        "| task | " + " | ".join(f"{arm} strict | {arm} mean cost" for arm in arms) + " |",
        "| --- | " + " | ".join("---: | ---:" for _ in arms) + " |",
    ]
    for task, entries in data["tasks"].items():
        cells = []
        for arm in arms:
            entry = entries.get(arm)
            cells.append(
                "– | –"
                if entry is None
                else f"{entry['strict_successes']}/{entry['cells']} | "
                f"{money(entry['cost_usd'] / entry['cells'])}"
            )
        lines.append(f"| {task} | " + " | ".join(cells) + " |")
    if data.get("audit"):
        lines += ["", f"Audit findings: {len(data['audit'])}; see `analysis.json`."]
    return "\n".join(lines) + "\n"


def report(round_dir: Path, output: Path, pricing_file: Path = PRICING_FILE) -> dict[str, Any]:
    config = json.loads((round_dir / "config.json").read_text(encoding="utf-8"))
    pricing = json.loads(pricing_file.read_text(encoding="utf-8"))
    price = model_price(pricing, config["model"])
    summary = json.loads((round_dir / "mutation-summary.json").read_text(encoding="utf-8"))
    scored = {entry["run_id"]: entry for entry in summary["runs"]}
    audit: list[dict[str, Any]] = []
    cells: list[dict[str, Any]] = []
    requests: list[dict[str, Any]] = []
    for path in sorted((round_dir / "schedule" / "runs").glob("*.json")):
        spec = json.loads(path.read_text(encoding="utf-8"))
        row, rows = collect_cell(
            round_dir, spec, scored.get(spec["run_id"]), price, config["max_wall_seconds"], audit
        )
        cells.append(row)
        requests.extend(rows)
    data = analyze(cells)
    data["audit"] = audit
    data["round_id"] = round_dir.name
    data["model"] = config["model"]
    output.mkdir(parents=True, exist_ok=True)
    write_csv(output / "cells.csv", cells)
    write_csv(output / "requests.csv", requests)
    (output / "analysis.json").write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    (output / "pricing.json").write_text(
        json.dumps(
            {"model": config["model"], "billing_note": pricing["billing_note"],
             "classification": pricing["classification"], **price},
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    (output / "REPORT.md").write_text(
        render_report(round_dir.name, config["model"], price, data), encoding="utf-8"
    )
    return data


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--round-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, help="default: <round-dir>/report")
    parser.add_argument("--pricing", type=Path, default=PRICING_FILE)
    args = parser.parse_args()
    round_dir = args.round_dir.resolve()
    data = report(round_dir, args.output or round_dir / "report", args.pricing)
    print(json.dumps({arm: {k: entry[k] for k in ("cells", "strict_successes", "cost_usd")}
                      for arm, entry in data["arms"].items()}, indent=2))


if __name__ == "__main__":
    main()

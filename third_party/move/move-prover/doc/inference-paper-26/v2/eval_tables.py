"""Emit the per-target evaluation table and headline numbers for one round.

Usage: python3 eval_tables.py ROUND_DIR --prices prices/MODEL.json --out tables/NAME.tex

ROUND_DIR is an evaluation-artifacts round directory; each runs/<run>/ holds
run.json, judge.json, controller-events.jsonl (per-attempt usage and judge
verdicts), the runtime's event log (claude-events.jsonl or codex-events.jsonl),
flow-events.jsonl, and, once harness.score_round has run, mutation-score.json.
The price file names the runtime and the per-million rates for its usage
fields.

Every round is read under one rule: a cell's outcome is the judge's verdict at
the end of the first controller turn and its cost is what the agent spent up
to then. Rounds differed in whether the controller handed a weak contract back
for strengthening, and the agent was never told that it might, so those later
turns are not counted anywhere. Infrastructure retries restart the first turn
and are included.

A cell is complete when its first turn succeeded and the specification killed
every held-out mutant. Cells without a verdict are lost and marked; cells that
succeeded but are not yet scored are marked pending. Per-task means average
the recorded replicates, and the reported means are task-weighted.

Intervals are 95% percentile bootstrap: tasks fixed, four replicate blocks
resampled with replacement within each task, the three arms kept together
(100,000 draws, seed 20260907). They describe this fixed corpus only.

The .tex output is the complete per-target tabular; headline numbers for the
prose are printed as JSON and written next to it as NAME.json, which
eval_summary.py turns into the compact model-by-arm table.
"""

import argparse
import collections
import json
import random
import statistics
from pathlib import Path

ARMS = ("agent_only", "hybrid_guided", "hybrid_flexible")
DRAWS = 100_000
SEED = 20260907
VERIFY_TOOLS = {"move_package_verify", "move_spec_check"}


def interval(values):
    values = sorted(values)

    def quantile(p):
        position = (len(values) - 1) * p
        lower = int(position)
        upper = min(lower + 1, len(values) - 1)
        return values[lower] + (position - lower) * (values[upper] - values[lower])

    return [quantile(0.025), quantile(0.975)]


def turn_one(run):
    """Usage, verdict, and end time of the first controller turn.

    The controller may hand a weak contract back for strengthening; whether it
    did so varied between rounds and the agent was never told a retry could
    follow, so every round is read the same way: the cell's outcome is the
    judge's verdict at the end of the first controller turn, and its cost is
    what the agent spent up to then. Infrastructure retries restart the same
    turn from a snapshot and are included; strengthening turns are not.
    """
    usage = collections.Counter()
    verdict = None
    end_ms = None
    retried = False
    for line in (run / "controller-events.jsonl").open():
        event = json.loads(line)
        turn = event.get("controller_turn")
        if turn is not None and turn > 1:
            retried = True
            continue
        kind = event.get("event")
        if kind == "agent_result":
            u = event["result"].get("usage") or {}
            for key in ("input_tokens", "cache_read_input_tokens", "output_tokens"):
                usage[key] += u.get(key, 0) or 0
            creation = u.get("cache_creation") or {}
            usage["cache_write_1h"] += creation.get("ephemeral_1h_input_tokens", 0) or 0
            usage["cache_write_5m"] += creation.get("ephemeral_5m_input_tokens", 0) or 0
        elif kind == "judge_result":
            verdict = event["state"]
            end_ms = event["utc_ms"]
    return usage, verdict, end_ms, retried


def normalize(usage, runtime):
    """Name the counters the way the price file does."""
    if runtime == "claude":
        c = collections.Counter({k: usage[k] for k in ("input_tokens", "cache_read_input_tokens", "cache_write_1h", "cache_write_5m", "output_tokens")})
        c["total_input"] = c["input_tokens"] + c["cache_read_input_tokens"] + c["cache_write_1h"] + c["cache_write_5m"]
    else:
        # Codex reports cached tokens as a subset of input; price them separately.
        c = collections.Counter({
            "input_tokens": usage["input_tokens"] - usage["cache_read_input_tokens"],
            "cached_input_tokens": usage["cache_read_input_tokens"],
            "cache_write_input_tokens": 0,
            "output_tokens": usage["output_tokens"],
        })
        c["total_input"] = usage["input_tokens"]
    return c


def edit_calls(run, runtime, until_ms):
    n = 0
    if runtime == "claude":
        for line in (run / "claude-events.jsonl").open():
            event = json.loads(line)
            if event.get("event") != "claude_message" or event["utc_ms"] > until_ms:
                continue
            for block in (event.get("message") or {}).get("content") or []:
                if block.get("name") in ("Edit", "Write"):
                    n += 1
    else:
        for line in (run / "codex-events.jsonl").open():
            event = json.loads(line)
            payload = event.get("payload") or {}
            if event["utc_ms"] <= until_ms and payload.get("type") == "item.completed" and (payload.get("item") or {}).get("type") == "file_change":
                n += 1
    return n


def flow_calls(run, until_ms):
    c = collections.Counter()
    for line in (run / "flow-events.jsonl").open():
        event = json.loads(line)
        if event.get("event") == "tool_end" and event["utc_unix_ms"] <= until_ms:
            c[event["tool_name"]] += 1
    return c


def load(round_dir, prices):
    runtime = prices["runtime"]
    rates = prices["per_million"]
    cells = {}
    for run in sorted((round_dir / "runs").iterdir()):
        if not run.is_dir() or run.name.startswith("."):
            continue
        record = json.loads((run / "run.json").read_text())
        key = (record["task_id"], record["arm"], int(record["replicate"]))
        if key in cells:
            raise ValueError(f"duplicate cell {key}")
        if not (run / "judge.json").exists():
            cells[key] = None  # lost to infrastructure
            continue
        raw, verdict, end_ms, retried = turn_one(run)
        if verdict is None:
            cells[key] = None
            continue
        usage = normalize(raw, runtime)
        unknown = set(rates) - set(usage)
        if unknown:
            raise ValueError(f"{run.name}: price fields {sorted(unknown)} not in usage {sorted(usage)}")
        success = verdict == "operational_success"
        strict = None
        score = run / "mutation-score.json"
        if success and score.exists():
            s = json.loads(score.read_text())
            strict = s["killed"] == s["essential_mutants"]
        tools = flow_calls(run, end_ms)
        cells[key] = {
            "cost": sum(usage[k] * rate for k, rate in rates.items()) / 1e6,
            "input_k": usage["total_input"] / 1000,
            "output_k": usage["output_tokens"] / 1000,
            "success": success,
            "verdict": verdict,
            "retried": retried,
            "strict": strict,
            "edits": edit_calls(run, runtime, end_ms),
            "verifies": sum(tools[t] for t in VERIFY_TOOLS),
            "wp": tools["move_package_wp"],
        }
    return cells


def mark(cells):
    """Table mark for one task-arm: complete, defeated, pending, or lost."""
    lost = sum(1 for c in cells if c is None)
    present = [c for c in cells if c is not None]
    failed = sum(1 for c in present if c["strict"] is False or not c["success"])
    pending = sum(1 for c in present if c["success"] and c["strict"] is None)

    if lost:
        return r"$\circ$" if lost == 1 else rf"$\circ_{{{lost}}}$"
    if failed:
        return r"\XSolidBrush" if failed == 1 else rf"\XSolidBrush$_{{{failed}}}$"
    if pending:
        return r"$\ast$" if pending == 1 else rf"$\ast_{{{pending}}}$"
    return r"\Checkmark"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("round_dir", type=Path)
    parser.add_argument("--prices", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    prices = json.loads(args.prices.read_text())
    cells = load(args.round_dir, prices)
    tasks = sorted({k[0] for k in cells})
    replicates = sorted({k[2] for k in cells})

    lines = []
    task_samples = []
    for task in tasks:
        blocks = [[cells.get((task, arm, r)) for arm in ARMS] for r in replicates]
        # Bootstrap over replicate blocks; a block missing any arm is dropped.
        task_samples.append([tuple(c["cost"] for c in b) for b in blocks if all(b)])
        row = [f"|{task}|"]
        base = None
        for i, arm in enumerate(ARMS):
            arm_cells = [b[i] for b in blocks]
            present = [c for c in arm_cells if c]
            cost = statistics.mean(c["cost"] for c in present)
            io = f"{statistics.mean(c['input_k'] for c in present):.0f}/{statistics.mean(c['output_k'] for c in present):.1f}"
            if i == 0:
                base = cost
                row += [f"\\${cost:.3f}", io, mark(arm_cells)]
            else:
                row += [f"\\${cost:.3f}", f"{100 * cost / base:.0f}\\%", io, mark(arm_cells)]
        lines.append(" & ".join(row) + r" \\")

    rng = random.Random(SEED)
    aggregate = [[] for _ in ARMS]
    ratios = [[], []]
    for _ in range(DRAWS):
        totals = [0.0, 0.0, 0.0]
        for samples in task_samples:
            picks = [samples[rng.randrange(len(samples))] for _ in range(len(replicates))]
            for i in range(3):
                totals[i] += statistics.mean(p[i] for p in picks)
        for i in range(3):
            aggregate[i].append(totals[i] / len(tasks))
        for i in range(2):
            ratios[i].append(totals[i + 1] / totals[0])

    summary = {"round": args.round_dir.name, "model": prices["model"], "tasks": len(tasks), "replicates": len(replicates), "arms": {}}
    row = [f"\\textbf{{all {len(tasks)}}}"]
    means = []
    for i, arm in enumerate(ARMS):
        present = [c for (t, a, r), c in cells.items() if a == arm and c]
        # Task-weighted mean, the bootstrap's estimand; equals the cell mean
        # once every task has the same number of recorded replicates.
        cost = statistics.mean(
            statistics.mean(c["cost"] for (t, a, r), c in cells.items() if t == task and a == arm and c)
            for task in tasks
        )
        means.append(cost)
        io = f"{statistics.mean(c['input_k'] for c in present):.0f}/{statistics.mean(c['output_k'] for c in present):.1f}"
        scored = [c for c in present if c["strict"] is not None]
        pending = sum(1 for c in present if c["success"] and c["strict"] is None)
        complete = sum(1 for c in scored if c["success"] and c["strict"])
        ok = f"{complete}/{len(present)}" + (r"$^\ast$" if pending else "")
        summary["arms"][arm] = {
            "cells": sum(1 for (t, a, r) in cells if a == arm),
            "recorded": len(present),
            "operational": sum(c["success"] for c in present),
            "scored": len(scored),
            "pending": pending,
            "complete": complete,
            "first_attempt_failures": sum(not c["success"] for c in present),
            "verdicts": dict(collections.Counter(c["verdict"] for c in present if not c["success"])),
            "total_cost": round(sum(c["cost"] for c in present), 2),
            "mean_cost": round(cost, 4),
            "ci95_mean_cost": [round(x, 4) for x in interval(aggregate[i])],
            "mean_input_k": round(statistics.mean(c["input_k"] for c in present), 1),
            "mean_output_k": round(statistics.mean(c["output_k"] for c in present), 2),
            "edits": sum(c["edits"] for c in present),
            "verifies": sum(c["verifies"] for c in present),
            "wp_calls": sum(c["wp"] for c in present),
        }
        if i == 0:
            row += [f"\\${cost:.3f}", io, ok]
        else:
            row += [f"\\${cost:.3f}", f"{100 * cost / means[0]:.0f}\\%", io, ok]
    lines.append(r"\midrule")
    lines.append(" & ".join(row) + r" \\")

    def task_mean(task, arm):
        return statistics.mean(c["cost"] for (t, a, r), c in cells.items() if t == task and a == arm and c)

    for i, arm in enumerate(ARMS[1:]):
        summary["arms"][arm]["ratio_vs_agent"] = {
            "estimate": round(means[i + 1] / means[0], 3),
            "ci95": [round(x, 3) for x in interval(ratios[i])],
        }
        summary["arms"][arm]["cheaper_on_tasks"] = sum(
            1 for task in tasks if task_mean(task, arm) < task_mean(task, ARMS[0])
        )

    header = [
        r"\begin{tabular}{lrrcrrrcrrrc}",
        r"\toprule",
        r"& \multicolumn{3}{c}{agent-only} & \multicolumn{4}{c}{hybrid-guided} & \multicolumn{4}{c}{hybrid-flexible} \\",
        r"\cmidrule(lr){2-4} \cmidrule(lr){5-8} \cmidrule(lr){9-12}",
        r"target & cost & I/O k & ok & cost & vs. agent & I/O k & ok & cost & vs. agent & I/O k & ok \\",
        r"\midrule",
    ]
    footer = [r"\bottomrule", r"\end{tabular}"]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(
        f"% Generated by eval_tables.py from {args.round_dir.name} at {args.prices.name} prices; do not edit.\n"
        + "\n".join(header + lines + footer) + "\n"
    )
    args.out.with_suffix(".json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()

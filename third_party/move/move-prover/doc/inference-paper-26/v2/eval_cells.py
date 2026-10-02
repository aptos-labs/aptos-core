"""Emit the evaluation tables and dot-plot data from the published round archives.

Usage: python3 eval_cells.py [--results DIR]

DIR holds the compact publication archives (`harness.publication`) of the
two rounds; the default is the repository's results/etna-v3 directory.
Only each archive's cells.csv is read. It has one row per scheduled cell with
the cell's model cost at list prices, its token counters, wall time, and the
outcome of the withheld mutant gate.

Both rounds ran the same protocol: acceptance feedback only, one
controller turn, the ordinary mutant set withheld until generation ended. A
cell is complete when its contract killed every withheld mutant, failed when
one survived, and unmeasured when scoring stayed inconclusive after the
permitted infrastructure retry. Per-task means average the four replicates and
the reported means are task-weighted.

Intervals are 95% percentile bootstrap: tasks fixed, four replicate blocks
resampled with replacement within each task, the three arms kept together
(100,000 draws, seed 20260907). They describe this fixed corpus only.

Outputs, all under tables/: NAME.tex (per-target tabular), NAME.json (headline
numbers), ARM-cells.dat (one row per cell of that arm across the rounds, for
the dot plot), tasks.tex, and summary.tex.
"""

import argparse
import collections
import csv
import io
import json
import random
import statistics
import tarfile
from pathlib import Path

ROUNDS = (
    # name, archive, label
    ("sol", "corpus3.2-run11-codex-sol56-high-exact-cost", "Sol~5.6"),
    ("terra", "corpus3.2-run10-codex-terra56-high-exact-cost", "Terra~5.6"),
)
ARMS = (("agent_only", "agent-only"), ("hybrid_guided", "hybrid-guided"), ("hybrid_flexible", "hybrid-flexible"))
DRAWS = 100_000
SEED = 20260907
DEFAULT_RESULTS = Path(__file__).resolve().parents[6] / "aptos-move/flow/evaluation/spec-inference/results/etna-v3"


def read_cells(archive):
    with tarfile.open(archive) as tar:
        member = next(m for m in tar.getmembers() if m.name.endswith("/cells.csv"))
        text = tar.extractfile(member).read().decode()
    return list(csv.DictReader(io.StringIO(text)))


def cell(row):
    """One cell in round-independent form."""
    if "provider_list_cost_usd" in row:  # Claude SDK archive
        cost = float(row["provider_list_cost_usd"])
        input_tokens = sum(int(row[k]) for k in ("input_tokens", "cache_read_input_tokens", "cache_creation_5m_input_tokens", "cache_creation_1h_input_tokens"))
        output_tokens = int(row["output_tokens"])
        wall = float(row["controller_wall_ms"]) / 1000
        outcome = row["scoring_outcome"]
    else:  # Codex archive; input_tokens includes the cached subset
        cost = float(row["canonical_cost_usd"])
        input_tokens = int(row["input_tokens"])
        output_tokens = int(row["visible_output_tokens"]) + int(row["reasoning_output_tokens"])
        wall = float(row["controller_wall_seconds"])
        outcome = row.get("score_outcome") or row["outcome"]
    strict = row["strict_success"] == "True"
    status = {"scored": "complete", "disqualified": "failed", "not_scorable": "unmeasured", "inconclusive": "unmeasured"}[outcome]
    if status == "complete" and not strict:
        raise ValueError(f"{row['run_id']}: scored but not strict")
    return {
        "task": row["task_id"],
        "arm": row["arm"],
        "replicate": int(row["replicate"]),
        "cost": cost,
        "input_k": input_tokens / 1000,
        "output_k": output_tokens / 1000,
        "wall": wall,
        "status": status,
    }


def interval(values):
    values = sorted(values)

    def quantile(p):
        position = (len(values) - 1) * p
        lower = int(position)
        upper = min(lower + 1, len(values) - 1)
        return values[lower] + (position - lower) * (values[upper] - values[lower])

    return [quantile(0.025), quantile(0.975)]


def mark(cells):
    """Table mark for one task-arm: complete, failed, or unmeasured."""
    failed = sum(1 for c in cells if c["status"] == "failed")
    unmeasured = sum(1 for c in cells if c["status"] == "unmeasured")
    if failed:
        return r"\XSolidBrush" if failed == 1 else rf"\XSolidBrush$_{{{failed}}}$"
    if unmeasured:
        return r"$\circ$" if unmeasured == 1 else rf"$\circ_{{{unmeasured}}}$"
    return r"\Checkmark"


def io_column(cells):
    return f"{statistics.mean(c['input_k'] for c in cells):.0f}/{statistics.mean(c['output_k'] for c in cells):.1f}"


def emit_round(name, archive, label, model_index, out_dir, dat):
    cells = {}
    for row in read_cells(archive):
        c = cell(row)
        key = (c["task"], c["arm"], c["replicate"])
        if key in cells:
            raise ValueError(f"duplicate cell {key}")
        cells[key] = c
    tasks = sorted({k[0] for k in cells})
    replicates = sorted({k[2] for k in cells})
    arms = [a for a, _ in ARMS]
    for task in tasks:
        for arm in arms:
            for r in replicates:
                if (task, arm, r) not in cells:
                    raise ValueError(f"missing cell {(task, arm, r)}")

    def task_cells(task, arm):
        return [cells[(task, arm, r)] for r in replicates]

    def task_mean(task, arm):
        return statistics.mean(c["cost"] for c in task_cells(task, arm))

    lines = []
    for i, task in enumerate(tasks):
        row = [f"|{task}|"]
        base = None
        for j, arm in enumerate(arms):
            arm_cells = task_cells(task, arm)
            cost = task_mean(task, arm)
            for c in arm_cells:
                kind = {"complete": "ok", "failed": "fail", "unmeasured": "unm"}[c["status"]]
                dat[arm].append(f"{i} {c['cost']:.4f} {model_index} {kind}{model_index}")
            if j == 0:
                base = cost
                row += [f"\\${cost:.3f}", io_column(arm_cells), mark(arm_cells)]
            else:
                row += [f"\\${cost:.3f}", f"{100 * cost / base:.0f}\\%", io_column(arm_cells), mark(arm_cells)]
        lines.append(" & ".join(row) + r" \\")

    rng = random.Random(SEED)
    aggregate = [[] for _ in arms]
    ratios = [[], []]
    blocks = {task: [tuple(cells[(task, arm, r)]["cost"] for arm in arms) for r in replicates] for task in tasks}
    for _ in range(DRAWS):
        totals = [0.0, 0.0, 0.0]
        for task in tasks:
            picks = [blocks[task][rng.randrange(len(replicates))] for _ in range(len(replicates))]
            for j in range(3):
                totals[j] += statistics.mean(p[j] for p in picks)
        for j in range(3):
            aggregate[j].append(totals[j] / len(tasks))
        for j in range(2):
            ratios[j].append(totals[j + 1] / totals[0])

    summary = {"round": archive.name, "label": label, "tasks": len(tasks), "replicates": len(replicates), "arms": {}}
    row = [f"\\textbf{{all {len(tasks)}}}"]
    means = []
    for j, arm in enumerate(arms):
        present = [c for (t, a, r), c in cells.items() if a == arm]
        cost = statistics.mean(task_mean(task, arm) for task in tasks)
        means.append(cost)
        counts = collections.Counter(c["status"] for c in present)
        summary["arms"][arm] = {
            "cells": len(present),
            "complete": counts["complete"],
            "failed": counts["failed"],
            "unmeasured": counts["unmeasured"],
            "failed_tasks": sorted({c["task"] for c in present if c["status"] == "failed"}),
            "total_cost": round(sum(c["cost"] for c in present), 2),
            "mean_cost": round(cost, 4),
            "ci95_mean_cost": [round(x, 4) for x in interval(aggregate[j])],
            "median_cost": round(statistics.median(c["cost"] for c in present), 4),
            "max_cost": round(max(c["cost"] for c in present), 4),
            "mean_input_k": round(statistics.mean(c["input_k"] for c in present), 1),
            "mean_output_k": round(statistics.mean(c["output_k"] for c in present), 2),
            "mean_wall_s": round(statistics.mean(c["wall"] for c in present), 1),
        }
        ok = f"{counts['complete']}/{len(present)}"
        if j == 0:
            row += [f"\\${cost:.3f}", io_column(present), ok]
        else:
            row += [f"\\${cost:.3f}", f"{100 * cost / means[0]:.0f}\\%", io_column(present), ok]
    lines.append(r"\midrule")
    lines.append(" & ".join(row) + r" \\")

    for j, arm in enumerate(arms[1:]):
        summary["arms"][arm]["ratio_vs_agent"] = {
            "estimate": round(means[j + 1] / means[0], 3),
            "ci95": [round(x, 3) for x in interval(ratios[j])],
        }
        summary["arms"][arm]["cheaper_on_tasks"] = sum(1 for task in tasks if task_mean(task, arm) < task_mean(task, arms[0]))
        summary["arms"][arm]["task_ratios"] = {task: round(task_mean(task, arm) / task_mean(task, arms[0]), 2) for task in tasks}

    header = [
        r"\begin{tabular}{lrrcrrrcrrrc}",
        r"\toprule",
        r"& \multicolumn{3}{c}{agent-only} & \multicolumn{4}{c}{hybrid-guided} & \multicolumn{4}{c}{hybrid-flexible} \\",
        r"\cmidrule(lr){2-4} \cmidrule(lr){5-8} \cmidrule(lr){9-12}",
        r"target & cost & I/O k & ok & cost & vs. agent & I/O k & ok & cost & vs. agent & I/O k & ok \\",
        r"\midrule",
    ]
    footer = [r"\bottomrule", r"\end{tabular}"]
    stamp = f"% Generated by eval_cells.py from {archive.name}; do not edit.\n"
    (out_dir / f"{name}.tex").write_text(stamp + "\n".join(header + lines + footer) + "\n")
    (out_dir / f"{name}.json").write_text(json.dumps(summary, indent=2) + "\n")
    (out_dir / "tasks.tex").write_text("% Generated by eval_cells.py; do not edit.\n\\newcommand{\\CellTaskLabels}{" + ",".join(tasks) + "}\n")
    return summary


def emit_summary(summaries, out_dir):
    lines = [
        r"\begin{tabular}{llrlrrrrr}",
        r"\toprule",
        r"model & arm & cost & vs. agent [95\% CI] & complete & failed & unmeas. & I/O k & wall s \\",
        r"\midrule",
    ]
    for n, s in enumerate(summaries):
        if n:
            lines.append(r"\midrule")
        for arm, arm_name in ARMS:
            a = s["arms"][arm]
            ratio = a.get("ratio_vs_agent")
            vs = "---" if ratio is None else f"{ratio['estimate']:.2f} [{ratio['ci95'][0]:.2f}, {ratio['ci95'][1]:.2f}]"
            lines.append(
                f"{s['label'] if arm == 'agent_only' else ''} & {arm_name} & \\${a['mean_cost']:.3f} & {vs} & "
                f"{a['complete']}/{a['cells']} & {a['failed']} & {a['unmeasured']} & "
                f"{a['mean_input_k']:.0f}/{a['mean_output_k']:.1f} & {a['mean_wall_s']:.0f} \\\\"
            )
    lines += [r"\bottomrule", r"\end{tabular}"]
    (out_dir / "summary.tex").write_text("% Generated by eval_cells.py; do not edit.\n" + "\n".join(lines) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--results", type=Path, default=DEFAULT_RESULTS)
    args = parser.parse_args()
    out_dir = Path(__file__).resolve().parent / "tables"
    out_dir.mkdir(exist_ok=True)
    summaries = []
    dat = {arm: [f"# task_index cost model class  (model: {', '.join(f'{i} {label}' for i, (_, _, label) in enumerate(ROUNDS))}; class: ok/fail/unm + model)"] for arm, _ in ARMS}
    for model_index, (name, archive, label) in enumerate(ROUNDS):
        summary = emit_round(name, args.results / f"{archive}.tar.gz", label, model_index, out_dir, dat)
        summaries.append(summary)
        headline = {arm: {k: v for k, v in a.items() if k != "task_ratios"} for arm, a in summary["arms"].items()}
        print(json.dumps({"round": summary["round"], "arms": headline}, indent=2))
    emit_summary(summaries, out_dir)
    for arm, rows in dat.items():
        (out_dir / f"{arm}-cells.dat").write_text("\n".join(rows) + "\n")


if __name__ == "__main__":
    main()

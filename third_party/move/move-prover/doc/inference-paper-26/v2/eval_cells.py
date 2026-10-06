"""Emit the evaluation tables and dot-plot data from the published round archives.

Usage: python3 eval_cells.py [--results DIR] [--manifest FILE]

DIR holds the aggregate archives (`harness.publication`) of the two corpus-v4
rounds; the default is the repository's results/corpus-v4 directory. Only each
archive's cells.csv is read. It has one row per scheduled cell with the cell's
API-equivalent cost (startup warmups excluded), its token counters, wall time,
candidate-check counts, and the outcome of mutation scoring. FILE is the corpus
manifest; its feature strata mark the targets with a loop that needs an
invariant.

Both rounds ran the same protocol: acceptance feedback only, both mutant sets
withheld until generation ended. A cell is complete when its contract killed
every mutant of both sets, failed when one survived, and unmeasured when a
mutant reached no verdict. Per-task means average the four replicates and the
reported means are task-weighted.

Intervals are 95% percentile bootstraps over tasks (10,000 resamples, seed
20260907), the unit of the preregistered round analysis.

Outputs, all under tables/: NAME.tex (per-target tabular), NAME.json (headline
numbers), ARM-cells.dat (one row per cell of that arm across the rounds, for
the dot plot), tasks.tex, summary.tex, and models.json (Terra/Sol ratios).
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
    ("sol", "corpus4-run8-codex-sol56-high", "Sol~5.6"),
    ("terra", "corpus4-run8-codex-terra56-high", "Terra~5.6"),
)
ARMS = (("agent_only", "agent-only"), ("hybrid_guided", "hybrid-guided"), ("hybrid_flexible", "hybrid-flexible"))
DRAWS = 10_000
SEED = 20260907
SPEC_INFERENCE = Path(__file__).resolve().parents[6] / "aptos-move/flow/evaluation/spec-inference"
DEFAULT_RESULTS = SPEC_INFERENCE / "results/corpus-v4"
DEFAULT_MANIFEST = SPEC_INFERENCE / "corpus-v4/manifest.json"
LOOP_STRATA = {"loop", "loops"}


def read_cells(archive):
    with tarfile.open(archive) as tar:
        member = next(m for m in tar.getmembers() if m.name.endswith("/cells.csv"))
        text = tar.extractfile(member).read().decode()
    return list(csv.DictReader(io.StringIO(text)))


def cell(row):
    """One cell in round-independent form."""
    outcome = row["outcome"]
    if outcome == "scored":
        status = "complete" if row["strict_success"] == "True" else "failed"
    elif outcome == "disqualified":
        status = "failed"
    elif outcome in ("not_scorable", "inconclusive"):
        status = "unmeasured"
    else:
        raise ValueError(f"{row['run_id']}: unexpected outcome {outcome}")
    return {
        "task": row["task_id"],
        "arm": row["arm"],
        "replicate": int(row["replicate"]),
        "cost": float(row["cost_usd"]),
        "input_k": int(row["input_tokens"]) / 1000,
        "output_k": int(row["output_tokens"]) / 1000,
        "wall": float(row["wall_seconds"]),
        "first_accepted": row["first_check_accepted"] == "True",
        "rejected_checks": int(row["failed_candidate_checks"]),
        "status": status,
    }


def percent(part, whole):
    """Percentage rounded half up, as in the text."""
    return int(100 * part / whole + 0.5)


def interval(values):
    values = sorted(values)

    def quantile(p):
        position = (len(values) - 1) * p
        lower = int(position)
        upper = min(lower + 1, len(values) - 1)
        return values[lower] + (position - lower) * (values[upper] - values[lower])

    return [quantile(0.025), quantile(0.975)]


def bootstrap_ratio(tasks, numerator, denominator):
    """Ratio of task-weighted means, with its task-bootstrap interval."""
    rng = random.Random(SEED)
    draws = []
    for _ in range(DRAWS):
        picks = [tasks[rng.randrange(len(tasks))] for _ in tasks]
        draws.append(sum(numerator[t] for t in picks) / sum(denominator[t] for t in picks))
    estimate = sum(numerator[t] for t in tasks) / sum(denominator[t] for t in tasks)
    return {"estimate": round(estimate, 3), "ci95": [round(x, 3) for x in interval(draws)]}


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


def load_round(archive):
    cells = {}
    for row in read_cells(archive):
        c = cell(row)
        key = (c["task"], c["arm"], c["replicate"])
        if key in cells:
            raise ValueError(f"duplicate cell {key}")
        cells[key] = c
    return cells


def task_order(cells, loops):
    """Loop-free targets first, then targets with a loop that needs an invariant."""
    tasks = sorted({k[0] for k in cells})
    return [t for t in tasks if t not in loops] + [t for t in tasks if t in loops]


def task_means(cells, tasks, replicates, arm, field="cost"):
    return {t: statistics.mean(cells[(t, arm, r)][field] for r in replicates) for t in tasks}


def emit_round(name, archive, label, model_index, cells, tasks, loops, names, out_dir, dat):
    replicates = sorted({k[2] for k in cells})
    arms = [a for a, _ in ARMS]
    for task in tasks:
        for arm in arms:
            for r in replicates:
                if (task, arm, r) not in cells:
                    raise ValueError(f"missing cell {(task, arm, r)}")

    def task_cells(task, arm):
        return [cells[(task, arm, r)] for r in replicates]

    means = {arm: task_means(cells, tasks, replicates, arm) for arm in arms}

    lines = []
    for i, task in enumerate(tasks):
        if i and task in loops and tasks[i - 1] not in loops:
            lines.append(r"\midrule")
        row = [f"|{names[task]}|"]
        for j, arm in enumerate(arms):
            arm_cells = task_cells(task, arm)
            for c in arm_cells:
                kind = {"complete": "ok", "failed": "fail", "unmeasured": "unm"}[c["status"]]
                dat[arm].append(f"{i} {c['cost']:.4f} {model_index} {kind}{model_index}")
            cost = means[arm][task]
            if j == 0:
                row += [f"\\${cost:.3f}", io_column(arm_cells), mark(arm_cells)]
            else:
                row += [f"\\${cost:.3f}", f"{100 * cost / means[arms[0]][task]:.0f}\\%", io_column(arm_cells), mark(arm_cells)]
        lines.append(" & ".join(row) + r" \\")

    summary = {"round": archive.name, "label": label, "tasks": len(tasks), "replicates": len(replicates), "arms": {}}
    row = [f"\\textbf{{all {len(tasks)}}}"]
    groups = {"loop_free": [t for t in tasks if t not in loops], "loop": [t for t in tasks if t in loops]}
    for j, arm in enumerate(arms):
        present = [c for (t, a, r), c in cells.items() if a == arm]
        cost = statistics.mean(means[arm].values())
        counts = collections.Counter(c["status"] for c in present)
        entry = {
            "cells": len(present),
            "complete": counts["complete"],
            "failed": counts["failed"],
            "unmeasured": counts["unmeasured"],
            "failed_cells": dict(sorted(collections.Counter(c["task"] for c in present if c["status"] == "failed").items())),
            "total_cost": round(sum(c["cost"] for c in present), 2),
            "mean_cost": round(cost, 4),
            "mean_input_k": round(statistics.mean(c["input_k"] for c in present), 1),
            "mean_output_k": round(statistics.mean(c["output_k"] for c in present), 2),
            "mean_wall_s": round(statistics.mean(c["wall"] for c in present), 1),
            "first_check_accepted": sum(c["first_accepted"] for c in present),
            "rejected_checks": sum(c["rejected_checks"] for c in present),
        }
        if j:
            entry["ratio_vs_agent"] = bootstrap_ratio(tasks, means[arm], means[arms[0]])
            entry["ratio_vs_agent_by_group"] = {g: bootstrap_ratio(ts, means[arm], means[arms[0]]) for g, ts in groups.items()}
            entry["cheaper_on_tasks"] = sum(1 for t in tasks if means[arm][t] < means[arms[0]][t])
            entry["task_ratios"] = {t: round(means[arm][t] / means[arms[0]][t], 2) for t in tasks}
        summary["arms"][arm] = entry
        ok = f"{counts['complete']}/{len(present)}"
        if j == 0:
            row += [f"\\${cost:.3f}", io_column(present), ok]
        else:
            row += [f"\\${cost:.3f}", f"{100 * cost / statistics.mean(means[arms[0]].values()):.0f}\\%", io_column(present), ok]
    lines.append(r"\midrule")
    lines.append(" & ".join(row) + r" \\")

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
    return summary, means


def emit_summary(summaries, out_dir):
    lines = [
        r"\begin{tabular}{llrlrrrrrr}",
        r"\toprule",
        r"model & arm & cost & vs. agent [95\% CI] & compl. & failed & unm. & 1st & I/O k & wall s \\",
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
                f"{percent(a['first_check_accepted'], a['cells'])}\\% & "
                f"{a['mean_input_k']:.0f}/{a['mean_output_k']:.1f} & {a['mean_wall_s']:.0f} \\\\"
            )
    lines += [r"\bottomrule", r"\end{tabular}"]
    (out_dir / "summary.tex").write_text("% Generated by eval_cells.py; do not edit.\n" + "\n".join(lines) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--results", type=Path, default=DEFAULT_RESULTS)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    args = parser.parse_args()
    out_dir = Path(__file__).resolve().parent / "tables"
    out_dir.mkdir(exist_ok=True)
    records = json.loads(args.manifest.read_text())["records"]
    loops = {r["task_id"] for r in records if LOOP_STRATA & set(r["feature_strata"])}
    # The corpus documentation names a task by its target function, which is unique.
    names = {r["task_id"]: r["function"] for r in records}
    if len(set(names.values())) != len(names):
        raise ValueError("target function names are not unique")
    rounds = {name: load_round(args.results / f"{archive}.tar.gz") for name, archive, _ in ROUNDS}
    tasks = task_order(next(iter(rounds.values())), loops)
    for name, cells in rounds.items():
        if task_order(cells, loops) != tasks:
            raise ValueError(f"{name}: task set differs between rounds")
    summaries, means = [], {}
    dat = {arm: [f"# task_index cost model class  (model: {', '.join(f'{i} {label}' for i, (_, _, label) in enumerate(ROUNDS))}; class: ok/fail/unm + model)"] for arm, _ in ARMS}
    for model_index, (name, archive, label) in enumerate(ROUNDS):
        summary, means[name] = emit_round(name, args.results / f"{archive}.tar.gz", label, model_index, rounds[name], tasks, loops, names, out_dir, dat)
        summaries.append(summary)
        headline = {arm: {k: v for k, v in a.items() if k != "task_ratios"} for arm, a in summary["arms"].items()}
        print(json.dumps({"round": summary["round"], "arms": headline}, indent=2))
    emit_summary(summaries, out_dir)
    models = {
        arm: {
            "terra_vs_sol": bootstrap_ratio(tasks, means["terra"][arm], means["sol"][arm]),
            "cheaper_on_tasks": sum(1 for t in tasks if means["terra"][arm][t] < means["sol"][arm][t]),
            "task_ratios": {t: round(means["terra"][arm][t] / means["sol"][arm][t], 2) for t in tasks},
        }
        for arm, _ in ARMS
    }
    (out_dir / "models.json").write_text(json.dumps(models, indent=2) + "\n")
    print(json.dumps({arm: {k: v for k, v in m.items() if k != "task_ratios"} for arm, m in models.items()}, indent=2))
    for arm, rows in dat.items():
        (out_dir / f"{arm}-cells.dat").write_text("\n".join(rows) + "\n")
    (out_dir / "tasks.tex").write_text(
        "% Generated by eval_cells.py; do not edit.\n"
        f"\\newcommand{{\\CellTaskLabels}}{{{','.join(f'\\detokenize{{{names[t]}}}' for t in tasks)}}}\n"
        f"\\newcommand{{\\CellLoopFree}}{{{len(tasks) - len(loops)}}}\n"
    )


if __name__ == "__main__":
    main()

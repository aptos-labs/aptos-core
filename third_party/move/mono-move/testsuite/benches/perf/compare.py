#!/usr/bin/env python3
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

"""Gate mono-move benches on the A/B results produced by run.sh.

run.sh runs each side's bench binaries as several separate processes, each with its
own criterion output directory:

  <results-dir>/<side>/<process>/<bench id>/new/estimates.json

where <side> is `base` or `head`. A process contributes its criterion median
(ns/iter); a side's value is the median across its processes. A bench regresses
when head is more than `threshold_percent` slower than base. Criterion's own
confidence interval is not used: it only covers noise inside one process, not the
larger process-to-process and build-to-build variation.
"""

import argparse
import json
import statistics
import sys
import traceback
from pathlib import Path


# Verdicts. REGRESSION fails the gate; INCOMPLETE, or no PR-side results for any
# bench, is a tooling failure.
REGRESSION = "regression"
IMPROVEMENT = "improvement"
NOTABLE = "notable"
OK = "ok"
NEW = "new"
ABSENT = "absent"
WORKLOAD_CHANGED = "workload changed"
INCOMPLETE = "incomplete"

SIDES = ("base", "head")


def evaluate(cfg, results_dir, workload_changed):
    """Evaluate every configured mono bench id; return a list of result dicts."""
    results = []
    expected = cfg["processes_per_side"]
    for entry in cfg["mono_benches"]:
        bench_id = entry["id"]
        samples = {side: process_medians(results_dir / side, bench_id) for side in SIDES}
        base, head = samples["base"], samples["head"]

        base_median = statistics.median(base) if base else None
        head_median = statistics.median(head) if head else None
        delta = head_median / base_median - 1.0 if base and head else None

        if any(0 < len(samples[side]) < expected for side in SIDES):
            # Some processes ran the bench but left no estimates (criterion only logs
            # write failures), so the median rests on fewer processes than intended.
            verdict = INCOMPLETE
        elif not head:
            # Configured bench produced no results on the PR (removed, renamed, or not run).
            verdict = ABSENT
        elif not base:
            verdict = NEW
        elif bench_id in workload_changed:
            verdict = WORKLOAD_CHANGED
        else:
            verdict = classify(delta, cfg)

        results.append({
            "id": bench_id,
            "verdict": verdict,
            "delta": delta,
            "base_median_ns": base_median,
            "head_median_ns": head_median,
            "base_spread": spread(base),
            "head_spread": spread(head),
            "base_processes": len(base),
            "head_processes": len(head),
        })
    return results


def classify(delta, cfg):
    """Verdict for a relative change `delta` (head / base - 1)."""
    if delta > cfg["threshold"]:
        return REGRESSION
    if delta < -cfg["threshold"]:
        return IMPROVEMENT
    if abs(delta) >= cfg["notable"]:
        return NOTABLE
    return OK


def process_medians(side_dir, bench_id):
    """Criterion median (ns/iter) of `bench_id` from each process run of one side."""
    medians = []
    for process_dir in sorted(side_dir.glob("*")):
        estimates = process_dir / bench_id / "new" / "estimates.json"
        if estimates.exists():
            with open(estimates) as handle:
                medians.append(json.load(handle)["median"]["point_estimate"])
    return medians


def spread(samples):
    """(max - min) / median of per-process samples, or None."""
    if len(samples) < 2:
        return None
    return (max(samples) - min(samples)) / statistics.median(samples)


VERDICT_CELL = {
    REGRESSION: "**regression**",
    IMPROVEMENT: "improved",
    NOTABLE: "notable",
    OK: "ok",
    NEW: "new",
    ABSENT: "absent",
    WORKLOAD_CHANGED: "workload changed",
    INCOMPLETE: "**incomplete**",
}


def render_markdown(results, cfg, base_sha, head_sha):
    counts = {verdict: sum(1 for result in results if result["verdict"] == verdict)
              for verdict in VERDICT_CELL}
    threshold = cfg["threshold_percent"]
    notable = cfg["notable_percent"]
    processes = cfg["processes_per_side"]
    gated = sum(counts[verdict] for verdict in (REGRESSION, IMPROVEMENT, NOTABLE, OK))

    problems = []
    if counts[REGRESSION]:
        problems.append(f"{counts[REGRESSION]} gated bench(es) more than {threshold:g}% "
                        "slower than base")
    if counts[INCOMPLETE]:
        problems.append(f"{counts[INCOMPLETE]} bench(es) missing results from some processes")
    if no_head_results(results):
        problems.append("no bench produced results on the PR, so nothing was measured")
    if problems:
        headline = "; ".join(problems)
        headline = headline[0].upper() + headline[1:]
    elif not gated:
        headline = "No bench was gated: none has results on both sides with an unchanged workload"
    else:
        headline = f"No gated bench more than {threshold:g}% slower than base"

    summary = " · ".join(f"{counts[verdict]} {VERDICT_CELL[verdict].strip('*')}"
                         for verdict in (REGRESSION, OK, NOTABLE, IMPROVEMENT, NEW, ABSENT,
                                         WORKLOAD_CHANGED, INCOMPLETE)
                         if counts[verdict] or verdict == OK)
    lines = [
        "### mono-move benchmark gate",
        "",
        f"base `{base_sha[:10]}` → PR `{head_sha[:10]}`: {headline}",
        "",
        f"`{summary}` (fails above +{threshold:g}%, notable beyond ±{notable:g}%; "
        f"median of {processes} alternating processes per side)",
        "",
        "| Benchmark | Δ | median (base → PR) | process spread (base / PR) | Verdict |",
        "| --- | ---: | :---: | :---: | :--- |",
    ]
    for result in results:
        delta_cell = "n/a" if result["delta"] is None else fmt_pct(result["delta"])
        median_cell = f"{fmt_ns(result['base_median_ns'])} → {fmt_ns(result['head_median_ns'])}"
        short = [f"{side} {result[side + '_processes']}/{processes}" for side in SIDES
                 if 0 < result[side + "_processes"] < processes]
        if short:
            median_cell += f" ({', '.join(short)} processes)"
        spread_cell = f"{fmt_spread(result['base_spread'])} / {fmt_spread(result['head_spread'])}"
        lines.append(
            f"| `{result['id']}` | {delta_cell} | {median_cell} | {spread_cell} | "
            f"{VERDICT_CELL[result['verdict']]} |"
        )
    lines.append("")
    lines.append(
        "> Timings come from a bench build with every function, and every block not entered "
        "by fall-through, aligned to 64 bytes, so they are not production numbers. Only "
        "the change between base and PR is meaningful, and code layout alone can still "
        "move it by a few percent, occasionally more."
    )
    lines.append("")
    if counts[WORKLOAD_CHANGED]:
        lines.append(
            "> `workload changed` means this PR edits the bench, its program wrapper, or its "
            "Move program, so base and PR measure different work. The delta is shown but "
            "not gated."
        )
        lines.append("")
    if counts[INCOMPLETE]:
        lines.append(
            "> `incomplete` means some processes ran the bench but produced no criterion "
            "results for it. The run is treated as a tooling failure; see the job logs."
        )
        lines.append("")
    if counts[ABSENT]:
        lines.append(
            "> `absent` means a configured bench produced no results on the PR (removed, "
            "renamed, or not run). Update `benches/perf/config.json` if a bench was renamed; "
            "an id's criterion group must match its bench file name."
        )
        lines.append("")
    return "\n".join(lines) + "\n"


def emit_json_lines(results, cfg):
    for result in results:
        line = {
            "grep": "grep_json_mono_move_bench",
            "id": result["id"],
            "verdict": result["verdict"],
            "delta_pct": None if result["delta"] is None else result["delta"] * 100.0,
            "base_median_ns": result["base_median_ns"],
            "head_median_ns": result["head_median_ns"],
            "base_processes": result["base_processes"],
            "head_processes": result["head_processes"],
            "threshold_percent": cfg["threshold_percent"],
        }
        print(json.dumps(line))


def cmd_ab(args, cfg):
    workload_changed = {bench_id for bench_id in args.workload_changed.split(",") if bench_id}
    results = evaluate(cfg, Path(args.results_dir), workload_changed)
    markdown = render_markdown(results, cfg, args.base_sha, args.head_sha)

    if args.out:
        with open(args.out, "w") as handle:
            handle.write(markdown)
    emit_json_lines(results, cfg)
    # Human-readable copy to stderr so it shows in CI logs regardless of `--out`.
    print(markdown, file=sys.stderr)

    verdicts = {result["verdict"] for result in results}
    if REGRESSION in verdicts:
        return 1
    if INCOMPLETE in verdicts or no_head_results(results):
        return 2
    return 0


def no_head_results(results):
    """Whether no configured bench produced any PR-side results, so the gate is off."""
    return all(result["head_processes"] == 0 for result in results)


def load_config(path):
    with open(path) as handle:
        cfg = json.load(handle)
    # Percentages in the file; ratios internally.
    cfg["threshold"] = cfg["threshold_percent"] / 100.0
    cfg["notable"] = cfg["notable_percent"] / 100.0
    return cfg


def fmt_pct(ratio):
    return f"{ratio * 100:+.1f}%"


def fmt_spread(ratio):
    return "n/a" if ratio is None else f"{ratio * 100:.1f}%"


def fmt_ns(ns):
    if ns is None:
        return "n/a"
    if ns < 1_000.0:
        return f"{ns:.1f}ns"
    if ns < 1_000_000.0:
        return f"{ns / 1_000.0:.2f}µs"
    if ns < 1_000_000_000.0:
        return f"{ns / 1_000_000.0:.2f}ms"
    return f"{ns / 1_000_000_000.0:.2f}s"


def main():
    parser = argparse.ArgumentParser(description="mono-move bench comparator")
    sub = parser.add_subparsers(dest="cmd", required=True)

    ab = sub.add_parser("ab", help="gate an A/B run from run.sh; exit 1 on regression")
    ab.add_argument("--results-dir", required=True,
                    help="directory holding base/<process>/ and head/<process>/ criterion output")
    ab.add_argument("--base-sha", required=True, help="commit measured as base")
    ab.add_argument("--head-sha", required=True, help="commit measured as the PR")
    ab.add_argument("--workload-changed", default="",
                    help="comma-separated gated ids whose workload differs between base and head")
    ab.add_argument("--config", default=str(Path(__file__).parent / "config.json"),
                    help="path to config.json")
    ab.add_argument("--out", default=None, help="write the markdown report here")

    args = parser.parse_args()
    # Exit 1 means a regression; any tooling failure maps to 2, like a failed bench.
    try:
        cfg = load_config(args.config)
        status = cmd_ab(args, cfg)
    except Exception:
        traceback.print_exc()
        sys.exit(2)
    sys.exit(status)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

"""Compare mono-move criterion benches against a baseline and gate on regressions.

This reads the JSON criterion writes under `target/criterion/<id>/`:
  - `change/estimates.json` exists after a `--baseline <name>` run and holds the
    relative change of this run vs the baseline. Its `mean`/`median` values are
    ratios (`new/old - 1`), so `+0.05` means 5% slower.
  - `new/estimates.json` holds the current run's absolute estimates, in
    nanoseconds per iteration.

The verdict uses the `mean` estimate's confidence interval: a regression needs the
whole CI above the noise threshold `T`, an improvement needs it below `-T`,
otherwise the change is within noise.
"""

import argparse
import json
import math
import sys
from pathlib import Path


# Verdicts. Only REGRESSION fails the gate.
REGRESSION = "regression"
IMPROVEMENT = "improvement"
NOISE = "noise"
NEW = "new"
ABSENT = "absent"


def load_config(path):
    with open(path) as f:
        cfg = json.load(f)
    # `threshold_pct` is a percentage (3.0 == 3%); the CI bounds are ratios.
    cfg["threshold"] = cfg["threshold_pct"] / 100.0
    return cfg


def read_estimates(path):
    if not path.exists():
        return None
    with open(path) as f:
        return json.load(f)


def fmt_pct(ratio):
    return f"{ratio * 100:+.1f}%"


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


def classify(change, threshold):
    """Verdict for a change `mean` estimate: regression if the whole CI is above
    `threshold`, improvement if below `-threshold`, else noise.
    """
    ci = change["mean"]["confidence_interval"]
    lo = ci["lower_bound"]
    hi = ci["upper_bound"]
    if lo > threshold:
        return REGRESSION
    if hi < -threshold:
        return IMPROVEMENT
    return NOISE


def evaluate(cfg, criterion_dir):
    """Evaluate every configured mono bench id; return a list of result dicts."""
    threshold = cfg["threshold"]
    results = []
    for entry in cfg["mono_benches"]:
        bench_id = entry["id"]
        id_dir = criterion_dir / bench_id
        change = read_estimates(id_dir / "change" / "estimates.json")
        new = read_estimates(id_dir / "new" / "estimates.json")
        base = read_estimates(id_dir / "main" / "estimates.json")

        new_median = new["median"]["point_estimate"] if new else None
        base_median = base["median"]["point_estimate"] if base else None

        if change is not None:
            verdict = classify(change, threshold)
            mean_pct = change["mean"]["point_estimate"]
            ci = change["mean"]["confidence_interval"]
            ci_lo, ci_hi = ci["lower_bound"], ci["upper_bound"]
        elif new is not None:
            # Ran but no baseline to compare against -- a bench new to this PR.
            verdict = NEW
            mean_pct = ci_lo = ci_hi = None
        else:
            # Configured bench produced no results (removed, renamed, or not run).
            verdict = ABSENT
            mean_pct = ci_lo = ci_hi = None

        results.append({
            "id": bench_id,
            "verdict": verdict,
            "mean_pct": mean_pct,
            "ci_lo": ci_lo,
            "ci_hi": ci_hi,
            "base_median_ns": base_median,
            "new_median_ns": new_median,
        })
    return results


VERDICT_CELL = {
    REGRESSION: "regression",
    IMPROVEMENT: "improved",
    NOISE: "ok",
    NEW: "new",
    ABSENT: "absent",
}


def render_markdown(results, cfg):
    threshold_pct = cfg["threshold_pct"]
    n_reg = sum(1 for r in results if r["verdict"] == REGRESSION)
    n_imp = sum(1 for r in results if r["verdict"] == IMPROVEMENT)
    n_ok = sum(1 for r in results if r["verdict"] == NOISE)
    n_new = sum(1 for r in results if r["verdict"] == NEW)
    n_absent = sum(1 for r in results if r["verdict"] == ABSENT)

    if n_reg:
        headline = f"{n_reg} regression(s) beyond ±{threshold_pct:g}% noise band"
    else:
        headline = f"No regressions beyond ±{threshold_pct:g}% noise band"

    lines = [
        "### mono-move benchmark gate",
        "",
        headline,
        "",
        f"`{n_ok} ok · {n_imp} improved · {n_new} new · {n_absent} absent` "
        f"(threshold T = ±{threshold_pct:g}%, criterion mean CI vs `main`)",
        "",
        "| Benchmark | mean Δ | 95% CI | median (main → PR) | Verdict |",
        "| --- | ---: | :---: | :---: | :--- |",
    ]
    for r in results:
        if r["mean_pct"] is None:
            mean_cell = "n/a"
            ci_cell = "n/a"
        else:
            mean_cell = fmt_pct(r["mean_pct"])
            ci_cell = f"[{fmt_pct(r['ci_lo'])}, {fmt_pct(r['ci_hi'])}]"
        median_cell = f"{fmt_ns(r['base_median_ns'])} → {fmt_ns(r['new_median_ns'])}"
        lines.append(
            f"| `{r['id']}` | {mean_cell} | {ci_cell} | {median_cell} | "
            f"{VERDICT_CELL[r['verdict']]} |"
        )
    lines.append("")
    if n_imp:
        lines.append(
            "> Improvements are not failures. `main` rebaselines on merge, so the "
            "next PR compares against the faster code automatically."
        )
        lines.append("")
    if n_absent:
        lines.append(
            "> `absent` means a configured bench produced no results (removed, or "
            "not run). Update `benches/perf/config.json` if a bench was renamed."
        )
        lines.append("")
    return "\n".join(lines) + "\n"


def emit_json_lines(results, cfg):
    for r in results:
        line = {
            "grep": "grep_json_mono_move_bench",
            "id": r["id"],
            "verdict": r["verdict"],
            "mean_pct": None if r["mean_pct"] is None else r["mean_pct"] * 100.0,
            "ci_lo_pct": None if r["ci_lo"] is None else r["ci_lo"] * 100.0,
            "ci_hi_pct": None if r["ci_hi"] is None else r["ci_hi"] * 100.0,
            "base_median_ns": r["base_median_ns"],
            "new_median_ns": r["new_median_ns"],
            "threshold_pct": cfg["threshold_pct"],
        }
        print(json.dumps(line))


def cmd_ab(args, cfg):
    criterion_dir = Path(args.criterion_dir)
    results = evaluate(cfg, criterion_dir)
    markdown = render_markdown(results, cfg)

    if args.out:
        with open(args.out, "w") as f:
            f.write(markdown)
    emit_json_lines(results, cfg)
    # Human-readable copy to stderr so it shows in CI logs regardless of `--out`.
    print(markdown, file=sys.stderr)

    regressions = [r for r in results if r["verdict"] == REGRESSION]
    return 1 if regressions else 0


def cmd_calibrate_noise(args, cfg):
    """Report the runner's observed noise floor from a `main`-vs-`main` A/B.

    With identical code on both sides every change should be ~0; the largest CI
    bound magnitude across benches is the floor `T` must sit above.
    """
    criterion_dir = Path(args.criterion_dir)
    floor = 0.0
    print(f"{'benchmark':<28} {'mean Δ':>9} {'|CI| max':>9}")
    for entry in cfg["mono_benches"]:
        bench_id = entry["id"]
        change = read_estimates(criterion_dir / bench_id / "change" / "estimates.json")
        if change is None:
            print(f"{bench_id:<28} {'n/a':>9} {'n/a':>9}")
            continue
        ci = change["mean"]["confidence_interval"]
        ci_max = max(abs(ci["lower_bound"]), abs(ci["upper_bound"]))
        floor = max(floor, ci_max)
        print(f"{bench_id:<28} {fmt_pct(change['mean']['point_estimate']):>9} "
              f"{fmt_pct(ci_max):>9}")
    # Suggest T as the floor rounded up to the next 0.5%, plus a 0.5% margin.
    floor_pct = floor * 100.0
    suggested = math.ceil(floor_pct / 0.5) * 0.5 + 0.5
    print()
    print(f"observed noise floor: {floor_pct:.2f}%")
    print(f"suggested threshold_pct: {suggested:.1f}  (set in config.json)")
    return 0


def main():
    # Parent parser so these options are accepted after the subcommand.
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument(
        "--criterion-dir", default="target/criterion",
        help="criterion output directory (default: target/criterion)",
    )
    common.add_argument(
        "--config", default=str(Path(__file__).parent / "config.json"),
        help="path to config.json",
    )

    parser = argparse.ArgumentParser(description="mono-move bench comparator")
    sub = parser.add_subparsers(dest="cmd", required=True)

    ab = sub.add_parser("ab", parents=[common],
                        help="gate a PR A/B run; exit 1 on regression")
    ab.add_argument("--out", default=None, help="write the markdown report here")

    sub.add_parser("calibrate-noise", parents=[common],
                   help="report the runner noise floor")

    args = parser.parse_args()
    cfg = load_config(args.config)

    if args.cmd == "ab":
        sys.exit(cmd_ab(args, cfg))
    if args.cmd == "calibrate-noise":
        sys.exit(cmd_calibrate_noise(args, cfg))
    parser.error(f"unknown command {args.cmd}")


if __name__ == "__main__":
    main()

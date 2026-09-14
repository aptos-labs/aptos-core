#!/usr/bin/env python3
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

"""Compare mono-move criterion benches against a baseline and gate on regressions.

Every mono bench is measured several ways; `config.json` lists the metrics and
names the one that gates. criterion writes each measurement under
`target/criterion/<bench id>/<metric>/`:
  - `change/estimates.json` exists after a `--baseline <name>` run and holds the
    relative change of this run vs the baseline. Its `mean`/`median` values are
    ratios (`new/old - 1`), so `+0.05` means 5% more.
  - `new/estimates.json` and `<baseline>/estimates.json` hold absolute
    per-iteration estimates: nanoseconds for `time`, plain counts otherwise.

The verdict uses the gate metric's `mean` confidence interval: a regression
needs the whole CI above the noise threshold `T`, an improvement needs it below
`-T`, otherwise the change is within noise. The other metrics are shown for
context and never fail the gate.
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

BASELINE = "main"


def load_config(path):
    with open(path) as f:
        cfg = json.load(f)
    if cfg["gate_metric"] not in cfg["metrics"]:
        sys.exit(f"config: gate_metric {cfg['gate_metric']!r} is not in metrics {cfg['metrics']}")
    # `threshold_percent` is a percentage (1.0 == 1%); the CI bounds are ratios.
    cfg["threshold"] = cfg["threshold_percent"] / 100.0
    return cfg


def read_estimates(path):
    if not path.exists():
        return None
    with open(path) as f:
        return json.load(f)


def fmt_pct(ratio):
    return f"{ratio * 100:+.2f}%"


def fmt_ns(ns):
    if ns < 1_000.0:
        return f"{ns:.1f}ns"
    if ns < 1_000_000.0:
        return f"{ns / 1_000.0:.2f}µs"
    if ns < 1_000_000_000.0:
        return f"{ns / 1_000_000.0:.2f}ms"
    return f"{ns / 1_000_000_000.0:.2f}s"


def fmt_count(count):
    if count < 1_000.0:
        return f"{count:.0f}"
    if count < 1_000_000.0:
        return f"{count / 1_000.0:.2f}K"
    if count < 1_000_000_000.0:
        return f"{count / 1_000_000.0:.3f}M"
    return f"{count / 1_000_000_000.0:.3f}G"


def fmt_value(metric, value):
    if value is None:
        return "n/a"
    return fmt_ns(value) if metric == "time" else fmt_count(value)


def read_metric(criterion_dir, bench_id, metric):
    """One bench's estimates for one metric, or None if it produced no results."""
    metric_dir = criterion_dir / bench_id / metric
    new = read_estimates(metric_dir / "new" / "estimates.json")
    if new is None:
        return None
    base = read_estimates(metric_dir / BASELINE / "estimates.json")
    change = read_estimates(metric_dir / "change" / "estimates.json")
    result = {
        "new_median": new["median"]["point_estimate"],
        "base_median": base["median"]["point_estimate"] if base else None,
        "mean_pct": None,
        "ci_lo": None,
        "ci_hi": None,
    }
    if change is not None:
        ci = change["mean"]["confidence_interval"]
        result["mean_pct"] = change["mean"]["point_estimate"]
        result["ci_lo"] = ci["lower_bound"]
        result["ci_hi"] = ci["upper_bound"]
    return result


def classify(metric, threshold):
    """Verdict for a metric's change: regression if the whole CI is above
    `threshold`, improvement if below `-threshold`, else noise.
    """
    if metric["ci_lo"] > threshold:
        return REGRESSION
    if metric["ci_hi"] < -threshold:
        return IMPROVEMENT
    return NOISE


def evaluate(cfg, criterion_dir):
    """Evaluate every configured mono bench id; return a list of result dicts."""
    results = []
    for entry in cfg["mono_benches"]:
        bench_id = entry["id"]
        metrics = {m: read_metric(criterion_dir, bench_id, m) for m in cfg["metrics"]}
        gate = metrics[cfg["gate_metric"]]
        if gate is None:
            # Configured bench produced no results (removed, renamed, not run,
            # or the counter could not be opened).
            verdict = ABSENT
        elif gate["mean_pct"] is None:
            # Ran but no baseline to compare against -- a bench new to this PR.
            verdict = NEW
        else:
            verdict = classify(gate, cfg["threshold"])
        results.append({"id": bench_id, "verdict": verdict, "metrics": metrics})
    return results


VERDICT_CELL = {
    REGRESSION: "regression",
    IMPROVEMENT: "improved",
    NOISE: "ok",
    NEW: "new",
    ABSENT: "absent",
}


def fmt_medians(metric, m, with_delta):
    if m is None:
        return "n/a"
    cell = f"{fmt_value(metric, m['base_median'])} → {fmt_value(metric, m['new_median'])}"
    if with_delta and m["mean_pct"] is not None:
        cell += f" ({fmt_pct(m['mean_pct'])})"
    return cell


def render_markdown(results, cfg):
    gate_metric = cfg["gate_metric"]
    threshold_percent = cfg["threshold_percent"]
    others = [m for m in cfg["metrics"] if m != gate_metric]
    n_reg = sum(1 for r in results if r["verdict"] == REGRESSION)
    n_imp = sum(1 for r in results if r["verdict"] == IMPROVEMENT)
    n_ok = sum(1 for r in results if r["verdict"] == NOISE)
    n_new = sum(1 for r in results if r["verdict"] == NEW)
    n_absent = sum(1 for r in results if r["verdict"] == ABSENT)

    if n_reg:
        headline = f"{n_reg} regression(s): `{gate_metric}` beyond ±{threshold_percent:g}%"
    else:
        headline = f"No regressions: `{gate_metric}` within ±{threshold_percent:g}%"

    others_list = ", ".join(f"`{m}`" for m in others)
    lines = [
        "### mono-move benchmark gate",
        "",
        headline,
        "",
        f"`{n_ok} ok · {n_imp} improved · {n_new} new · {n_absent} absent` "
        f"(gate: criterion mean CI of `{gate_metric}` per iteration vs `{BASELINE}`, "
        f"T = ±{threshold_percent:g}%; {others_list} are informational)",
        "",
        f"| Benchmark | {gate_metric} Δ | 95% CI | {gate_metric} ({BASELINE} → PR) | "
        + " | ".join(f"{m} ({BASELINE} → PR)" for m in others)
        + " | Verdict |",
        "| --- | ---: | :---: | :---: | " + " | ".join(":---:" for _ in others) + " | :--- |",
    ]
    for r in results:
        gate = r["metrics"][gate_metric]
        if gate is None or gate["mean_pct"] is None:
            delta_cell = "n/a"
            ci_cell = "n/a"
        else:
            delta_cell = fmt_pct(gate["mean_pct"])
            ci_cell = f"[{fmt_pct(gate['ci_lo'])}, {fmt_pct(gate['ci_hi'])}]"
        cells = [f"`{r['id']}`", delta_cell, ci_cell, fmt_medians(gate_metric, gate, False)]
        cells += [fmt_medians(m, r["metrics"][m], True) for m in others]
        cells.append(VERDICT_CELL[r["verdict"]])
        lines.append("| " + " | ".join(cells) + " |")
    lines.append("")
    if n_imp:
        lines.append(
            f"> Improvements are not failures. `{BASELINE}` rebaselines on merge, so the "
            "next PR compares against the improved code automatically."
        )
        lines.append("")
    if n_absent:
        lines.append(
            f"> `absent` means `{gate_metric}` produced no results for a configured bench "
            "(removed, not run, or the hardware counter could not be opened). Update "
            "`benches/perf/config.json` if a bench was renamed."
        )
        lines.append("")
    return "\n".join(lines) + "\n"


def emit_json_lines(results, cfg):
    for r in results:
        line = {
            "grep": "grep_json_mono_move_bench",
            "id": r["id"],
            "verdict": r["verdict"],
            "gate_metric": cfg["gate_metric"],
            "threshold_percent": cfg["threshold_percent"],
        }
        for metric, m in r["metrics"].items():
            if m is None:
                line[metric] = None
                continue
            line[metric] = {
                "mean_pct": None if m["mean_pct"] is None else m["mean_pct"] * 100.0,
                "ci_lo_pct": None if m["ci_lo"] is None else m["ci_lo"] * 100.0,
                "ci_hi_pct": None if m["ci_hi"] is None else m["ci_hi"] * 100.0,
                "base_median": m["base_median"],
                "new_median": m["new_median"],
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
    bound magnitude across benches is the floor `T` must sit above. Every
    metric is reported, but only the gate metric drives the suggestion.
    """
    criterion_dir = Path(args.criterion_dir)
    metrics = cfg["metrics"]
    floors = {m: 0.0 for m in metrics}
    print(f"{'benchmark':<28}" + "".join(f" {m + ' |CI| max':>22}" for m in metrics))
    for entry in cfg["mono_benches"]:
        row = f"{entry['id']:<28}"
        for metric in metrics:
            m = read_metric(criterion_dir, entry["id"], metric)
            if m is None or m["mean_pct"] is None:
                row += f" {'n/a':>22}"
                continue
            ci_max = max(abs(m["ci_lo"]), abs(m["ci_hi"]))
            floors[metric] = max(floors[metric], ci_max)
            row += f" {fmt_pct(ci_max):>22}"
        print(row)
    # Suggest T as the floor rounded up to the next 0.5%, plus a 0.5% margin.
    gate_metric = cfg["gate_metric"]
    floor_pct = floors[gate_metric] * 100.0
    suggested = math.ceil(floor_pct / 0.5) * 0.5 + 0.5
    print()
    print(f"observed {gate_metric} noise floor: {floor_pct:.3f}%")
    print(f"suggested threshold_percent: {suggested:.1f}  (set in config.json)")
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

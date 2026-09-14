#!/usr/bin/env python

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

"""Calibrated speedup bands for the mono-move e2e performance job.

The calibrated quantity is a speedup ratio -- MonoMove throughput over legacy
MoveVM throughput on the same recorded blocks -- not an absolute TPS. Machine
speed cancels out of a ratio, so these numbers stay meaningful across runner
changes in a way absolute TPS does not.

Imported by run_e2e_perf_test.py for `speedup_band` and `load_calibration`, and
run directly to recalibrate.
"""

import argparse
import datetime
import json
import os
import re
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
TSV_PATH = os.path.join(HERE, "e2e_perf_speedup.tsv")

# The Humio grep key and the CI job name. The job name is what the query filters
# on, so renaming the job in the workflow empties the calibration history.
GREP_KEY = "grep_json_mono_move_e2e_perf"
JOB_NAME = "mono-move-e2e-perf"

# A drift backed by fewer than this many runs in the query window is too noisy to
# act on, so the row keeps its calibrated value until enough runs accumulate.
MIN_RECALIBRATION_SAMPLES = 5

# Column order of e2e_perf_speedup.tsv. The first two are the key.
COLUMNS = [
    "workload",
    "metric",
    "num_samples",
    "lowest_over_median",
    "highest_over_median",
    "median_speedup",
]
KEY_COLUMNS = 2

# Metrics that get a calibrated row. `execution` is the one verdicts are read
# from; the rest are recorded so drift in them is visible.
CALIBRATED_METRICS = ["total", "execution", "inner_block_executor", "output_bytes_per_txn"]


def speedup_band(median_speedup, num_samples, lowest_over_median, highest_over_median):
    """Band a new speedup must fall in to count as unchanged.

    Widens the observed spread by a factor that shrinks as samples accumulate, so
    a thinly sampled workload gets a forgiving band. Same formula as `tps_band` in
    testsuite/single_node_performance_calibration.py, applied to a ratio.
    """
    widen = 1 + 10.0 / num_samples
    slack = 1.0 / num_samples
    low = median_speedup * (1 - (1 - lowest_over_median) * widen - slack)
    high = median_speedup * (1 + (highest_over_median - 1) * widen + slack)
    return low, high


def load_calibration(path=TSV_PATH):
    """Read e2e_perf_speedup.tsv into {(workload, metric): dict}.

    Columns are read by name from the header comment, so adding a column later
    does not silently shift every row.
    """
    rows = {}
    if not os.path.exists(path):
        return rows
    header = COLUMNS
    with open(path) as f:
        for line in f:
            line = line.rstrip("\n")
            if not line.strip():
                continue
            if line.startswith("#"):
                names = line.lstrip("#").split()
                if len(names) >= len(COLUMNS):
                    header = names
                continue
            cells = line.split("\t")
            if len(cells) != len(header):
                print(f"Skipping malformed calibration row: {line!r}")
                continue
            row = dict(zip(header, cells))
            rows[(row["workload"], row["metric"])] = {
                "num_samples": int(row["num_samples"]),
                "lowest_over_median": float(row["lowest_over_median"]),
                "highest_over_median": float(row["highest_over_median"]),
                "median_speedup": float(row["median_speedup"]),
            }
    return rows


def humio_secret():
    token = os.environ.get("HUMIO_READ_TOKEN", "").strip()
    if token:
        return token

    print(
        "trying to get a humio secret from gcloud. if it asks for a password, abort "
        "and run `gcloud auth login --update-adc` first"
    )
    import subprocess

    return subprocess.run(
        [
            "gcloud",
            "secrets",
            "versions",
            "access",
            "--secret=ci_humio_read_token",
            "--project=aptos-shared-secrets",
            "latest",
        ],
        capture_output=True,
    ).stdout.decode("utf-8")


def query_humio(query_string, time_interval):
    import requests

    resp = requests.post(
        url="https://cloud.us.humio.com/api/v1/repositories/github/query",
        json={"queryString": query_string, "start": time_interval},
        headers={
            "Authorization": f"Bearer {humio_secret()}",
            "Content-Type": "application/json",
        },
    )
    return resp.text.strip()


def humio_query(branch):
    if branch is not None:
        prefix = f"""
        github.job.name = "{JOB_NAME}"
        | github.workflow.head_branch = "{branch}"
        | "{GREP_KEY}"
        | parseJson(message)
        """
    else:
        prefix = f"""
        github.job.name = "{JOB_NAME}"
        | github.workflow.head_branch = "main"
        | "{GREP_KEY}"
        | parseJson(message)
        """
    return (
        prefix
        + """
        | groupBy([workload, metric, code_perf_version], function=[count(as="num_samples"), min(speedup, as="min_speedup"), max(speedup, as="max_speedup"), percentile(field=speedup, accuracy=0.001, percentiles=[50])])
        | lowest_over_median := min_speedup / _50
        | highest_over_median := max_speedup / _50
        | format("%.3f", field=_50, as="median_speedup")
        | format("%.3f", field=lowest_over_median, as="lowest_over_median")
        | format("%.3f", field=highest_over_median, as="highest_over_median")
        | table([workload, metric, num_samples, lowest_over_median, highest_over_median, median_speedup])
        """
    )


def rows_from_humio(branch, time_interval):
    response = query_humio(humio_query(branch), time_interval)
    rows = []
    for line in response.split("\n"):
        if not line.strip():
            continue
        row = {}
        for key_value in line.strip().split(", "):
            parts = key_value.split("->")
            if len(parts) == 2:
                row[parts[0]] = parts[1]
        if all(c in row for c in COLUMNS):
            rows.append(row)
    return rows


def rows_from_jsonl(paths):
    """Aggregate the runner's own JSON lines, computing what Humio would.

    This is the bootstrap path -- no history exists until the job has run a few
    times -- and the fallback if Humio ingestion for the job name does not work.
    """
    samples = defaultdict(list)
    for path in paths:
        with open(path) as f:
            for line in f:
                line = line.strip()
                if not line.startswith("{"):
                    continue
                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if record.get("grep") != GREP_KEY:
                    continue
                samples[(record["workload"], record["metric"])].append(
                    float(record["speedup"])
                )

    rows = []
    for (workload, metric), values in sorted(samples.items()):
        values.sort()
        median = values[len(values) // 2]
        rows.append(
            {
                "workload": workload,
                "metric": metric,
                "num_samples": str(len(values)),
                "lowest_over_median": f"{values[0] / median:.3f}",
                "highest_over_median": f"{values[-1] / median:.3f}",
                "median_speedup": f"{median:.3f}",
            }
        )
    return rows


def changelog_path(tsv_path):
    return tsv_path[: -len(".tsv")] + "_changelog.md"


def changelog_header():
    return (
        "# mono-move e2e performance calibration log\n\n"
        "Recalibration history, newest first. Each entry lists the workloads whose "
        "calibrated speedup drifted out of band, as `old -> new`; new rows show `new`. "
        "A speedup is MonoMove throughput over legacy MoveVM throughput on the same "
        "recorded blocks, so a number above 1.00x means MonoMove is faster.\n"
    )


def format_changelog_entry(date_str, triggers, unparseable):
    lines = [f"## {date_str}", ""]
    if not triggers:
        lines.append(f"_Refreshed; {unparseable} unparseable row(s) forced an update._")
        lines.append("")
        return "\n".join(lines) + "\n"

    lines.append("| workload | metric | runs | speedup |")
    lines.append("| --- | --- | --- | --- |")
    for (workload, metric), old, new, kind, runs in triggers:
        if kind == "new" or old is None:
            change = "new"
        else:
            change = f"{old:.2f}x -> {new:.2f}x ({(new - old) / old * 100:+.1f}%)"
        lines.append(f"| {workload} | {metric} | {runs} | {change} |")
    lines.append("")
    return "\n".join(lines) + "\n"


def update_changelog(tsv_path, triggers, unparseable):
    """Insert a recalibration entry beside `tsv_path`, newest first.

    Only called when the .tsv was actually rewritten, so the calibration workflow
    never opens a PR that adds nothing but an empty changelog.
    """
    path = changelog_path(tsv_path)
    current = ""
    if os.path.exists(path):
        with open(path) as f:
            current = f.read()
    content = current if current.strip() else changelog_header()
    entry = format_changelog_entry(
        datetime.date.today().isoformat(), triggers, unparseable
    )
    match = re.search(r"^## ", content, re.MULTILINE)
    if match:
        content = content[: match.start()] + entry + content[match.start() :]
    else:
        content = content.rstrip("\n") + "\n\n" + entry
    if content != current:
        with open(path, "w") as f:
            f.write(content)
        print(f"Updated {path}")


def write_tsv(path, rows, keep_old, existing):
    with open(path, "w") as f:
        f.write("# " + "  ".join(COLUMNS) + "\n")
        for row in rows:
            key = (row["workload"], row["metric"])
            if key in keep_old:
                cells = [
                    key[0],
                    key[1],
                    str(existing[key]["num_samples"]),
                    f"{existing[key]['lowest_over_median']:.3f}",
                    f"{existing[key]['highest_over_median']:.3f}",
                    f"{existing[key]['median_speedup']:.3f}",
                ]
            else:
                cells = [row[c] for c in COLUMNS]
            f.write("\t".join(cells) + "\n")
    print(f"Written to {path}")


def parse_args():
    parser = argparse.ArgumentParser(
        description="Recalibrate mono-move e2e speedup bands"
    )
    parser.add_argument(
        "--branch",
        type=str,
        help="Only look at Humio results from this branch; defaults to main",
    )
    parser.add_argument(
        "--time-interval",
        default="5d",
        help="Humio lookback window",
    )
    parser.add_argument(
        "--from-jsonl",
        nargs="+",
        metavar="FILE",
        help="Aggregate these runner logs instead of querying Humio",
    )
    parser.add_argument(
        "--output",
        default=TSV_PATH,
        help="Calibration file to update",
    )
    return parser.parse_args()


def main():
    args = parse_args()

    if args.from_jsonl:
        rows = rows_from_jsonl(args.from_jsonl)
    else:
        rows = rows_from_humio(args.branch, args.time_interval)

    if not rows:
        print("No samples found; nothing to calibrate.")
        return

    existing = load_calibration(args.output)

    needs_update = False
    in_band = 0
    out_of_band = 0
    low_sample_skipped = 0
    new_rows = 0
    unparseable = 0
    triggers = []
    # Rows whose drift was ignored as too-few-samples keep their old values when
    # the file is rewritten for some other row's sake.
    keep_old = set()

    for row in rows:
        key = (row["workload"], row["metric"])

        if key not in existing:
            new_rows += 1
            needs_update = True
            triggers.append((key, None, None, "new", row["num_samples"]))
            continue

        old = existing[key]
        try:
            new_median = float(row["median_speedup"])
            new_samples = int(row["num_samples"])
        except ValueError as e:
            print(f"Could not parse band inputs for {key}: {e}; treating as out of band.")
            unparseable += 1
            needs_update = True
            continue

        low, high = speedup_band(
            old["median_speedup"],
            old["num_samples"],
            old["lowest_over_median"],
            old["highest_over_median"],
        )

        if low <= new_median <= high:
            in_band += 1
        elif new_samples < MIN_RECALIBRATION_SAMPLES:
            low_sample_skipped += 1
            keep_old.add(key)
        else:
            out_of_band += 1
            needs_update = True
            triggers.append(
                (key, old["median_speedup"], new_median, "drift", row["num_samples"])
            )

    print(
        f"Calibration check for {args.output}: "
        f"out_of_band={out_of_band}, in_band={in_band}, "
        f"low_sample_skipped={low_sample_skipped}, new_rows={new_rows}, "
        f"unparseable={unparseable}, needs_update={needs_update}"
    )

    if needs_update:
        write_tsv(args.output, rows, keep_old, existing)
        update_changelog(args.output, triggers, unparseable)


if __name__ == "__main__":
    main()

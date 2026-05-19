#!/usr/bin/env python

import argparse
import os


# Executor types still under active development. Their results are reported
# as warnings only -- never block CI. Calibration drift on these rows must
# not trigger a PR. Imported by single_node_performance.py.
NON_BLOCKING_EXECUTOR_TYPES = frozenset(
    {
        "NativeVM",
        "NativeSpeculative",
        "AptosVMSpeculative",
        "NativeValueCacheSpeculative",
        "NativeNoStorageSpeculative",
        "sharded",
    }
)


# Move e2e benchmark band constants. These must match the constants in
# aptos-move/e2e-benchmark/src/main.rs; update both files together.
ALLOWED_REGRESSION = 0.15
ALLOWED_IMPROVEMENT = 0.15
ABSOLUTE_BUFFER_US = 2.0


def tps_band(expected_tps, count, min_ratio, max_ratio):
    min_tps = expected_tps * (
        1 - (1 - min_ratio) * (1 + 10.0 / count) - 1.0 / count
    )
    max_tps = expected_tps * (
        1 + (max_ratio - 1) * (1 + 10.0 / count) + 1.0 / count
    )
    return min_tps, max_tps


def wall_time_band(expected_us, min_ratio, max_ratio):
    max_us = max(
        expected_us * (1.0 + ALLOWED_REGRESSION) + ABSOLUTE_BUFFER_US,
        expected_us * max_ratio,
    )
    min_us = min(
        expected_us * (1.0 - ALLOWED_IMPROVEMENT) - ABSOLUTE_BUFFER_US,
        expected_us * min_ratio,
    )
    return min_us, max_us


def humio_secret():
    token = os.environ.get("HUMIO_READ_TOKEN", "").strip()
    if token:
        return token

    print(
        "trying to get a humio secret from gcloud. if it asks for a password, abort and run `gcloud auth login --update-adc` first"
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


def humio_url():
    return "https://cloud.us.humio.com/api/v1/repositories/github/query"


def parse_args():
    parser = argparse.ArgumentParser(description="Benchmark calibration tools")

    parser.add_argument(
        "--branch",
        type=str,
        help="Optional branch, if passed - only looks at results run on that branch through adhoc runs",
    )

    parser.add_argument(
        "--move-e2e",
        action="store_true",
        help="Calibrate move e2e test",
    )

    parser.add_argument(
        "--time-interval", default="5d", help="Time interval to look at humio for"
    )

    return parser.parse_args()


def query_humio(query_string, time_interval):
    import requests

    query = {
        "queryString": query_string,
        "start": time_interval,
    }

    secret = humio_secret()

    resp = requests.post(
        url=humio_url(),
        json=query,
        headers={
            "Authorization": f"Bearer {secret}",
            "Content-Type": "application/json",
        },
    )

    return resp.text.strip()


def _load_existing_tsv(path, key_columns_count):
    rows = {}
    if not os.path.exists(path):
        return rows
    with open(path) as f:
        for line in f:
            line = line.rstrip("\n")
            if not line:
                continue
            cells = line.split("\t")
            rows[tuple(cells[:key_columns_count])] = cells
    return rows


def main():
    args = parse_args()

    if args.move_e2e:
        prefix = (
            """
        github.job.name = "single-node-performance"
        | github.workflow.head_branch = "{branch}"
        | "grep_json_aptos_move_vm_perf"
        | parseJson(message)
        """.format(
                branch=args.branch
            )
            if args.branch is not None
            else """
        github.job.name = "execution-performance / single-node-performance"
        | github.workflow.head_branch = "main"
        | "grep_json_aptos_move_vm_perf"
        | parseJson(message)
        """
        )

        query_string = (
            prefix
            + """
        | groupBy([test_index, transaction_type], function=[count(as="count"), avg(expected_wall_time_us, as="expected"), avg(wall_time_us, as="avg_wall_time_us"), min(wall_time_us, as="min_wall_time_us"), max(wall_time_us, as="max_wall_time_us"), percentile(field=wall_time_us, accuracy=0.001, percentiles=[50])])
        | min_ratio := min_wall_time_us / _50
        | avg_ratio := avg_wall_time_us / _50
        | max_ratio := max_wall_time_us / _50
        | offset_avg_from_expected := _50 / expected
        | format("%.1f", field=_50, as="median")
        | format("%.1f", field=avg_wall_time_us, as="avg_wall_time_us")
        | format("%.1f", field=min_wall_time_us, as="min_wall_time_us")
        | format("%.1f", field=max_wall_time_us, as="max_wall_time_us")
        | format("%.3f", field=min_ratio, as="min_ratio")
        | format("%.3f", field=max_ratio, as="max_ratio")
        | format("%.3f", field=offset_avg_from_expected, as="offset_median_from_expected")
        | table([transaction_type, count, min_ratio, max_ratio, median, expected], sortby=test_index, reverse=false)
        """
        )

        columns = ["transaction_type", "count", "min_ratio", "max_ratio", "median"]
        key_columns_count = 1

        def split_line(line):
            line = line.strip()
            if "}" in line:
                parts = line.split("}")
                res = ["}".join(parts[:-1]) + "}"] + parts[-1].split(", ")[1:]
                return res
            else:
                return line.split(", ")

        output_file_name = "aptos-move/e2e-benchmark/data/calibration_values.tsv"

    else:
        prefix = (
            """
        github.job.name = "single-node-performance"
        | github.workflow.head_branch = "{branch}"
        | "grep_json_single_node_perf"
        | parseJson(message)
        | source = "ADHOC"
        """.format(
                branch=args.branch
            )
            if args.branch is not None
            else """
        github.job.name = "execution-performance / single-node-performance"
        | github.workflow.head_branch = "main"
        | "grep_json_single_node_perf"
        | parseJson(message)
        """
        )

        query_string = (
            prefix
            + """
        | groupBy([test_index, transaction_type, module_working_set_size, executor_type, code_perf_version], function=[count(as="count"), avg(expected_tps, as="expected"), avg(tps, as="avg_tps"), min(tps, as="min_tps"), max(tps, as="max_tps"), percentile(field=tps, accuracy=0.001, percentiles=[50])])
        | min_ratio := min_tps / _50
        | avg_ratio := avg_tps / _50
        | max_ratio := max_tps / _50
        | offset_avg_from_expected := _50 / expected
        | format("%.1f", field=_50, as="median")
        | format("%.1f", field=avg_tps, as="avg_tps")
        | format("%.1f", field=min_tps, as="min_tps")
        | format("%.1f", field=max_tps, as="max_tps")
        | format("%.3f", field=min_ratio, as="min_ratio")
        | format("%.3f", field=max_ratio, as="max_ratio")
        | format("%.3f", field=offset_avg_from_expected, as="offset_median_from_expected")
        | table([transaction_type, module_working_set_size, executor_type, count, min_ratio, max_ratio, median], sortby=test_index, reverse=false)
        """
        )

        columns = [
            "transaction_type",
            "module_working_set_size",
            "executor_type",
            "count",
            "min_ratio",
            "max_ratio",
            "median",
        ]
        key_columns_count = 3

        def split_line(line):
            return line.strip().split(", ")

        output_file_name = "testsuite/single_node_performance_values.tsv"

    response_text = query_humio(query_string, time_interval=args.time_interval)

    parsed = [
        {
            (parts := key_value.split("->"))[0]: parts[1]
            for key_value in split_line(line)
        }
        for line in response_text.split("\n")
    ]

    existing = _load_existing_tsv(output_file_name, key_columns_count)

    needs_update = False
    in_band = 0
    out_of_band = 0
    experimental_skipped = 0
    new_tests = 0
    unparseable = 0

    for new_row in parsed:
        try:
            key = tuple(new_row[c] for c in columns[:key_columns_count])
            executor_type = new_row["executor_type"]
        except KeyError as e:
            print(f"Row missing required column {e}; "
                  f"treating as unparseable.")
            unparseable += 1
            needs_update = True
            continue

        # Non-blocking executor types are warning-only in the perf gate, so
        # their drift must not be the reason a calibration PR opens. They
        # are still refreshed alongside production rows when a write does
        # happen.
        if not args.move_e2e and executor_type in NON_BLOCKING_EXECUTOR_TYPES:
            experimental_skipped += 1
            continue

        if key not in existing:
            new_tests += 1
            needs_update = True
            continue

        old = existing[key]
        try:
            old_expected = float(old[-1])
            old_count = int(old[-4])
            old_min_ratio = float(old[-3])
            old_max_ratio = float(old[-2])
            new_median = float(new_row["median"])
        except (ValueError, IndexError, KeyError) as e:
            print(f"Could not parse band inputs for {key}: {e}; "
                  f"treating as out-of-band.")
            unparseable += 1
            needs_update = True
            continue

        if args.move_e2e:
            lo, hi = wall_time_band(old_expected, old_min_ratio, old_max_ratio)
        else:
            lo, hi = tps_band(
                old_expected, old_count, old_min_ratio, old_max_ratio
            )

        if lo <= new_median <= hi:
            in_band += 1
        else:
            out_of_band += 1
            needs_update = True

    print(
        f"Calibration check for {output_file_name}: "
        f"out_of_band={out_of_band}, in_band={in_band}, "
        f"experimental_skipped={experimental_skipped}, "
        f"new_tests={new_tests}, unparseable={unparseable}, "
        f"needs_update={needs_update}"
    )

    if not needs_update:
        print(
            f"No production rows outside band; "
            f"leaving {output_file_name} unchanged."
        )
        return

    with open(output_file_name, "w") as f:
        for line in parsed:
            f.write("\t".join([line[column] for column in columns]))
            f.write("\n")

    print(f"Written to {output_file_name}")


if __name__ == "__main__":
    main()

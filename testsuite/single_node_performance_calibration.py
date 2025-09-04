#!/usr/bin/env python

import argparse
import requests


def humio_secret():
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
            "--project=velor-shared-secrets",
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


def main():
    args = parse_args()

    if args.move_e2e:
        prefix = (
            """
        github.job.name = "single-node-performance"
        | github.workflow.head_branch = "{branch}"
        | "grep_json_velor_move_vm_perf"
        | parseJson(message)
        """.format(
                branch=args.branch
            )
            if args.branch is not None
            else """
        github.job.name = "execution-performance / single-node-performance"
        | github.workflow.head_branch = "main"
        | "grep_json_velor_move_vm_perf"
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

        def split_line(line):
            line = line.strip()
            if "}" in line:
                parts = line.split("}")
                res = ["}".join(parts[:-1]) + "}"] + parts[-1].split(", ")[1:]
                return res
            else:
                return line.split(", ")

        output_file_name = "velor-move/e2e-benchmark/data/calibration_values.tsv"

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

    with open(output_file_name, "w") as f:
        for line in parsed:
            f.write("\t".join([line[column] for column in columns]))
            f.write("\n")

    print(f"Written to {output_file_name}")


if __name__ == "__main__":
    main()

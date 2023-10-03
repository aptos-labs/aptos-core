#!/usr/bin/env python3

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import os
import subprocess
import shutil
import sys
import math
from multiprocessing import Pool, freeze_support
from typing import Tuple

from verify_core.common import clear_artifacts, query_backup_latest_version

# This script runs the replay-verify from the root of aptos-core
# It assumes the aptos-db-tool binary is already built with the release profile


# The key is the runner's number and the value is the range of txns that the runner is responsible for
# This mapping is generated in 3 steps:
# (1) allocate the txns to runners based on how much time the runners in past takes to finish evenly distributed txns, example script https://gist.github.com/areshand/d00ce302d3bafe0f7b97c311113eba1b
# (2) rerun the flow with new ranges and check the time each runner takes to finish again
# (3) manual adjust the range based on times each takes to finish the work to make sure each runner takes similar time to finish

#  Note: this range needs to be updated when the last range's time is over 2 hrs. (we will send a low pri alert once the time is over 2 hrs)
#  Oncall should
#  1. seal the last range with the latest txn version and start a new range with the [latest_txn_version + 1, sys.maxsize]
#  2. meanwhile, the oncall should delete the old ranges that are beyond 300M window that we want to scan
#

testnet_runner_mapping = [
    [250000000, 255584106],
    [255584107, 271874718],
    [271874719, 300009463],
    [300009464, 324904819],
    [324904820, 347234877],
    [347234878, 366973577],
    [366973578, 399489396],
    [399489397, 430909965],
    [430909966, 449999999],
    [450000000, 462114510],
    [462114511, 478825432],
    [478825433, 483500000],
    [483500001, 516281795],
    [516281796, 551052675],
    [551052676, 582481398],
    [582481399, 640_000_000],
    [640_000_001, sys.maxsize],
]

mainnet_runner_mapping = [
    [0, 50_000_000],
    [50_000_001, 100_000_000],
    [100_000_001, 110_000_000],
    [110_000_001, 150_000_000],
    [150_000_001, 170_000_000],
    [170_000_001, 180_000_000],
    [180_000_001, 190_000_000],
    [190_000_001, 200_000_000],
    [200_000_001, 210_000_000],
    [210_000_001, 220_000_000],
    [220_000_001, 230_000_000],
    [230_000_001, 240_000_000],
    [240_000_001, 250_000_000],
    [250_000_001, 260_000_000],
    [260_000_001, 270_000_000],
    [270_000_001, 278_000_000],
    [278_000_001, sys.maxsize],
]


def replay_verify_partition(
    n: int,
    N: int,
    history_start: int,
    per_partition: int,
    latest_version: int,
    txns_to_skip: Tuple[int],
    backup_config_template_path: str,
) -> Tuple[int, int]:
    """
    Run replay-verify for a partition of the backup, returning a tuple of the (partition number, return code)

    n: partition number
    N: total number of partitions
    history_start: start version of the history to verify
    per_partition: number of versions per partition
    latest_version: last version to verify
    txns_to_skip: list of transactions to skip
    backup_config_template_path: path to the backup config template
    """
    end = history_start + n * per_partition
    if n == N - 1 and end < latest_version:
        end = latest_version

    start = end - per_partition
    partition_name = f"run_{n}_{start}_{end}"

    print(f"[partition {n}] spawning {partition_name}")
    os.mkdir(partition_name)
    shutil.copytree("metadata-cache", f"{partition_name}/metadata-cache")

    txns_to_skip_args = [f"--txns-to-skip={txn}" for txn in txns_to_skip]

    # run and print output
    process = subprocess.Popen(
        [
            "target/release/aptos-db-tool",
            "replay-verify",
            *txns_to_skip_args,
            "--concurrent-downloads",
            "2",
            "--replay-concurrency-level",
            "2",
            "--metadata-cache-dir",
            f"./{partition_name}/metadata-cache",
            "--target-db-dir",
            f"./{partition_name}/db",
            "--start-version",
            str(start),
            "--end-version",
            str(end),
            "--lazy-quit",
            "--command-adapter-config",
            backup_config_template_path,
        ],
        stdout=subprocess.PIPE,
    )
    if process.stdout is None:
        raise Exception(f"[partition {n}] stdout is None")
    for line in iter(process.stdout.readline, b""):
        print(f"[partition {n}] {line}", flush=True)

    # set the returncode
    process.communicate()

    return (n, process.returncode)


def main():
    # collect all required ENV variables
    REQUIRED_ENVS = [
        "BUCKET",
        "SUB_DIR",
        "HISTORY_START",
        "TXNS_TO_SKIP",
        "BACKUP_CONFIG_TEMPLATE_PATH",
    ]

    if not all(env in os.environ for env in REQUIRED_ENVS):
        raise Exception("Missing required ENV variables")
    (runner_no, runner_cnt) = (
        (int(sys.argv[1]), int(sys.argv[2])) if len(sys.argv) > 2 else (None, None)
    )
    # by default we only run one job
    if runner_no is None or runner_cnt is None:
        runner_no = 0
        runner_cnt = 1

    assert (
        runner_no >= 0 and runner_no < runner_cnt
    ), "runner_no must be between 0 and runner_cnt"

    TXNS_TO_SKIP = [int(txn) for txn in os.environ["TXNS_TO_SKIP"].split(" ")]
    BACKUP_CONFIG_TEMPLATE_PATH = os.environ["BACKUP_CONFIG_TEMPLATE_PATH"]

    if not os.path.exists(BACKUP_CONFIG_TEMPLATE_PATH):
        raise Exception("BACKUP_CONFIG_TEMPLATE_PATH does not exist")
    with open(BACKUP_CONFIG_TEMPLATE_PATH, "r") as f:
        config = f.read()
        if "aws" in config and shutil.which("aws") is None:
            raise Exception("Missing required AWS CLI for pulling backup data from S3")

    if os.environ.get("REUSE_BACKUP_ARTIFACTS", "true") == "true":
        print("[main process] clearing existing backup artifacts")
        clear_artifacts()
    else:
        print("[main process] skipping clearing backup artifacts")

    LATEST_VERSION = query_backup_latest_version(BACKUP_CONFIG_TEMPLATE_PATH)

    # the runner may have small overlap at the boundary to prevent missing any transactions
    runner_mapping = (
        testnet_runner_mapping
        if "testnet" in os.environ["BUCKET"]
        else mainnet_runner_mapping
    )

    assert runner_cnt == len(
        runner_mapping
    ), "runner_cnt must match the number of runners in the mapping"
    runner_start = runner_mapping[runner_no][0]
    runner_end = runner_mapping[runner_no][1]
    if runner_no == runner_cnt - 1:
        runner_end = LATEST_VERSION
    print("runner start %d end %d" % (runner_start, runner_end))
    # run replay-verify in parallel
    N = 16
    PER_PARTITION = (runner_end - runner_start) // N

    with Pool(N) as p:
        all_partitions = p.starmap(
            replay_verify_partition,
            [
                (
                    n,
                    N,
                    runner_start,
                    PER_PARTITION,
                    runner_end,
                    TXNS_TO_SKIP,
                    BACKUP_CONFIG_TEMPLATE_PATH,
                )
                for n in range(1, N)
            ],
        )

    print("[main process] finished")

    err = False
    for partition_num, return_code in all_partitions:
        if return_code != 0:
            print("======== ERROR ========")
            print(f"ERROR: partition {partition_num} failed (exit {return_code})")
            err = True

    if err:
        sys.exit(1)


if __name__ == "__main__":
    freeze_support()
    main()

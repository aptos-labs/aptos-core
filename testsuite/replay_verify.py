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

    HISTORY_START = int(os.environ["HISTORY_START"])
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
    runner_load = math.ceil((LATEST_VERSION - HISTORY_START) / runner_cnt)
    runner_start = HISTORY_START + runner_no * runner_load
    runner_end = runner_start + runner_load
    print("runner start %d end %d" % (runner_start, runner_end))
    # run replay-verify in parallel
    N = 32
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

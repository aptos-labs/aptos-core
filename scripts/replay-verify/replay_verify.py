#!/usr/bin/env python3

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

import os
import re
import subprocess
import shutil
from multiprocessing import Pool, freeze_support
from typing import IO, Tuple

# This script runs the replay-verify from the root of aptos-core
# It assumes the aptos-backup-cli, replay-verify, and db-backup binaries are already built with the release profile


def replay_verify_partition(
    n: int,
    N: int,
    history_start: int,
    per_partition: int,
    latest_version: int,
    txns_to_skip: Tuple[int],
    backup_config_template_path: str,
):
    """
    Run replay-verify for a partition of the backup

    n: partition number
    N: total number of partitions
    history_start: start version of the history
    per_partition: number of versions per partition
    latest_version: latest version in the backup
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
            "target/release/replay-verify",
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
            "command-adapter",
            "--config",
            backup_config_template_path,
        ],
        stdout=subprocess.PIPE,
    )
    for line in iter(process.stdout.readline, b""):
        print(f"[partition {n}] {line}", flush=True)


def find_latest_version_from_db_back_log_line(log_line: str):
    match = re.search("latest_transaction_version: (\d+)", log_line)
    if match:
        print(match.group(1))
        return int(match.group(1))
    else:
        return -1


def find_latest_version_from_db_backup_output(output: IO[bytes]):
    latest_version = -1
    for line in iter(output.readline, b""):
        log_line = line.decode("utf-8")
        latest_version = find_latest_version_from_db_back_log_line(log_line)
        print(log_line.strip(), flush=True)
        if latest_version > 0:
            break

    return latest_version


def clear_artifacts():
    """Clears artifacts from previous runs"""
    shutil.rmtree("metadata-cache", ignore_errors=True)
    files = [f for f in os.listdir(".") if re.match(r"run_[0-9]+_[0-9]+", f)]

    for f in files:
        shutil.rmtree(f)


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

    HISTORY_START = int(os.environ["HISTORY_START"])
    TXNS_TO_SKIP = [int(txn) for txn in os.environ["TXNS_TO_SKIP"].split(" ")]
    BACKUP_CONFIG_TEMPLATE_PATH = os.environ["BACKUP_CONFIG_TEMPLATE_PATH"]

    if not os.path.exists(BACKUP_CONFIG_TEMPLATE_PATH):
        raise Exception("BACKUP_CONFIG_TEMPLATE_PATH does not exist")
    with open(BACKUP_CONFIG_TEMPLATE_PATH, "r") as f:
        config = f.read()
        if "aws" in config and shutil.which("aws") is None:
            raise Exception("Missing required AWS CLI for pulling backup data from S3")

    if os.environ.get("CLEAR_BACKUP_ARTIFACTS", "true") == "true":
        print("[main process] clearing artifacts")
        clear_artifacts()
    else:
        print("[main process] skipping clearing artifacts")

    # query latest version in backup, at the same time, pre-heat metadata cache
    db_backup_result = subprocess.Popen(
        [
            "target/release/db-backup",
            "one-shot",
            "query",
            "backup-storage-state",
            "--metadata-cache-dir",
            "./metadata-cache",
            "command-adapter",
            "--config",
            BACKUP_CONFIG_TEMPLATE_PATH,
        ],
        stdout=subprocess.PIPE,
    )
    LATEST_VERSION = find_latest_version_from_db_backup_output(db_backup_result.stdout)
    if LATEST_VERSION < 0:
        raise Exception("Failed to find latest version")
    db_backup_result.wait()

    # run replay-verify in parallel
    N = 32
    PER_PARTITION = (LATEST_VERSION - HISTORY_START) // N

    with Pool(N) as p:
        p.starmap(
            replay_verify_partition,
            [
                (
                    n,
                    N,
                    HISTORY_START,
                    PER_PARTITION,
                    LATEST_VERSION,
                    TXNS_TO_SKIP,
                    BACKUP_CONFIG_TEMPLATE_PATH,
                )
                for n in range(1, N)
            ],
        )

    print("[main process] finished")


if __name__ == "__main__":
    freeze_support()
    main()

#!/usr/bin/env python3

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import subprocess
import re
import os
import tempfile
from tabulate import tabulate


# numbers are based on the machine spec used by github action
# Local machine numbers will be higher.
EXPECTED_TPS = {
    ("no-op", False): 18500.0,
    ("coin-transfer", False): 12000.0,
    ("coin-transfer", True): 19500.0,
    ("account-generation", False): 10200.0,
    ("account-generation", True): 16900.0,
    ("create-new-account-resource", False): 12000.0,
    ("modify-global-resource", False): 4000.0,
    ("modify-ten-global-resources", False): 10700.0,
    ("large-module-working-set-no-op", False): 2550.0,
    ("publish-package", False): 135.0,
    ("batch100-transfer", False): 300,
    ("batch100-transfer", True): 500,
    # ("token-v1ft-mint-and-store", False): 1000.0,
    ("token-v1ft-mint-and-transfer", False): 1700.0,
    ("token-v1nft-mint-and-transfer-sequential", False): 1100.0,
    ("token-v1ft-mint-and-transfer20-collections", False): 6000.0,
    ("token-v1nft-mint-and-transfer-sequential20-collections", False): 4000.0,
    # ("token-v1nft-mint-and-transfer-parallel", False): 1000.0,
    # ("token-v1nft-mint-and-store-sequential", False): 1000.0,
    # ("token-v1nft-mint-and-store-parallel", False): 1000.0,
}

NOISE_FRACTION = 0.1

# use production concurrency level for assertions
CONCURRENCY_LEVEL = 8
BLOCK_SIZE = 10000
NUM_BLOCKS = 15
NUM_BLOCKS_DETAILED = 10
NUM_ACCOUNTS = max([2000000, 4 * NUM_BLOCKS * BLOCK_SIZE])
ADDITIONAL_DST_POOL_ACCOUNTS = 2 * NUM_BLOCKS * BLOCK_SIZE
MAIN_SIGNER_ACCOUNTS = 2 * BLOCK_SIZE

if os.environ.get("DETAILED"):
    EXECUTION_ONLY_CONCURRENCY_LEVELS = [1, 2, 4, 8, 16, 32, 60]
else:
    EXECUTION_ONLY_CONCURRENCY_LEVELS = []

# Run the single node with performance optimizations enabled
target_directory = "execution/executor-benchmark/src"


def execute_command(command):
    try:
        output = subprocess.check_output(
            command, shell=True, text=True, cwd=target_directory
        )
    except subprocess.CalledProcessError as e:
        print(e.output)
        raise e

    print(output)
    return output


errors = []

with tempfile.TemporaryDirectory() as tmpdirname:
    create_db_command = f"cargo run --profile performance -- --block-size {BLOCK_SIZE} --concurrency-level {CONCURRENCY_LEVEL} --use-state-kv-db --use-sharded-state-merkle-db create-db --data-dir {tmpdirname}/db --num-accounts {NUM_ACCOUNTS}"
    output = execute_command(create_db_command)

    achieved_tps = {}
    achieved_gps = {}

    rows = []
    gas_rows = []

    for (transaction_type, use_native_executor), expected_tps in EXPECTED_TPS.items():
        print(f"Testing {transaction_type}")
        cur_block_size = int(min([expected_tps, BLOCK_SIZE]))

        achieved_tps[transaction_type] = {}
        achieved_gps[transaction_type] = {}
        use_native_executor_row_str = "native" if use_native_executor else "VM"
        row = [
            "grep_sn_perf_tps",
            transaction_type,
            use_native_executor_row_str,
            cur_block_size,
            expected_tps,
        ]
        gas_row = [
            "grep_sn_perf_gps",
            transaction_type,
            use_native_executor_row_str,
            cur_block_size,
        ]

        use_native_executor_str = "--use-native-executor" if use_native_executor else ""
        common_command_suffix = f"{use_native_executor_str} --generate-then-execute --transactions-per-sender 1 --block-size {cur_block_size} --use-state-kv-db --use-sharded-state-merkle-db run-executor --transaction-type {transaction_type} --main-signer-accounts {MAIN_SIGNER_ACCOUNTS} --additional-dst-pool-accounts {ADDITIONAL_DST_POOL_ACCOUNTS} --data-dir {tmpdirname}/db  --checkpoint-dir {tmpdirname}/cp"
        for concurrency_level in EXECUTION_ONLY_CONCURRENCY_LEVELS:
            test_db_command = f"cargo run --profile performance -- --concurrency-level {concurrency_level}  --skip-commit {common_command_suffix} --blocks {NUM_BLOCKS_DETAILED}"
            output = execute_command(test_db_command)

            tps = float(
                re.findall(r"Overall execution TPS: (\d+\.?\d*) txn/s", output)[-1]
            )
            gps = float(
                re.findall(r"Overall execution GPS: (\d+\.?\d*) gas/s", output)[-1]
            )

            achieved_tps[transaction_type][concurrency_level] = tps
            achieved_gps[transaction_type][concurrency_level] = gps
            row.append(int(round(tps)))
            gas_row.append(int(round(gps)))

        test_db_command = f"cargo run --profile performance -- --concurrency-level {CONCURRENCY_LEVEL} {common_command_suffix} --blocks {NUM_BLOCKS}"
        output = execute_command(test_db_command)

        tps = float(re.findall(r"Overall TPS: (\d+\.?\d*) txn/s", output)[0])
        gps = float(re.findall(r"Overall GPS: (\d+\.?\d*) gas/s", output)[-1])
        achieved_tps[transaction_type][0] = tps
        achieved_gps[transaction_type][0] = gps
        row.append(int(round(tps)))
        gas_row.append(int(round(gps)))

        rows.append(row)
        gas_rows.append(gas_row)

        print(
            tabulate(
                rows,
                headers=[
                    "grep",
                    "transaction_type",
                    "executor",
                    "block_size",
                    "expected t/s",
                ]
                + [
                    f"exe_only {concurrency_level}"
                    for concurrency_level in EXECUTION_ONLY_CONCURRENCY_LEVELS
                ]
                + ["t/s"],
            )
        )

        print(
            tabulate(
                gas_rows,
                headers=["grep", "transaction_type", "executor", "block_size"]
                + [
                    f"exe_only {concurrency_level}"
                    for concurrency_level in EXECUTION_ONLY_CONCURRENCY_LEVELS
                ]
                + ["g/s"],
            )
        )

        if tps < expected_tps * (1 - NOISE_FRACTION):
            errors.append(
                f"regression detected {tps} < {expected_tps} * {1 - NOISE_FRACTION}, {transaction_type} with {use_native_executor_row_str} executor didn't meet TPS requirements"
            )
        elif tps > expected_tps * (1 + NOISE_FRACTION):
            errors.append(
                f"perf improvement detected {tps} > {expected_tps} * {1 + NOISE_FRACTION}, {transaction_type} with {use_native_executor_row_str} executor exceeded TPS requirements, increase TPS requirements to match new baseline"
            )

if errors:
    print("\n".join(errors))
    exit(1)

exit(0)

# # Check if any threshold is not met
# if tps_1k < THRESHOLD_1k or tps_10k < THRESHOLD_10k or tps_50k < THRESHOLD_50k:
#     print("Sequential TPS below the threshold")
#     exit(1)
# else:
#     print("Sequential TPS above the threshold")
#     exit(0)

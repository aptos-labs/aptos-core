#!/usr/bin/env python3

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import subprocess
import re
import os
import tempfile
import json
from tabulate import tabulate


# numbers are based on the machine spec used by github action
# Local machine numbers will be higher.
EXPECTED_TPS = {
    ("no-op", False, 1): (18200.0, True),
    ("no-op", False, 1000): (2550.0, True),
    ("coin-transfer", False, 1): (11800.0, True),
    ("coin-transfer", True, 1): (18900.0, True),
    ("account-generation", False, 1): (9900.0, True),
    ("account-generation", True, 1): (16300.0, True),
    ("create-new-account-resource", False, 1): (11700.0, True),
    ("modify-global-resource", False, 1): (4000.0, True),
    ("modify-global-resource", False, 10): (10500.0, True),
    ("publish-package", False, 1): (130.0, True),
    ("batch100-transfer", False, 1): (300, True),
    ("batch100-transfer", True, 1): (500, True),
    ("token-v1ft-mint-and-transfer", False, 1): (1700.0, True),
    ("token-v1ft-mint-and-transfer", False, 20): (6400.0, False),
    ("token-v1nft-mint-and-transfer-sequential", False, 1): (1100.0, True),
    ("token-v1nft-mint-and-transfer-sequential", False, 20): (4400.0, False),
    ("token-v1nft-mint-and-transfer-parallel", False, 1): (1700.0, False),
    ("token-v1nft-mint-and-transfer-parallel", False, 20): (5000.0, False),
    # ("token-v1ft-mint-and-store", False): 1000.0,
    # ("token-v1nft-mint-and-store-sequential", False): 1000.0,
    # ("token-v1nft-mint-and-store-parallel", False): 1000.0,
    ("no-op2-signers", False, 1): (18200.0, False),
    ("no-op5-signers", False, 1): (18200.0, False),
    ("token-v2-ambassador-mint", False, 1): (2000.0, False),
    ("token-v2-ambassador-mint", False, 20): (5000.0, False),
}

NOISE_LOWER_LIMIT = 0.8
# temporarily increasing upper threshold for perf improvements in #8002,
# TODO will reduce back after calibration
NOISE_UPPER_LIMIT = 1.3

# bump after a perf improvement, so you can easily distinguish runs
# that are on top of this commit
CODE_PERF_VERSION = "v2"

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

if os.environ.get("DEFAULT_BUILD"):
    BUILD_FLAG = ""  # "--release"
else:
    BUILD_FLAG = "--profile performance"

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
warnings = []

with tempfile.TemporaryDirectory() as tmpdirname:
    create_db_command = f"cargo run {BUILD_FLAG} -- --block-size {BLOCK_SIZE} --concurrency-level {CONCURRENCY_LEVEL} --use-state-kv-db --use-sharded-state-merkle-db create-db --data-dir {tmpdirname}/db --num-accounts {NUM_ACCOUNTS}"
    output = execute_command(create_db_command)

    achieved_tps = {}
    achieved_gps = {}

    rows = []
    gas_rows = []

    for (transaction_type, use_native_executor, module_working_set_size), (
        expected_tps,
        check_active,
    ) in EXPECTED_TPS.items():
        print(f"Testing {transaction_type}")
        cur_block_size = int(min([expected_tps, BLOCK_SIZE]))

        achieved_tps[transaction_type] = {}
        achieved_gps[transaction_type] = {}
        use_native_executor_row_str = "native" if use_native_executor else "VM"
        row = [
            transaction_type,
            module_working_set_size,
            use_native_executor_row_str,
            cur_block_size,
            expected_tps,
        ]
        gas_row = [
            transaction_type,
            module_working_set_size,
            use_native_executor_row_str,
            cur_block_size,
        ]

        use_native_executor_str = "--use-native-executor" if use_native_executor else ""
        common_command_suffix = f"{use_native_executor_str} --generate-then-execute --transactions-per-sender 1 --block-size {cur_block_size} --use-state-kv-db --use-sharded-state-merkle-db run-executor --transaction-type {transaction_type} --module-working-set-size {module_working_set_size} --main-signer-accounts {MAIN_SIGNER_ACCOUNTS} --additional-dst-pool-accounts {ADDITIONAL_DST_POOL_ACCOUNTS} --data-dir {tmpdirname}/db  --checkpoint-dir {tmpdirname}/cp"
        for concurrency_level in EXECUTION_ONLY_CONCURRENCY_LEVELS:
            test_db_command = f"cargo run {BUILD_FLAG} -- --concurrency-level {concurrency_level}  --skip-commit {common_command_suffix} --blocks {NUM_BLOCKS_DETAILED}"
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

        test_db_command = f"cargo run {BUILD_FLAG} -- --concurrency-level {CONCURRENCY_LEVEL} {common_command_suffix} --blocks {NUM_BLOCKS}"
        output = execute_command(test_db_command)

        tps = float(re.findall(r"Overall TPS: (\d+\.?\d*) txn/s", output)[0])
        gps = float(re.findall(r"Overall GPS: (\d+\.?\d*) gas/s", output)[-1])
        achieved_tps[transaction_type][0] = tps
        achieved_gps[transaction_type][0] = gps

        # line to be able to aggreate and visualize in Humio
        print(
            json.dumps(
                {
                    "grep": "grep_json_single_node_perf",
                    "transaction_type": transaction_type,
                    "module_working_set_size": module_working_set_size,
                    "executor_type": use_native_executor_row_str,
                    "block_size": cur_block_size,
                    "expected_tps": expected_tps,
                    "tps": tps,
                    "gps": gps,
                    "code_perf_version": CODE_PERF_VERSION,
                }
            )
        )

        row.append(int(round(tps)))
        gas_row.append(int(round(gps)))

        rows.append(row)
        gas_rows.append(gas_row)

        print(
            tabulate(
                rows,
                headers=[
                    "transaction_type",
                    "module_working_set",
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
                headers=["transaction_type", "executor", "block_size"]
                + [
                    f"exe_only {concurrency_level}"
                    for concurrency_level in EXECUTION_ONLY_CONCURRENCY_LEVELS
                ]
                + ["g/s"],
            )
        )

        if tps < expected_tps * NOISE_LOWER_LIMIT:
            text = f"regression detected {tps} < {expected_tps * NOISE_LOWER_LIMIT} = {expected_tps} * {NOISE_LOWER_LIMIT}, {transaction_type} with {use_native_executor_row_str} executor didn't meet TPS requirements"
            if check_active:
                errors.append(text)
            else:
                warnings.append(text)
        elif tps > expected_tps * NOISE_UPPER_LIMIT:
            text = f"perf improvement detected {tps} > {expected_tps * NOISE_UPPER_LIMIT} = {expected_tps} * {NOISE_UPPER_LIMIT}, {transaction_type} with {use_native_executor_row_str} executor exceeded TPS requirements, increase TPS requirements to match new baseline"
            if check_active:
                errors.append(text)
            else:
                warnings.append(text)

if warnings:
    print("\n".join(warnings))

if errors:
    print("\n".join(errors))
    exit(1)

exit(0)

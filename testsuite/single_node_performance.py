#!/usr/bin/env python3

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import re
import os
import tempfile
import json
from typing import Callable, Optional, Tuple, Mapping, Sequence, Any
from tabulate import tabulate
from subprocess import Popen, PIPE, CalledProcessError
from dataclasses import dataclass


# numbers are based on the machine spec used by github action
# Calibrate from https://gist.github.com/igor-aptos/7b12ca28de03894cddda8e415f37889e
# Local machine numbers will be higher.
EXPECTED_TPS = {
    ("no-op", False, 1): (18000.0, True),
    ("no-op", False, 1000): (2800.0, True),
    ("coin-transfer", False, 1): (12500.0, True),
    ("coin-transfer", True, 1): (30300.0, True),
    ("account-generation", False, 1): (10500.0, True),
    ("account-generation", True, 1): (26500.0, True),
    ("account-resource32-b", False, 1): (15400.0, True),
    ("modify-global-resource", False, 1): (3400.0, True),
    ("modify-global-resource", False, 10): (10100.0, True),
    ("publish-package", False, 1): (120.0, True),
    ("mix_publish_transfer", False, 1): (1400.0, False),
    ("batch100-transfer", False, 1): (370, True),
    ("batch100-transfer", True, 1): (940, True),
    ("token-v1ft-mint-and-transfer", False, 1): (1550.0, True),
    ("token-v1ft-mint-and-transfer", False, 20): (7000.0, True),
    ("token-v1nft-mint-and-transfer-sequential", False, 1): (1000.0, True),
    ("token-v1nft-mint-and-transfer-sequential", False, 20): (5150.0, True),
    ("token-v1nft-mint-and-transfer-parallel", False, 1): (1300.0, True),
    ("token-v1nft-mint-and-transfer-parallel", False, 20): (5300.0, True),
    # ("token-v1ft-mint-and-store", False): 1000.0,
    # ("token-v1nft-mint-and-store-sequential", False): 1000.0,
    # ("token-v1nft-mint-and-store-parallel", False): 1000.0,
    ("no-op2-signers", False, 1): (18000.0, True),
    ("no-op5-signers", False, 1): (18000.0, True),
    ("token-v2-ambassador-mint", False, 1): (1500.0, True),
    ("token-v2-ambassador-mint", False, 20): (5000.0, True),
}

NOISE_LOWER_LIMIT = 0.8
NOISE_LOWER_LIMIT_WARN = 0.9
# If you want to calibrate the upper limit for perf improvement, you can
# increase this value temporarily (i.e. to 1.3) and readjust back after a day or two of runs
NOISE_UPPER_LIMIT = 1.15
NOISE_UPPER_LIMIT_WARN = 1.05

# bump after a perf improvement, so you can easily distinguish runs
# that are on top of this commit
CODE_PERF_VERSION = "v4"

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

if os.environ.get("RELEASE_BUILD"):
    BUILD_FLAG = "--release"
else:
    BUILD_FLAG = "--profile performance"

# Run the single node with performance optimizations enabled
target_directory = "execution/executor-benchmark/src"


def execute_command(command):
    result = []
    with Popen(
        command,
        shell=True,
        text=True,
        cwd=target_directory,
        stdout=PIPE,
        bufsize=1,
        universal_newlines=True,
    ) as p:
        # stream to output while command is executing
        if p.stdout is not None:
            for line in p.stdout:
                print(line, end="")
                result.append(line)

    if p.returncode != 0:
        raise CalledProcessError(p.returncode, p.args)

    # return the full output in the end for postprocessing
    full_result = "\n".join(result)

    if " ERROR " in full_result:
        print("ERROR log line in execution")
        exit(1)

    return full_result


@dataclass
class RunGroupKey:
    transaction_type: str
    module_working_set_size: int
    executor_type: str


@dataclass
class RunResults:
    tps: float
    gps: float
    gpt: float
    fraction_in_execution: float
    fraction_of_execution_in_vm: float
    fraction_in_commit: float


@dataclass
class RunGroupInstance:
    key: RunGroupKey
    single_node_result: RunResults
    concurrency_level_results: Mapping[int, RunResults]
    block_size: int
    expected_tps: float


def get_only(values):
    assert len(values) == 1, "Multiple values parsed: " + str(values)
    return values[0]


def extract_run_results(output: str, execution_only: bool) -> RunResults:
    if execution_only:
        tps = float(re.findall(r"Overall execution TPS: (\d+\.?\d*) txn/s", output)[-1])
        gps = float(re.findall(r"Overall execution GPS: (\d+\.?\d*) gas/s", output)[-1])
        gpt = float(
            re.findall(r"Overall execution GPT: (\d+\.?\d*) gas/txn", output)[-1]
        )

    else:
        tps = float(get_only(re.findall(r"Overall TPS: (\d+\.?\d*) txn/s", output)))
        gps = float(get_only(re.findall(r"Overall GPS: (\d+\.?\d*) gas/s", output)))
        gpt = float(get_only(re.findall(r"Overall GPT: (\d+\.?\d*) gas/txn", output)))

    fraction_in_execution = float(
        re.findall(r"Overall fraction of total: (\d+\.?\d*) in execution", output)[-1]
    )
    fraction_of_execution_in_vm = float(
        re.findall(r"Overall fraction of execution (\d+\.?\d*) in VM", output)[-1]
    )
    fraction_in_commit = float(
        re.findall(r"Overall fraction of total: (\d+\.?\d*) in commit", output)[-1]
    )

    return RunResults(
        tps,
        gps,
        gpt,
        fraction_in_execution,
        fraction_of_execution_in_vm,
        fraction_in_commit,
    )


def print_table(
    results: Sequence[RunGroupInstance],
    by_levels: bool,
    single_field: Optional[Tuple[str, Callable[[RunResults], Any]]],
    concurrency_levels=EXECUTION_ONLY_CONCURRENCY_LEVELS,
):
    headers = [
        "transaction_type",
        "module_working_set",
        "executor",
        "block_size",
        "expected t/s",
    ]
    if by_levels:
        headers.extend(
            [
                f"exe_only {concurrency_level}"
                for concurrency_level in concurrency_levels
            ]
        )
        assert single_field is not None

    if single_field is not None:
        field_name, _ = single_field
        headers.append(field_name)
    else:
        headers.extend(["t/s", "exe/total", "vm/exe", "commit/total", "g/s", "g/t"])

    rows = []
    for result in results:
        row = [
            result.key.transaction_type,
            result.key.module_working_set_size,
            result.key.executor_type,
            result.block_size,
            result.expected_tps,
        ]
        if by_levels:
            if single_field is not None:
                _, field_getter = single_field
                for concurrency_level in concurrency_levels:
                    row.append(
                        field_getter(
                            result.concurrency_level_results[concurrency_level]
                        )
                    )

        if single_field is not None:
            _, field_getter = single_field
            row.append(field_getter(result.single_node_result))
        else:
            row.append(int(round(result.single_node_result.tps)))
            row.append(round(result.single_node_result.fraction_in_execution, 3))
            row.append(round(result.single_node_result.fraction_of_execution_in_vm, 3))
            row.append(round(result.single_node_result.fraction_in_commit, 3))
            row.append(int(round(result.single_node_result.gps)))
            row.append(int(round(result.single_node_result.gpt)))
        rows.append(row)

    print(tabulate(rows, headers=headers))


errors = []
warnings = []

with tempfile.TemporaryDirectory() as tmpdirname:
    create_db_command = f"cargo run {BUILD_FLAG} -- --block-size {BLOCK_SIZE} --concurrency-level {CONCURRENCY_LEVEL} --split-ledger-db --use-sharded-state-merkle-db --skip-index-and-usage create-db --data-dir {tmpdirname}/db --num-accounts {NUM_ACCOUNTS}"
    output = execute_command(create_db_command)

    results = []

    for (
        test_index,
        (
            (transaction_type_name, use_native_executor, module_working_set_size),
            (
                expected_tps,
                check_active,
            ),
        ),
    ) in enumerate(EXPECTED_TPS.items()):
        print(f"Testing {transaction_type_name}")
        if transaction_type_name == "mix_publish_transfer":
            transaction_type = "publish-package coin-transfer"
            transaction_weights = "1 500"
        else:
            transaction_type = transaction_type_name
            transaction_weights = "1"

        cur_block_size = int(min([expected_tps, BLOCK_SIZE]))

        executor_type = "native" if use_native_executor else "VM"

        use_native_executor_str = "--use-native-executor" if use_native_executor else ""
        common_command_suffix = f"{use_native_executor_str} --generate-then-execute --transactions-per-sender 1 --block-size {cur_block_size} --split-ledger-db --use-sharded-state-merkle-db --skip-index-and-usage run-executor --transaction-type {transaction_type} --transaction-weights {transaction_weights} --module-working-set-size {module_working_set_size} --main-signer-accounts {MAIN_SIGNER_ACCOUNTS} --additional-dst-pool-accounts {ADDITIONAL_DST_POOL_ACCOUNTS} --data-dir {tmpdirname}/db  --checkpoint-dir {tmpdirname}/cp"

        concurrency_level_results = {}

        for concurrency_level in EXECUTION_ONLY_CONCURRENCY_LEVELS:
            test_db_command = f"cargo run {BUILD_FLAG} -- --concurrency-level {concurrency_level}  --skip-commit {common_command_suffix} --blocks {NUM_BLOCKS_DETAILED}"
            output = execute_command(test_db_command)

            concurrency_level_results[concurrency_level] = extract_run_results(
                output, execution_only=True
            )

        test_db_command = f"cargo run {BUILD_FLAG} -- --concurrency-level {CONCURRENCY_LEVEL} {common_command_suffix} --blocks {NUM_BLOCKS}"
        output = execute_command(test_db_command)

        current_run_key = RunGroupKey(
            transaction_type_name, module_working_set_size, executor_type
        )
        single_node_result = extract_run_results(output, execution_only=False)

        results.append(
            RunGroupInstance(
                key=current_run_key,
                single_node_result=single_node_result,
                concurrency_level_results=concurrency_level_results,
                block_size=cur_block_size,
                expected_tps=expected_tps,
            )
        )

        # line to be able to aggreate and visualize in Humio
        print(
            json.dumps(
                {
                    "grep": "grep_json_single_node_perf",
                    "transaction_type": transaction_type_name,
                    "module_working_set_size": module_working_set_size,
                    "executor_type": executor_type,
                    "block_size": cur_block_size,
                    "expected_tps": expected_tps,
                    "tps": single_node_result.tps,
                    "gps": single_node_result.gps,
                    "gpt": single_node_result.gpt,
                    "code_perf_version": CODE_PERF_VERSION,
                    "test_index": test_index,
                }
            )
        )

        print_table(
            results, by_levels=True, single_field=("t/s", lambda r: int(round(r.tps)))
        )
        print_table(
            results, by_levels=True, single_field=("g/s", lambda r: int(round(r.gps)))
        )
        print_table(
            results,
            by_levels=True,
            single_field=("exe/total", lambda r: round(r.fraction_in_execution, 3)),
        )
        print_table(
            results,
            by_levels=True,
            single_field=("vm/exe", lambda r: round(r.fraction_of_execution_in_vm, 3)),
        )
        print_table(results, by_levels=False, single_field=None)

        if single_node_result.tps < expected_tps * NOISE_LOWER_LIMIT:
            text = f"regression detected {single_node_result.tps} < {expected_tps * NOISE_LOWER_LIMIT} = {expected_tps} * {NOISE_LOWER_LIMIT}, {current_run_key} didn't meet TPS requirements"
            if check_active:
                errors.append(text)
            else:
                warnings.append(text)
        elif single_node_result.tps < expected_tps * NOISE_LOWER_LIMIT_WARN:
            text = f"potential (but within normal noise) regression detected {single_node_result.tps} < {expected_tps * NOISE_LOWER_LIMIT_WARN} = {expected_tps} * {NOISE_LOWER_LIMIT_WARN}, {current_run_key} didn't meet TPS requirements"
            warnings.append(text)
        elif single_node_result.tps > expected_tps * NOISE_UPPER_LIMIT:
            text = f"perf improvement detected {single_node_result.tps} > {expected_tps * NOISE_UPPER_LIMIT} = {expected_tps} * {NOISE_UPPER_LIMIT}, {current_run_key} exceeded TPS requirements, increase TPS requirements to match new baseline"
            if check_active:
                errors.append(text)
            else:
                warnings.append(text)
        elif single_node_result.tps > expected_tps * NOISE_UPPER_LIMIT_WARN:
            text = f"potential (but within normal noise) perf improvement detected {single_node_result.tps} > {expected_tps * NOISE_UPPER_LIMIT_WARN} = {expected_tps} * {NOISE_UPPER_LIMIT_WARN}, {current_run_key} exceeded TPS requirements, increase TPS requirements to match new baseline"
            warnings.append(text)

if warnings:
    print("Warnings: ")
    print("\n".join(warnings))

if errors:
    print("Errors: ")
    print("\n".join(errors))
    exit(1)

exit(0)

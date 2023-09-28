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
from dataclasses import dataclass, field
from enum import Flag, auto


class Flow(Flag):
    # Tests that are run on PRs
    LAND_BLOCKING = auto()
    # Tests that are run continuously on main (in addition to LAND_BLOCKING ones)
    CONTINUOUS = auto()
    # Tests that are run manually when using a smaller representative mode.
    # (i.e. for measuring speed of the machine)
    REPRESENTATIVE = auto()


@dataclass
class RunGroupKey:
    transaction_type: str
    module_working_set_size: int = field(default=1)
    executor_type: str = field(default="VM")

    transaction_type_override: Optional[str] = field(default=None)
    transaction_weights_override: Optional[str] = field(default=None)
    sharding_traffic_flags: Optional[str] = field(default=None)


@dataclass
class RunGroupConfig:
    key: RunGroupKey
    expected_tps: float
    included_in: Flow
    waived: bool = field(default=False)


# numbers are based on the machine spec used by github action
# Calibrate from https://gist.github.com/igor-aptos/7b12ca28de03894cddda8e415f37889e
# Local machine numbers will be higher.
# fmt: off
TESTS = [
    RunGroupConfig(expected_tps=22700, key=RunGroupKey("no-op"), included_in=Flow.LAND_BLOCKING),
    RunGroupConfig(expected_tps=3200, key=RunGroupKey("no-op", module_working_set_size=1000), included_in=Flow.LAND_BLOCKING),
    RunGroupConfig(expected_tps=15000, key=RunGroupKey("coin-transfer"), included_in=Flow.LAND_BLOCKING | Flow.REPRESENTATIVE),
    RunGroupConfig(expected_tps=26300, key=RunGroupKey("coin-transfer", executor_type="native"), included_in=Flow.LAND_BLOCKING),
    RunGroupConfig(expected_tps=12700, key=RunGroupKey("account-generation"), included_in=Flow.LAND_BLOCKING | Flow.REPRESENTATIVE),
    RunGroupConfig(expected_tps=26500, key=RunGroupKey("account-generation", executor_type="native"), included_in=Flow.CONTINUOUS),
    RunGroupConfig(expected_tps=20000, key=RunGroupKey("account-resource32-b"), included_in=Flow.LAND_BLOCKING),
    RunGroupConfig(expected_tps=4050, key=RunGroupKey("modify-global-resource"), included_in=Flow.LAND_BLOCKING | Flow.REPRESENTATIVE),
    RunGroupConfig(expected_tps=12500, key=RunGroupKey("modify-global-resource", module_working_set_size=10), included_in=Flow.LAND_BLOCKING),
    RunGroupConfig(expected_tps=140, key=RunGroupKey("publish-package"), included_in=Flow.LAND_BLOCKING | Flow.REPRESENTATIVE),
    RunGroupConfig(expected_tps=2600, key=RunGroupKey(
        "mix_publish_transfer",
        transaction_type_override="publish-package coin-transfer",
        transaction_weights_override="1 500",
    ), included_in=Flow.LAND_BLOCKING, waived=True),
    RunGroupConfig(expected_tps=365, key=RunGroupKey("batch100-transfer"), included_in=Flow.LAND_BLOCKING),
    RunGroupConfig(expected_tps=940, key=RunGroupKey("batch100-transfer", executor_type="native"), included_in=Flow.CONTINUOUS),

    RunGroupConfig(expected_tps=1890, key=RunGroupKey("token-v1ft-mint-and-transfer"), included_in=Flow.LAND_BLOCKING),
    RunGroupConfig(expected_tps=8800, key=RunGroupKey("token-v1ft-mint-and-transfer", module_working_set_size=20), included_in=Flow.LAND_BLOCKING),
    RunGroupConfig(expected_tps=1000, key=RunGroupKey("token-v1nft-mint-and-transfer-sequential"), included_in=Flow.CONTINUOUS),
    RunGroupConfig(expected_tps=5150, key=RunGroupKey("token-v1nft-mint-and-transfer-sequential", module_working_set_size=20), included_in=Flow.CONTINUOUS),
    RunGroupConfig(expected_tps=1300, key=RunGroupKey("token-v1nft-mint-and-transfer-parallel"), included_in=Flow.CONTINUOUS),
    RunGroupConfig(expected_tps=5300, key=RunGroupKey("token-v1nft-mint-and-transfer-parallel", module_working_set_size=20), included_in=Flow.CONTINUOUS),

    # RunGroupConfig(expected_tps=1000, key=RunGroupKey("token-v1ft-mint-and-store"), included_in=Flow(0)),
    # RunGroupConfig(expected_tps=1000, key=RunGroupKey("token-v1nft-mint-and-store-sequential"), included_in=Flow(0)),
    # RunGroupConfig(expected_tps=1000, key=RunGroupKey("token-v1nft-mint-and-transfer-parallel"), included_in=Flow(0)),

    RunGroupConfig(expected_tps=18000, key=RunGroupKey("no-op2-signers"), included_in=Flow.CONTINUOUS),
    RunGroupConfig(expected_tps=18000, key=RunGroupKey("no-op5-signers"), included_in=Flow.CONTINUOUS),
   
    RunGroupConfig(expected_tps=1710, key=RunGroupKey("token-v2-ambassador-mint"), included_in=Flow.LAND_BLOCKING | Flow.REPRESENTATIVE),
    RunGroupConfig(expected_tps=5800, key=RunGroupKey("token-v2-ambassador-mint", module_working_set_size=20), included_in=Flow.LAND_BLOCKING | Flow.REPRESENTATIVE),

    RunGroupConfig(expected_tps=50000, key=RunGroupKey("coin_transfer_connected_components", executor_type="sharded", sharding_traffic_flags="--connected-tx-grps 5000", transaction_type_override=""), included_in=Flow.REPRESENTATIVE),
    RunGroupConfig(expected_tps=50000, key=RunGroupKey("coin_transfer_hotspot", executor_type="sharded", sharding_traffic_flags="--hotspot-probability 0.8", transaction_type_override=""), included_in=Flow.REPRESENTATIVE),
]
# fmt: on

NOISE_LOWER_LIMIT = 0.8
NOISE_LOWER_LIMIT_WARN = 0.9
# If you want to calibrate the upper limit for perf improvement, you can
# increase this value temporarily (i.e. to 1.3) and readjust back after a day or two of runs
NOISE_UPPER_LIMIT = 1.15
NOISE_UPPER_LIMIT_WARN = 1.05

# bump after a perf improvement, so you can easily distinguish runs
# that are on top of this commit
CODE_PERF_VERSION = "v4"

NUMBER_OF_EXECUTION_THREADS = 8
MAX_BLOCK_SIZE = int(os.environ.get("MAX_BLOCK_SIZE", default="10000"))
NUM_BLOCKS = 15
NUM_BLOCKS_DETAILED = 10
NUM_ACCOUNTS = max([2000000, 4 * NUM_BLOCKS * MAX_BLOCK_SIZE])
ADDITIONAL_DST_POOL_ACCOUNTS = 2 * NUM_BLOCKS * MAX_BLOCK_SIZE
MAIN_SIGNER_ACCOUNTS = 2 * MAX_BLOCK_SIZE

# default to using production number of execution threads for assertions
NUMBER_OF_EXECUTION_THREADS = os.environ.get("NUMBER_OF_EXECUTION_THREADS", default=8)

if os.environ.get("DETAILED"):
    EXECUTION_ONLY_NUMBER_OF_THREADS = [1, 2, 4, 8, 16, 32, 60]
else:
    EXECUTION_ONLY_NUMBER_OF_THREADS = []

if os.environ.get("RELEASE_BUILD"):
    BUILD_FLAG = "--release"
else:
    BUILD_FLAG = "--profile performance"

SELECTED_FLOW = Flow[os.environ.get("FLOW", default="LAND_BLOCKING")]

if os.environ.get("PROD_DB_FLAGS"):
    DB_CONFIG_FLAGS = ""
else:
    DB_CONFIG_FLAGS = (
        "--split-ledger-db --use-sharded-state-merkle-db --skip-index-and-usage"
    )

# Run the single node with performance optimizations enabled
target_directory = "execution/executor-benchmark/src"


def execute_command(command):
    print(f"Executing command:\n\t{command}\nand waiting for it to finish...")
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
    number_of_threads_results: Mapping[int, RunResults]
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
    number_of_execution_threads=EXECUTION_ONLY_NUMBER_OF_THREADS,
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
            [f"exe_only {num_threads}" for num_threads in number_of_execution_threads]
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
                for num_threads in number_of_execution_threads:
                    row.append(
                        field_getter(result.number_of_threads_results[num_threads])
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
    create_db_command = f"cargo run {BUILD_FLAG} -- --block-size {MAX_BLOCK_SIZE} --execution-threads {NUMBER_OF_EXECUTION_THREADS} {DB_CONFIG_FLAGS} create-db --data-dir {tmpdirname}/db --num-accounts {NUM_ACCOUNTS}"
    output = execute_command(create_db_command)

    results = []

    for (
        test_index,
        test,
    ) in enumerate(TESTS):
        if SELECTED_FLOW not in test.included_in:
            continue

        print(f"Testing {test.key}")
        if test.key.transaction_type_override == "":
            workload_args_str = ""
        else:
            transaction_type_list = (
                test.key.transaction_type_override or test.key.transaction_type
            )
            transaction_weights_list = test.key.transaction_weights_override or "1"
            workload_args_str = f"--transaction-type {transaction_type_list} --transaction-weights {transaction_weights_list}"

        cur_block_size = int(min([test.expected_tps, MAX_BLOCK_SIZE]))

        sharding_traffic_flags = test.key.sharding_traffic_flags or ""

        if test.key.executor_type == "VM":
            executor_type_str = "--transactions-per-sender 1"
        elif test.key.executor_type == "native":
            executor_type_str = "--use-native-executor --transactions-per-sender 1"
        elif test.key.executor_type == "sharded":
            executor_type_str = f"--async-partitioning --num-executor-shards {NUMBER_OF_EXECUTION_THREADS} {sharding_traffic_flags}"
        else:
            raise Exception(f"executor type not supported {test.key.executor_type}")
        common_command_suffix = f"{executor_type_str} --generate-then-execute --block-size {cur_block_size} {DB_CONFIG_FLAGS} run-executor {workload_args_str} --module-working-set-size {test.key.module_working_set_size} --main-signer-accounts {MAIN_SIGNER_ACCOUNTS} --additional-dst-pool-accounts {ADDITIONAL_DST_POOL_ACCOUNTS} --data-dir {tmpdirname}/db  --checkpoint-dir {tmpdirname}/cp"

        number_of_threads_results = {}

        for execution_threads in EXECUTION_ONLY_NUMBER_OF_THREADS:
            test_db_command = f"cargo run {BUILD_FLAG} -- --execution-threads {execution_threads} {common_command_suffix} --skip-commit --blocks {NUM_BLOCKS_DETAILED}"
            output = execute_command(test_db_command)

            number_of_threads_results[execution_threads] = extract_run_results(
                output, execution_only=True
            )

        test_db_command = f"cargo run {BUILD_FLAG} -- --execution-threads {NUMBER_OF_EXECUTION_THREADS} {common_command_suffix} --blocks {NUM_BLOCKS}"
        output = execute_command(test_db_command)

        single_node_result = extract_run_results(output, execution_only=False)

        results.append(
            RunGroupInstance(
                key=test.key,
                single_node_result=single_node_result,
                number_of_threads_results=number_of_threads_results,
                block_size=cur_block_size,
                expected_tps=test.expected_tps,
            )
        )

        # line to be able to aggreate and visualize in Humio
        print(
            json.dumps(
                {
                    "grep": "grep_json_single_node_perf",
                    "transaction_type": test.key.transaction_type,
                    "module_working_set_size": test.key.module_working_set_size,
                    "executor_type": test.key.executor_type,
                    "block_size": cur_block_size,
                    "expected_tps": test.expected_tps,
                    "waived": test.waived,
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

        if single_node_result.tps < test.expected_tps * NOISE_LOWER_LIMIT:
            text = f"regression detected {single_node_result.tps} < {test.expected_tps * NOISE_LOWER_LIMIT} = {test.expected_tps} * {NOISE_LOWER_LIMIT}, {test.key} didn't meet TPS requirements"
            if not test.waived:
                errors.append(text)
            else:
                warnings.append(text)
        elif single_node_result.tps < test.expected_tps * NOISE_LOWER_LIMIT_WARN:
            text = f"potential (but within normal noise) regression detected {single_node_result.tps} < {test.expected_tps * NOISE_LOWER_LIMIT_WARN} = {test.expected_tps} * {NOISE_LOWER_LIMIT_WARN}, {test.key} didn't meet TPS requirements"
            warnings.append(text)
        elif single_node_result.tps > test.expected_tps * NOISE_UPPER_LIMIT:
            text = f"perf improvement detected {single_node_result.tps} > {test.expected_tps * NOISE_UPPER_LIMIT} = {test.expected_tps} * {NOISE_UPPER_LIMIT}, {test.key} exceeded TPS requirements, increase TPS requirements to match new baseline"
            if not test.waived:
                errors.append(text)
            else:
                warnings.append(text)
        elif single_node_result.tps > test.expected_tps * NOISE_UPPER_LIMIT_WARN:
            text = f"potential (but within normal noise) perf improvement detected {single_node_result.tps} > {test.expected_tps * NOISE_UPPER_LIMIT_WARN} = {test.expected_tps} * {NOISE_UPPER_LIMIT_WARN}, {test.key} exceeded TPS requirements, increase TPS requirements to match new baseline"
            warnings.append(text)

if warnings:
    print("Warnings: ")
    print("\n".join(warnings))

if errors:
    print("Errors: ")
    print("\n".join(errors))
    exit(1)

exit(0)

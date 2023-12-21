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
    # Tests that are run continuously on main
    CONTINUOUS = auto()
    # Tests that are run manually when using a smaller representative mode.
    # (i.e. for measuring speed of the machine)
    REPRESENTATIVE = auto()
    # Tests used for previewnet evaluation
    PREVIEWNET = auto()
    # Tests used for previewnet evaluation
    PREVIEWNET_LARGE_DB = auto()
    # Tests for Agg V2 performance
    AGG_V2 = auto()
    # Test resource groups
    RESOURCE_GROUPS = auto()


# Tests that are run on LAND_BLOCKING and continuously on main
LAND_BLOCKING_AND_C = Flow.LAND_BLOCKING | Flow.CONTINUOUS


@dataclass
class RunGroupKey:
    transaction_type: str
    module_working_set_size: int = field(default=1)
    executor_type: str = field(default="VM")

    transaction_type_override: Optional[str] = field(default=None)
    transaction_weights_override: Optional[str] = field(default=None)
    sharding_traffic_flags: Optional[str] = field(default=None)

    smaller_working_set: bool = field(default=False)


@dataclass
class RunGroupConfig:
    key: RunGroupKey
    expected_tps: float
    included_in: Flow
    waived: bool = field(default=False)


SELECTED_FLOW = Flow[os.environ.get("FLOW", default="LAND_BLOCKING")]
IS_PREVIEWNET = SELECTED_FLOW in [Flow.PREVIEWNET, Flow.PREVIEWNET_LARGE_DB]

DEFAULT_NUM_INIT_ACCOUNTS = (
    "100000000" if SELECTED_FLOW == Flow.PREVIEWNET_LARGE_DB else "2000000"
)
DEFAULT_MAX_BLOCK_SIZE = "25000" if IS_PREVIEWNET else "10000"

MAX_BLOCK_SIZE = int(os.environ.get("MAX_BLOCK_SIZE", default=DEFAULT_MAX_BLOCK_SIZE))
NUM_BLOCKS = int(os.environ.get("NUM_BLOCKS_PER_TEST", default=15))
NUM_BLOCKS_DETAILED = 10
NUM_ACCOUNTS = max(
    [
        int(os.environ.get("NUM_INIT_ACCOUNTS", default=DEFAULT_NUM_INIT_ACCOUNTS)),
        (2 + 2 * NUM_BLOCKS) * MAX_BLOCK_SIZE,
    ]
)
MAIN_SIGNER_ACCOUNTS = 2 * MAX_BLOCK_SIZE

# numbers are based on the machine spec used by github action
# Calibrate using median value from
# Axiom: https://app.axiom.co/aptoslabs-hghf/explorer?qid=88fegG0H1si-s3x8pv&relative=1
# Humio: https://gist.github.com/igor-aptos/7b12ca28de03894cddda8e415f37889e
# Local machine numbers will be higher.
# For charts over time, you can modify the following query:
# https://app.axiom.co/aptoslabs-hghf/explorer?qid=29zYzeVi7FX-s4ukl5&relative=1
# fmt: off
TESTS = [
    RunGroupConfig(expected_tps=25300, key=RunGroupKey("no-op"), included_in=LAND_BLOCKING_AND_C),
    RunGroupConfig(expected_tps=3500, key=RunGroupKey("no-op", module_working_set_size=1000), included_in=LAND_BLOCKING_AND_C),
    RunGroupConfig(expected_tps=16200, key=RunGroupKey("coin-transfer"), included_in=LAND_BLOCKING_AND_C | Flow.REPRESENTATIVE),
    # this was changed from 42000 to make landings not flaky, needs follow up
    RunGroupConfig(expected_tps=35500, key=RunGroupKey("coin-transfer", executor_type="native"), included_in=LAND_BLOCKING_AND_C),
    RunGroupConfig(expected_tps=13500, key=RunGroupKey("account-generation"), included_in=LAND_BLOCKING_AND_C | Flow.REPRESENTATIVE),
    RunGroupConfig(expected_tps=33200, key=RunGroupKey("account-generation", executor_type="native"), included_in=Flow.CONTINUOUS),
    RunGroupConfig(expected_tps=22000, key=RunGroupKey("account-resource32-b"), included_in=LAND_BLOCKING_AND_C),
    RunGroupConfig(expected_tps=4150, key=RunGroupKey("modify-global-resource"), included_in=LAND_BLOCKING_AND_C | Flow.REPRESENTATIVE),
    RunGroupConfig(expected_tps=13000, key=RunGroupKey("modify-global-resource", module_working_set_size=10), included_in=Flow.CONTINUOUS),
    RunGroupConfig(expected_tps=140, key=RunGroupKey("publish-package"), included_in=LAND_BLOCKING_AND_C | Flow.REPRESENTATIVE),
    RunGroupConfig(expected_tps=2600, key=RunGroupKey(
        "mix_publish_transfer",
        transaction_type_override="publish-package coin-transfer",
        transaction_weights_override="1 500",
    ), included_in=LAND_BLOCKING_AND_C),
    RunGroupConfig(expected_tps=365, key=RunGroupKey("batch100-transfer"), included_in=LAND_BLOCKING_AND_C),
    RunGroupConfig(expected_tps=1100, key=RunGroupKey("batch100-transfer", executor_type="native"), included_in=Flow.CONTINUOUS),

    RunGroupConfig(expected_tps=165, key=RunGroupKey("vector-picture40"), included_in=Flow(0), waived=True),
    RunGroupConfig(expected_tps=1000, key=RunGroupKey("vector-picture40", module_working_set_size=20), included_in=Flow(0), waived=True),
    RunGroupConfig(expected_tps=160, key=RunGroupKey("vector-picture30k"), included_in=LAND_BLOCKING_AND_C),
    RunGroupConfig(expected_tps=1000, key=RunGroupKey("vector-picture30k", module_working_set_size=20), included_in=Flow.CONTINUOUS),
    RunGroupConfig(expected_tps=17, key=RunGroupKey("smart-table-picture30-k-with200-change"), included_in=LAND_BLOCKING_AND_C),
    RunGroupConfig(expected_tps=82, key=RunGroupKey("smart-table-picture30-k-with200-change", module_working_set_size=20), included_in=Flow.CONTINUOUS),
    RunGroupConfig(expected_tps=3, key=RunGroupKey("smart-table-picture1-m-with1-k-change"), included_in=LAND_BLOCKING_AND_C, waived=True),
    RunGroupConfig(expected_tps=12, key=RunGroupKey("smart-table-picture1-m-with1-k-change", module_working_set_size=20), included_in=Flow.CONTINUOUS),
    RunGroupConfig(expected_tps=5, key=RunGroupKey("smart-table-picture1-b-with1-k-change"), included_in=Flow(0), waived=True),
    RunGroupConfig(expected_tps=10, key=RunGroupKey("smart-table-picture1-b-with1-k-change", module_working_set_size=20), included_in=Flow(0), waived=True),

    RunGroupConfig(expected_tps=4050, key=RunGroupKey("modify-global-resource-agg-v2"), included_in=Flow.AGG_V2),
    RunGroupConfig(expected_tps=12500, key=RunGroupKey("modify-global-resource-agg-v2", module_working_set_size=50), included_in=Flow.AGG_V2),
    RunGroupConfig(expected_tps=4050, key=RunGroupKey("modify-global-flag-agg-v2"), included_in=Flow.AGG_V2),
    RunGroupConfig(expected_tps=12500, key=RunGroupKey("modify-global-flag-agg-v2", module_working_set_size=50), included_in=Flow.AGG_V2),
    RunGroupConfig(expected_tps=4050, key=RunGroupKey("modify-global-bounded-agg-v2"), included_in=Flow.AGG_V2),
    RunGroupConfig(expected_tps=12500, key=RunGroupKey("modify-global-bounded-agg-v2", module_working_set_size=50), included_in=Flow.AGG_V2),

    RunGroupConfig(expected_tps=4000, key=RunGroupKey("resource-groups-global-write-tag1-kb"), included_in=LAND_BLOCKING_AND_C | Flow.RESOURCE_GROUPS, waived=True),
    RunGroupConfig(expected_tps=8000, key=RunGroupKey("resource-groups-global-write-tag1-kb", module_working_set_size=20), included_in=Flow.RESOURCE_GROUPS, waived=True),
    RunGroupConfig(expected_tps=4000, key=RunGroupKey("resource-groups-global-write-and-read-tag1-kb"), included_in=Flow.CONTINUOUS | Flow.RESOURCE_GROUPS, waived=True),
    RunGroupConfig(expected_tps=8000, key=RunGroupKey("resource-groups-global-write-and-read-tag1-kb", module_working_set_size=20), included_in=Flow.RESOURCE_GROUPS, waived=True),
    RunGroupConfig(expected_tps=8000, key=RunGroupKey("resource-groups-sender-write-tag1-kb"), included_in=Flow.CONTINUOUS | Flow.RESOURCE_GROUPS, waived=True),
    RunGroupConfig(expected_tps=8000, key=RunGroupKey("resource-groups-sender-write-tag1-kb", module_working_set_size=20), included_in=Flow.RESOURCE_GROUPS, waived=True),
    RunGroupConfig(expected_tps=8000, key=RunGroupKey("resource-groups-sender-multi-change1-kb"), included_in=LAND_BLOCKING_AND_C | Flow.RESOURCE_GROUPS, waived=True),
    RunGroupConfig(expected_tps=8000, key=RunGroupKey("resource-groups-sender-multi-change1-kb", module_working_set_size=20), included_in=Flow.RESOURCE_GROUPS, waived=True),
    
    RunGroupConfig(expected_tps=1890, key=RunGroupKey("token-v1ft-mint-and-transfer"), included_in=Flow.CONTINUOUS),
    RunGroupConfig(expected_tps=9250, key=RunGroupKey("token-v1ft-mint-and-transfer", module_working_set_size=20), included_in=Flow.CONTINUOUS),
    RunGroupConfig(expected_tps=1100, key=RunGroupKey("token-v1nft-mint-and-transfer-sequential"), included_in=Flow.CONTINUOUS),
    RunGroupConfig(expected_tps=5900, key=RunGroupKey("token-v1nft-mint-and-transfer-sequential", module_working_set_size=20), included_in=Flow.CONTINUOUS),
    RunGroupConfig(expected_tps=1300, key=RunGroupKey("token-v1nft-mint-and-transfer-parallel"), included_in=Flow(0)),
    RunGroupConfig(expected_tps=5300, key=RunGroupKey("token-v1nft-mint-and-transfer-parallel", module_working_set_size=20), included_in=Flow(0)),

    # RunGroupConfig(expected_tps=1000, key=RunGroupKey("token-v1ft-mint-and-store"), included_in=Flow(0)),
    # RunGroupConfig(expected_tps=1000, key=RunGroupKey("token-v1nft-mint-and-store-sequential"), included_in=Flow(0)),
    # RunGroupConfig(expected_tps=1000, key=RunGroupKey("token-v1nft-mint-and-transfer-parallel"), included_in=Flow(0)),

    RunGroupConfig(expected_tps=25500, key=RunGroupKey("no-op5-signers"), included_in=Flow.CONTINUOUS),
   
    RunGroupConfig(expected_tps=1710, key=RunGroupKey("token-v2-ambassador-mint"), included_in=LAND_BLOCKING_AND_C | Flow.REPRESENTATIVE),
    RunGroupConfig(expected_tps=6900, key=RunGroupKey("token-v2-ambassador-mint", module_working_set_size=20), included_in=LAND_BLOCKING_AND_C | Flow.REPRESENTATIVE),

    RunGroupConfig(expected_tps=50000, key=RunGroupKey("coin_transfer_connected_components", executor_type="sharded", sharding_traffic_flags="--connected-tx-grps 5000", transaction_type_override=""), included_in=Flow.REPRESENTATIVE),
    RunGroupConfig(expected_tps=50000, key=RunGroupKey("coin_transfer_hotspot", executor_type="sharded", sharding_traffic_flags="--hotspot-probability 0.8", transaction_type_override=""), included_in=Flow.REPRESENTATIVE),

    # setting separately for previewnet, as we run on a different number of cores.
    RunGroupConfig(expected_tps=29000 if NUM_ACCOUNTS < 5000000 else 20000, key=RunGroupKey("coin-transfer", smaller_working_set=True), included_in=Flow.PREVIEWNET | Flow.PREVIEWNET_LARGE_DB),
    RunGroupConfig(expected_tps=23000 if NUM_ACCOUNTS < 5000000 else 15000, key=RunGroupKey("account-generation"), included_in=Flow.PREVIEWNET | Flow.PREVIEWNET_LARGE_DB),
    RunGroupConfig(expected_tps=130 if NUM_ACCOUNTS < 5000000 else 60, key=RunGroupKey("publish-package"), included_in=Flow.PREVIEWNET | Flow.PREVIEWNET_LARGE_DB),
    RunGroupConfig(expected_tps=1500 if NUM_ACCOUNTS < 5000000 else 1500, key=RunGroupKey("token-v2-ambassador-mint"), included_in=Flow.PREVIEWNET | Flow.PREVIEWNET_LARGE_DB),
    RunGroupConfig(expected_tps=12000 if NUM_ACCOUNTS < 5000000 else 7000, key=RunGroupKey("token-v2-ambassador-mint", module_working_set_size=100), included_in=Flow.PREVIEWNET | Flow.PREVIEWNET_LARGE_DB),
    RunGroupConfig(expected_tps=35000 if NUM_ACCOUNTS < 5000000 else 28000, key=RunGroupKey("coin_transfer_connected_components", executor_type="sharded", sharding_traffic_flags="--connected-tx-grps 5000", transaction_type_override=""), included_in=Flow.PREVIEWNET | Flow.PREVIEWNET_LARGE_DB, waived=True),
    RunGroupConfig(expected_tps=27000 if NUM_ACCOUNTS < 5000000 else 23000, key=RunGroupKey("coin_transfer_hotspot", executor_type="sharded", sharding_traffic_flags="--hotspot-probability 0.8", transaction_type_override=""), included_in=Flow.PREVIEWNET | Flow.PREVIEWNET_LARGE_DB, waived=True),
]
# fmt: on

NOISE_LOWER_LIMIT = 0.98 if IS_PREVIEWNET else 0.8
NOISE_LOWER_LIMIT_WARN = None if IS_PREVIEWNET else 0.9
# If you want to calibrate the upper limit for perf improvement, you can
# increase this value temporarily (i.e. to 1.3) and readjust back after a day or two of runs
NOISE_UPPER_LIMIT = 5 if IS_PREVIEWNET else 1.15
NOISE_UPPER_LIMIT_WARN = None if IS_PREVIEWNET else 1.05

# bump after a perf improvement, so you can easily distinguish runs
# that are on top of this commit
CODE_PERF_VERSION = "v4"

# default to using production number of execution threads for assertions
NUMBER_OF_EXECUTION_THREADS = int(
    os.environ.get("NUMBER_OF_EXECUTION_THREADS", default=8)
)

if os.environ.get("DETAILED"):
    EXECUTION_ONLY_NUMBER_OF_THREADS = [1, 2, 4, 8, 16, 32, 60]
else:
    EXECUTION_ONLY_NUMBER_OF_THREADS = []

if os.environ.get("RELEASE_BUILD"):
    BUILD_FLAG = "--release"
    BUILD_FOLDER = "target/release"
else:
    BUILD_FLAG = "--profile performance"
    BUILD_FOLDER = "target/performance"

if os.environ.get("PROD_DB_FLAGS"):
    DB_CONFIG_FLAGS = ""
else:
    DB_CONFIG_FLAGS = "--enable-storage-sharding"

if os.environ.get("ENABLE_PRUNER"):
    DB_PRUNER_FLAGS = "--enable-state-pruner --enable-ledger-pruner --enable-epoch-snapshot-pruner --ledger-pruning-batch-size 10000 --state-prune-window 3000000 --epoch-snapshot-prune-window 3000000 --ledger-prune-window 3000000"
else:
    DB_PRUNER_FLAGS = ""

HIDE_OUTPUT = os.environ.get("HIDE_OUTPUT")

# Run the single node with performance optimizations enabled
target_directory = "execution/executor-benchmark/src"


def execute_command(command):
    print(f"Executing command:\n\t{command}\nand waiting for it to finish...")
    result = []
    with Popen(
        command,
        shell=True,
        text=True,
        stdout=PIPE,
        bufsize=1,
        universal_newlines=True,
    ) as p:
        # stream to output while command is executing
        if p.stdout is not None:
            for line in p.stdout:
                if not HIDE_OUTPUT:
                    print(line, end="")
                result.append(line)

    # return the full output in the end for postprocessing
    full_result = "\n".join(result)

    if p.returncode != 0:
        if HIDE_OUTPUT:
            print(full_result)
        raise CalledProcessError(p.returncode, p.args)

    if " ERROR " in full_result:
        print("ERROR log line in execution")
        if HIDE_OUTPUT:
            print(full_result)
        exit(1)

    return full_result


@dataclass
class RunResults:
    tps: float
    gps: float
    effective_gps: float
    io_gps: float
    execution_gps: float
    gpt: float
    output_bps: float
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


def extract_run_results(
    output: str, execution_only: bool, create_db: bool = False
) -> RunResults:
    if execution_only:
        tps = float(re.findall(r"Overall execution TPS: (\d+\.?\d*) txn/s", output)[-1])
        gps = float(re.findall(r"Overall execution GPS: (\d+\.?\d*) gas/s", output)[-1])
        effective_gps = float(
            re.findall(r"Overall execution effectiveGPS: (\d+\.?\d*) gas/s", output)[-1]
        )
        io_gps = float(
            re.findall(r"Overall execution ioGPS: (\d+\.?\d*) gas/s", output)[-1]
        )
        execution_gps = float(
            re.findall(r"Overall execution executionGPS: (\d+\.?\d*) gas/s", output)[-1]
        )
        gpt = float(
            re.findall(r"Overall execution GPT: (\d+\.?\d*) gas/txn", output)[-1]
        )
        output_bps = float(
            re.findall(r"Overall execution output: (\d+\.?\d*) bytes/s", output)[-1]
        )
    elif create_db:
        tps = float(
            get_only(
                re.findall(
                    r"Overall TPS: create_db: account creation: (\d+\.?\d*) txn/s",
                    output,
                )
            )
        )
        gps = 0
        effective_gps = 0
        io_gps = 0
        execution_gps = 0
        gpt = 0
        output_bps = 0
    else:
        tps = float(get_only(re.findall(r"Overall TPS: (\d+\.?\d*) txn/s", output)))
        gps = float(get_only(re.findall(r"Overall GPS: (\d+\.?\d*) gas/s", output)))
        effective_gps = float(
            get_only(re.findall(r"Overall effectiveGPS: (\d+\.?\d*) gas/s", output))
        )
        io_gps = float(
            get_only(re.findall(r"Overall ioGPS: (\d+\.?\d*) gas/s", output))
        )
        execution_gps = float(
            get_only(re.findall(r"Overall executionGPS: (\d+\.?\d*) gas/s", output))
        )
        gpt = float(get_only(re.findall(r"Overall GPT: (\d+\.?\d*) gas/txn", output)))
        output_bps = float(
            re.findall(r"Overall output: (\d+\.?\d*) bytes/s", output)[-1]
        )

    if create_db:
        fraction_in_execution = 0
        fraction_of_execution_in_vm = 0
        fraction_in_commit = 0
    else:
        fraction_in_execution = float(
            re.findall(r"Overall fraction of total: (\d+\.?\d*) in execution", output)[
                -1
            ]
        )
        fraction_of_execution_in_vm = float(
            re.findall(r"Overall fraction of execution (\d+\.?\d*) in VM", output)[-1]
        )
        fraction_in_commit = float(
            re.findall(r"Overall fraction of total: (\d+\.?\d*) in commit", output)[-1]
        )

    return RunResults(
        tps=tps,
        gps=gps,
        effective_gps=effective_gps,
        io_gps=io_gps,
        execution_gps=execution_gps,
        gpt=gpt,
        output_bps=output_bps,
        fraction_in_execution=fraction_in_execution,
        fraction_of_execution_in_vm=fraction_of_execution_in_vm,
        fraction_in_commit=fraction_in_commit,
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
        headers.extend(
            [
                "t/s",
                "exe/total",
                "vm/exe",
                "commit/total",
                "g/s",
                "eff g/s",
                "io g/s",
                "exe g/s",
                "g/t",
                "out B/s",
            ]
        )

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
            row.append(int(round(result.single_node_result.effective_gps)))
            row.append(int(round(result.single_node_result.io_gps)))
            row.append(int(round(result.single_node_result.execution_gps)))
            row.append(int(round(result.single_node_result.gpt)))
            row.append(int(round(result.single_node_result.output_bps)))
        rows.append(row)

    print(tabulate(rows, headers=headers))


errors = []
warnings = []

with tempfile.TemporaryDirectory() as tmpdirname:
    execute_command(f"cargo build {BUILD_FLAG} --package aptos-executor-benchmark")

    print(f"Warmup - creating DB with {NUM_ACCOUNTS} accounts")
    create_db_command = f"RUST_BACKTRACE=1 {BUILD_FOLDER}/aptos-executor-benchmark --block-size {MAX_BLOCK_SIZE} --execution-threads {NUMBER_OF_EXECUTION_THREADS} {DB_CONFIG_FLAGS} {DB_PRUNER_FLAGS} create-db --data-dir {tmpdirname}/db --num-accounts {NUM_ACCOUNTS}"
    output = execute_command(create_db_command)

    results = []

    results.append(
        RunGroupInstance(
            key=RunGroupKey("warmup"),
            single_node_result=extract_run_results(
                output, execution_only=False, create_db=True
            ),
            number_of_threads_results={},
            block_size=MAX_BLOCK_SIZE,
            expected_tps=0,
        )
    )

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
            executor_type_str = f"--num-executor-shards {NUMBER_OF_EXECUTION_THREADS} {sharding_traffic_flags}"
        else:
            raise Exception(f"executor type not supported {test.key.executor_type}")
        txn_emitter_prefix_str = "" if NUM_BLOCKS > 200 else " --generate-then-execute"

        ADDITIONAL_DST_POOL_ACCOUNTS = (
            2 * MAX_BLOCK_SIZE * (1 if test.key.smaller_working_set else NUM_BLOCKS)
        )

        common_command_suffix = f"{executor_type_str} {txn_emitter_prefix_str} --block-size {cur_block_size} {DB_CONFIG_FLAGS} {DB_PRUNER_FLAGS} run-executor {workload_args_str} --module-working-set-size {test.key.module_working_set_size} --main-signer-accounts {MAIN_SIGNER_ACCOUNTS} --additional-dst-pool-accounts {ADDITIONAL_DST_POOL_ACCOUNTS} --data-dir {tmpdirname}/db  --checkpoint-dir {tmpdirname}/cp"

        number_of_threads_results = {}

        for execution_threads in EXECUTION_ONLY_NUMBER_OF_THREADS:
            test_db_command = f"RUST_BACKTRACE=1 {BUILD_FOLDER}/aptos-executor-benchmark --execution-threads {execution_threads} {common_command_suffix} --skip-commit --blocks {NUM_BLOCKS_DETAILED}"
            output = execute_command(test_db_command)

            number_of_threads_results[execution_threads] = extract_run_results(
                output, execution_only=True
            )

        test_db_command = f"RUST_BACKTRACE=1 {BUILD_FOLDER}/aptos-executor-benchmark --execution-threads {NUMBER_OF_EXECUTION_THREADS} {common_command_suffix} --blocks {NUM_BLOCKS}"
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

        if not HIDE_OUTPUT:
            print_table(
                results,
                by_levels=True,
                single_field=("t/s", lambda r: int(round(r.tps))),
            )
            print_table(
                results,
                by_levels=True,
                single_field=("g/s", lambda r: int(round(r.gps))),
            )
            print_table(
                results,
                by_levels=True,
                single_field=("exe/total", lambda r: round(r.fraction_in_execution, 3)),
            )
            print_table(
                results,
                by_levels=True,
                single_field=(
                    "vm/exe",
                    lambda r: round(r.fraction_of_execution_in_vm, 3),
                ),
            )
            print_table(results, by_levels=False, single_field=None)

        if (
            NOISE_LOWER_LIMIT is not None
            and single_node_result.tps < test.expected_tps * NOISE_LOWER_LIMIT
        ):
            text = f"regression detected {single_node_result.tps} < {test.expected_tps * NOISE_LOWER_LIMIT} = {test.expected_tps} * {NOISE_LOWER_LIMIT}, {test.key} didn't meet TPS requirements"
            if not test.waived:
                errors.append(text)
            else:
                warnings.append(text)
        elif (
            NOISE_LOWER_LIMIT_WARN is not None
            and single_node_result.tps < test.expected_tps * NOISE_LOWER_LIMIT_WARN
        ):
            text = f"potential (but within normal noise) regression detected {single_node_result.tps} < {test.expected_tps * NOISE_LOWER_LIMIT_WARN} = {test.expected_tps} * {NOISE_LOWER_LIMIT_WARN}, {test.key} didn't meet TPS requirements"
            warnings.append(text)
        elif (
            NOISE_UPPER_LIMIT is not None
            and single_node_result.tps > test.expected_tps * NOISE_UPPER_LIMIT
        ):
            text = f"perf improvement detected {single_node_result.tps} > {test.expected_tps * NOISE_UPPER_LIMIT} = {test.expected_tps} * {NOISE_UPPER_LIMIT}, {test.key} exceeded TPS requirements, increase TPS requirements to match new baseline"
            if not test.waived:
                errors.append(text)
            else:
                warnings.append(text)
        elif (
            NOISE_UPPER_LIMIT_WARN is not None
            and single_node_result.tps > test.expected_tps * NOISE_UPPER_LIMIT_WARN
        ):
            text = f"potential (but within normal noise) perf improvement detected {single_node_result.tps} > {test.expected_tps * NOISE_UPPER_LIMIT_WARN} = {test.expected_tps} * {NOISE_UPPER_LIMIT_WARN}, {test.key} exceeded TPS requirements, increase TPS requirements to match new baseline"
            warnings.append(text)

if HIDE_OUTPUT:
    print_table(results, by_levels=False, single_field=None)

if warnings:
    print("Warnings: ")
    print("\n".join(warnings))

if errors:
    print("Errors: ")
    print("\n".join(errors))
    exit(1)

exit(0)

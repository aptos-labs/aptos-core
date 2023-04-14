#!/usr/bin/env python3

# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

import subprocess
import re

# Set the tps and speedup ratio threshold for block size 1k, 10k and 50k
THRESHOLDS = {
    "1k_8": 16000,
    "1k_16": 19000,
    "1k_32": 18000,
    "10k_8": 33000,
    "10k_16": 56000,
    "10k_32": 80000,
    "50k_8": 37000,
    "50k_16": 68000,
    "50k_32": 109000,
}

SPEEDUPS = {
    "1k_8": 2,
    "1k_16": 3,
    "1k_32": 3,
    "10k_8": 5,
    "10k_16": 9,
    "10k_32": 12,
    "50k_8": 5,
    "50k_16": 11,
    "50k_32": 17,
}

THREADS = [8, 16, 32]
BLOCK_SIZES = ["1k", "10k", "50k"]
target_directory = "aptos-move/aptos-transaction-benchmarks/src/"

tps_set = {}
speedups_set = {}

for threads in THREADS:
    # command = f"cargo run --profile performance main true true"
    command = f"taskset -c 0-{threads-1} cargo run --profile performance main true true"
    output = subprocess.check_output(
        command, shell=True, text=True, cwd=target_directory
    )
    print(output)
    for i, block_size in enumerate(BLOCK_SIZES):
        tps_index = i * 2
        speedup_index = i * 2 + 1
        key = f"{block_size}_{threads}"
        tps = int(re.findall(r"Avg Parallel TPS = (\d+)", output)[i])
        speedups = int(re.findall(r"Speed up (\d+)x over sequential", output)[i])
        tps_set[key] = tps
        speedups_set[key] = speedups
        if tps < THRESHOLDS[key]:
            print(
                f"Parallel TPS {tps} below the threshold {THRESHOLDS[key]} for {block_size} block size with {threads} threads"
            )
            exit(1)
        if speedups < SPEEDUPS[key]:
            print(
                f"Parallel SPEEDUPS {speedups} below the threshold {SPEEDUPS[key]} for {block_size} block size with {threads} threads"
            )
            exit(1)

for block_size in BLOCK_SIZES:
    for threads in THREADS:
        key = f"{block_size}_{threads}"
        print(
            f"Average Parallel TPS with {threads} threads for {block_size} block: TPS {tps_set[key]}, Threshold TPS: {THRESHOLDS[key]}, Speedup: {speedups_set[key]}x, Speedup Threshold: {SPEEDUPS[key]}x"
        )

print("Parallel TPS and Speedup are above the threshold")

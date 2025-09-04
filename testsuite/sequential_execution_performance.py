#!/usr/bin/env python3

import subprocess, re

# Set the tps threshold for block size 1k, 10k and 50k
BLOCK_SIZES = ["1k", "10k", "50k"]
THRESHOLDS = {"1k": 3500, "10k": 4000, "50k": 4200}
THRESHOLD_NOISE = 0.1

# Run the VM sequential execution with performance optimizations enabled
target_directory = "velor-move/velor-transaction-benchmarks/src/"
output = subprocess.check_output(
    "cargo run --profile performance  param-sweep  --skip-parallel",
    shell=True,
    text=True,
    cwd=target_directory,
)
print(output)

fail = False
for i, block_size in enumerate(BLOCK_SIZES):
    tps = int(re.findall(r"Avg Sequential TPS = (\d+)", output)[i])
    print(
        f"Average Sequential TPS for {block_size} block: {tps}, Threshold TPS: {THRESHOLDS[block_size]}"
    )
    diff = (tps - THRESHOLDS[block_size]) / THRESHOLDS[block_size]
    if abs(diff) > THRESHOLD_NOISE:
        print(
            f"Sequential TPS {tps} {'below' if diff < 0 else 'above'} the threshold {THRESHOLDS[block_size]} by {abs(diff)*100:.0f}% (more than {THRESHOLD_NOISE*100:.0f}%). Please {'optimize' if diff < 0 else 'increase the hard-coded TPS threshold since you improved'} the execution performance. :)\n"
        )
        fail = True
if fail:
    exit(1)

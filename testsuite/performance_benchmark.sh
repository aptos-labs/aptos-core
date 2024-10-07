#!/bin/bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

# Set the maximum number of file descriptors
ulimit -n 1048576

# Function to run the benchmark
run_benchmark() {
    FLOW=$1\
    NUM_BLOCKS_PER_TEST=$2\
    HIDE_OUTPUT=$3\
    ENABLE_PRUNER=$4\
    NUMBER_OF_EXECUTION_THREADS=$5\
    SKIP_MOVE_E2E=1\
    ./testsuite/single_node_performance.py
}

VIRTUAL_CORES=$(getconf _NPROCESSORS_ONLN)

DEFAULT_THREADS=$(($VIRTUAL_CORES > 64 ? 32 : $VIRTUAL_CORES / 2))

THREADS="${NUMBER_OF_EXECUTION_THREADS:-$DEFAULT_THREADS}"

echo "Using NUMBER_OF_EXECUTION_THREADS = $THREADS (found $VIRTUAL_CORES virtual cores)"

# Check for the flag
if [ "$1" == "--short" ]; then
    echo "Running short benchmark..."
    run_benchmark "MAINNET" 50 1 0 $THREADS
elif [ "$1" == "--long" ]; then
    echo "Running long benchmark..."
    run_benchmark "MAINNET_LARGE_DB" 300 1 1 $THREADS
else
    echo "Usage: $0 [--short | --long]"
    exit 1
fi

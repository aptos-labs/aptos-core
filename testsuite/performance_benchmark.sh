#!/bin/bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

# Set the maximum number of file descriptors
ulimit -n 1048576

# Function to run the benchmark
run_benchmark() {
    FLOW=$1
    NUM_BLOCKS_PER_TEST=$2
    HIDE_OUTPUT=$3
    ENABLE_PRUNER=$4
    NUMBER_OF_EXECUTION_THREADS=$5

    ./testsuite/single_node_performance.py \
        FLOW=$FLOW \
        NUM_BLOCKS_PER_TEST=$NUM_BLOCKS_PER_TEST \
        HIDE_OUTPUT=$HIDE_OUTPUT \
        ENABLE_PRUNER=$ENABLE_PRUNER \
        NUMBER_OF_EXECUTION_THREADS=$NUMBER_OF_EXECUTION_THREADS
}

# Check for the flag
if [ "$1" == "--short" ]; then
    echo "Running short benchmark..."
    run_benchmark "MAINNET" 50 1 0 32
elif [ "$1" == "--long" ]; then
    echo "Running long benchmark..."
    run_benchmark "MAINNET_LARGE_DB" 300 1 1 32
else
    echo "Usage: $0 [--short | --long]"
    exit 1
fi


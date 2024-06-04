#!/bin/bash

# Usage:
# coverage.sh gen <fuzz_target>
# coverage.sh clean <fuzz_target>
# coverage.sh clean all
#
# This script runs `cargo cov -- show` to generate the coverage report from a
# fuzzing session in HTML format.
# The script should only be run after the fuzzing session has been completed,
# since it uses `fuzz/corpus/<fuzz_target>` to generate the coverage report.
# The coverage report is generated under the `coverage` directory.
# The script can also be used to cleanup the generated coverage files.

MOVE_SMITH_DIR=$(realpath $(dirname $0)/..)

function usage() {
    case "$1" in
        "gen")
            echo "Usage: $0 gen <fuzz_target>"
            ;;
        "clean")
            echo "Usage: $0 clean <fuzz_target|all>"
            ;;
        *)
            echo "Usage: $0 <gen|clean>"
            echo "    gen               generate the HTML coverage report"
            echo "    clean             cleanup generated coverage files"
            ;;
    esac
    exit 1
}

function gen() {
    if [ "$#" -ne 1 ]; then
        usage gen
    fi

    local fuzz_target="$1"
    local target_dir="coverage/$fuzz_target"
    mkdir -p $target_dir

    if [ ! -d "fuzz/coverage/$fuzz_target" ]; then
        cargo fuzz coverage $fuzz_target
    fi

    fuzz_target_bin=$(find target/*/coverage -name $fuzz_target -type f)
    echo "Found fuzz target binary: $fuzz_target_bin"
    # Generate the coverage report
    cargo cov -- show $fuzz_target_bin \
        --format=html \
        --instr-profile=fuzz/coverage/$fuzz_target/coverage.profdata \
        --show-directory-coverage \
        --output-dir=$target_dir \
        -Xdemangler=rustfilt \
        --ignore-filename-regex='rustc/.*/library|\.cargo'
}

function clean() {
    if [ "$#" -ne 1 ]; then
        usage clean
    fi

    local fuzz_target="$1"
    local target_dir="coverage/$fuzz_target"

    if [ "$fuzz_target" == "all" ]; then
        rm -rf coverage
    else
        rm -rf $target_dir
    fi
}

curr=$(pwd)
if [ $curr != $MOVE_SMITH_DIR ]; then
    echo "Please run the script from the move-smith directory"
    exit 1
fi

case "$1" in
  "gen")
    shift
    gen "$@"
    ;;
  "clean")
    shift
    clean "$@"
    ;;
  *)
    usage general
    ;;
esac

#!/bin/bash

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

    # Generate the coverage report
    # TODO: Do not hardcode the target triple
    cargo cov -- show target/aarch64-apple-darwin/coverage/aarch64-apple-darwin/release/$fuzz_target \
        --format=html \
        --instr-profile=fuzz/coverage/$fuzz_target/coverage.profdata \
        --show-directory-coverage \
        --output-dir=$target_dir \
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

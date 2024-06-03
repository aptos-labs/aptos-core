#!/bin/bash

# Usage: transactional-test.sh [NUM_PROG]
#
# This script generates NUM_PROG Move programs and runs them as transactional
# tests by:
# 1. Applying a patch to run only MoveSmith generated files.
# 2. Generating Move programs and store them under:
#    `third_party/move/move-compiler-v2/transactional-tests/tests/move-smith`
# 3. Running the transactional tests with `cargo nextest run`

NUM_PROG=${1:-10}
PROJ_ROOT=$(realpath $(dirname $0)/../../../../..)
TEST_DIR=$PROJ_ROOT/third_party/move/move-compiler-v2/transactional-tests/tests/move-smith
PATCH_FILE=$PROJ_ROOT/third_party/move/tools/move-smith/scripts/transactional-tests.patch

(
    cd $PROJ_ROOT
    git apply $PATCH_FILE
)

mkdir -p $TEST_DIR
rm -rf $TEST_DIR/*.move
rm -rf $TEST_DIR/*.exp
cargo run --bin generator -- -o $TEST_DIR -s 1234 -n $NUM_PROG

cd $TEST_DIR
UB=1 cargo nextest run
cargo nextest run

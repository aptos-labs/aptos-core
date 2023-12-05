#!/bin/bash
set -e
BASE=$(git rev-parse --show-toplevel)
echo "Assuming repo root at $BASE"
cd $BASE/third_party/move
cargo run -p testdiff -- -m > $BASE/third_party/move/move-compiler-v2/tests/v1.matched
cargo run -p testdiff -- -u > $BASE/third_party/move/move-compiler-v2/tests/v1.unmatched

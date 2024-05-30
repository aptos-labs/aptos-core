#!/bin/bash

NUM_PROG=${1:-10}
PARENT_DIR=$(realpath $(dirname $0)/..)
OUTPUT_DIR="$PARENT_DIR/output"

function get_error() {
  find $1 -name compile.log | while read f; do
    grep "error\[E" $f
  done | sort | uniq
}

function check_compile() {
  if [ -d "$p" ]; then
    echo "Checking $1"
    (
      cd "$1"
      aptos move compile > compile.log 2>&1
      if [ $? -ne 0 ]; then
        echo "Compile $1: failed"
        get_error .
      else
        echo "Compile $1: success"
      fi
    )
  fi
}

rm -rf $OUTPUT_DIR
cargo run --bin generator -- -o $OUTPUT_DIR -s 1234 -p -n $NUM_PROG

N=8
for p in $OUTPUT_DIR/*; do
  ((i=i%N)); ((i++==0)) && wait
  check_compile $p &
done

wait

echo
echo "Errors are:"
get_error $OUTPUT_DIR

#!/bin/bash

# Usage: check_output.sh [NUM_PROG]
#
# This script runs the static generator to generate NUM_PROG Move packages
# and checks if they can compile with compiler V1.
# The generated packages are stored under `output`.

NUM_PROG=${1:-10}
MOVE_SMITH_DIR=$(realpath $(dirname $0)/..)
OUTPUT_DIR="$MOVE_SMITH_DIR/output"
APTOS_DIR=$(realpath $MOVE_SMITH_DIR/../../../..)

function get_error() {
  find $1 -name compile.log | while read f; do
    grep "error\[E" $f
  done | sort | uniq
}

function check_compile() {
  if [ -d "$1" ]; then
    echo "Checking $1"
    (
      cd "$1"
      aptos move compile --compiler-version v2 --language-version 2 > /dev/null 2>&1
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

function check_run_transactional() {
  RT_BIN=$2
  if [ -d "$p" ]; then
    move_file=$(realpath $(find "$1/sources" -name "*.move"))
    timeout 3 $RT_BIN "$move_file"
  fi
}

start_time=$(date +%s)

rm -rf $OUTPUT_DIR
cargo_start_time=$(date +%s)
cargo run --bin generator -- -o $OUTPUT_DIR -s 1234 -p -n $NUM_PROG
cargo_end_time=$(date +%s)

N=8
for p in $OUTPUT_DIR/*; do
  ((i=i%N)); ((i++==0)) && wait
  check_compile $p &
done
wait
compile_end_time=$(date +%s)

# Run transactional tests
cargo build --bin run_transactional
RT_BIN=$(realpath $(find $APTOS_DIR/target/ -name "run_transactional"))

for p in $OUTPUT_DIR/*; do
  ((i=i%N)); ((i++==0)) && wait
  check_run_transactional $p $RT_BIN &
done
wait
end_time=$(date +%s)


echo
printf "\033[1;32mChecking stats:\n"
num_succ=$(find output -type d -name build | wc -l | xargs)
echo "Out out $NUM_PROG packages, $num_succ can be compiled successfully."

errors=$(get_error $OUTPUT_DIR)
if [ -z "$errors" ]; then
  echo "No errors found"
else
  printf "Errors are:\033[0m\n"
  echo "$errors"
fi

total_time=$((end_time - start_time))
cargo_time=$((cargo_end_time - cargo_start_time))
compile_time=$((compile_end_time - cargo_end_time))
transactional_time=$((end_time - compile_end_time))

# Print out time profiling results
echo
printf "\033[1;33mTime profiling results:\n"
echo "Total time: $total_time seconds"
echo "Time to generate $NUM_PROG packages: $cargo_time seconds x 1 thread"
echo "Time to compile $NUM_PROG packages: $compile_time seconds x $N threads"
echo "Time to run $NUM_PROG transactional: $transactional_time seconds x $N threads"
printf "\033[0m\n"

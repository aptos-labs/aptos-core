#!/usr/bin/env bash

# usage: ./scripts/check_output.sh [options...]
# options:
#   --num [TXT]: Number of packages to generate. Default: 10
#   --jobs [TXT]: Number of threads. Defaul: 8
#   --skiprun: Skip running transactional test.
#   --skipcompile: Skip checking compilation.
#
# This script runs the static generator to generate $num Move packages
# and checks if they can compile with compiler V1.
# The generated packages are stored under `output`.

MOVE_SMITH_DIR=$(realpath $(dirname $0)/..)
OUTPUT_DIR="$MOVE_SMITH_DIR/output"
APTOS_DIR=$(realpath $MOVE_SMITH_DIR/../../../..)

source $MOVE_SMITH_DIR/scripts/argparse.sh

RED="\033[0;31m"
NOCOLOR="\033[0m"
YELLOW="\033[1;33m"
GREEN="\033[1;32m"

function get_compile_error() {
  find $1 -name compile.log | while read f; do
    grep "error\[E" $f
  done | sort | uniq -c | sort -nr
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
        get_compile_error .
      else
        echo "Compile $1: success"
      fi
    )
  fi
}

function get_failed_files() {
  local wkd=$1
  local fn=$2
  local pattern=$3
  find $wkd -name $fn | while read f; do
    if grep -q "$pattern" $f; then
      echo $(dirname $f)
    fi
  done
}

function check_run_transactional() {
  RT_BIN=$2
  if [ -d "$1" ]; then
    move_file=$(realpath $(find "$1/sources" -name "*.move"))
    (
      cd "$1"
      timeout 3 $RT_BIN "$move_file" | tee run.log 2>&1
      if [ $? -ne 0 ]; then
        echo "Run $1: failed"
      else
        echo "Run $1: success"
      fi
    )
  fi
}


define_arg "num" "10" "Number of packages to generate. Default: 10" "string" "false"
define_arg "jobs" "8" "Number of threads. Defaul: 8" "string" "false"
define_arg "skipcompile" "false" "Skip checking compilation." "store_true" "false"
define_arg "skiprun" "false" "Skip running transactional test." "store_true" "false"
parse_args "$@"

num_prog=$num

start_time=$(date +%s)

rm -rf $OUTPUT_DIR
cargo_start_time=$(date +%s)
cargo run --bin generator -- -o $OUTPUT_DIR -s 1234 -p -n $num_prog
if [ $? -ne 0 ]; then
  echo "Failed to generate Move packages"
  exit 1
fi
cargo_end_time=$(date +%s)

if $skipcompile; then
  echo "Skip compiling..."
else
  for p in $OUTPUT_DIR/*; do
    ((i=i%jobs)); ((i++==0)) && wait
    check_compile $p &
  done
  wait
fi
compile_end_time=$(date +%s)

transactional_start_time=$(date +%s)
if $skiprun; then
  echo "Skip running transactional..."
else
  echo "Building run_transactional binary"
  cargo build --bin run_transactional > /dev/null 2>&1
  RT_BIN=$(realpath $(find $APTOS_DIR/target/ -name "run_transactional"))

  transactional_start_time=$(date +%s)
  for p in $OUTPUT_DIR/*; do
    ((i=i%jobs)); ((i++==0)) && wait
    check_run_transactional $p $RT_BIN &
  done
  wait
fi
end_time=$(date +%s)


echo
printf "${GREEN}Checking stats:\n"
num_succ=$(find output -type d -name build | wc -l | xargs)
echo "Out out $num_prog packages, $num_succ can be compiled successfully."
printf $NOCOLOR

errors=$(get_compile_error $OUTPUT_DIR)
if [ -z "$errors" ]; then
  echo "No errors found"
else
  printf $RED
  echo "Errors are:"
  printf $NOCOLOR
  echo "$errors"
fi

echo "Packages failed to compile:"
printf $RED
get_failed_files $OUTPUT_DIR compile.log "error\[E"
printf $NOCOLOR

echo "Packages failed to run transactional test:"
printf $RED
get_failed_files $OUTPUT_DIR run.log "Transactional test failed"
printf $NOCOLOR

total_time=$((end_time - start_time))
cargo_time=$((cargo_end_time - cargo_start_time))
compile_time=$((compile_end_time - cargo_end_time))
transactional_time=$((end_time - transactional_start_time))

# Print out time profiling results
echo
printf $YELLOW
echo "Time profiling results:"
echo "Total time: $total_time seconds"
echo "Time to generate $num_prog packages: $cargo_time seconds x 1 thread"
echo "Time to compile $num_prog packages: $compile_time seconds x $jobs threads"
echo "Time to run $num_prog transactional: $transactional_time seconds x $jobs threads"
printf $NOCOLOR

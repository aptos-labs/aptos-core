#!/bin/bash

# This script runs a fuzz target for a given number of hours.
#
# Usage: ./scripts/fuzz.sh <fuzz_target> [total_hour] [max_input_len]
#
# * Keep a log file under move-smith/logs
# * Creates an initial corpus with 8KB inputs by default, specified by `max_input_len``
# * Runs the fuzz target for the given number of hours
# * If the fuzz target starts with 'afl', it runs AFL in tmux sessions

MOVE_SMITH_DIR=$(realpath $(dirname $0)/..)
APTOS_DIR=$(realpath $MOVE_SMITH_DIR/../../../..)

JOBS=10
TMUX_SESSION="afl_fuzzing"

function create_log() {
    local log_dir=$1
    mkdir -p $log_dir
    # Count the number of files in the logs directory
    log_count=$(ls logs | wc -l | xargs)
    # Return the next log file name
    echo "$log_dir/fuzz-$log_count.log"
}

# This is needed since libfuzzer doesn't respect the -max_len flag
function create_initial_corpus() {
    local corpus_dir=$1
    local input_len=$2    # in KB

    mkdir -p $corpus_dir

    for i in {0..9}; do
        large=$corpus_dir/random_input_large_$i
        mid=$corpus_dir/random_input_mid_$i
        small=$corpus_dir/random_input_small_$i
        dd if=/dev/urandom of=$large ibs=1024 count=$input_len 2>/dev/null
        dd if=/dev/urandom of=$mid ibs=512 count=$input_len 2>/dev/null
        dd if=/dev/urandom of=$small ibs=256 count=$input_len 2>/dev/null
    done
}

function run_libfuzzer() {
    local fuzz_target=$1
    local total_hour=$2
    local input_len=$3

    # Convert hours to seconds, convert to integer
    local total_seconds=$(echo "$total_hour * 3600" | bc)
    local log_file=$(create_log "$MOVE_SMITH_DIR/logs")
    echo "Writing logs to $log_file"

    local corpus_dir=$MOVE_SMITH_DIR/fuzz/corpus/$fuzz_target
    create_initial_corpus $corpus_dir $input_len

    echo "Current date time: $(date)" | tee -a $log_file
    echo "Created initial corpus for $fuzz_target, size: $input_len KB" | tee -a $log_file
    echo "Running fuzz target: $fuzz_target for $total_hour hours" | tee -a $log_file

    # Disable ASAN only on Linux
    # Disabling ASAN on macOS fails to build
    local asan_flag=""
    if [[ "$OSTYPE" == "linux-gnu" ]]; then
        asan_flag="-s=none"
    fi
    echo "ASAN flag: $asan_flag" | tee -a $log_file

    cargo fuzz run $asan_flag $fuzz_target -- \
        -max_total_time=$total_seconds \
        -max_len=819200 \
        -keep_seed=1 \
        -fork=$JOBS \
        -timeout=20 \
        -ignore_timeouts=1 \
        -ignore_crashes=1 \
        -print_final_stats=1 2>&1 | tee -a $log_file
}

function afl_in_tmux() {
    local window_id=$1
    local name=$2
    local command=$3

    if [[ window_id -eq 1 ]]; then
        tmux rename-window -t $TMUX_SESSION:$window_id $name
    else
        node_type="-S"
        tmux new-window -t $TMUX_SESSION -n $name
    fi
    if [ $? -ne 0 ]; then
        echo "Failed to create a new window in tmux session: $TMUX_SESSION"
        exit 1
    fi
    tmux send-keys -t $TMUX_SESSION:$window_id "$command" C-m
}

function run_afl() {
    local fuzz_target=$1
    local total_hour=$2
    local input_len=$3

    local total_seconds=$(echo "$total_hour * 3600" | bc)
    local log_file=$(create_log "$MOVE_SMITH_DIR/logs")
    echo "Writing logs to $log_file"

    echo "Running AFL: $fuzz_target for $total_hour hours with input size: $input_len KB" | tee -a $log_file

    echo "Building AFL fuzz target: $fuzz_target" | tee -a $log_file
    (
        cd $MOVE_SMITH_DIR/fuzz
        cargo afl build --bin $fuzz_target
    )
    TARGET_BIN=$(realpath $(find $APTOS_DIR/target/ -name $fuzz_target))

    if [ -z "$TARGET_BIN" ]; then
        echo "Failed to build AFL fuzz target: $fuzz_target" | tee -a $log_file
        exit 1
    fi

    echo "Built AFL fuzz target: $TARGET_BIN" | tee -a $log_file

    local input_dir="$MOVE_SMITH_DIR/fuzz/afl/${fuzz_target}_in"
    local output_dir="$MOVE_SMITH_DIR/fuzz/afl/${fuzz_target}_out"
    mkdir -p $input_dir
    mkdir -p $output_dir
    echo "Created input and output directories for AFL: $input_dir, $output_dir" | tee -a $log_file

    echo "Creating initial corpus for $fuzz_target, max size: $input_len KB" | tee -a $log_file
    create_initial_corpus $input_dir $input_len

    tmux new-session -d -s $TMUX_SESSION
    if [ $? -ne 0 ]; then
        echo "Failed to start a new tmux session: $TMUX_SESSION"
        exit 1
    fi
    echo "Started a new tmux session: $TMUX_SESSION" | tee -a $log_file

    afl_flags=""
    if [[ "$OSTYPE" == "linux-gnu" ]]; then
        afl_flags="AFL_I_DONT_CARE_ABOUT_MISSING_CRASHES=1 AFL_SKIP_CPUFREQ=1"
    fi

    afl_prefix="$afl_flags AFL_AUTORESUME=1 cargo afl fuzz -i $input_dir -o $output_dir -V $total_seconds"
    afl_suffix="-- $TARGET_BIN"

    echo "Running AFL Main node, for $total_hour hours"
    afl_in_tmux 1 "Main" "$afl_prefix -M fuzzer0 $afl_suffix"

    for ((i=2; i<=JOBS; i++)); do
        echo "Running AFL Secondary node $i, for $total_hour hours" | tee -a $log_file
        afl_in_tmux $i "S$i" "AFL_FINAL_SYNC=1 $afl_prefix -S fuzzer$i $afl_suffix"
    done

    log_time=$(echo "$total_seconds + 10" | bc)
    log_id=$(($JOBS+1))
    afl_in_tmux $log_id "Stats" "for ((i=0; i<=$log_time; i+=10)); do cargo afl whatsup $output_dir 2>&1 | tee -a $log_file; sleep 10; done"

    tmux select-window -t $TMUX_SESSION:1
    tmux attach-session -t $TMUX_SESSION
}

if [ "$#" -gt 3 ]; then
    echo "Usage: ./scripts/fuzz.sh <fuzz_target> [total_hour] [max_input_len]"
    exit 1
fi

fuzz_target=${1:-"transactional"}
total_hour=${2:-24} # Default to 24 hours
input_len=${3:-4}   # Default to 4 KB

# Check if the fuzz target is libfuzzer or afl
if [[ $fuzz_target == "afl"* ]]; then
    run_afl $fuzz_target $total_hour $input_len
else
    run_libfuzzer $fuzz_target $total_hour $input_len
fi

#!/bin/bash

export RUSTFLAGS="${RUSTFLAGS} --cfg tokio_unstable"
export EXTRAFLAGS="-Ztarget-applies-to-host -Zhost-config"
# Nightly version control
# Pin nightly-2024-02-12 because of https://github.com/google/oss-fuzz/issues/11626
NIGHTLY_VERSION="nightly-2024-04-06"

# GDRIVE format https://docs.google.com/uc?export=download&id=DOCID
CORPUS_ZIPS=("https://storage.googleapis.com/aptos-core-corpora/move_aptosvm_publish_and_run_seed_corpus.zip" "https://storage.googleapis.com/aptos-core-corpora/move_aptosvm_publish_seed_corpus.zip")

# This save time excluding some features needed only for specific targets
# Downside: fuzzers which require  specific features need to recompile all dependencies
function get_features_for_target() {
    local target=$1
    local toml_file="fuzz/Cargo.toml"

    # Find the section containing our target and extract required-features if present
    features=$(sed -n "/name = \"$target\"/,/\[\[bin]]/p" "$toml_file" | \
              grep "required-features" | \
              sed -E 's/.*required-features *= *\[(.*)\].*/\1/' | \
              tr -d '", ')

    if [ -n "$features" ]; then
        echo "--features $features"
    else
        echo ""
    fi
}

function info() {
    echo "[info] $1"
}

function error() {
    echo "[error] $1"
    exit 1
}

function cargo_fuzz() {
    rustup install $NIGHTLY_VERSION
    if [ -z "$1" ]; then
        error "error using cargo()"
    fi
    cargo_fuzz_cmd="cargo "+$NIGHTLY_VERSION" fuzz $1"
    shift
    $cargo_fuzz_cmd $EXTRAFLAGS $@
}

function cargo_local() {
    rustup install $NIGHTLY_VERSION
    if [ -z "$1" ]; then
        error "error using cargo()"
    fi
    cargo_cmd="cargo "+$NIGHTLY_VERSION" $1"
    shift
    $cargo_cmd $EXTRAFLAGS $@
}

function usage() {
    case "$1" in
        "add")
            echo "Usage: $0 add <fuzz_target>"
            ;;
        "block-builder")
            #echo "Usage: $0 block-builder <command> [argumetns]"
            cargo_local run --quiet -- --help
            ;;
        "block-builder-recursive")
            echo "Usage: $0 block-builder-recursive <search_directory> <destination_directory>"
            ;;
        "build")
            echo "Usage: $0 build <fuzz_target|all> [target_dir]"
            ;;
        "build-oss-fuzz")
            echo "Usage: $0 build-oss-fuzz <target_dir>"
            ;;
        "clean-coverage")
            echo "Usage: $0 clean-coverage <fuzz_target>"
            ;;
        "cmin")
            echo "Usage: $0 cmin <fuzz_target> [corpus_dir]"
            ;;
        "coverage")
            echo "Usage: $0 coverage <fuzz_target>"
            ;;
        "debug")
            echo "Usage: $0 debug <fuzz_target> <testcase>"
            ;;
        "flamegraph")
            echo "Usage: $0 flamegraph <fuzz_target> <testcase>"
            ;;
        "list")
            echo "Usage: $0 list"
            ;;
        "monitor-coverage")
            echo "Usage: $0 monitor-coverage <fuzz_target>"
            ;;
        "run")
            echo "Usage: $0 run <fuzz_target> [testcase]"
            ;;
        "test")
            echo "Usage: $0 test"
            ;;
        "tmin")
            echo "Usage: $0 tmin <fuzz_target> <crashing_input>"
            ;;
        *)
            echo "Usage: $0 <add|block-builder|block-builder-recursive|build|build-oss-fuzz|clean-coverage|cmin|coverage|debug|flamegraph|list|monitor-coverage|run|test|tmin>"
            echo "    add                   adds a new fuzz target"
            echo "    block-builder         runs rust tool to help build fuzzers"
            echo "    block-builder-recursive runs block-builder on all Move.toml files in directory"
            echo "    build                 builds fuzz targets"
            echo "    build-oss-fuzz        builds fuzz targets for oss-fuzz"
            echo "    clean-coverage        clean coverage for a fuzz target"
            echo "    cmin                  minimizes a corpus for a target"
            echo "    coverage              generates coverage for a fuzz target"
            echo "    debug                 debugs a fuzz target with a testcase"
            echo "    flamegraph           generates a flamegraph for a fuzz target with a testcase"
            echo "    list                 lists existing fuzz targets"
            echo "    monitor-coverage     monitors coverage for a fuzz target"
            echo "    run                  runs a fuzz target"
            echo "    test                 tests all fuzz targets"
            echo "    tmin                 minimizes a crashing input for a target"
            ;;
    esac
    exit 1
}

function block-builder() {
    if [ -z "$1" ]; then
        usage block-builder
    fi
    command=$1
    shift
    cargo_local run --release --quiet -- $command $@
    exit 0
}

function block-builder-recursive() {
    # Check if both arguments are provided
    if [ "$#" -ne 2 ]; then
        usage block-builder-recursive
        exit 1 # Ensure exit after usage message
    fi
    search_directory=$1
    destination_dir=$2 # Get destination directory from the second argument

    # Ensure the destination directory exists (still good practice)
    mkdir -p "$destination_dir"
    if [ $? -ne 0 ]; then
      error "Failed to create destination directory '$destination_dir'."
    fi

    # Directly invoke the Rust command for recursive generation.
    # Pass the search directory as the first argument (base_dir)
    # and the destination directory as the second argument (destination_path).
    info "Invoking Rust command for recursive generation: base_dir='$search_directory' destination='$destination_dir'"
    if ! cargo_local run --release --quiet -- generate_runnable_states_recursive "$search_directory" "$destination_dir"; then
         error "Rust command 'generate_runnable_states_recursive' failed processing '$search_directory'"
         # Exit the script if the command fails
         exit 1
    fi

    info "Recursive generation command completed."
    exit 0
}

function build() {
    if [ -z "$1" ]; then
        usage build
    fi
    fuzz_target=$1
    if [ "$fuzz_target" = "all" ]; then
        fuzz_target=""
    fi
    target_dir=${2:-./target}
    info "Target directory: $target_dir"
    mkdir -p $target_dir
    info "Building $fuzz_target"
    features=$(get_features_for_target $fuzz_target)
    cargo_fuzz build --sanitizer none --verbose -O --target-dir $target_dir $features $fuzz_target
}

function build-oss-fuzz() {
    if [ -z "$1" ]; then
        usage build-oss-fuzz
    fi
    oss_fuzz_out=$1
    mkdir -p "$oss_fuzz_out"
    mkdir -p ./target

    # Apply all git patch from Patches directory
    wd=$(pwd)
    for patch in $(find "$wd/Patches" -type f); do
        info "Applying patch $patch"
        git -C "$wd/../.." apply "$patch"
    done


    # Workaround for build failures on oss-fuzz
    # Owner: @zi0Black
    # Issue: Some dependencies requires to compile C/C++ code and it result in build failure on oss-fuzz using provided flags.
    # Solution: We have fixed some lib, but not all of them. So we just disable all C/C++ code compilation using libFuzzer.
    # Note: We will revert this when we manage to understand how to work with each dependency.
    export CFLAGS="-O1 -fno-omit-frame-pointer -gline-tables-only -DFUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION"
    export CXXFLAGS_EXTRA="-stdlib=libc++"
    export CXXFLAGS="$CFLAGS $CXXFLAGS_EXTRA"

    # component versions good to have in logs
    ld.lld --version
    clang --version

    # Limit the number of parallel jobs to avoid OOM
    export CARGO_BUILD_JOBS=4

    # Build the fuzz targets
    # Doing one target at the time should prevent OOM, but use all thread while bulding dependecies
    for fuzz_target in $(list); do
        if ! build $fuzz_target ./target ; then
            env
            error "Build failed. Exiting."
        fi
    done

    find ./target/*/release/ -maxdepth 1 -type f -perm /111 -exec cp {} $oss_fuzz_out \;

    # Download corpus zip
    for corpus_zip in "${CORPUS_ZIPS[@]}"; do
        wget --content-disposition -P "$oss_fuzz_out" "$corpus_zip"
    done
}

function cmin() {
    if [ -z "$1" ]; then
        usage cmin
    fi
    fuzz_target=$1
    corpus_dir=${2:-./fuzz/corpus/$fuzz_target}
    cargo_fuzz cmin $fuzz_target $corpus_dir
}

function install-coverage-tools() {
     cargo +$NIGHTLY_VERSION install cargo-binutils
     cargo +$NIGHTLY_VERSION install rustfilt
}

function coverage() {
    if [ -z "$1" ]; then
        usage coverage
    fi
    fuzz_target=$1

    if ! cargo +$NIGHTLY_VERSION cov -V &> /dev/null; then
        install-coverage-tools
    fi

    clean-coverage $fuzz_target
    local corpus_dir="fuzz/corpus/$fuzz_target"
    local coverage_dir="./fuzz/coverage/$fuzz_target/report"
    mkdir -p $coverage_dir
    
    if [ ! -d "fuzz/coverage/$fuzz_target/raw" ]; then
        cargo_fuzz coverage $fuzz_target $corpus_dir
    fi
    
    info "Generating coverage for $fuzz_target"

    PERM="/111"
    if [[ $OSTYPE == 'darwin'* ]]; then
        PERM="+111"
    fi

    fuzz_target_bin=$(find ./target/*/coverage -name $fuzz_target -type f -perm $PERM)
    echo "Found fuzz target binary: $fuzz_target_bin"
    # Generate the coverage report
    cargo +$NIGHTLY_VERSION cov -- show $fuzz_target_bin \
        --format=html \
        --instr-profile=fuzz/coverage/$fuzz_target/coverage.profdata \
        --show-directory-coverage \
        --output-dir=$coverage_dir \
        -Xdemangler=rustfilt \
        --show-branches=count \
        --ignore-filename-regex='rustc/.*/library|\.cargo'
}

function clean-coverage() {
    if [ "$#" -ne 1 ]; then
        usage clean
    fi

    local fuzz_target="$1"
    if [ "$fuzz_target" == "all" ]; then
        rm -rf ./fuzz/coverage
    else
        local coverage_dir="./fuzz/coverage/$fuzz_target/"
        rm -rf $coverage_dir
    fi
}

# use rust-gdb to debug a fuzz target with a testcase
function debug() {
    if [ -z "$2" ]; then
        usage debug
    fi
    fuzz_target=$1
    testcase=$2
    if [ ! -f "$testcase" ]; then
        error "$testcase does not exist"
    fi
    info "Debugging $fuzz_target with $testcase"
    # find the binary
    binary=$(find ./target -name $fuzz_target -type f -perm /111)
    if [ -z "$binary" ]; then
        error "Could not find binary for $fuzz_target. Run `./fuzz.sh build $fuzz_target` first"
    fi
    # run the binary with rust-gdb
    export LSAN_OPTIONS=verbosity=1:log_threads=1
    export RUST_BACKTRACE=1 
    rust-gdb --args $binary $testcase -- -runs=1
}

# use cargo-flamegraph to generate a flamegraph for a fuzz target with a testcase
function flamegraph() {
    if [ -z "$2" ]; then
        usage flamegraph
    fi
    fuzz_target=$1
    testcase=$2
    if [ ! -f "$testcase" ]; then
        error "$testcase does not exist"
    fi
    info "Generating flamegraph for $fuzz_target with $testcase"
    # run the binary with cargo-flamegraph
    time=$(date +%s)
    cargo flamegraph -o "${fuzz_target}_${time}.svg" --root -p="fuzzer-fuzz" --bin="$fuzz_target" -- "$testcase" "-- -runs=1"
}

function run() {
    if [ -z "$1" ]; then
        usage run
    fi
    fuzz_target=$1
    testcase=$2
    if [ ! -z "$testcase" ]; then
        if [ -f "$testcase" ]; then
            testcase="$testcase -- -runs=1"
        else
            error "$testcase does not exist"
        fi
    fi
    info "Running $fuzz_target"
    features=$(get_features_for_target $fuzz_target)
    if [[ "$OSTYPE" == "darwin"* ]]; then
        cargo_fuzz run $features --sanitizer address -O $fuzz_target $testcase -- -fork=4 #-ignore_crashes=1
    else
        cargo_fuzz run $features --sanitizer address -O $fuzz_target $testcase -- -fork=15 #-ignore_crashes=1
    fi
}

function test() {
    for fuzz_target in $(list); do
        info "Testing $fuzz_target"
        features=$(get_features_for_target $fuzz_target)
        cargo_fuzz run $features $fuzz_target -- -max_len=1024 -jobs=4 -workers=4 -runs=1000
        if [ $? -ne 0 ]; then
            error "Failed to run $fuzz_target"
        fi
        info "$fuzz_target ok!"
    done
}

function add() {
    if [ -z "$1" ]; then
        usage add
    fi

    fuzz_target=$1
    fuzz_target_path="$fuzz_target.rs"

    mkdir -p fuzz/fuzz_targets/$(dirname $fuzz_target_path) && touch fuzz/fuzz_targets/$fuzz_target_path

    if [ $? -eq 0 ]; then
        {
            echo ""
            echo "[[bin]]"
            echo "name = \"$fuzz_target\""
            echo "path = \"$fuzz_target_path\""
            echo "test = false"
            echo "doc = false"
        } >> fuzz/Cargo.toml
        info "Fuzzing target '$fuzz_target' added successfully at $fuzz_target_path."
    else
        error "Failed to create directory or file for fuzzing target."
    fi
}

function list() {
    cargo fuzz list
}

# Helper function to get the latest modification time in a directory
function get_latest_mtime() {
  local dir=$1
  # Find all files, get their mtime (Unix timestamp), sort numerically, take the last one (latest)
  # Handle empty dir case with default 0. Use awk to ensure numeric output.
  find "$dir" -type f -exec stat -f %m {} \; | sort -n | tail -n 1 | awk '{print $1+0}' || echo 0
}

function monitor-coverage() {
    if [ -z "$1" ]; then
        usage monitor-coverage
    fi
    local fuzz_target=$1
    local corpus_dir="fuzz/corpus/$fuzz_target"

    if [ ! -d "$corpus_dir" ]; then
        error "Corpus directory $corpus_dir does not exist for target $fuzz_target"
    fi

    info "Starting coverage monitoring for target '$fuzz_target' on directory '$corpus_dir'..."
    
    # Initial coverage run
    info "Running initial coverage generation..."
    coverage "$fuzz_target"
    local last_mtime=$(get_latest_mtime "$corpus_dir")
    info "Initial coverage done. Last modification time: $last_mtime"

    while true; do
        local current_mtime=$(get_latest_mtime "$corpus_dir")
        
        # Check if current_mtime is greater than last_mtime
        # Using awk for reliable floating point/integer comparison in bash
        if awk -v current="$current_mtime" -v last="$last_mtime" 'BEGIN { exit !(current > last) }'; then
            info "Detected changes in corpus directory (new mtime: $current_mtime > last mtime: $last_mtime). Regenerating coverage..."
            coverage "$fuzz_target"
            # Update last_mtime only after successful coverage run
            last_mtime=$current_mtime
            info "Coverage updated. New last modification time: $last_mtime"
        fi
        
        sleep 10 # Check every 10 seconds
    done
}

function check_cargo_fuzz() {
    if ! command -v cargo-fuzz &> /dev/null; then
        info "cargo-fuzz is not installed. Installing..."
        cargo install cargo-fuzz
    fi
}

check_cargo_fuzz

# use cargo fuzz tmin to minimize a crashing testcase
function tmin() {
    if [ "$#" -ne 2 ]; then
        usage tmin
    fi
    fuzz_target=$1
    input_case=$2
    if [ ! -f "$input_case" ]; then
        error "$input_case does not exist"
    fi
    info "Minimizing $input_case for target $fuzz_target"
    features=$(get_features_for_target $fuzz_target)
    cargo_fuzz tmin $features --sanitizer address -O $fuzz_target "$input_case"
}

case "$1" in
  "add")
    shift
    add "$@"
    ;;
  "block-builder")
    shift
    block-builder "$@"
    ;;
  "block-builder-recursive")
    shift
    block-builder-recursive "$@"
    ;;
  "build")
    shift
    build "$@"
    ;;
  "build-oss-fuzz")
    shift
    build-oss-fuzz "$@"
    ;;
  "cmin")
    shift
    cmin "$@"
    ;;
  "coverage")
    shift
    coverage "$@"
    ;;
  "clean-coverage")
    shift
    clean-coverage "$@"
    ;;
  "debug")
    shift
    debug "$@"
    ;;
  "flamegraph")
    shift
    flamegraph "$@"
    ;;
   "list")
    shift
    list
    ;;
  "run")
    shift
    run  "$@"
    ;;
  "test")
    shift
    test
    ;;
  "monitor-coverage")
    shift
    monitor-coverage "$@"
    ;;
  "tmin")
    shift
    tmin "$@"
    ;;
  *)
    usage general
    ;;
esac

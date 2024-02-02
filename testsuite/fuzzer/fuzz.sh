#!/bin/bash

export RUSTFLAGS="${RUSTFLAGS} --cfg tokio_unstable"
export EXTRAFLAGS="-Ztarget-applies-to-host -Zhost-config"

# GDRIVE format https://docs.google.com/uc?export=download&id=DOCID
CORPUS_ZIPS=()

function info() {
    echo "[info] $1"
}

function error() {
    echo "[error] $1"
    exit 1
}

function cargo_fuzz() {
    rustup install nightly
    if [ -z "$1" ]; then
        error "error using cargo()"
    fi
    cargo_fuzz_cmd="cargo +nightly fuzz $1"
    shift
    $cargo_fuzz_cmd $EXTRAFLAGS $@
}

function usage() {
    case "$1" in
        "add")
            echo "Usage: $0 add <fuzz_target>"
            ;;
        "build")
            echo "Usage: $0 build <fuzz_target|all> [target_dir]"
            ;;
        "build-oss-fuzz")
            echo "Usage: $0 build-oss-fuzz <target_dir>"
            ;;
        "list")
            echo "Usage: $0 list"
            ;;
        "run")
            echo "Usage: $0 run <fuzz_target> [testcase]"
            ;;
        "debug")
            echo "Usage: $0 debug <fuzz_target> <testcase>"
            ;;
        "test")
            echo "Usage: $0 test"
            ;;
        *)
            echo "Usage: $0 <build|build-oss-fuzz|list|run|debug|test>"
            echo "    add               adds a new fuzz target"
            echo "    build             builds fuzz targets"
            echo "    build-oss-fuzz    builds fuzz targets for oss-fuzz"
            echo "    list              lists existing fuzz targets"
            echo "    run               runs a fuzz target"
            echo "    debug             debugs a fuzz target with a testcase"
            echo "    test              tests all fuzz targets"
            ;;
    esac
    exit 1
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
    cargo_fuzz build --verbose -O --target-dir $target_dir $fuzz_target
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

    if ! build all ./target; then
        env
        error "Build failed. Exiting."
    fi
    find ./target/*/release/ -maxdepth 1 -type f -perm /111 -exec cp {} $oss_fuzz_out \;

    # Download corpus zip
    for corpus_zip in "${CORPUS_ZIPS[@]}"; do
        wget --content-disposition -P "$oss_fuzz_out" "$corpus_zip"
    done
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
    cargo_fuzz run $fuzz_target $testcase
}

# use rust-gdb to debug a fuzz target with a testcase
function debug() {
    if [ -z "$1" ]; then
        usage debug
    fi
    fuzz_target=$1
    testcase=$2
    if [ -z "$testcase" ]; then
        error "No testcase provided"
    fi
    if [ ! -f "$testcase" ]; then
        error "$testcase does not exist"
    fi
    info "Debugging $fuzz_target with $testcase"
    # find the binary
    binary=$(find ./target -name $fuzz_target -type f -executable)
    if [ -z "$binary" ]; then
        error "Could not find binary for $fuzz_target"
    fi
    # run the binary with rust-gdb
    export LSAN_OPTIONS=verbosity=1:log_threads=1
    export RUST_BACKTRACE=1 
    rust-gdb --args $binary $testcase -- -runs=1
}

function test() {
    for fuzz_target in $(list); do
        info "Testing $fuzz_target"
        cargo_fuzz run $fuzz_target -- -max_len=1024 -jobs=4 -workers=4 -runs=1000
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
        } >> $fuzz_path/Cargo.toml
        info "Fuzzing target '$fuzz_target' added successfully at $fuzz_target_path."
    else
        error "Failed to create directory or file for fuzzing target."
    fi

    mkdir -p fuzz/fuzz_targets/$(dirname $fuzz_target_path) && touch fuzz/fuzz_targets/$fuzz_target_path

    if [ $? -eq 0 ]; then
        {
            echo ""
            echo "[[bin]]"
            echo "name = \"$fuzz_target\""
            echo "path = \"$fuzz_target_path\""
            echo "test = false"
            echo "doc = false"
        } >> $fuzz_path/Cargo.toml
        info "Fuzzing target '$fuzz_target' added successfully at $fuzz_target_path."
    else
        error "Failed to create directory or file for fuzzing target."
    fi
}

function list() {
    cargo fuzz list
}

function check_cargo_fuzz() {
    if ! command -v cargo-fuzz &> /dev/null; then
        info "cargo-fuzz is not installed. Installing..."
        cargo install cargo-fuzz
    fi
}

check_cargo_fuzz

case "$1" in
  "add")
    shift
    add "$@"
    ;;
  "build")
    shift
    build "$@"
    ;;
  "build-oss-fuzz")
    shift
    build-oss-fuzz "$@"
    ;;
   "list")
    shift
    list
    ;;
  "run")
    shift
    run  "$@"
    ;;
  "debug")
    shift
    debug "$@"
    ;;
  "test")
    shift
    test
    ;;
  *)
    usage general
    ;;
esac

#!/bin/bash

export RUSTFLAGS="${RUSTFLAGS} --cfg tokio_unstable"
export EXTRAFLAGS="-Ztarget-applies-to-host -Zhost-config"

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
        "test")
            echo "Usage: $0 test"
            ;;
        *)
            echo "Usage: $0 <build|build-oss-fuzz|list|run|test>"
            echo "    build             builds fuzz targets"
            echo "    build-oss-fuzz    builds fuzz targets for oss-fuzz"
            echo "    list              lists existing fuzz targets"
            echo "    run               runs a fuzz target"
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
    cargo_fuzz build -O --target-dir $target_dir $fuzz_target
}

function build-oss-fuzz() {
    if [ -z "$1" ]; then
        usage build-oss-fuzz
    fi
    oss_fuzz_out=$1
    mkdir -p $oss_fuzz_out
    mkdir ./target
    build all ./target
    find ./target/*/release/ -maxdepth 1 -type f -executable -exec cp {} $oss_fuzz_out \\;
}

function run() {
    if [ -z "$1" ]; then
        usage run
    fi
    fuzz_target=$1
    testcase=$2
    if [ ! -z "$testcase" ]; then
        if [ -f "$testcase" ]; then
            testcase="-runs=1 $testcase"
        else
            error "$testcase does not exist"
        fi
    fi
    info "Running $fuzz_target"
    cargo_fuzz run $fuzz_target $testcase
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

function list() {
    cargo fuzz list
}

case "$1" in
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
  "test")
    shift
    test
    ;;
  *)
    usage general
    ;;
esac

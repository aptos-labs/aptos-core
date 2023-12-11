#!/bin/bash

# Function to display usage
usage() {
  echo "Usage: $0 (fuzz|build) <fuzzer_name> [testcase]"
  exit 1
}

# Function to display errors
error() {
  echo "ERROR: $1"
  exit 1
}

# Function to display info
info() {
  echo "INFO: $1"
}

# Check minimum number of arguments
[ "$#" -lt 2 ] && usage

MODE="$1"
FUZZER_NAME="$2"
TESTCASE="$3"

# Common command prefix
CMD_PREFIX="cargo +nightly fuzz -Ztarget-applies-to-host -Zhost-config move_value_deserialize"

# Set environment variable
export RUSTFLAGS="--cfg tokio_unstable"

# Validate options and execute corresponding actions
case "$MODE" in
  "build")
    [ -n "$TESTCASE" ] && info "Testcase ignored for build mode"
    eval "$CMD_PREFIX build -O --target-dir ./target $FUZZER_NAME"
    if [ $? -eq 0 ]; then
      info "Fuzzer binary ===> $(echo ./target/*/release/$FUZZER_NAME)"
    else
      error "Build failed"
    fi
    ;;
  "fuzz")
    CMD="$CMD_PREFIX run $FUZZER_NAME"
    [ -n "$TESTCASE" ] && CMD="$CMD $TESTCASE"
    eval "$CMD"
    ;;
  *)
    usage
    ;;
esac

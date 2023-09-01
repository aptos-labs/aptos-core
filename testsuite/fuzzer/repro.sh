#!/bin/bash

if [ "$#" -ne 2 ]; then
  echo "Usage: $0 <fuzzer_name> <testcase>"
  exit 1
fi

FUZZER_NAME="$1"
TESTCASE="$2"

if [ ! -f "$TESTCASE" ]; then
  echo "Testcase not found: $TESTCASE"
  exit 1
fi

RUSTFLAGS="--cfg tokio_unstable" cargo +nightly fuzz run $FUZZER_NAME $TESTCASE

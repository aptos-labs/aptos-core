#!/bin/bash

export RUSTFLAGS="$RUSTFLAGS --cfg tokio_unstable"

for fuzzer in $(cargo +nightly fuzz list); do
    cargo +nightly fuzz run -O -a $fuzzer -- -runs=100
    if [ "$?" -ne "0" ]; then
        echo "[error] failed to run $fuzzer"
        break
    fi
done
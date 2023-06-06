#!/bin/bash

export RUSTFLAGS="${RUSTFLAGS} --cfg tokio_unstable"
export RUNS="1000"

for fuzzer in $(cargo +nightly fuzz list); do
    echo "[info] compiling and running ${fuzzer} ${RUNS} times"
    cargo +nightly fuzz run -O -a $fuzzer -- -runs=$RUNS
    if [ "$?" -ne "0" ]; then
        echo "[error] failed to run ${fuzzer}"
        return -1
    else
        echo "[ok] ${fuzzer}"
    fi
done
#!/bin/bash
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
set -ex

PROFILE=${PROFILE:-release}
FEATURES=${FEATURES:-""}

echo "Building aptos-node and aptos-debugger"
echo "PROFILE: $PROFILE"
echo "FEATURES: $FEATURES"
echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

PERF_FLAGS=()
if [[ "$PROFILE" == "performance" ]]; then
  PERF_FLAGS=(--config .cargo/performance.toml)
fi

if [ -n "$FEATURES" ]; then
    echo "Building aptos-node with features ${FEATURES}"
    cargo build --profile=$PROFILE --features=$FEATURES "${PERF_FLAGS[@]}" -p aptos-node "$@"
    cargo build --locked --profile=$PROFILE "${PERF_FLAGS[@]}" -p aptos-debugger "$@"
else
    cargo build --locked --profile=$PROFILE "${PERF_FLAGS[@]}" \
        -p aptos-node \
        -p aptos-debugger \
        "$@"
fi

mkdir dist
cp $CARGO_TARGET_DIR/$PROFILE/aptos-node dist/aptos-node
cp $CARGO_TARGET_DIR/$PROFILE/aptos-debugger dist/aptos-debugger

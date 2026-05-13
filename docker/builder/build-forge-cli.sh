#!/bin/bash
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
set -ex

PROFILE=${PROFILE:-release}

echo "Building forge"
echo "PROFILE: $PROFILE"
echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

PERF_FLAGS=()
if [[ "$PROFILE" == "performance" ]]; then
  PERF_FLAGS=(--config .cargo/performance.toml)
fi

cargo build --locked --profile=$PROFILE "${PERF_FLAGS[@]}" \
    -p aptos-forge-cli \
    "$@"

mkdir dist
cp $CARGO_TARGET_DIR/$PROFILE/forge dist/forge

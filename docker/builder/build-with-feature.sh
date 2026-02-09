#!/bin/bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
set -ex

PROFILE=${PROFILE:-release}
FEATURES=${FEATURES:-""}

echo "Building aptos-node and aptos-debugger"
echo "PROFILE: $PROFILE"
echo "FEATURES: $FEATURES"
echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

BUILD_ENV=()
if [[ "$PROFILE" == "performance" ]]; then
  source "$(dirname -- "${BASH_SOURCE[0]}")/performance_rustflags.sh"
  BUILD_ENV=(RUSTFLAGS="${PERFORMANCE_RUSTFLAGS[*]}")
fi

if [ -n "$FEATURES" ]; then
    echo "Building aptos-node with features ${FEATURES}"
    env "${BUILD_ENV[@]}" cargo build --profile=$PROFILE --features=$FEATURES -p aptos-node "$@"
    env "${BUILD_ENV[@]}" cargo build --locked --profile=$PROFILE -p aptos-debugger "$@"
else
    env "${BUILD_ENV[@]}" cargo build --locked --profile=$PROFILE \
        -p aptos-node \
        -p aptos-debugger \
        "$@"
fi

mkdir dist
cp $CARGO_TARGET_DIR/$PROFILE/aptos-node dist/aptos-node
cp $CARGO_TARGET_DIR/$PROFILE/aptos-debugger dist/aptos-debugger

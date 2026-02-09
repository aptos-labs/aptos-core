#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
set -ex

PROFILE=${PROFILE:-release}

echo "Building forge"
echo "PROFILE: $PROFILE"
echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

BUILD_ENV=()
if [[ "$PROFILE" == "performance" ]]; then
  source "$(dirname -- "${BASH_SOURCE[0]}")/performance_rustflags.sh"
  BUILD_ENV=(RUSTFLAGS="${PERFORMANCE_RUSTFLAGS[*]}")
fi

env "${BUILD_ENV[@]}" cargo build --locked --profile=$PROFILE \
    -p aptos-forge-cli \
    "$@"

mkdir dist
cp $CARGO_TARGET_DIR/$PROFILE/forge dist/forge

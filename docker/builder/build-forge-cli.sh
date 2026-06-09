#!/bin/bash
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
set -ex

PROFILE=${PROFILE:-release}

BUILD_PROFILE=$PROFILE
if [[ "$PROFILE" == "performance" ]]; then
  # No need to build with `--profile performance` since the most demanding
  # thing forge does is generate load.
  BUILD_PROFILE=release
fi

echo "Building forge"
echo "PROFILE: $PROFILE"
echo "BUILD_PROFILE: $BUILD_PROFILE"
echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

cargo build --locked --profile=$BUILD_PROFILE \
    -p aptos-forge-cli \
    "$@"

mkdir dist
cp $CARGO_TARGET_DIR/$BUILD_PROFILE/forge dist/forge

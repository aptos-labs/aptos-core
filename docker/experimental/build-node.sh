#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
set -e

PROFILE=${PROFILE:-release}
FEATURES=${FEATURES:-""}

echo "Building aptos-node"
echo "PROFILE: $PROFILE"
echo "FEATURES: $FEATURES"
echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

# Build and overwrite the aptos-node binary with features if specified
if [ -n "$FEATURES" ]; then
    echo "Building aptos-node with features ${FEATURES}"
    cargo build --profile=$PROFILE --features=$FEATURES -p aptos-node "$@"
else 
    # Build aptos-node separately
    cargo build --locked --profile=$PROFILE \
        -p aptos-node \
        "$@"
fi

# After building, copy the binaries we need to `dist` since the `target` directory is used as docker cache mount and only available during the RUN step
BINS=(
    aptos-node
)

mkdir dist

for BIN in "${BINS[@]}"; do
    cp $CARGO_TARGET_DIR/$PROFILE/$BIN dist/$BIN
done

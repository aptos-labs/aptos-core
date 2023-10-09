#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
set -e

PROFILE=${PROFILE:-release}
FEATURES=${FEATURES:-""}

# set rust target based on TARGETPLATFORM
case $TARGETPLATFORM in
    "linux/amd64")
        export RUST_TARGET="x86_64-unknown-linux-gnu"
        export X86_64_UNKNOWN_LINUX_GNU_OPENSSL_LIB_DIR="/usr/lib/x86_64-linux-gnu"
        export X86_64_UNKNOWN_LINUX_GNU_OPENSSL_INCLUDE_DIR="/usr/include/x86_64-linux-gnu/openssl"
        ;;
    "linux/arm64")
        export RUST_TARGET="aarch64-unknown-linux-gnu"
        ;;
    *)
        echo "Unsupported TARGETPLATFORM: $TARGETPLATFORM"
        exit 1
        ;;
esac

echo "Building aptos-node"
echo "PROFILE: $PROFILE"
echo "FEATURES: $FEATURES"
echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

# Build and overwrite the aptos-node binary with features if specified
if [ -n "$FEATURES" ]; then
    echo "Building aptos-node with features ${FEATURES}"
    cargo build --locked --profile=$PROFILE --features=$FEATURES \
        --target $RUST_TARGET \
        -p aptos-node  \
        "$@"
else 
    # Build aptos-node separately
    cargo build --locked --profile=$PROFILE \
        --target $RUST_TARGET \
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

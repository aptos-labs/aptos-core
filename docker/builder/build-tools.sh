#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
set -e

echo "Building tools and services docker images"
echo "PROFILE: $PROFILE"
echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

# After building, copy the binaries we need to `dist` since the `target` directory is used as docker cache mount and only available during the RUN step
mkdir dist

cargo build --locked --profile=$PROFILE -p aptos-debugger "$@"
cp $CARGO_TARGET_DIR/$PROFILE/aptos-debugger dist/aptos-debugger

# Build all the rust binaries. We usually just need to build with `cli` profile,
# except for `aptos-debugger` above which could benefit from better performance
# for replay-verify.
PROFILE=cli
cargo build --locked --profile=$PROFILE \
    -p aptos \
    -p aptos-backup-cli \
    -p aptos-faucet-service \
    -p aptos-openapi-spec-generator \
    -p aptos-telemetry-service \
    -p aptos-keyless-pepper-service \
    -p aptos-transaction-emitter \
    -p aptos-release-builder \
    "$@"

BINS=(
    aptos
    aptos-faucet-service
    aptos-openapi-spec-generator
    aptos-telemetry-service
    aptos-keyless-pepper-service
    aptos-transaction-emitter
    aptos-release-builder
)
for BIN in "${BINS[@]}"; do
    cp $CARGO_TARGET_DIR/$PROFILE/$BIN dist/$BIN
done

# Build the Aptos Move framework and place it in dist. It can be found afterwards in the current directory.
echo "Building the Aptos Move framework..."
(cd dist && cargo run --locked --profile=$PROFILE --package aptos-framework -- release)

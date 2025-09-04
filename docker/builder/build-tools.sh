#!/bin/bash
# Copyright (c) Velor
# SPDX-License-Identifier: Apache-2.0
set -e

PROFILE=cli

echo "Building tools and services docker images"
echo "PROFILE: $PROFILE"
echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

# Build all the rust binaries
cargo build --locked --profile=$PROFILE \
    -p velor \
    -p velor-backup-cli \
    -p velor-faucet-service \
    -p velor-fn-check-client \
    -p velor-node-checker \
    -p velor-openapi-spec-generator \
    -p velor-telemetry-service \
    -p velor-keyless-pepper-service \
    -p velor-debugger \
    -p velor-transaction-emitter \
    -p velor-api-tester \
    "$@"

# After building, copy the binaries we need to `dist` since the `target` directory is used as docker cache mount and only available during the RUN step
BINS=(
    velor
    velor-faucet-service
    velor-node-checker
    velor-openapi-spec-generator
    velor-telemetry-service
    velor-keyless-pepper-service
    velor-fn-check-client
    velor-debugger
    velor-transaction-emitter
    velor-api-tester
)

mkdir dist

for BIN in "${BINS[@]}"; do
    cp $CARGO_TARGET_DIR/$PROFILE/$BIN dist/$BIN
done

# Build the Velor Move framework and place it in dist. It can be found afterwards in the current directory.
echo "Building the Velor Move framework..."
(cd dist && cargo run --locked --profile=$PROFILE --package velor-framework -- release)

#!/bin/bash
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
set -ex

echo "Building tools and services docker images"
echo "PROFILE: $PROFILE"
echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

# Build all the rust binaries
CLI_PROFILE=cli
cargo build --locked --profile=$CLI_PROFILE \
    -p aptos \
    -p aptos-faucet-service \
    -p aptos-openapi-spec-generator \
    -p aptos-telemetry-service \
    -p aptos-keyless-pepper-service \
    -p aptos-transaction-emitter \
    -p aptos-release-builder \
    "$@"

# After building, copy the binaries we need to `dist` since the `target` directory is used as docker cache mount and only available during the RUN step
BINS=(
    aptos
    aptos-faucet-service
    aptos-openapi-spec-generator
    aptos-telemetry-service
    aptos-keyless-pepper-service
    aptos-transaction-emitter
    aptos-release-builder
)

mkdir dist

for BIN in "${BINS[@]}"; do
    cp $CARGO_TARGET_DIR/$CLI_PROFILE/$BIN dist/$BIN
done

# Build the Aptos Move framework and place it in dist. It can be found afterwards in the current directory.
echo "Building the Aptos Move framework..."
(cd dist && cargo run --locked --profile=$CLI_PROFILE --package aptos-framework -- release)

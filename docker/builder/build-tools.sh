#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
set -e

PROFILE=cli

echo "Building tools and services docker images"
echo "PROFILE: $PROFILE"
echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

# Build all the rust binaries
cargo build --locked --profile=$PROFILE \
    -p aptos \
#    -p aptos-backup-cli \
#    -p aptos-faucet-service \
#    -p aptos-fn-check-client \
#    -p aptos-node-checker \
#    -p aptos-openapi-spec-generator \
#    -p aptos-telemetry-service \
#    -p aptos-keyless-pepper-service \
#    -p aptos-debugger \
#    -p aptos-transaction-emitter \
#    -p aptos-api-tester \
    "$@"

# After building, copy the binaries we need to `dist` since the `target` directory is used as docker cache mount and only available during the RUN step
BINS=(
    aptos
#    aptos-faucet-service
#    aptos-node-checker
#    aptos-openapi-spec-generator
#    aptos-telemetry-service
#    aptos-keyless-pepper-service
#    aptos-fn-check-client
#    aptos-debugger
#    aptos-transaction-emitter
#    aptos-api-tester
)

mkdir dist

for BIN in "${BINS[@]}"; do
    cp $CARGO_TARGET_DIR/$PROFILE/$BIN dist/$BIN
done

# Build the Aptos Move framework and place it in dist. It can be found afterwards in the current directory.
echo "MOVE_COMPILER_V2: ${MOVE_COMPILER_V2:-not set}"
echo "Building the Aptos Move framework..."
(cd dist && cargo run --locked --profile=$PROFILE --package aptos-framework -- release)

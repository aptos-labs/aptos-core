#!/bin/bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
set -e

PROFILE=${PROFILE:-release}
FEATURES=${FEATURES:-""}

echo "Building all rust-based docker images"
echo "PROFILE: $PROFILE"
echo "FEATURES: $FEATURES"

# Build all the rust binaries
cargo build --locked --profile=$PROFILE \
    -p aptos \
    -p aptos-backup-cli \
    -p aptos-faucet \
    -p aptos-forge-cli \
    -p aptos-fn-check-client \
    -p aptos-node-checker \
    -p aptos-openapi-spec-generator \
    -p aptos-telemetry-service \
    -p aptos-db-bootstrapper \
    -p aptos-db-tool \
    -p aptos-transaction-emitter \
    -p aptos-indexer-grpc-cache-worker \
    -p aptos-indexer-grpc-file-store \
    -p aptos-indexer-grpc-data-service \
    -p aptos-indexer-grpc-parser \
    "$@"

# Build aptos-node separately
cargo build --locked --profile=$PROFILE \
    -p aptos-node \
    "$@"

# Build and overwrite the aptos-node binary with features if specified
if [ -n "$FEATURES" ]; then
    echo "Building aptos-node with features ${FEATURES}"
    (cd aptos-node && cargo build --profile=$PROFILE --features=$FEATURES "$@")
fi

# After building, copy the binaries we need to `dist` since the `target` directory is used as docker cache mount and only available during the RUN step
BINS=(
    aptos
    aptos-faucet
    aptos-node
    aptos-node-checker
    aptos-openapi-spec-generator
    aptos-telemetry-service
    aptos-indexer-grpc-cache-worker
    aptos-indexer-grpc-file-store
    aptos-indexer-grpc-data-service
    aptos-indexer-grpc-parser
    aptos-fn-check-client
    aptos-db-tool
    aptos-db-bootstrapper
    forge
    aptos-transaction-emitter
)

mkdir dist

for BIN in "${BINS[@]}"; do
    cp target/$PROFILE/$BIN dist/$BIN
done

# Build the Aptos Move framework and place it in dist. It can be found afterwards in the current directory.
(cd dist && cargo run --package aptos-framework -- release)

#!/bin/bash
# Copyright (c) Velor
# SPDX-License-Identifier: Apache-2.0
set -e

PROFILE=${PROFILE:-release}

echo "Building indexer and related binaries"
echo "PROFILE: $PROFILE"

echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

# Build all the rust binaries
cargo build --locked --profile=$PROFILE \
    -p velor-indexer-grpc-cache-worker \
    -p velor-indexer-grpc-file-store \
    -p velor-indexer-grpc-data-service \
    -p velor-nft-metadata-crawler \
    -p velor-indexer-grpc-file-checker \
    -p velor-indexer-grpc-data-service-v2 \
    -p velor-indexer-grpc-manager \
    "$@"

# After building, copy the binaries we need to `dist` since the `target` directory is used as docker cache mount and only available during the RUN step
BINS=(
    velor-indexer-grpc-cache-worker
    velor-indexer-grpc-file-store
    velor-indexer-grpc-data-service
    velor-nft-metadata-crawler
    velor-indexer-grpc-file-checker
    velor-indexer-grpc-data-service-v2
    velor-indexer-grpc-manager
)

mkdir dist

for BIN in "${BINS[@]}"; do
    cp $CARGO_TARGET_DIR/$PROFILE/$BIN dist/$BIN
done

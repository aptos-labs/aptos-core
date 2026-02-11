#!/bin/bash
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
set -ex

PROFILE=${PROFILE:-release}

echo "Building indexer and related binaries"
echo "PROFILE: $PROFILE"

echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

BUILD_ENV=()
if [[ "$PROFILE" == "performance" ]]; then
  source "$(dirname -- "${BASH_SOURCE[0]}")/performance_rustflags.sh"
  BUILD_ENV=(RUSTFLAGS="${PERFORMANCE_RUSTFLAGS[*]}")
fi

# Build all the rust binaries
env "${BUILD_ENV[@]}" cargo build --locked --profile=$PROFILE \
    -p aptos-indexer-grpc-cache-worker \
    -p aptos-indexer-grpc-file-store \
    -p aptos-indexer-grpc-data-service \
    -p aptos-nft-metadata-crawler \
    -p aptos-indexer-grpc-file-checker \
    -p aptos-indexer-grpc-data-service-v2 \
    -p aptos-indexer-grpc-manager \
    -p aptos-indexer-grpc-gateway \
    "$@"

# After building, copy the binaries we need to `dist` since the `target` directory is used as docker cache mount and only available during the RUN step
BINS=(
    aptos-indexer-grpc-cache-worker
    aptos-indexer-grpc-file-store
    aptos-indexer-grpc-data-service
    aptos-nft-metadata-crawler
    aptos-indexer-grpc-file-checker
    aptos-indexer-grpc-data-service-v2
    aptos-indexer-grpc-manager
    aptos-indexer-grpc-gateway
)

mkdir dist

for BIN in "${BINS[@]}"; do
    cp $CARGO_TARGET_DIR/$PROFILE/$BIN dist/$BIN
done

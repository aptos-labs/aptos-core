#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
set -e

CMD="--release"
[ -n "$BUILD_PROFILE" ] && CMD="--profile $BUILD_PROFILE"
[ -z "$BUILD_PROFILE" ] && BUILD_PROFILE="release"

# Build all the rust release binaries
BUILD_FLAGS="--cfg tokio_unstable"
echo "RUSTFLAGS=$BUILD_FLAGS cargo build $CMD -p aptos-node [--profile performance]"
RUSTFLAGS=$BUILD_FLAGS cargo build --profile performance \
        -p aptos-node


echo "RUSTFLAGS=$BUILD_FLAGS cargo build $CMD all the rest ... $@ [--profile performance] "
RUSTFLAGS=$BUILD_FLAGS cargo build --profile performance \
	-p aptos
        -p aptos-faucet \
        -p aptos-indexer \
        -p aptos-node-checker \
        -p aptos-openapi-spec-generator \
        -p aptos-telemetry-service \
        -p backup-cli \
        -p db-bootstrapper \
        -p forge-cli \
        -p transaction-emitter \
        "$@"

BUILD_PROFILE="performance"

# After building, copy the binaries we need to `dist` since the `target` directory is used as docker cache mount and only available during the RUN step
BINS=(
    aptos
    aptos-faucet
    aptos-indexer
    aptos-node
    aptos-node-checker
    aptos-openapi-spec-generator
    aptos-telemetry-service
    db-backup
    db-backup-verify
    db-bootstrapper
    db-restore
    forge
    transaction-emitter
)

mkdir dist

for BIN in "${BINS[@]}"
do
	cp target/$BUILD_PROFILE/$BIN dist/$BIN
done

# Build the Aptos Move framework
cargo run --package framework -- --package aptos-framework --output current
cargo run --package framework -- --package aptos-token --output current

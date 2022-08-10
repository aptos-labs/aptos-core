#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
set -e

# Build all the rust release binaries
RUSTFLAGS="--cfg tokio_unstable" cargo build --performance \
        -p aptos \
        -p aptos-faucet \
        -p aptos-indexer \
        -p aptos-node \
        -p aptos-node-checker \
        -p aptos-openapi-spec-generator \
        -p backup-cli \
        -p db-bootstrapper \
        -p forge-cli \
        -p transaction-emitter \
        "$@"

# After building, copy the binaries we need to `dist` since the `target` directory is used as docker cache mount and only available during the RUN step
BINS=(
    aptos
    aptos-faucet
    aptos-indexer
    aptos-node
    aptos-node-checker
    aptos-openapi-spec-generator
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
    cp target/release/$BIN dist/$BIN
done

# Build the Aptos Move framework
cargo run --package framework -- --package aptos-framework --output current
cargo run --package framework -- --package aptos-token --output current

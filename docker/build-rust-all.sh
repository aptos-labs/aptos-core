#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
set -e

# Build all the rust release binaries
cargo build --release \
        -p aptos \
        -p aptos-node \
        -p aptos-indexer \
        -p aptos-faucet \
        -p aptos-node-checker \
        -p backup-cli \
        -p db-bootstrapper \
        -p transaction-emitter \
        -p forge-cli \
        "$@"

# after building, copy the binaries we need to `dist` since the `target` directory is used as docker cache mount and only available during the RUN step
BINS=(
    aptos
    aptos-node
    aptos-node-checker
    aptos-indexer
    aptos-faucet
    db-backup
    db-backup-verify
    db-restore
    db-bootstrapper
    transaction-emitter
    forge
)

mkdir dist

for BIN in "${BINS[@]}"
do
        cp target/release/$BIN dist/$BIN
done

# Build the Aptos Move framework
cargo run --package framework -- --package aptos-framework --output current

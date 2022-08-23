#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
set -e

# Build all the rust release binaries
RUSTFLAGS="--cfg tokio_unstable" cargo build --profile performance \
        -p aptos \
        -p aptos-faucet \
        -p aptos-indexer \
        -p aptos-node \
        -p aptos-node-checker \
        -p aptos-openapi-spec-generator \
        -p aptos-telemetry-service \
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
    cp target/performance/$BIN dist/$BIN
done

# Build the Aptos Move framework and place it in dist. It can be found afterwards in the current directory.
( cd dist && cargo run --package framework -- release )

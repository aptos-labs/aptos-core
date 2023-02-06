#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
set -e

PROFILE=${PROFILE:-release}
FEATURES=${FEATURES:-""}
TARGET=${TARGET:-$(rustc -vV | sed -n 's|host: ||p')}

echo "Building all rust-based docker images"
echo "PROFILE: $PROFILE"
echo "FEATURES: $FEATURES"

# Build all the rust binaries
cargo build --locked --target=$TARGET --profile=$PROFILE \
    -p aptos \
    -p aptos-backup-cli \
    -p aptos-faucet \
    -p aptos-forge-cli \
    -p aptos-fn-check-client \
    -p aptos-node-checker \
    -p aptos-openapi-spec-generator \
    -p aptos-telemetry-service \
    -p aptos-db-bootstrapper \
    -p aptos-transaction-emitter \
    "$@"

# Build aptos-node separately
cargo build --locked --target=$TARGET --profile=$PROFILE \
    -p aptos-node \
    "$@"

# Build and overwrite the aptos-node binary with features if specified
if [ -n "$FEATURES" ]; then
    echo "Building aptos-node with features ${FEATURES}"
    (cd aptos-node && cargo build --target=$TARGET --profile=$PROFILE --features=$FEATURES "$@")
fi

# After building, copy the binaries we need to `dist` since the `target` directory is used as docker cache mount and only available during the RUN step
BINS=(
    aptos
    aptos-faucet
    aptos-node
    aptos-node-checker
    aptos-openapi-spec-generator
    aptos-telemetry-service
    aptos-fn-check-client
    db-backup
    db-backup-verify
    aptos-db-bootstrapper
    db-restore
    forge
    aptos-transaction-emitter
)

mkdir -p dist

for BIN in "${BINS[@]}"; do
    cp target/*/$PROFILE/$BIN dist/$BIN
done

# Build the Aptos Move framework and place it in dist. It can be found afterwards in the current directory.
(cd dist && PKG_CONFIG_SYSROOT_DIR= RUSTFLAGS= cargo run --profile=$PROFILE --package aptos-framework -- release)

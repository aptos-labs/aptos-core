#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
set -e

# Build the rust release binaries
cargo build --release \
        -p aptos-genesis-tool \
        -p aptos-operational-tool \
        -p aptos-node \
        -p safety-rules \
        -p db-bootstrapper \
        -p backup-cli \
        -p aptos-transaction-replay \
        -p aptos-writeset-generator \
        -p transaction-emitter \
        -p aptos-indexer \
        -p aptos-node-checker \
        -p aptos \
        -p aptos-faucet \
        -p forge-cli 
        "$@"

mkdir dist

# for BIN in aptos-node aptos-genesis-tool aptos-operational-tool safety-rules db-bootstrapper backup-cli aptos-transaction-replay aptos-writeset-generator transaction-emitter aptos-indexer aptos-node-checker aptos aptos-faucet forge-cli;
# do
#         # cp target/release/$BIN dist/$BIN
# done

# Build the aptos move framework
cargo run --package framework -- --package aptos-framework --output current

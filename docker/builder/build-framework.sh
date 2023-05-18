#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
set -e

PROFILE=${PROFILE:-release}

echo "Building Aptos Move framework..."
echo "PROFILE: $PROFILE"
echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

mkdir dist

# Build the Aptos Move framework and place it in dist. It can be found afterwards in the current directory.
(cd dist && cargo run --locked --profile=$PROFILE --package aptos-framework -- release)
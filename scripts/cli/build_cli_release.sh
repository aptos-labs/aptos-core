#!/bin/bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

###########################################
# Build and package a release for the CLI #
###########################################
# Example:
# build_cli_release.sh macOS 1.0.0
#
# To skip checks:
# build_cli_release.sh macOS 1.0.0 true
#

# Note: This must be run from the root of the aptos-core repository

set -e

NAME='aptos-cli'
CRATE_NAME='aptos'
CARGO_PATH="crates/$CRATE_NAME/Cargo.toml"
PLATFORM_NAME="$1"
EXPECTED_VERSION="$2"
SKIP_CHECKS="$3"
COMPATIBILITY_MODE="$4"

# Grab system information
ARCH=$(uname -m)
OS=$(uname -s)
VERSION=$(sed -n '/^\w*version = /p' "$CARGO_PATH" | sed 's/^.*=[ ]*"//g' | sed 's/".*$//g')

if [[ "$SKIP_CHECKS" != "true" ]]; then
  # Check that the version is well-formed, note that it should already be correct, but this double checks it
  if ! [[ "$EXPECTED_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "$EXPECTED_VERSION is malformed, must be of the form '^[0-9]+\.[0-9]+\.[0-9]+$'"
    exit 1
  fi

  # Check that the version matches the Cargo.toml
  if [[ "$EXPECTED_VERSION" != "$VERSION" ]]; then
    echo "Wanted to release for $EXPECTED_VERSION, but Cargo.toml says the version is $VERSION"
    exit 2
  fi

  # Check that the release doesn't already exist
  if curl -s --stderr /dev/null --output /dev/null --head -f "https://github.com/aptos-labs/aptos-core/releases/download/aptos-cli-v$EXPECTED_VERSION/aptos-cli-$EXPECTED_VERSION-Ubuntu-22.04-x86_64.zip"; then
    echo "$EXPECTED_VERSION already released"
    exit 3
  fi
else
  echo "WARNING: Skipping version checks!"
fi

echo "Building release $VERSION of $NAME for $OS-$PLATFORM_NAME on $ARCH"
if [[ "$COMPATIBILITY_MODE" == "true" ]]; then
  RUSTFLAGS="-C target-cpu=generic --cfg tokio_unstable -C target-feature=-sse4.2,-avx" cargo build -p "$CRATE_NAME" --profile cli
else
  cargo build -p "$CRATE_NAME" --profile cli
fi
cd target/cli/

# Compress the CLI
ZIP_NAME="$NAME-$VERSION-$PLATFORM_NAME-$ARCH.zip"

echo "Zipping release: $ZIP_NAME"
zip "$ZIP_NAME" "$CRATE_NAME"
mv "$ZIP_NAME" ../..

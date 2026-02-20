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
STATIC_LINK="$5"

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

TARGET_FLAG=""
OUTPUT_DIR="target/cli"

# Static linking via musl eliminates GLIBC and OpenSSL runtime dependencies.
if [[ "$STATIC_LINK" == "true" ]]; then
  case "$ARCH" in
    x86_64)  MUSL_TARGET="x86_64-unknown-linux-musl" ;;
    aarch64) MUSL_TARGET="aarch64-unknown-linux-musl" ;;
    *)
      echo "Unsupported architecture for static linking: $ARCH"
      exit 4
      ;;
  esac

  echo "Static build enabled, using target: $MUSL_TARGET"
  rustup target add "$MUSL_TARGET"
  TARGET_FLAG="--target $MUSL_TARGET"
  OUTPUT_DIR="target/$MUSL_TARGET/cli"
fi

if [[ "$COMPATIBILITY_MODE" == "true" ]]; then
  RUSTFLAGS="-C target-cpu=generic --cfg tokio_unstable -C target-feature=-sse4.2,-avx" cargo build -p "$CRATE_NAME" --profile cli $TARGET_FLAG
else
  cargo build -p "$CRATE_NAME" --profile cli $TARGET_FLAG
fi
cd "$OUTPUT_DIR"

# Compress the CLI
ZIP_NAME="$NAME-$VERSION-$PLATFORM_NAME-$ARCH.zip"

echo "Zipping release: $ZIP_NAME"
zip "$ZIP_NAME" "$CRATE_NAME"
mv "$ZIP_NAME" "$OLDPWD"

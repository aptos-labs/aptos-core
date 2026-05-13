#!/bin/sh
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0

#########################################################
# Build and package a binary release for any executable #
#########################################################
# Example:
# build_binary_release.sh my-tool my-crate tool 1.0.0
#
# To skip version checks:
# build_binary_release.sh my-tool my-crate tool 1.0.0 true
#

# Note: This must be run from the root of the aptos-core repository

set -e

BINARY_NAME="$1"
CRATE_NAME="$2"
BUILD_PROFILE="$3"
EXPECTED_VERSION="$4"
SKIP_CHECKS="${5:-false}"

# Validate inputs
if [ -z "$BINARY_NAME" ] || [ -z "$CRATE_NAME" ] || [ -z "$BUILD_PROFILE" ] || [ -z "$EXPECTED_VERSION" ]; then
  echo "Usage: $0 <binary-name> <crate-name> <tool|performance> <version> [skip_checks]"
  echo "Example: $0 aptos-node aptos-node performance 1.0.0"
  exit 1
fi

# Validate build profile
if [ "$BUILD_PROFILE" != "tool" ] && [ "$BUILD_PROFILE" != "performance" ]; then
  echo "Build profile must be either 'tool' or 'performance', got: $BUILD_PROFILE"
  exit 1
fi

# Determine the cargo path - try common locations
CARGO_PATH=""
if [ -f "crates/$CRATE_NAME/Cargo.toml" ]; then
  CARGO_PATH="crates/$CRATE_NAME/Cargo.toml"
elif [ -f "$CRATE_NAME/Cargo.toml" ]; then
  CARGO_PATH="$CRATE_NAME/Cargo.toml"
elif [ -f "aptos-move/$CRATE_NAME/Cargo.toml" ]; then
  CARGO_PATH="aptos-move/$CRATE_NAME/Cargo.toml"
else
  # Search for Cargo.toml with matching crate name
  echo "Searching for Cargo.toml with name = \"$CRATE_NAME\"..."
  CARGO_PATH=$(find crates aptos-move -name "Cargo.toml" -type f 2>/dev/null | while read -r toml; do
    if grep -q "^name = \"$CRATE_NAME\"" "$toml"; then
      echo "$toml"
      break
    fi
  done)

  if [ -z "$CARGO_PATH" ]; then
    echo "Could not find Cargo.toml for crate $CRATE_NAME"
    echo "Searched in: crates/, aptos-move/, and root directory"
    exit 1
  fi
  echo "Found Cargo.toml at: $CARGO_PATH"
fi

# Get version from Cargo.toml
VERSION=$(sed -n '/^\w*version = /p' "$CARGO_PATH" | head -n 1 | sed 's/^.*=[ ]*"//g' | sed 's/".*$//g')

# Grab system information
ARCH=$(uname -m)
OS=$(uname -s)

# Map to Rust target triple
case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64)
        TARGET_TRIPLE="x86_64-unknown-linux-gnu"
        ;;
      aarch64|arm64)
        TARGET_TRIPLE="aarch64-unknown-linux-gnu"
        ;;
      *)
        echo "Unsupported Linux architecture: $ARCH"
        exit 1
        ;;
    esac
    ;;
  Darwin)
    case "$ARCH" in
      x86_64)
        TARGET_TRIPLE="x86_64-apple-darwin"
        ;;
      arm64)
        TARGET_TRIPLE="aarch64-apple-darwin"
        ;;
      *)
        echo "Unsupported macOS architecture: $ARCH"
        exit 1
        ;;
    esac
    ;;
  *)
    echo "Unsupported operating system: $OS"
    exit 1
    ;;
esac

if [ "$SKIP_CHECKS" != "true" ]; then
  # Check that the version is well-formed
  if ! echo "$EXPECTED_VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "$EXPECTED_VERSION is malformed, must be of the form '^[0-9]+\.[0-9]+\.[0-9]+$'"
    exit 1
  fi

  # Check that the version matches the Cargo.toml
  if [ "$EXPECTED_VERSION" != "$VERSION" ]; then
    echo "Wanted to release for $EXPECTED_VERSION, but Cargo.toml says the version is $VERSION"
    exit 2
  fi
else
  echo "WARNING: Skipping version checks!"
fi

echo "Building release $VERSION of $BINARY_NAME for $TARGET_TRIPLE using profile '$BUILD_PROFILE'"

# Build the binary
cargo build -p "$CRATE_NAME" --profile "$BUILD_PROFILE"

# Determine the output directory based on profile
if [ "$BUILD_PROFILE" = "tool" ]; then
  BUILD_DIR="target/tool"
elif [ "$BUILD_PROFILE" = "performance" ]; then
  BUILD_DIR="target/performance"
else
  # Fallback for other profiles
  BUILD_DIR="target/$BUILD_PROFILE"
fi

cd "$BUILD_DIR"

# Compress the binary with 'v' prefix in version
ZIP_NAME="$BINARY_NAME-v$VERSION-$TARGET_TRIPLE.zip"

echo "Zipping release: $ZIP_NAME"

# Handle the case where the binary might have a different name than the crate
if [ -f "$BINARY_NAME" ]; then
  zip "$ZIP_NAME" "$BINARY_NAME"
elif [ -f "$CRATE_NAME" ]; then
  # If binary name differs from crate name, copy it
  cp "$CRATE_NAME" "$BINARY_NAME"
  zip "$ZIP_NAME" "$BINARY_NAME"
  rm "$BINARY_NAME"
else
  echo "Could not find binary $BINARY_NAME or $CRATE_NAME in $BUILD_DIR"
  exit 1
fi

# Generate SHA256 checksum
echo "Generating SHA256 checksum"
if command -v shasum >/dev/null 2>&1; then
  shasum -a 256 "$ZIP_NAME" > "$ZIP_NAME.sha256"
elif command -v sha256sum >/dev/null 2>&1; then
  sha256sum "$ZIP_NAME" > "$ZIP_NAME.sha256"
else
  echo "Warning: Neither shasum nor sha256sum found, skipping checksum generation"
fi

mv "$ZIP_NAME"* ../..

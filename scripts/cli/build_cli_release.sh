#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

###########################################
# Build and package a release for the CLI #
###########################################

# Note: This must be run from the root of the aptos-core repository

set -e

NAME='aptos-cli'
CRATE_NAME='aptos'
CARGO_PATH="crates/$CRATE_NAME/Cargo.toml"

# Grab system information
ARCH=`uname -m`
OS=`uname -s`
VERSION=`cat "$CARGO_PATH" | grep "^\w*version =" | sed 's/^.*=[ ]*"//g' | sed 's/".*$//g'`

if [ "$OS" == "Darwin" ]; then
  # Rename Darwin to MacOSX so it's less confusing
  OS="MacOSX"
elif [ "$OS" == "Linux" ]; then
  # Get linux flavor
  OS=`cat /etc/os-release | grep '^NAME=' | sed 's/^.*=//g' | sed 's/"//g'`
fi

echo "Building release $VERSION of $NAME for $OS-$ARCH"
cargo build -p $CRATE_NAME --profile cli

cd target/cli/

# Compress the CLI
ZIP_NAME="$NAME-$VERSION-$OS-$ARCH.zip"

echo "Zipping release: $ZIP_NAME"
zip $ZIP_NAME $CRATE_NAME
mv $ZIP_NAME ../..

# TODO: Add installation instructions?

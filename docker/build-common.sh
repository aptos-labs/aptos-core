#!/bin/bash
# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0
set -e

# This is a common compilation scripts across different docker file
# It unifies RUSFLAGS, compilation flags (like --release) and set of binary crates to compile in common docker layer

export RUSTFLAGS="-Ctarget-cpu=skylake -Ctarget-feature=+aes,+sse2,+sse4.1,+ssse3"

# We are using a pinned nightly cargo until feature resolver v2 is released (around 10/2020), but compiling with stable rustc
export CARGO_PROFILE_RELEASE_LTO=thin # override lto setting to turn on thin-LTO for release builds

# Disable the workspace-hack package to prevent extra features and packages from being enabled.
# Can't use ${CARGO} because of https://github.com/rust-lang/rustup/issues/2647 and
# https://github.com/env-logger-rs/env_logger/issues/190.
# TODO: consider using ${CARGO} once upstream issues are fixed.
# cargo x generate-workspace-hack --mode disable

if [ "$IMAGE_TARGET" != "release" ] && [ "$IMAGE_TARGET" != "test" ]; then
  echo "Error: IMAGE_TARGET must one of: release,test but received: $IMAGE_TARGET"
  exit -1
fi

if [ "$IMAGE_TARGET" = "release" ]; then
  # Build release binaries (TODO: use x to run this?)
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
          -p aptos \
          "$@"

  # Build our core modules!
  cargo run --package framework -- --package aptos-framework --output current

fi


if [ "$IMAGE_TARGET" = "test" ]; then
  # These non-release binaries are built separately to avoid feature unification issues
  cargo build --release \
          -p aptos-faucet \
          -p forge-cli \
          "$@"
fi

rm -rf target/release/{build,deps,incremental}

STRIP_DIR=${STRIP_DIR:-/aptos/target}
find "$STRIP_DIR/release" -maxdepth 1 -executable -type f | grep -Ev 'aptos-node|safety-rules' | xargs strip

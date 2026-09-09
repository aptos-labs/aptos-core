#!/usr/bin/env bash

# This script checks if the CLI depends on external deps that it shouldn't. We run this
# in CI to make sure we don't accidentally reintroduce deps that would make the CLI
# unusable on most systems.
#
# While it would be more reliable to actually build the CLI and check what libraries it
# links to, e.g. with otool, it is much cheaper to use cargo tree. As far as I can tell
# the entire Rust ecosystem makes use of these `x-sys` libraries to depend on external
# dynamically linked libraries.
#
# We can almost use cargo deny but it doesn't support checking specific build paths. We
# don't care if openssl-sys for example is used at build time (which it is, indirectly
# by shadow-rs), only at run time. See more here:
# https://github.com/EmbarkStudios/cargo-deny/issues/563
#
# It assumes cargo and friends are available.
#
# Run this from the root of the repo.

set -euo pipefail

# Resolve the forward graph once. An inverse lookup exits nonzero both when a
# package is absent and when resolution fails, which would hide broken checks.
dependency_tree=$(cargo tree --locked -e features,no-build,no-dev \
    --target aarch64-apple-darwin -p aptos --prefix none)

for dep in pq-sys openssl-sys; do
    echo "Checking for banned dependency $dep..."
    if grep -Eq "^${dep}( v| feature )" <<< "$dependency_tree"; then
        echo "Banned dependency $dep found!"
        exit 1
    fi
done

echo
echo "None of the banned dependencies are in use, great!"
exit 0

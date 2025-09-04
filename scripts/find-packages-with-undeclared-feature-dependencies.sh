#!/bin/bash

# Copyright © Velor Foundation
# SPDX-License-Identifier: Apache-2.0

# This script builds all packages in the repo individually in order to find packages that may depend on features that they didn't declare in their Cargo.toml .
# Since in CI and for production we usually only build the entire workspace, some these undeclared features may not appear immediately.
# See also https://github.com/rust-lang/cargo/issues/4463

# Example usage:
# $ ./scripts/cargo_update_outdated.sh
# $ git commit --all -m "Update dependencies"
set -ex

echo "Building all workspace packages."
cargo install cargo-workspaces && for package in $(cargo workspaces list --all --json | jq ".[].name" -r); do cargo build -p $package; done

# When building in test mode, we pass in the name of a non-existent
# test to prevent the tests from actually running. We just want to
# see if the packages can build.
echo "Building all workspace packages in test compilation mode."
for package in $(cargo workspaces list --all --json | jq ".[].name" -r); do cargo test -p $package "test_name_does_not_exist_just_build"; done

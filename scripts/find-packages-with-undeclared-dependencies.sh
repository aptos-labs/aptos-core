#!/bin/bash

# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

# This script builds all packages in the repo individually in order to find packages that may depend on features that they didn't declare in their Cargo.toml .
# Since in CI and for production we usually only build the entire workspace, some these undeclared features may not appear immediately.
# See also https://github.com/rust-lang/cargo/issues/4463

# Example usage:
# $ ./scripts/cargo_update_outdated.sh
# $ git commit --all -m "Update dependencies"
set -ex

cargo install cargo-workspaces && for package in $(cargo workspaces list --all --json | jq ".[].name" -r); do cargo build -p $package; done

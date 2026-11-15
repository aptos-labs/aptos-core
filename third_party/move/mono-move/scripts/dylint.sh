#!/usr/bin/env bash
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
#
# Runs the custom lints in devtools/lints over the mono-move crates. One entry
# point for CI and local runs, so they cannot diverge.
#
#   ./scripts/dylint.sh            lint every mono-move crate
#   ./scripts/dylint.sh <args>     forward extra `cargo dylint` arguments
#
# Which crates each lint applies to is configured in /dylint.toml. Needs
# cargo-dylint and dylint-link (scripts/dev_setup.sh installs them); the lint
# library's nightly (devtools/lints/rust-toolchain.toml) is installed by rustup
# on first use.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
cd "$REPO_ROOT"

# Every package whose manifest lives under mono-move/, except benches.
mapfile -t PACKAGES < <(
  cargo metadata --format-version 1 --no-deps \
    | python3 -c '
import json, sys
for pkg in json.load(sys.stdin)["packages"]:
    if "/third_party/move/mono-move/" in pkg["manifest_path"]:
        print(pkg["name"])
' | sort -u
)

args=()
for pkg in "${PACKAGES[@]}"; do
  args+=(-p "$pkg")
done

exec cargo dylint --all "${args[@]}" "$@"

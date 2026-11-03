#!/usr/bin/env bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
#
# N4 (plan §7): checks that the model accepts every module of the Aptos
# framework's release bundle (head.mrb), as the Rust verifier does, and
# reports how much of the framework the theorems' supported subset covers
# (Tests/Framework.lean). The exported modules are not committed: they
# change with every change to the framework.
set -euo pipefail
cd "$(dirname "$0")/.."

out=.lake/framework-check/head.json
mkdir -p "$(dirname "$out")"
cargo run --quiet --locked -p move-bytecode-lean-vm-oracle -- \
  Tests/framework/head.spec.json --output "$out"
lake build Model Import
lake env lean --run Tests/Framework.lean "$out"

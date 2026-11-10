#!/usr/bin/env bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
#
# Regenerates the generated fixtures under Tests/fixtures/. CI runs this and
# fails if the committed fixtures differ.
set -euo pipefail
cd "$(dirname "$0")/.."

run() {
  local package="$1"
  shift
  cargo run --quiet --locked -p "$package" -- "$@"
}

# Exporter: instruction catalog and the import fixture.
run move-bytecode-lean-export --instruction-catalog --output Tests/fixtures/instructions.json
run move-bytecode-lean-export Tests/fixtures/core_math.masm Tests/fixtures/core_client.masm \
  --output Tests/fixtures/core.json

# The production verifier configuration (A3).
run move-bytecode-lean-config-export --output Tests/fixtures/verifier_config.json

# Differential runs on the legacy Move VM and MonoVM.
for spec in Tests/differential/*.spec.json; do
  name="$(basename "$spec" .spec.json)"
  run move-bytecode-lean-vm-oracle "$spec" --output "Tests/fixtures/diff_${name}.json"
done

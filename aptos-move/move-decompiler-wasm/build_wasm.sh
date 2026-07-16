#!/usr/bin/env bash
# Copyright (c) Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
#
# Build the bytecode-v10 Move decompiler/disassembler WASM and refresh dist/.
#
# Usage:
#   ./build_wasm.sh            # build web target + samples, verify in node
#   ./build_wasm.sh --no-verify
#
# Requirements: rustup (with wasm32-unknown-unknown), wasm-pack, node (for verify).
set -euo pipefail

cd "$(dirname "$0")"

VERIFY=1
for arg in "$@"; do
  case "$arg" in
    --no-verify) VERIFY=0 ;;
    *) echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

echo "==> Ensuring wasm32-unknown-unknown target is installed"
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "==> Installing wasm-pack"
  cargo install wasm-pack
fi

echo "==> Building web package (pkg/)"
wasm-pack build --release --target web --out-dir pkg

echo "==> Building nodejs package (pkg-node/)"
wasm-pack build --release --target nodejs --out-dir pkg-node

echo "==> Generating sample bytecode-v10 artifacts (samples/)"
cargo run --release --example gen_samples

echo "==> Refreshing dist/"
mkdir -p dist/samples
cp pkg/move_decompiler_wasm_bg.wasm       dist/
cp pkg/move_decompiler_wasm_bg.wasm.d.ts  dist/
cp pkg/move_decompiler_wasm.js            dist/
cp pkg/move_decompiler_wasm.d.ts          dist/
cp samples/*.mv                           dist/samples/

if [ "$VERIFY" -eq 1 ]; then
  if command -v node >/dev/null 2>&1; then
    echo "==> Verifying against real v10 module + script in node"
    node examples/verify_node.mjs
  else
    echo "==> node not found; skipping end-to-end verification"
  fi
fi

echo
echo "Done. WASM: $(du -h dist/move_decompiler_wasm_bg.wasm | cut -f1) at dist/move_decompiler_wasm_bg.wasm"

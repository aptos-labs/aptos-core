#!/bin/bash
ROOT="$(pwd)"
cargo build --release
cd ./aptos-move/framework/supra-framework && "$ROOT"/target/release/aptos move test || exit 1
cd ..
cargo test --release -p aptos-framework -- --skip prover || exit 2
RUST_MIN_STACK=104857600 cargo nextest run -p e2e-move-tests || exit 3
RUST_MIN_STACK=104857600 cargo nextest run -p language-e2e-testsuite --no-fail-fast || exit 4

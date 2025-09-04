#!/bin/bash

FRAMEWORK="../../../../../../velor-move/framework/velor-framework/sources"

# Benchmark per function
cargo run --release -p prover-lab -- bench -f -c prover.toml $FRAMEWORK/*.move

# Benchmark per module
cargo run --release -p prover-lab -- bench -c prover.toml $FRAMEWORK/*.move

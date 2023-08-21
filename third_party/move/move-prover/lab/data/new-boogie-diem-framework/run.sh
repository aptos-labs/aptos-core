#!/bin/bash

FRAMEWORK="../../../../documentation/examples/diem-framework/move-packages/DPN/sources"
STDLIB="../../../../move-stdlib/sources"
NURSERY="../../../../move-stdlib/nursery/sources"

# Benchmark per function
cargo run --release -p prover-lab -- bench -f -c new_boogie.toml $FRAMEWORK/*.move

# Benchmark per module
cargo run --release -p prover-lab -- bench -c new_boogie.toml $FRAMEWORK/*.move

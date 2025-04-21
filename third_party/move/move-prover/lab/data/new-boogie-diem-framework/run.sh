#!/bin/bash

FRAMEWORK="../../../../move-examples/diem-framework/move-packages/DPN/sources"
STDLIB="../../../../move-stdlib/sources"
NURSERY="../../../../move-stdlib/nursery/sources"

# Check if the first argument is either "new" or "current"
if [[ "$1" != "new" && "$1" != "current" ]]; then
    echo "Invalid argument. The first argument must be 'new' or 'current'."
    exit 1
fi

# Benchmark per function
cargo run --release -p prover-lab -- bench -f -c $1_boogie_1.toml $FRAMEWORK/*.move

# Benchmark per module
cargo run --release -p prover-lab -- bench -c $1_boogie_1.toml $FRAMEWORK/*.move

# Benchmark per function
cargo run --release -p prover-lab -- bench -f -c $1_boogie_2.toml $FRAMEWORK/*.move

# Benchmark per module
cargo run --release -p prover-lab -- bench -c $1_boogie_2.toml $FRAMEWORK/*.move

# Benchmark per function
cargo run --release -p prover-lab -- bench -f -c $1_boogie_3.toml $FRAMEWORK/*.move

# Benchmark per module
cargo run --release -p prover-lab -- bench -c $1_boogie_3.toml $FRAMEWORK/*.move

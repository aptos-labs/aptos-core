#!/bin/bash

APTOS_STD="../../../../../../aptos-move/framework/aptos-stdlib/sources"

# Check if the first argument is either "new" or "current"
if [[ "$1" != "new" && "$1" != "current" ]]; then
    echo "Invalid argument. The first argument must be 'new' or 'current'."
    exit 1
fi

# Benchmark per function (with `-f``). `-a` is for including the aptos-natives.
cargo run --release -p prover-lab -- bench -a -f -c $1_boogie_1.toml $APTOS_STD/*.move $APTOS_STD/cryptography/*.move $APTOS_STD/data_structures/*.move

# Benchmark per module (without `-f`). `-a` is for including the aptos-natives.
cargo run --release -p prover-lab -- bench -a -c $1_boogie_1.toml $APTOS_STD/*.move $APTOS_STD/cryptography/*.move $APTOS_STD/data_structures/*.move

# Benchmark per function (with `-f``). `-a` is for including the aptos-natives.
cargo run --release -p prover-lab -- bench -a -f -c $1_boogie_2.toml $APTOS_STD/*.move $APTOS_STD/cryptography/*.move $APTOS_STD/data_structures/*.move

# Benchmark per module (without `-f`). `-a` is for including the aptos-natives.
cargo run --release -p prover-lab -- bench -a -c $1_boogie_2.toml $APTOS_STD/*.move $APTOS_STD/cryptography/*.move $APTOS_STD/data_structures/*.move

# Benchmark per function (with `-f``). `-a` is for including the aptos-natives.
cargo run --release -p prover-lab -- bench -a -f -c $1_boogie_3.toml $APTOS_STD/*.move $APTOS_STD/cryptography/*.move $APTOS_STD/data_structures/*.move

# Benchmark per module (without `-f`). `-a` is for including the aptos-natives.
cargo run --release -p prover-lab -- bench -a -c $1_boogie_3.toml $APTOS_STD/*.move $APTOS_STD/cryptography/*.move $APTOS_STD/data_structures/*.move

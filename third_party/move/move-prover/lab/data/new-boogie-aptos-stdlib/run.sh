#!/bin/bash

APTOS_STD="../../../../../../aptos-move/framework/aptos-stdlib/sources"

# Benchmark per function (with `-f``). `-a` is for including the aptos-natives.
cargo run --release -p prover-lab -- bench -a -f -c new_boogie_1.toml $APTOS_STD/*.move $APTOS_STD/cryptography/*.move $APTOS_STD/data_structures/*.move

# Benchmark per module (without `-f`). `-a` is for including the aptos-natives.
cargo run --release -p prover-lab -- bench -a -c new_boogie_1.toml $APTOS_STD/*.move $APTOS_STD/cryptography/*.move $APTOS_STD/data_structures/*.move

# Benchmark per function (with `-f``). `-a` is for including the aptos-natives.
cargo run --release -p prover-lab -- bench -a -f -c new_boogie_2.toml $APTOS_STD/*.move $APTOS_STD/cryptography/*.move $APTOS_STD/data_structures/*.move

# Benchmark per module (without `-f`). `-a` is for including the aptos-natives.
cargo run --release -p prover-lab -- bench -a -c new_boogie_2.toml $APTOS_STD/*.move $APTOS_STD/cryptography/*.move $APTOS_STD/data_structures/*.move

# Benchmark per function (with `-f``). `-a` is for including the aptos-natives.
cargo run --release -p prover-lab -- bench -a -f -c new_boogie_3.toml $APTOS_STD/*.move $APTOS_STD/cryptography/*.move $APTOS_STD/data_structures/*.move

# Benchmark per module (without `-f`). `-a` is for including the aptos-natives.
cargo run --release -p prover-lab -- bench -a -c new_boogie_3.toml $APTOS_STD/*.move $APTOS_STD/cryptography/*.move $APTOS_STD/data_structures/*.move

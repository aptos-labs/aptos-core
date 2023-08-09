#!/bin/bash

APTOS_STDLIB="../../../../../../aptos-move/framework/aptos-stdlib/sources"

# Benchmark per function (with `-f``). `-a` is for including the aptos-natives.
cargo run --release -p prover-lab -- bench -a -f -c prover.toml $APTOS_STDLIB/*.move $APTOS_STDLIB/data_structures/*.move $APTOS_STDLIB/cryptography/*.move

# Benchmark per module (without `-f`). `-a` is for including the aptos-natives.
cargo run --release -p prover-lab -- bench -a -c prover.toml $APTOS_STDLIB/*.move $APTOS_STDLIB/data_structures/*.move $APTOS_STDLIB/cryptography/*.move

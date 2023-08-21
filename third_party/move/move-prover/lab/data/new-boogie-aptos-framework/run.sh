#!/bin/bash

FRAMEWORK="../../../../../../aptos-move/framework/aptos-framework/sources"

# Benchmark per function (with `-f``). `-a` is for including the aptos-natives.
cargo run --release -p prover-lab -- bench -a -f -c current_boogie_1.toml $FRAMEWORK/*.move $FRAMEWORK/configs/*.move $FRAMEWORK/aggregator/*.move

# Benchmark per module (without `-f`). `-a` is for including the aptos-natives.
cargo run --release -p prover-lab -- bench -a -c current_boogie_1.toml $FRAMEWORK/*.move $FRAMEWORK/configs/*.move $FRAMEWORK/aggregator/*.move

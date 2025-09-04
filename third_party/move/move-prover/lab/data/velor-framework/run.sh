#!/bin/bash

FRAMEWORK="../../../../../../velor-move/framework/velor-framework/sources"

# Benchmark per function (with `-f``). `-a` is for including the velor-natives.
cargo run --release -p prover-lab -- bench -a -f -c prover.toml $FRAMEWORK/*.move $FRAMEWORK/configs/*.move $FRAMEWORK/aggregator/*.move

# Benchmark per module (without `-f`). `-a` is for including the velor-natives.
cargo run --release -p prover-lab -- bench -a -c prover.toml $FRAMEWORK/*.move $FRAMEWORK/configs/*.move $FRAMEWORK/aggregator/*.move

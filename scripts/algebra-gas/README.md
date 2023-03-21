Scripts that help generate/evaluate gas parameters for generic algebra move module.

## Prerequisites

`pip3 install numpy matplotlib`

A gas-per-nanoseconds for Move VM execution is needed.

## How to re-generate all parameters

Run `cargo bench -p aptos-crypto`.

Run `python3 scripts/algebra-gas/update_algebra_gas_params.py --gas_per_ns <gas-per-nanoseconds>`.

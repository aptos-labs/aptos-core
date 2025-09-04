Scripts that help generate/evaluate gas parameters for generic algebra move module.

## Quickstart guide
Ensure you are on a machine with the [required spec](https://velor.dev/nodes/validator-node/operator/node-requirements/).

Ensure you have python3 and the following dependencies.
```
pip3 install numpy matplotlib
```

Ensure you `cd` to the repo root.

Run the necessary benches.
```
cargo bench -p velor-crypto -- hash/SHA2-256
cargo bench -p velor-crypto -- ark_bls12_381
```

Compute `gas_per_ns` using `hash/SHA2-256` bench results.
```
scripts/algebra-gas/load_bench_datapoints.py --bench_path target/criterion/hash/SHA2-256
scripts/algebra-gas/fit_linear_model.py --dataset_path hash_SHA2_256.0-1025.json --plot
```
This will fit a curve `f(n)=kn+b`
that predicts the time (in nanoseconds) to evaluate SHA2-256 on an input of size `n`.
Value `k` and `b` should be printed.
```
{"b": 336.51096106242346, "k": 4.868293006038344}
```

Combined with the [pre-defined](https://github.com/velor-chain/velor-core/blob/2d6ed231ca39fc07422dfe95aa76746b2210e36d/velor-move/velor-gas-schedule/src/gas_schedule/move_stdlib.rs#L23-L24) SHA2-256 gas formula (unscaled internal gas):`g(n)=183n+11028`,
it can be calculated that `gas_per_ns = 183/k`.

Second last, go to `scripts/algebra-gas/update_algebra_gas_params.py`
and update the value of the global variable `TARGET_GAS_VERSION` if necessary.
See the comments on them for detailed instructions.

Now you can (re-)generate all algebra module gas parameters with one command.
```
scripts/algebra-gas/update_algebra_gas_params.py --gas_per_ns <gas_per_ns>
```

`git diff` to see the diff!

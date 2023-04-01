Scripts that help generate/evaluate gas parameters for generic algebra move module.

## Quickstart guide
Ensure you are on a machine with the [required spec](https://aptos.dev/nodes/validator-node/operator/node-requirements/).

Ensure you have python3 and the following dependencies.
```
pip3 install numpy matplotlib
```

Ensure you `cd` to the repo root.

Run the necessary benches.
```
cargo bench -p aptos-crypto -- hash/SHA2-256
cargo bench -p aptos-crypto -- ark_bls12_381
```

If you don't have a `gas_per_ns`, estimate one using `hash/SHA2-256` bench results.
```
scripts/algebra-gas/load_bench_datapoints.py --bench_path target/criterion/hash/SHA2-256
scripts/algebra-gas/fit_linear_model.py --dataset_path hash_SHA2_256.0-1025.json --plot
```
This will fit a curve $f(x)=k\cdot x+b$
that predicts the time (in nanoseconds) to evaluate SHA2-256 on an input of size $x$.
Value `k` and `b` should be printed.
```
{"b": 336.51096106242346, "k": 4.868293006038344}
```
Take the `k`, and now you can do the calculation `gas_per_ns = 1000/k`.
(1000 units of gas is defined to be equivalent to
the average change in ShA2-256 computational cost if the input length (in bytes) increases by 1).

Second last, go to `scripts/algebra-gas/update_algebra_gas_params.py`
and update the value of global variables `TARGET_GAS_VERSION` and `MUL` if necessary.
See the comments on them for detailed instructions.

Now you can (re-)generate all algebra module gas parameters with one command.
```
scripts/algebra-gas/update_algebra_gas_params.py --gas_per_ns <gas_per_ns>
```

`git diff` to see the diff!

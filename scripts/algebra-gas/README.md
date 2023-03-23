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
You will see some output like:
```
{"b": 336.51096106242346, "k": 4.868293006038344}
```
Take the `k`, and now you can do the calculation `gas_per_ns = 1000/k`.
Save the result somewhere.

Second last, go to `scripts/algebra-gas/update_algebra_gas_params.py`
and update the value of global variables `TARGET_GAS_VERSION` and `MUL` if necessary.
See the comments on them for detailed instructions.

Now you can (re-)generate all algebra module gas parameters with one command.
```
scripts/algebra-gas/update_algebra_gas_params.py --gas_per_ns <gas_per_ns>
```

`git diff` to see the diff!

## Background

We measure the execution cost of SHA2-256 first,
then we define 1000 gas units as the marginal execution cost of SHA-256 (per input byte).
That's where the formula `gas_per_ns = 1000/k` comes from.

We measure execution cost by its wall-clock time.

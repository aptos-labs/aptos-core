# README

## Move unit tests

To run, use the following command in this directory:
```
TEST_FILTER=conf cargo test -- experimental --skip prover
```

## Gas benchmarks

Relative to the root of the `aptos-core` repository, run:
```
cd aptos-move/e2e-move-tests/src/
cargo test -- bench_gas
```

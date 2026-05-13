# README

## Move unit tests

To run, use the following command in this directory:
```
TEST_FILTER=conf cargo test -- framework --skip prover
```

## Gas benchmarks

Relative to the root of the `aptos-core` repository, run:
```
cargo run -p aptos-framework -- update-cached-packages --with-test-mode

cd aptos-move/e2e-move-tests/src/
cargo test --features move-harness-with-test-only -- bench_gas --nocapture
```

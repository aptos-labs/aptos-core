# e2e-move-tests

## Keyless

To run the keyless VM tests:

```
cargo test -- keyless
```

## Confidential assets

To run the keyless confidential asset tests:

```
cargo test -- confidential
```

To run the gas benchmarks, you first have to generate a `head.mrb` with the test-only functions included:
```
cargo run -p aptos-framework -- update-cached-packages --with-test-mode
```
Then, you can run the benches by filtering for them using `cargo test` with a certain feature flag on:
```
cargo test --features move-harness-with-test-only -- bench_gas --nocapture
```

# Benchmarks

## Batched Bulletproofs and DeKART

Go to `aptos-crypto`:
```
cd crates/aptos-crypto
```

Install [`criterion-means`](https://crates.io/crates/cargo-criterion-means):

```
cargo install criterion-means
```

Run the Bulletproof and DeKART benchmarks in one line via:
```
./run-range-proof-benches.sh
```

This will generate CSV data with the benchmark data, format it as Markdown and copy it to your clipboard!

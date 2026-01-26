# Benchmarks

## Batched Bulletproofs and DeKART

Go to `aptos-crypto/benches`:
```
cd crates/aptos-crypto/benches
```

Install [`cargo-criterion-means`](https://crates.io/crates/cargo-criterion-means):

```
cargo install cargo-criterion-means
```

Run the Bulletproof and DeKART benchmarks in one line via:
```
./run-range-proof-benches.sh
```

This will generate CSV data with the benchmark data, format it as Markdown and copy it to your clipboard!

## Chunky PVSS

Follow the same steps, but run the benchmarks via:

```
./run-pvss-benches.sh
```
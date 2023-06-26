---
id: Aptos-vm-benchmarks
title: Aptos VM Benchmarks
---

## Aptos VM Benchmarks

The Aptos VM Benchmark allows anyone to specify a set of Move modules and benchmark 
the running time of transactions in the AptosVM measured in milliseconds.

## Usage

Move packages should be placed under the `/samples` directory. In order to benchmark
a transaction, the function must be marked with `entry` and have a prefix of `benchmark`.
For example, the following funcitons would work:

```Move

//// acceptable formats
public entry fun benchmark() {}

public entry fun benchmark_another_txn() {}

public entry fun benchmark123() {}

//// inacceptable formats
public fun benchmark() {}

public fun test_my_txn() {}

public entry fun benchmark_addition(_x: u64, _y: u64) {}

```

To run the benchmarks, simply run `cargo run`.
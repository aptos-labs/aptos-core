---
id: Aptos-vm-benchmarks
title: Aptos VM Benchmarks
---

## Aptos VM Benchmarks

The Aptos VM Benchmark allows anyone to specify a set of Move modules and benchmark 
the running time of transactions in the AptosVM measured in milliseconds. The Aptos
VM Benchmark allows an arbitrary amount of packages to be benchmarked.

## Usage

Move packages should be placed under the `/samples` directory. In order to benchmark
a transaction, the function must be marked with `entry` and have a prefix of `benchmark`.
For example, the following functions would work:

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

To run the benchmarks, simply run `cargo run`. There is also support for filtering specific 
tests to run. If no pattern is provided, it will run all the benchmarks.

```Bash
cargo run --release PATTERN
```

## Adding a benchmark

```
cd aptos-vm-benchmarks/samples
mkdir proj-name
cd proj-name
cargo run -p aptos -- move init --name proj-name
```

## Examples

Two basic examples are included under the `/samples` directory to demonstrate how the 
crate works: `/samples/add-numbers` and `/samples/do-nothing`. The `do-nothing` package 
demonstrates basic usage, as well as situations where the benchmark script will complain.

Additionally, `add-numbers` shows a simple implementation of a Move module and how one
might benchmark it.
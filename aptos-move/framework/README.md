---
id: Aptos-framework
title: Aptos Framework
custom_edit_url: https://github.com/aptos-labs/aptos-core/edit/main/Aptos-move/Aptos-framework/README.md
---

## The Aptos Framework

The Aptos Framework defines the standard actions that can be performed on-chain
both by the Aptos VM---through the various prologue/epilogue functions---and by
users of the blockchain---through the allowed set of transactions. This
directory contains different directories that hold the source Move
modules and transaction scripts, along with a framework for generation of
documentation, ABIs, and error information from the Move source
files. See the [Layout](#layout) section for a more detailed overview of the structure.

## Documentation

Reference documentation for every framework module lives in the
**[Aptos Framework Book](https://aptos-labs.github.io/framework-book/)**,
auto-generated from the source in this directory:

* [Move Standard Library](https://aptos-labs.github.io/framework-book/move-stdlib/overview.html)
* [Aptos Standard Library](https://aptos-labs.github.io/framework-book/aptos-stdlib/overview.html)
* [Aptos Framework](https://aptos-labs.github.io/framework-book/aptos-framework/overview.html)
* [Aptos Token Objects](https://aptos-labs.github.io/framework-book/aptos-token-objects/overview.html)
* [Aptos Trading Framework](https://aptos-labs.github.io/framework-book/aptos-trading/overview.html)
* [Aptos Experimental Framework](https://aptos-labs.github.io/framework-book/aptos-experimental/overview.html)

The book is built and deployed by
`third_party/move/documentation/framework-book/deploy.sh`; see
[`third_party/move/documentation/README.md`](../../third_party/move/documentation/README.md)
for the deploy workflow.

Follow our [contributing guidelines](CONTRIBUTING.md) and basic coding standards for the Aptos Framework.

## Generating documentation for your own packages

The Move documentation generator is also available as part of the Aptos CLI for
documenting user packages:

```shell
aptos move document --help
```

## Running Move tests

To test our Move code while developing the Aptos Framework, run `cargo test` inside this directory:

```
cargo test
```

(Alternatively, run `cargo test -p aptos-framework` from anywhere.)

To skip the Move prover tests, run:

```
cargo test -- --skip prover
```

To filter and run **all** the tests in specific packages (e.g., `aptos_stdlib`), run:

```
cargo test -- aptos_stdlib --skip prover
```

(See tests in `tests/move_unit_test.rs` to determine which filter to use; e.g., to run the tests in `aptos_framework` you must filter by `move_framework`.)

To **filter by test name or module name** in a specific package (e.g., run the `test_empty_range_proof` in `aptos_stdlib::ristretto255_bulletproofs`), run:

```
TEST_FILTER="test_range_proof" cargo test -- aptos_stdlib --skip prover
```

Or, e.g., run all the Bulletproof tests:
```
TEST_FILTER="bulletproofs" cargo test -- aptos_stdlib --skip prover
```

To show the amount of time and gas used in every test, set env var `REPORT_STATS=1`.
E.g.,
```
REPORT_STATS=1 TEST_FILTER="bulletproofs" cargo test -- aptos_stdlib --skip prover
```

Sometimes, Rust runs out of stack memory in dev build mode.  You can address this by either:
1. Adjusting the stack size

```
export RUST_MIN_STACK=4297152
```

2. Compiling in release mode

```
cargo test --release -- --skip prover
```

## Layout
The overall structure of the Aptos Framework is as follows:

```
├── aptos-framework                                 # Sources, testing and generated documentation for Aptos framework component
├── aptos-token                                 # Sources, testing and generated documentation for Aptos token component
├── aptos-stdlib                                 # Sources, testing and generated documentation for Aptos stdlib component
├── move-stdlib                                 # Sources, testing and generated documentation for Move stdlib component
├── cached-packages                                 # Tooling to generate SDK from move sources.
├── src                                     # Compilation and generation of information from Move source files in the Aptos Framework. Not designed to be used as a Rust library
├── releases                                    # Move release bundles
└── tests
```

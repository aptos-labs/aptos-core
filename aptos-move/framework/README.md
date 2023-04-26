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

Each of the main components of the Aptos Framework and contributing guidelines are documented separately. See them by version below:

* *Aptos tokens* - [main](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/doc/overview.md), [testnet](https://github.com/aptos-labs/aptos-core/blob/testnet/aptos-move/framework/aptos-token/doc/overview.md), [devnet](https://github.com/aptos-labs/aptos-core/blob/devnet/aptos-move/framework/aptos-token/doc/overview.md)
* *Aptos framework* - [main](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/doc/overview.md), [testnet](https://github.com/aptos-labs/aptos-core/blob/testnet/aptos-move/framework/aptos-framework/doc/overview.md), [devnet](https://github.com/aptos-labs/aptos-core/blob/devnet/aptos-move/framework/aptos-framework/doc/overview.md)
* *Aptos stdlib* - [main](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/doc/overview.md), [testnet](https://github.com/aptos-labs/aptos-core/blob/testnet/aptos-move/framework/aptos-stdlib/doc/overview.md), [devnet](https://github.com/aptos-labs/aptos-core/blob/devnet/aptos-move/framework/aptos-stdlib/doc/overview.md)
* *Move stdlib* - [main](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/move-stdlib/doc/overview.md), [testnet](https://github.com/aptos-labs/aptos-core/blob/testnet/aptos-move/framework/move-stdlib/doc/overview.md), [devnet](https://github.com/aptos-labs/aptos-core/blob/devnet/aptos-move/framework/move-stdlib/doc/overview.md)

Follow our [contributing guidelines](CONTRIBUTING.md) and basic coding standards for the Aptos Framework.

## Compilation and Generation

The documents above were created by the Move documentation generator for Aptos. It is available as part of the Aptos CLI. To see its options, run:
```shell
aptos move document --help
```

The documentation process is also integrated into the framework building process and will be automatically triggered like other derived artifacts, via `cached-packages` or explicit release building.

## Layout
The overall structure of the Aptos Framework is as follows:

```
├── aptos-framework                                 # Sources, testing and generated documentation for Aptos framework component
├── aptos-token                                 # Sources, testing and generated documentation for Aptos token component
├── aptos-stdlib                                 # Sources, testing and generated documentation for Aptos stdlib component
├── move-stdlib                                 # Sources, testing and generated documentation for Move stdlib component
├── cached-packages                                 # Tooling to generate SDK from mvoe sources.
├── src                                     # Compilation and generation of information from Move source files in the Aptos Framework. Not designed to be used as a Rust library
├── releases                                    # Move release bundles
└── tests
```

## Move unit test selection
In this package, unit test run triggered by `cargo test <filter>` will only select rust test wrappers
(for example, those marked as `#[test]` in `tests/move_unit_test.rs`).

To select the actual test cases in Move, use the following command.
```bash
MOVE_TEST_FILTER=<move-test-filter> cargo test <the-rest-params>
```
A test case is selected if and only if its full name (e.g. `0x1::ristretto255::test_point_neg`)
contains the value of `MOVE_TEST_FILTER` as a substring.

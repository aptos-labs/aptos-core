---
id: Supra-framework
title: Supra Framework
custom_edit_url: https://github.com/aptos-labs/aptos-core/edit/main/Supra-move/Supra-framework/README.md
---

## The Supra Framework

The Supra Framework defines the standard actions that can be performed on-chain
both by the Supra VM---through the various prologue/epilogue functions---and by
users of the blockchain---through the allowed set of transactions. This
directory contains different directories that hold the source Move
modules and transaction scripts, along with a framework for generation of
documentation, ABIs, and error information from the Move source
files. See the [Layout](#layout) section for a more detailed overview of the structure.

## Documentation

Each of the main components of the Supra Framework and contributing guidelines are documented separately. See them by version below:

* *Supra tokens* - [main](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-token/doc/overview.md), [testnet](https://github.com/aptos-labs/aptos-core/blob/testnet/aptos-move/framework/aptos-token/doc/overview.md), [devnet](https://github.com/aptos-labs/aptos-core/blob/devnet/aptos-move/framework/aptos-token/doc/overview.md)
* *Supra framework* - [main](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-framework/doc/overview.md), [testnet](https://github.com/aptos-labs/aptos-core/blob/testnet/aptos-move/framework/aptos-framework/doc/overview.md), [devnet](https://github.com/aptos-labs/aptos-core/blob/devnet/aptos-move/framework/aptos-framework/doc/overview.md)
* *Supra stdlib* - [main](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/aptos-stdlib/doc/overview.md), [testnet](https://github.com/aptos-labs/aptos-core/blob/testnet/aptos-move/framework/aptos-stdlib/doc/overview.md), [devnet](https://github.com/aptos-labs/aptos-core/blob/devnet/aptos-move/framework/aptos-stdlib/doc/overview.md)
* *Move stdlib* - [main](https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/move-stdlib/doc/overview.md), [testnet](https://github.com/aptos-labs/aptos-core/blob/testnet/aptos-move/framework/move-stdlib/doc/overview.md), [devnet](https://github.com/aptos-labs/aptos-core/blob/devnet/aptos-move/framework/move-stdlib/doc/overview.md)

Follow our [contributing guidelines](CONTRIBUTING.md) and basic coding standards for the Supra Framework.

## Compilation and Generation

The documents above were created by the Move documentation generator for Supra. It is available as part of the Supra CLI. To see its options, run:
```shell
aptos move document --help
```

The documentation process is also integrated into the framework building process and will be automatically triggered like other derived artifacts, via `cached-packages` or explicit release building.

## Running Move tests

To test our Move code while developing the Supra Framework, run `cargo test` inside this directory:

```
cargo test
```

(Alternatively, run `cargo test -p aptos-framework` from anywhere.)

To skip the Move prover tests, run:

```
cargo test -- --skip prover
```

To filter and run only the tests in specific packages (e.g., `aptos_stdlib`), run:

```
cargo test -- aptos_stdlib --skip prover
```

(See tests in `tests/move_unit_test.rs` to determine which filter to use; e.g., to run the tests in `aptos_framework` you must filter by `move_framework`.)

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
The overall structure of the Supra Framework is as follows:

```
├── aptos-framework                                 # Sources, testing and generated documentation for Supra framework component
├── aptos-token                                 # Sources, testing and generated documentation for Supra token component
├── aptos-stdlib                                 # Sources, testing and generated documentation for Supra stdlib component
├── move-stdlib                                 # Sources, testing and generated documentation for Move stdlib component
├── cached-packages                                 # Tooling to generate SDK from mvoe sources.
├── src                                     # Compilation and generation of information from Move source files in the Supra Framework. Not designed to be used as a Rust library
├── releases                                    # Move release bundles
└── tests
```

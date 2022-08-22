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

Each of the main components of the Aptos Framework and contributing guidelines are documented separately. Particularly:
* Documentation for the set of allowed transaction scripts in aptos-framework can be found in [script_documentation.md](aptos-framework/releases/artifacts/current/build/AptosFramework/docs/script_documentation.md).
* The overview documentation for the Move modules can be found in [overview.md](aptos-framework/releases/artifacts/current/build/AptosFramework/docs/overview.md).
* An overview of the approach to formal verification of the framework can be found in [spec_documentation.md](aptos-framework/releases/artifacts/current/build/AptosFramework/docs/spec_documentation.md).
* Contributing guidelines and basic coding standards for the Aptos Framework can be found in [CONTRIBUTING.md](CONTRIBUTING.md).

## Compilation and Generation

Recompilation of the Aptos Framework and the regeneration of the documents,
ABIs, and error information can be performed by running `cargo run` from this
directory. There are a number of different options available and these are
explained in the help for this command by running `cargo run -- --help` in this
directory. Compilation and generation will be much faster if run in release
mode (`cargo run --release`).

## Layout
The overall structure of the Aptos Framework is as follows:

```
├── compiled                                # Generated files and public rust interface to the Aptos Framework
│   ├── error_descriptions/*.errmap         # Generated error descriptions for use by the Move Explain tool
│   ├── src                                 # External Rust interface/library to use the Aptos Framework
│   ├── stdlib                              # The compiled Move bytecode of the Aptos Framework source modules
│   ├── script_abis                         # Generated ABIs for entry function transactions, and all new transactions
│   └── legacy/transaction_scripts          # Legacy generated ABIs and bytecode for each transaction script in the allowlist
│       ├── abi/*.abi                       # Directory containing generated ABIs for legacy transaction scripts
│       └── *.mv
├── modules                                 # Aptos Framework source modules, script modules, and generated documentation
│   ├── *.move
│   └── doc/*.md                            # Generated documentation for the Aptos Framework modules
├── nursery/*.move                          # Move modules that are not published on-chain, but are used for testing and debugging locally
├── src                                     # Compilation and generation of information from Move source files in the Aptos Framework. Not designed to be used as a Rust library
├── tests
└── script_documentation/*.md               # Generated documentation for allowed transaction scripts
```

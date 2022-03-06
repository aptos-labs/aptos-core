# aptos-sdk

[![aptos-sdk on crates.io](https://img.shields.io/crates/v/aptos-sdk)](https://crates.io/crates/aptos-sdk)
[![Documentation (latest release)](https://docs.rs/aptos-sdk/badge.svg)](https://docs.rs/aptos-sdk/)
[![Documentation (master)](https://img.shields.io/badge/docs-master-59f)](https://aptos.github.io/aptos/aptos_sdk/)
[![License](https://img.shields.io/badge/license-Apache-green.svg)](https://github.com/aptos-labs/aptos-core/blob/main/LICENSE)

The official Rust SDK for Aptos.

## Usage

This SDK provides all the necessary components for building on top of the Aptos Blockchain. Some of the important modules are:

* `client` - Includes a [JSON-RPC client](https://github.com/aptos-labs/aptos-core/blob/master/json-rpc/json-rpc-spec.md) implementation
* `crypto` - Types used for signing and verifying
* `transaction_builder` - Includes helpers for constructing transactions
* `types` - Includes types for Aptos on-chain data structures

## License

Aptos Core is licensed as [Apache 2.0](https://github.com/aptos-labs/aptos-core/blob/main/LICENSE).

# aptos-sdk

[![aptos-sdk on crates.io](https://img.shields.io/crates/v/aptos-sdk)](https://crates.io/crates/aptos-sdk)
[![Documentation (latest release)](https://docs.rs/aptos-sdk/badge.svg)](https://docs.rs/aptos-sdk/)
[![Documentation (master)](https://img.shields.io/badge/docs-master-59f)](https://diem.github.io/diem/aptos_sdk/)
[![License](https://img.shields.io/badge/license-Apache-green.svg)](https://github.com/diem/diem/blob/main/LICENSE)

The official Rust SDK for Diem.

## Usage

This SDK provides all the necessary components for building on top of the Diem Blockchain. Some of the important modules are:

* `client` - Includes a [JSON-RPC client](https://github.com/diem/diem/blob/master/json-rpc/json-rpc-spec.md) implementation
* `crypto` - Types used for signing and verifying
* `transaction_builder` - Includes helpers for constructing transactions
* `types` - Includes types for Diem on-chain data structures

## License

Diem Core is licensed as [Apache 2.0](https://github.com/diem/diem/blob/main/LICENSE).

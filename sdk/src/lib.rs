// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! The official Rust SDK for Aptos.
//!
//! ## Modules
//!
//! This SDK provides all the necessary components for building on top of the Aptos Blockchain. Some of the important modules are:
//!
//! * `crypto` - Types used for signing and verifying
//! * `transaction_builder` - Includes helpers for constructing transactions
//! * `types` - Includes types for Aptos on-chain data structures
//!
//! ## Example
//!
//! Here is a simple example to show how to create two accounts and do a p2p transfer on testnet:
//! todo(davidiw) bring back example using rest
//!

pub mod crypto {
    pub use aptos_crypto::*;
}

pub mod transaction_builder;

pub mod types;

pub mod move_types {
    pub use move_deps::move_core_types::*;
}

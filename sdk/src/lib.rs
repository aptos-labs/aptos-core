// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! The official Rust SDK for Aptos.
//!
//! ## Modules
//!
//! This SDK provides all the necessary components for building on top of the Aptos Blockchain. Some of the important modules are:
//!
//! * `crypto` - Types used for signing and verifying
//! * `move_types` - Includes types used when interacting with the Move VM
//! * `rest_client` - The Aptos API Client, used for sending requests to the Aptos Blockchain.
//! * `transaction_builder` - Includes helpers for constructing transactions
//! * `types` - Includes types for Aptos on-chain data structures
//!
//! ## Example
//!
//! Here is a simple example to show how to create two accounts and do a P2p transfer on testnet:
//! todo(davidiw) bring back example using rest
//!

pub use bcs;

pub mod coin_client;

pub mod crypto {
    pub use aptos_crypto::*;
}

pub mod move_types {
    pub use move_deps::move_core_types::*;
}

pub mod rest_client {
    pub use aptos_rest_client::*;
}

pub mod transaction_builder;

pub mod types;

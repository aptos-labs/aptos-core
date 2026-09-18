// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Construction APIs
//!
//! The construction APIs break down transactions into composable parts that are
//! used to be generic across blockchains.  A flow of operations can be found
//! in the [specifications](https://www.rosetta-api.org/docs/construction_api_introduction.html)
//!
//! This is broken down in the following flow:
//!
//! * Preprocess (based on operations) gets information to fetch from metadata (on-chain)
//! * Metadata fetches on-chain information e.g. sequence number
//! * Payloads generates an unsigned transaction
//! * Application outside signs the payload from the transaction
//! * Combine puts the signed transaction payload with the unsigned transaction
//! * Submit submits the signed transaction to the blockchain
//!
//! There are also 2 other sometimes used APIs
//! * Derive (get an account from the private key)
//! * Hash (get a hash of the transaction to lookup in mempool)
//!
//! Note: there is an "online" mode and an "offline" mode.  The offline APIs can run without
//! a connection to a full node.  The online ones need a connection to a full node.
//!

mod combine;
mod derive;
mod hash;
mod helpers;
mod metadata;
mod parse;
mod payloads;
mod preprocess;
mod routes;
mod submit;

pub(crate) use combine::*;
pub(crate) use derive::*;
pub(crate) use hash::*;
pub(crate) use helpers::*;
pub(crate) use metadata::*;
pub use parse::*;
pub(crate) use payloads::*;
pub(crate) use preprocess::*;
pub use routes::*;
pub(crate) use submit::*;

// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Core types for Move.

pub mod abi;
pub mod ability;
pub mod account_address;
pub mod diag_writer;
pub mod effects;
pub mod errmap;
pub mod function;
pub mod gas_algebra;
pub mod identifier;
pub mod int256;
pub mod language_storage;
pub mod metadata;
pub mod move_resource;
pub mod parser;
#[cfg(any(test, feature = "fuzzing"))]
pub mod proptest_types;
mod safe_serialize;
pub mod state;
pub mod transaction_argument;
#[cfg(test)]
mod unit_tests;
pub mod value;
pub mod vm_status;

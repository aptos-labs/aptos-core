// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Core types for Move.

pub mod abi;
pub mod ability;
pub mod account_address;
pub mod effects;
pub mod errmap;
pub mod function;
pub mod gas_algebra;
pub mod identifier;
pub mod language_storage;
pub mod metadata;
pub mod move_resource;
pub mod parser;
#[cfg(any(test, feature = "fuzzing"))]
pub mod proptest_types;
mod safe_serialize;
pub mod state;
pub mod transaction_argument;
pub mod u256;
#[cfg(test)]
mod unit_tests;
pub mod value;
pub mod vm_status;

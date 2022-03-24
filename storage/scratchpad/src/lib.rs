// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This crate provides in-memory representation of Aptos core data structures used by the executor.

mod sparse_merkle;

pub use crate::sparse_merkle::{
    FrozenSparseMerkleTree, ProofRead, SparseMerkleTree, StateStoreStatus,
};

#[cfg(any(test, feature = "bench", feature = "fuzzing"))]
pub use crate::sparse_merkle::test_utils;

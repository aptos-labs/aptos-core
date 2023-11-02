// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This crate provides in-memory representation of Aptos core data structures used by the executor.

mod sparse_merkle;

#[cfg(any(test, feature = "bench", feature = "fuzzing"))]
pub use crate::sparse_merkle::test_utils;
pub use crate::sparse_merkle::{
    ancestors::SmtAncestors, utils::get_state_shard_id, FrozenSparseMerkleTree, ProofRead,
    SparseMerkleTree, StateStoreStatus,
};

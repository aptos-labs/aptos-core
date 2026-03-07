// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Defines types to version cache entries during execution of transactions.

/// Transaction index within a block.
pub type TxnIndex = u32;

/// Block index, incremented at the end of every block.
pub type BlockIndex = u32;

/// Represents a version when data was created (e.g., written by a transaction
/// or read from storage) that can be stored in cache. If loaded from storage,
/// transaction index is set to 0. If written by transaction, indices are
/// shifted by 1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub block_idx: BlockIndex,
    pub txn_idx: TxnIndex,
}

impl Version {
    /// Returns a new version corresponding to the specified transaction in
    /// the block.
    pub fn from_txn_idx(block_idx: BlockIndex, txn_idx: TxnIndex) -> Self {
        Self {
            block_idx,
            txn_idx: txn_idx + 1,
        }
    }

    /// Returns a new version corresponding to the pre-block state.
    pub fn from_storage(block_idx: BlockIndex) -> Self {
        Self {
            block_idx,
            txn_idx: 0,
        }
    }
}

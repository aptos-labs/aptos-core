// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Data persistence utilities for testing tools.
//!
//! This module provides utilities for persisting transaction data, primarily
//! focused on file-based storage of transaction blocks for replay and testing.

use anyhow::{anyhow, Result};
use aptos_types::transaction::{Transaction, Version};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// On-disk representation of a transaction block.
///
/// This structure represents a block of transactions that can be serialized
/// and persisted to disk for later replay or analysis.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionBlock {
    /// The version of the first transaction in the block.
    pub begin_version: Version,
    /// Non-empty list of transactions in a block.
    pub transactions: Vec<Transaction>,
}

impl TransactionBlock {
    /// Creates a new TransactionBlock.
    ///
    /// # Arguments
    /// * `begin_version` - The version of the first transaction
    /// * `transactions` - List of transactions in the block
    pub fn new(begin_version: Version, transactions: Vec<Transaction>) -> Self {
        Self {
            begin_version,
            transactions,
        }
    }

    /// Serializes the transaction block to bytes using BCS.
    ///
    /// # Returns
    /// Serialized bytes, or error if serialization fails
    pub fn serialize_to_bytes(&self) -> Result<Vec<u8>> {
        bcs::to_bytes(self).map_err(|err| anyhow!("Error serializing transaction block: {:?}", err))
    }

    /// Deserializes a transaction block from bytes using BCS.
    ///
    /// # Arguments
    /// * `bytes` - BCS-encoded transaction block bytes
    ///
    /// # Returns
    /// Deserialized TransactionBlock, or error if deserialization fails
    pub fn deserialize_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes)
            .map_err(|err| anyhow!("Error deserializing transaction block: {:?}", err))
    }

    /// Saves the transaction block to a file.
    ///
    /// # Arguments
    /// * `path` - Path where the transaction block will be saved
    ///
    /// # Returns
    /// Ok if successful, error otherwise
    ///
    /// # Example
    /// ```no_run
    /// # use aptos_move_testing_utils::TransactionBlock;
    /// # use std::path::Path;
    /// # async fn example() {
    /// # let block = TransactionBlock::new(0, vec![]);
    /// block.save_to_file(Path::new("transactions.bcs")).await.unwrap();
    /// # }
    /// ```
    pub async fn save_to_file(&self, path: &Path) -> Result<()> {
        let bytes = self.serialize_to_bytes()?;
        tokio::fs::write(path, &bytes).await?;
        Ok(())
    }

    /// Loads a transaction block from a file.
    ///
    /// # Arguments
    /// * `path` - Path to the file containing the transaction block
    ///
    /// # Returns
    /// Deserialized TransactionBlock, or error if loading fails
    ///
    /// # Example
    /// ```no_run
    /// # use aptos_move_testing_utils::TransactionBlock;
    /// # use std::path::Path;
    /// # async fn example() {
    /// let block = TransactionBlock::load_from_file(Path::new("transactions.bcs"))
    ///     .await
    ///     .unwrap();
    /// # }
    /// ```
    pub async fn load_from_file(path: &Path) -> Result<Self> {
        let bytes = tokio::fs::read(path).await?;
        Self::deserialize_from_bytes(&bytes)
    }
}

/// Serializes a vector of transaction blocks to bytes.
///
/// # Arguments
/// * `blocks` - Vector of transaction blocks to serialize
///
/// # Returns
/// Serialized bytes, or error if serialization fails
pub fn serialize_blocks(blocks: &[TransactionBlock]) -> Result<Vec<u8>> {
    bcs::to_bytes(blocks).map_err(|err| anyhow!("Error serializing transaction blocks: {:?}", err))
}

/// Deserializes a vector of transaction blocks from bytes.
///
/// # Arguments
/// * `bytes` - BCS-encoded transaction blocks bytes
///
/// # Returns
/// Vector of deserialized TransactionBlocks, or error if deserialization fails
pub fn deserialize_blocks(bytes: &[u8]) -> Result<Vec<TransactionBlock>> {
    bcs::from_bytes(bytes)
        .map_err(|err| anyhow!("Error deserializing transaction blocks: {:?}", err))
}

/// Saves multiple transaction blocks to a file.
///
/// # Arguments
/// * `blocks` - Vector of transaction blocks to save
/// * `path` - Path where the blocks will be saved
///
/// # Returns
/// Ok if successful, error otherwise
///
/// # Example
/// ```no_run
/// # use aptos_move_testing_utils::{TransactionBlock, save_blocks_to_file};
/// # use std::path::Path;
/// # async fn example() {
/// let blocks = vec![
///     TransactionBlock::new(0, vec![]),
///     TransactionBlock::new(10, vec![]),
/// ];
/// save_blocks_to_file(&blocks, Path::new("blocks.bcs")).await.unwrap();
/// # }
/// ```
pub async fn save_blocks_to_file(blocks: &[TransactionBlock], path: &Path) -> Result<()> {
    let bytes = serialize_blocks(blocks)?;
    tokio::fs::write(path, &bytes).await?;
    Ok(())
}

/// Loads multiple transaction blocks from a file.
///
/// # Arguments
/// * `path` - Path to the file containing transaction blocks
///
/// # Returns
/// Vector of deserialized TransactionBlocks, or error if loading fails
///
/// # Example
/// ```no_run
/// # use aptos_move_testing_utils::load_blocks_from_file;
/// # use std::path::Path;
/// # async fn example() {
/// let blocks = load_blocks_from_file(Path::new("blocks.bcs")).await.unwrap();
/// # }
/// ```
pub async fn load_blocks_from_file(path: &Path) -> Result<Vec<TransactionBlock>> {
    let bytes = tokio::fs::read(path).await?;
    deserialize_blocks(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_block_creation() {
        let block = TransactionBlock::new(100, vec![]);
        assert_eq!(block.begin_version, 100);
        assert!(block.transactions.is_empty());
    }

    #[test]
    fn test_transaction_block_serialization() {
        let block = TransactionBlock::new(42, vec![]);
        let bytes = block.serialize_to_bytes().unwrap();
        let deserialized = TransactionBlock::deserialize_from_bytes(&bytes).unwrap();
        assert_eq!(deserialized.begin_version, 42);
        assert!(deserialized.transactions.is_empty());
    }

    #[test]
    fn test_blocks_serialization() {
        let blocks = vec![
            TransactionBlock::new(0, vec![]),
            TransactionBlock::new(10, vec![]),
        ];
        let bytes = serialize_blocks(&blocks).unwrap();
        let deserialized = deserialize_blocks(&bytes).unwrap();
        assert_eq!(deserialized.len(), 2);
        assert_eq!(deserialized[0].begin_version, 0);
        assert_eq!(deserialized[1].begin_version, 10);
    }
}

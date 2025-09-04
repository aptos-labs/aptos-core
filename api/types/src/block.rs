// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{HashValue, Transaction, TransactionOnChainData, U64};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

/// A Block with or without transactions
///
/// This contains the information about a transactions along with
/// associated transactions if requested
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct Block {
    pub block_height: U64,
    pub block_hash: HashValue,
    pub block_timestamp: U64,
    pub first_version: U64,
    pub last_version: U64,
    /// The transactions in the block in sequential order
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transactions: Option<Vec<Transaction>>,
}

/// A Block with or without transactions for encoding in BCS
///
/// This contains the information about a transactions along with
/// associated transactions if requested
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BcsBlock {
    /// The block height (number of the block from 0)
    pub block_height: u64,
    pub block_hash: velor_crypto::HashValue,
    /// The block timestamp in Unix epoch microseconds
    pub block_timestamp: u64,
    /// The first ledger version of the block inclusive
    pub first_version: u64,
    /// The last ledger version of the block inclusive
    pub last_version: u64,
    /// The transactions in the block in sequential order
    pub transactions: Option<Vec<TransactionOnChainData>>,
}

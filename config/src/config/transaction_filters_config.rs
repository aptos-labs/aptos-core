// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_transaction_filters::{
    batch_transaction_filter::BatchTransactionFilter,
    block_transaction_filter::BlockTransactionFilter, transaction_filter::TransactionFilter,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransactionFiltersConfig {
    pub api_filter: TransactionFilterConfig, // Filter for the API (e.g., txn simulation)
    pub consensus_filter: BlockTransactionFilterConfig, // Filter for consensus (e.g., proposal voting)
    pub execution_filter: BlockTransactionFilterConfig, // Filter for execution (e.g., block execution)
    pub mempool_filter: TransactionFilterConfig,        // Filter for mempool (e.g., txn submission)
    pub quorum_store_filter: BatchTransactionFilterConfig, // Filter for quorum store (e.g., batch voting)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransactionFilterConfig {
    filter_enabled: bool,                  // Whether the filter is enabled
    transaction_filter: TransactionFilter, // The transaction filter to apply
}

impl TransactionFilterConfig {
    pub fn new(filter_enabled: bool, transaction_filter: TransactionFilter) -> Self {
        Self {
            filter_enabled,
            transaction_filter,
        }
    }

    /// Returns true iff the filter is enabled and not empty
    pub fn is_enabled(&self) -> bool {
        self.filter_enabled && !self.transaction_filter.is_empty()
    }

    /// Returns a reference to the transaction filter
    pub fn transaction_filter(&self) -> &TransactionFilter {
        &self.transaction_filter
    }
}

impl Default for TransactionFilterConfig {
    fn default() -> Self {
        Self {
            filter_enabled: false,                          // Disable the filter
            transaction_filter: TransactionFilter::empty(), // Use an empty filter
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct BatchTransactionFilterConfig {
    filter_enabled: bool, // Whether the filter is enabled
    batch_transaction_filter: BatchTransactionFilter, // The batch transaction filter to apply
}

impl BatchTransactionFilterConfig {
    pub fn new(filter_enabled: bool, batch_transaction_filter: BatchTransactionFilter) -> Self {
        Self {
            filter_enabled,
            batch_transaction_filter,
        }
    }

    /// Returns true iff the filter is enabled and not empty
    pub fn is_enabled(&self) -> bool {
        self.filter_enabled && !self.batch_transaction_filter.is_empty()
    }

    /// Returns a reference to the batch transaction filter
    pub fn batch_transaction_filter(&self) -> &BatchTransactionFilter {
        &self.batch_transaction_filter
    }
}

impl Default for BatchTransactionFilterConfig {
    fn default() -> Self {
        Self {
            filter_enabled: false,                                     // Disable the filter
            batch_transaction_filter: BatchTransactionFilter::empty(), // Use an empty filter
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct BlockTransactionFilterConfig {
    filter_enabled: bool, // Whether the filter is enabled
    block_transaction_filter: BlockTransactionFilter, // The block transaction filter to apply
}

impl BlockTransactionFilterConfig {
    pub fn new(filter_enabled: bool, block_transaction_filter: BlockTransactionFilter) -> Self {
        Self {
            filter_enabled,
            block_transaction_filter,
        }
    }

    /// Returns true iff the filter is enabled and not empty
    pub fn is_enabled(&self) -> bool {
        self.filter_enabled && !self.block_transaction_filter.is_empty()
    }

    /// Returns a reference to the block transaction filter
    pub fn block_transaction_filter(&self) -> &BlockTransactionFilter {
        &self.block_transaction_filter
    }
}

impl Default for BlockTransactionFilterConfig {
    fn default() -> Self {
        Self {
            filter_enabled: false,                                     // Disable the filter
            block_transaction_filter: BlockTransactionFilter::empty(), // Use an empty filter
        }
    }
}

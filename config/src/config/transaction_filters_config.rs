// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_transaction_filters::transaction_filter::TransactionFilter;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransactionFiltersConfig {
    pub api_filter: TransactionFilterConfig, // Filter configuration for the API
    pub consensus_filter: BlockTransactionFilterConfig, // Filter configuration for consensus
    pub execution_filter: TransactionFilterConfig, // Filter configuration for execution
    pub mempool_and_quorum_store_filter: BlockTransactionFilterConfig, // Filter configuration for mempool and quorum store
}

#[allow(clippy::derivable_impls)]
impl Default for TransactionFiltersConfig {
    fn default() -> Self {
        Self {
            api_filter: TransactionFilterConfig::default(),
            consensus_filter: BlockTransactionFilterConfig::default(),
            execution_filter: TransactionFilterConfig::default(),
            mempool_and_quorum_store_filter: BlockTransactionFilterConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransactionFilterConfig {
    pub filter_enabled: bool,                  // Whether the filter is enabled
    pub transaction_filter: TransactionFilter, // The transaction filter to apply
}

#[allow(clippy::derivable_impls)]
impl Default for TransactionFilterConfig {
    fn default() -> Self {
        Self {
            filter_enabled: false,                          // Disable the filter by default
            transaction_filter: TransactionFilter::empty(), // Use an empty filter by default
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct BlockTransactionFilterConfig {
    pub filter_enabled: bool, // Whether the filter is enabled
    pub block_transaction_filter: BlockTransactionFilter, // The block transaction filter to apply
}

#[allow(clippy::derivable_impls)]
impl Default for BlockTransactionFilterConfig {
    fn default() -> Self {
        Self {
            filter_enabled: false, // Disable the filter by default
            block_transaction_filter: BlockTransactionFilter::empty(), // Use an empty filter by default
        }
    }
}

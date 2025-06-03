// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_transactions_filter::transaction_matcher::Filter;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TransactionFilterConfig {
    pub enable_consensus_filter: bool, // Whether to enable the filter in consensus
    pub enable_mempool_filter: bool,   // Whether to enable the filter in mempool
    pub enable_quorum_store_filter: bool, // Whether to enable the filter in quorum store
    pub transaction_filter: Filter,    // The transaction filter to use
}

#[allow(clippy::derivable_impls)]
impl Default for TransactionFilterConfig {
    fn default() -> Self {
        Self {
            enable_consensus_filter: false,    // Disable the consensus filter
            enable_mempool_filter: false,      // Disable the mempool filter
            enable_quorum_store_filter: false, // Disable the quorum store filter
            transaction_filter: Filter::default(),
        }
    }
}

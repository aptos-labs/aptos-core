// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct StateSyncConfig {
    // Size of chunk to request for state synchronization
    pub chunk_limit: u64,
    // The timeout of the state sync client to process a commit notification (in milliseconds)
    pub client_commit_timeout_ms: u64,
    // default timeout used for long polling to remote peer
    pub long_poll_timeout_ms: u64,
    // valid maximum chunk limit for sanity check
    pub max_chunk_limit: u64,
    // valid maximum timeout limit for sanity check
    pub max_timeout_ms: u64,
    // The timeout of the state sync coordinator to receive a commit ack from mempool (in milliseconds)
    pub mempool_commit_timeout_ms: u64,
    // default timeout to make state sync progress by sending chunk requests to a certain number of networks
    // if no progress is made by sending chunk requests to a number of networks,
    // the next sync request will be multicasted, i.e. sent to more networks
    pub multicast_timeout_ms: u64,
    // The timeout for ensuring sync requests are making progress (i.e., the maximum time between
    // commits when processing a sync request).
    pub sync_request_timeout_ms: u64,
    // interval used for checking state synchronization progress
    pub tick_interval_ms: u64,

    // Everything above belongs to state sync v1 and will be removed in the future.

    // The config for the storage service running on each node. Required by
    // state sync v2.
    pub storage_service: StorageServiceConfig,
}

impl Default for StateSyncConfig {
    fn default() -> Self {
        Self {
            chunk_limit: 1000,
            client_commit_timeout_ms: 5_000,
            long_poll_timeout_ms: 10_000,
            max_chunk_limit: 1000,
            max_timeout_ms: 120_000,
            mempool_commit_timeout_ms: 5_000,
            multicast_timeout_ms: 30_000,
            sync_request_timeout_ms: 60_000,
            tick_interval_ms: 100,
            storage_service: StorageServiceConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct StorageServiceConfig {
    pub max_account_states_chunk_sizes: u64, // Max num of accounts per chunk
    pub max_concurrent_requests: u64,        // Max num of concurrent storage server tasks
    pub max_epoch_chunk_size: u64,           // Max num of epoch ending ledger infos per chunk
    pub max_transaction_chunk_size: u64,     // Max num of transactions per chunk
    pub max_transaction_output_chunk_size: u64, // Max num of transaction outputs per chunk
}

impl Default for StorageServiceConfig {
    fn default() -> Self {
        Self {
            max_account_states_chunk_sizes: 1000,
            max_concurrent_requests: 50,
            max_epoch_chunk_size: 100,
            max_transaction_chunk_size: 1000,
            max_transaction_output_chunk_size: 1000,
        }
    }
}

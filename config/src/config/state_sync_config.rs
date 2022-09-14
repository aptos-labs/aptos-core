// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::config::MAX_APPLICATION_MESSAGE_SIZE;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct StateSyncConfig {
    pub data_streaming_service: DataStreamingServiceConfig,
    pub aptos_data_client: AptosDataClientConfig,
    pub state_sync_driver: StateSyncDriverConfig,
    pub storage_service: StorageServiceConfig,
}

/// The bootstrapping mode determines how the node will bootstrap to the latest
/// blockchain state, e.g., directly download the latest states.
#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum BootstrappingMode {
    ApplyTransactionOutputsFromGenesis, // Applies transaction outputs (starting at genesis)
    DownloadLatestStates, // Downloads the state keys and values (at the latest version)
    ExecuteTransactionsFromGenesis, // Executes transactions (starting at genesis)
}

impl BootstrappingMode {
    pub fn to_label(&self) -> &'static str {
        match self {
            BootstrappingMode::ApplyTransactionOutputsFromGenesis => {
                "apply_transaction_outputs_from_genesis"
            }
            BootstrappingMode::DownloadLatestStates => "download_latest_states",
            BootstrappingMode::ExecuteTransactionsFromGenesis => {
                "execute_transactions_from_genesis"
            }
        }
    }
}

/// The continuous syncing mode determines how the node will stay up-to-date
/// once it has bootstrapped and the blockchain continues to grow, e.g.,
/// continuously executing all transactions.
#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum ContinuousSyncingMode {
    ApplyTransactionOutputs, // Applies transaction outputs to stay up-to-date
    ExecuteTransactions,     // Executes transactions to stay up-to-date
}

impl ContinuousSyncingMode {
    pub fn to_label(&self) -> &'static str {
        match self {
            ContinuousSyncingMode::ApplyTransactionOutputs => "apply_transaction_outputs",
            ContinuousSyncingMode::ExecuteTransactions => "execute_transactions",
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct StateSyncDriverConfig {
    pub bootstrapping_mode: BootstrappingMode, // The mode by which to bootstrap
    pub commit_notification_timeout_ms: u64, // The max time taken to process a commit notification
    pub continuous_syncing_mode: ContinuousSyncingMode, // The mode by which to sync after bootstrapping
    pub progress_check_interval_ms: u64, // The interval (ms) at which to check state sync progress
    pub max_connection_deadline_secs: u64, // The max time (secs) to wait for connections from peers
    pub max_consecutive_stream_notifications: u64, // The max number of notifications to process per driver loop
    pub max_pending_data_chunks: u64, // The max number of data chunks pending execution or commit
    pub max_stream_wait_time_ms: u64, // The max time (ms) to wait for a data stream notification
    pub num_versions_to_skip_snapshot_sync: u64, // The version lag we'll tolerate before snapshot syncing
}

/// The default state sync driver config will be the one that gets (and keeps)
/// the node up-to-date as quickly and cheaply as possible.
impl Default for StateSyncDriverConfig {
    fn default() -> Self {
        Self {
            bootstrapping_mode: BootstrappingMode::ApplyTransactionOutputsFromGenesis,
            commit_notification_timeout_ms: 5000,
            continuous_syncing_mode: ContinuousSyncingMode::ApplyTransactionOutputs,
            progress_check_interval_ms: 100,
            max_connection_deadline_secs: 10,
            max_consecutive_stream_notifications: 10,
            max_pending_data_chunks: 100,
            max_stream_wait_time_ms: 5000,
            num_versions_to_skip_snapshot_sync: 100_000_000, // At 5k TPS, this allows a node to fail for about 6 hours.
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct StorageServiceConfig {
    pub max_concurrent_requests: u64, // Max num of concurrent storage server tasks
    pub max_epoch_chunk_size: u64,    // Max num of epoch ending ledger infos per chunk
    pub max_lru_cache_size: u64,      // Max num of items in the lru cache before eviction
    pub max_network_channel_size: u64, // Max num of pending network messages
    pub max_network_chunk_bytes: u64, // Max num of bytes to send per network message
    pub max_state_chunk_size: u64,    // Max num of state keys and values per chunk
    pub max_subscription_period_ms: u64, // Max period (ms) of pending subscription requests
    pub max_transaction_chunk_size: u64, // Max num of transactions per chunk
    pub max_transaction_output_chunk_size: u64, // Max num of transaction outputs per chunk
    pub storage_summary_refresh_interval_ms: u64, // The interval (ms) to refresh the storage summary
}

impl Default for StorageServiceConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 4000,
            max_epoch_chunk_size: 100,
            max_lru_cache_size: 500, // At ~0.6MiB per chunk, this should take no more than 0.5GiB
            max_network_channel_size: 4000,
            max_network_chunk_bytes: MAX_APPLICATION_MESSAGE_SIZE as u64,
            max_state_chunk_size: 2000,
            max_subscription_period_ms: 5000,
            max_transaction_chunk_size: 2000,
            max_transaction_output_chunk_size: 2000,
            storage_summary_refresh_interval_ms: 50,
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DataStreamingServiceConfig {
    // The interval (milliseconds) at which to refresh the global data summary.
    pub global_summary_refresh_interval_ms: u64,

    // Maximum number of concurrent data client requests (per stream).
    pub max_concurrent_requests: u64,

    // Maximum number of concurrent data client requests (per stream) for state keys/values.
    pub max_concurrent_state_requests: u64,

    // Maximum channel sizes for each data stream listener. If messages are not
    // consumed, they will be dropped (oldest messages first). The remaining
    // messages will be retrieved using FIFO ordering.
    pub max_data_stream_channel_sizes: u64,

    // Maximum number of retries for a single client request before a data
    // stream will terminate.
    pub max_request_retry: u64,

    // Maximum number of notification ID to response context mappings held in
    // memory. Once the number grows beyond this value, garbage collection occurs.
    pub max_notification_id_mappings: u64,

    // The interval (milliseconds) at which to check the progress of each stream.
    pub progress_check_interval_ms: u64,
}

impl Default for DataStreamingServiceConfig {
    fn default() -> Self {
        Self {
            global_summary_refresh_interval_ms: 50,
            max_concurrent_requests: 3,
            max_concurrent_state_requests: 6,
            max_data_stream_channel_sizes: 300,
            max_request_retry: 3,
            max_notification_id_mappings: 300,
            progress_check_interval_ms: 100,
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AptosDataClientConfig {
    pub max_num_in_flight_priority_polls: u64, // Max num of in-flight polls for priority peers
    pub max_num_in_flight_regular_polls: u64,  // Max num of in-flight polls for regular peers
    pub response_timeout_ms: u64, // Timeout (in milliseconds) when waiting for a response
    pub summary_poll_interval_ms: u64, // Interval (in milliseconds) between data summary polls
    pub use_compression: bool,    // Whether or not to request compression for incoming data
}

impl Default for AptosDataClientConfig {
    fn default() -> Self {
        Self {
            max_num_in_flight_priority_polls: 10,
            max_num_in_flight_regular_polls: 10,
            response_timeout_ms: 5000,
            summary_poll_interval_ms: 200,
            use_compression: true,
        }
    }
}

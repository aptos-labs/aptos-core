// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::config::{
    config_optimizer::ConfigOptimizer, config_sanitizer::ConfigSanitizer,
    node_config_loader::NodeType, Error, NodeConfig,
};
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

// The maximum message size per state sync message
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024; /* 10 MiB */

// The maximum chunk sizes for data client requests and response
const MAX_EPOCH_CHUNK_SIZE: u64 = 200;
const MAX_STATE_CHUNK_SIZE: u64 = 4000;
const MAX_TRANSACTION_CHUNK_SIZE: u64 = 3000;
const MAX_TRANSACTION_OUTPUT_CHUNK_SIZE: u64 = 3000;

// The maximum number of concurrent requests to send
const MAX_CONCURRENT_REQUESTS: u64 = 6;
const MAX_CONCURRENT_STATE_REQUESTS: u64 = 6;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct StateSyncConfig {
    pub data_streaming_service: DataStreamingServiceConfig,
    pub aptos_data_client: AptosDataClientConfig,
    pub state_sync_driver: StateSyncDriverConfig,
    pub storage_service: StorageServiceConfig,
}

/// The bootstrapping mode determines how the node will bootstrap to the latest
/// blockchain state, e.g., directly download the latest states.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum BootstrappingMode {
    /// Applies transaction outputs (starting at genesis)
    ApplyTransactionOutputsFromGenesis,
    /// Downloads the state keys and values (at the latest version)
    DownloadLatestStates,
    /// Executes transactions (starting at genesis)
    ExecuteTransactionsFromGenesis,
    /// Executes transactions or applies outputs from genesis (whichever is faster)
    ExecuteOrApplyFromGenesis,
}

impl BootstrappingMode {
    pub fn to_label(&self) -> &'static str {
        match self {
            BootstrappingMode::ApplyTransactionOutputsFromGenesis => {
                "apply_transaction_outputs_from_genesis"
            },
            BootstrappingMode::DownloadLatestStates => "download_latest_states",
            BootstrappingMode::ExecuteTransactionsFromGenesis => {
                "execute_transactions_from_genesis"
            },
            BootstrappingMode::ExecuteOrApplyFromGenesis => "execute_or_apply_from_genesis",
        }
    }

    /// Returns true iff the bootstrapping mode is fast sync
    pub fn is_fast_sync(&self) -> bool {
        *self == BootstrappingMode::DownloadLatestStates
    }
}

/// The continuous syncing mode determines how the node will stay up-to-date
/// once it has bootstrapped and the blockchain continues to grow, e.g.,
/// continuously executing all transactions.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum ContinuousSyncingMode {
    /// Applies transaction outputs to stay up-to-date
    ApplyTransactionOutputs,
    /// Executes transactions to stay up-to-date
    ExecuteTransactions,
    /// Executes transactions or applies outputs to stay up-to-date (whichever is faster)
    ExecuteTransactionsOrApplyOutputs,
}

impl ContinuousSyncingMode {
    pub fn to_label(&self) -> &'static str {
        match self {
            ContinuousSyncingMode::ApplyTransactionOutputs => "apply_transaction_outputs",
            ContinuousSyncingMode::ExecuteTransactions => "execute_transactions",
            ContinuousSyncingMode::ExecuteTransactionsOrApplyOutputs => {
                "execute_transactions_or_apply_outputs"
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct StateSyncDriverConfig {
    /// The mode by which to bootstrap
    pub bootstrapping_mode: BootstrappingMode,
    /// The maximum time taken to process a commit notification
    pub commit_notification_timeout_ms: u64,
    /// The mode by which to sync after bootstrapping
    pub continuous_syncing_mode: ContinuousSyncingMode,
    /// Enable auto-bootstrapping if no peers are found after `max_connection_deadline_secs`
    pub enable_auto_bootstrapping: bool,
    /// The interval (ms) to refresh the storage summary
    pub fallback_to_output_syncing_secs: u64,
    /// The interval (ms) at which to check state sync progress
    pub progress_check_interval_ms: u64,
    /// The maximum time (secs) to wait for connections from peers before auto-bootstrapping
    pub max_connection_deadline_secs: u64,
    /// The maximum number of notifications to process per driver loop
    pub max_consecutive_stream_notifications: u64,
    /// The maximum number of stream timeouts allowed before termination
    pub max_num_stream_timeouts: u64,
    /// The maximum number of data chunks pending execution or commit
    pub max_pending_data_chunks: u64,
    /// The maximum number of pending mempool commit notifications
    pub max_pending_mempool_notifications: u64,
    /// The maximum time (ms) to wait for a data stream notification
    pub max_stream_wait_time_ms: u64,
    /// The version lag we'll tolerate before snapshot syncing
    pub num_versions_to_skip_snapshot_sync: u64,
}

/// The default state sync driver config will be the one that gets (and keeps)
/// the node up-to-date as quickly and cheaply as possible.
impl Default for StateSyncDriverConfig {
    fn default() -> Self {
        Self {
            bootstrapping_mode: BootstrappingMode::ExecuteTransactionsFromGenesis,
            commit_notification_timeout_ms: 5000,
            continuous_syncing_mode: ContinuousSyncingMode::ExecuteTransactions,
            enable_auto_bootstrapping: false,
            fallback_to_output_syncing_secs: 180, // 3 minutes
            progress_check_interval_ms: 100,
            max_connection_deadline_secs: 10,
            max_consecutive_stream_notifications: 10,
            max_num_stream_timeouts: 12,
            max_pending_data_chunks: 50,
            max_pending_mempool_notifications: 100,
            max_stream_wait_time_ms: 5000,
            num_versions_to_skip_snapshot_sync: 400_000_000, // At 5k TPS, this allows a node to fail for about 24 hours.
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct StorageServiceConfig {
    /// Whether transaction data v2 is enabled
    pub enable_transaction_data_v2: bool,
    /// Maximum number of epoch ending ledger infos per chunk
    pub max_epoch_chunk_size: u64,
    /// Maximum number of invalid requests per peer
    pub max_invalid_requests_per_peer: u64,
    /// Maximum number of items in the lru cache before eviction
    pub max_lru_cache_size: u64,
    /// Maximum number of pending network messages
    pub max_network_channel_size: u64,
    /// Maximum number of bytes to send per network message
    pub max_network_chunk_bytes: u64,
    /// Maximum number of active subscriptions (per peer)
    pub max_num_active_subscriptions: u64,
    /// Maximum period (ms) of pending optimistic fetch requests
    pub max_optimistic_fetch_period_ms: u64,
    /// Maximum number of state keys and values per chunk
    pub max_state_chunk_size: u64,
    /// Maximum period (ms) of pending subscription requests
    pub max_subscription_period_ms: u64,
    /// Maximum number of transactions per chunk
    pub max_transaction_chunk_size: u64,
    /// Maximum number of transaction outputs per chunk
    pub max_transaction_output_chunk_size: u64,
    /// Minimum time (secs) to ignore peers after too many invalid requests
    pub min_time_to_ignore_peers_secs: u64,
    /// The interval (ms) to refresh the request moderator state
    pub request_moderator_refresh_interval_ms: u64,
    /// The interval (ms) to refresh the storage summary
    pub storage_summary_refresh_interval_ms: u64,
}

impl Default for StorageServiceConfig {
    fn default() -> Self {
        Self {
            enable_transaction_data_v2: false, // TODO: flip this once V2 data is enabled
            max_epoch_chunk_size: MAX_EPOCH_CHUNK_SIZE,
            max_invalid_requests_per_peer: 500,
            max_lru_cache_size: 500, // At ~0.6MiB per chunk, this should take no more than 0.5GiB
            max_network_channel_size: 4000,
            max_network_chunk_bytes: MAX_MESSAGE_SIZE as u64,
            max_num_active_subscriptions: 30,
            max_optimistic_fetch_period_ms: 5000, // 5 seconds
            max_state_chunk_size: MAX_STATE_CHUNK_SIZE,
            max_subscription_period_ms: 30_000, // 30 seconds
            max_transaction_chunk_size: MAX_TRANSACTION_CHUNK_SIZE,
            max_transaction_output_chunk_size: MAX_TRANSACTION_OUTPUT_CHUNK_SIZE,
            min_time_to_ignore_peers_secs: 300, // 5 minutes
            request_moderator_refresh_interval_ms: 1000, // 1 second
            storage_summary_refresh_interval_ms: 100, // Optimal for <= 10 blocks per second
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DataStreamingServiceConfig {
    /// The dynamic prefetching config for the data streaming service
    pub dynamic_prefetching: DynamicPrefetchingConfig,

    /// Whether or not to enable data subscription streaming.
    pub enable_subscription_streaming: bool,

    /// The interval (milliseconds) at which to refresh the global data summary.
    pub global_summary_refresh_interval_ms: u64,

    /// Maximum number of in-flight data client requests (per stream).
    pub max_concurrent_requests: u64,

    /// Maximum number of in-flight data client requests (per stream) for state keys/values.
    pub max_concurrent_state_requests: u64,

    /// Maximum channel sizes for each data stream listener (per stream).
    pub max_data_stream_channel_sizes: u64,

    /// Maximum number of notification ID to response context mappings held in
    /// memory. Once the number grows beyond this value, garbage collection occurs.
    pub max_notification_id_mappings: u64,

    /// Maximum number of consecutive subscriptions that can be made before
    /// the subscription stream is terminated and a new stream must be created.
    pub max_num_consecutive_subscriptions: u64,

    /// Maximum number of pending requests per data stream. This includes the
    /// requests that have already succeeded but have not yet been consumed
    /// because they're head-of-line blocked by other requests.
    pub max_pending_requests: u64,

    /// Maximum number of retries for a single client request before a data
    /// stream will terminate.
    pub max_request_retry: u64,

    /// Maximum lag (in seconds) we'll tolerate when sending subscription requests
    pub max_subscription_stream_lag_secs: u64,

    /// The interval (milliseconds) at which to check the progress of each stream.
    pub progress_check_interval_ms: u64,
}

impl Default for DataStreamingServiceConfig {
    fn default() -> Self {
        Self {
            dynamic_prefetching: DynamicPrefetchingConfig::default(),
            enable_subscription_streaming: true,
            global_summary_refresh_interval_ms: 50,
            max_concurrent_requests: MAX_CONCURRENT_REQUESTS,
            max_concurrent_state_requests: MAX_CONCURRENT_STATE_REQUESTS,
            max_data_stream_channel_sizes: 50,
            max_notification_id_mappings: 300,
            max_num_consecutive_subscriptions: 45, // At ~3 blocks per second, this should last ~15 seconds
            max_pending_requests: 50,
            max_request_retry: 5,
            max_subscription_stream_lag_secs: 10, // 10 seconds
            progress_check_interval_ms: 50,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DynamicPrefetchingConfig {
    /// Whether or not to enable dynamic prefetching
    pub enable_dynamic_prefetching: bool,

    /// The initial number of concurrent prefetching requests
    pub initial_prefetching_value: u64,

    /// Maximum number of in-flight subscription requests
    pub max_in_flight_subscription_requests: u64,

    /// The maximum number of concurrent prefetching requests
    pub max_prefetching_value: u64,

    /// The minimum number of concurrent prefetching requests
    pub min_prefetching_value: u64,

    /// The amount by which to increase the concurrent prefetching value (i.e., on a successful response)
    pub prefetching_value_increase: u64,

    /// The amount by which to decrease the concurrent prefetching value (i.e., on a timeout)
    pub prefetching_value_decrease: u64,

    /// The duration by which to freeze the prefetching value on a timeout
    pub timeout_freeze_duration_secs: u64,
}

impl Default for DynamicPrefetchingConfig {
    fn default() -> Self {
        Self {
            enable_dynamic_prefetching: true,
            initial_prefetching_value: 3,
            max_in_flight_subscription_requests: 9, // At ~3 blocks per second, this should last ~3 seconds
            max_prefetching_value: 30,
            min_prefetching_value: 3,
            prefetching_value_increase: 1,
            prefetching_value_decrease: 2,
            timeout_freeze_duration_secs: 30,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AptosDataPollerConfig {
    /// The additional number of polls to send per peer bucket (per second)
    pub additional_polls_per_peer_bucket: u64,
    /// The minimum number of polls that should be sent per second
    pub min_polls_per_second: u64,
    /// The maximum number of in-flight polls for priority peers
    pub max_num_in_flight_priority_polls: u64,
    /// The maximum number of in-flight polls for regular peers
    pub max_num_in_flight_regular_polls: u64,
    /// The maximum number of polls that should be sent per second
    pub max_polls_per_second: u64,
    /// The number of peers per bucket
    pub peer_bucket_size: u64,
    /// Interval (in ms) between summary poll loop executions
    pub poll_loop_interval_ms: u64,
}

impl Default for AptosDataPollerConfig {
    fn default() -> Self {
        Self {
            additional_polls_per_peer_bucket: 1,
            min_polls_per_second: 5,
            max_num_in_flight_priority_polls: 30,
            max_num_in_flight_regular_polls: 30,
            max_polls_per_second: 20,
            peer_bucket_size: 10,
            poll_loop_interval_ms: 100,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AptosDataMultiFetchConfig {
    /// Whether or not to enable multi-fetch for data client requests
    pub enable_multi_fetch: bool,
    /// The number of additional requests to send per peer bucket
    pub additional_requests_per_peer_bucket: usize,
    /// The minimum number of peers for each multi-fetch request
    pub min_peers_for_multi_fetch: usize,
    /// The maximum number of peers for each multi-fetch request
    pub max_peers_for_multi_fetch: usize,
    /// The number of peers per multi-fetch bucket. We use buckets
    /// to track the number of peers that can service a multi-fetch
    /// request and determine the number of requests to send based on
    /// the configured min, max and additional requests per bucket.
    pub multi_fetch_peer_bucket_size: usize,
}

impl Default for AptosDataMultiFetchConfig {
    fn default() -> Self {
        Self {
            enable_multi_fetch: true,
            additional_requests_per_peer_bucket: 1,
            min_peers_for_multi_fetch: 2,
            max_peers_for_multi_fetch: 3,
            multi_fetch_peer_bucket_size: 10,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AptosLatencyFilteringConfig {
    /// The reduction factor for latency filtering when selecting peers
    pub latency_filtering_reduction_factor: u64,
    /// Minimum peer ratio for latency filtering
    pub min_peer_ratio_for_latency_filtering: u64,
    /// Minimum number of peers before latency filtering can occur
    pub min_peers_for_latency_filtering: u64,
}

impl Default for AptosLatencyFilteringConfig {
    fn default() -> Self {
        Self {
            latency_filtering_reduction_factor: 2, // Only consider the best 50% of peers
            min_peer_ratio_for_latency_filtering: 5, // Only filter if we have at least 5 potential peers per request
            min_peers_for_latency_filtering: 10, // Only filter if we have at least 10 total peers
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AptosDataClientConfig {
    /// Whether transaction data v2 is enabled
    pub enable_transaction_data_v2: bool,
    /// The aptos data poller config for the data client
    pub data_poller_config: AptosDataPollerConfig,
    /// The aptos data multi-fetch config for the data client
    pub data_multi_fetch_config: AptosDataMultiFetchConfig,
    /// Whether or not to ignore peers with low peer scores
    pub ignore_low_score_peers: bool,
    /// The aptos latency filtering config for the data client
    pub latency_filtering_config: AptosLatencyFilteringConfig,
    /// The interval (milliseconds) at which to refresh the latency monitor
    pub latency_monitor_loop_interval_ms: u64,
    /// Maximum number of epoch ending ledger infos per chunk
    pub max_epoch_chunk_size: u64,
    /// Maximum number of output reductions (division by 2) before transactions are returned,
    /// e.g., if 1000 outputs are requested in a single data chunk, and this is set to 1, then
    /// we'll accept anywhere between 1000 and 500 outputs. Any less, and the server should
    /// return transactions instead of outputs.
    // TODO: migrate away from this, and use cleaner chunk packing configs and logic.
    pub max_num_output_reductions: u64,
    /// Maximum lag (in seconds) we'll tolerate when sending optimistic fetch requests
    pub max_optimistic_fetch_lag_secs: u64,
    /// Maximum number of bytes to send in a single response
    pub max_response_bytes: u64,
    /// Maximum timeout (in ms) when waiting for a response (after exponential increases)
    pub max_response_timeout_ms: u64,
    /// Maximum number of state keys and values per chunk
    pub max_state_chunk_size: u64,
    /// Maximum lag (in seconds) we'll tolerate when sending subscription requests
    pub max_subscription_lag_secs: u64,
    /// Maximum number of transactions per chunk
    pub max_transaction_chunk_size: u64,
    /// Maximum number of transaction outputs per chunk
    pub max_transaction_output_chunk_size: u64,
    /// Timeout (in ms) when waiting for an optimistic fetch response
    pub optimistic_fetch_timeout_ms: u64,
    /// First timeout (in ms) when waiting for a response
    pub response_timeout_ms: u64,
    /// Timeout (in ms) when waiting for a subscription response
    pub subscription_response_timeout_ms: u64,
    /// Whether or not to request compression for incoming data
    pub use_compression: bool,
}

impl Default for AptosDataClientConfig {
    fn default() -> Self {
        Self {
            enable_transaction_data_v2: false, // TODO: flip this once V2 data is enabled
            data_poller_config: AptosDataPollerConfig::default(),
            data_multi_fetch_config: AptosDataMultiFetchConfig::default(),
            ignore_low_score_peers: true,
            latency_filtering_config: AptosLatencyFilteringConfig::default(),
            latency_monitor_loop_interval_ms: 100,
            max_epoch_chunk_size: MAX_EPOCH_CHUNK_SIZE,
            max_num_output_reductions: 0,
            max_optimistic_fetch_lag_secs: 20, // 20 seconds
            max_response_bytes: MAX_MESSAGE_SIZE as u64,
            max_response_timeout_ms: 60_000, // 60 seconds
            max_state_chunk_size: MAX_STATE_CHUNK_SIZE,
            max_subscription_lag_secs: 20, // 20 seconds
            max_transaction_chunk_size: MAX_TRANSACTION_CHUNK_SIZE,
            max_transaction_output_chunk_size: MAX_TRANSACTION_OUTPUT_CHUNK_SIZE,
            optimistic_fetch_timeout_ms: 5000,        // 5 seconds
            response_timeout_ms: 10_000,              // 10 seconds
            subscription_response_timeout_ms: 15_000, // 15 seconds (longer than a regular timeout because of prefetching)
            use_compression: true,
        }
    }
}

impl ConfigSanitizer for StateSyncConfig {
    fn sanitize(
        node_config: &NodeConfig,
        node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        // Sanitize the state sync driver config
        StateSyncDriverConfig::sanitize(node_config, node_type, chain_id)
    }
}

impl ConfigSanitizer for StateSyncDriverConfig {
    fn sanitize(
        node_config: &NodeConfig,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();
        let state_sync_driver_config = &node_config.state_sync.state_sync_driver;

        // Verify that auto-bootstrapping is not enabled for
        // nodes that are fast syncing.
        let fast_sync_enabled = state_sync_driver_config.bootstrapping_mode.is_fast_sync();
        if state_sync_driver_config.enable_auto_bootstrapping && fast_sync_enabled {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "Auto-bootstrapping should not be enabled for nodes that are fast syncing!"
                    .to_string(),
            ));
        }

        Ok(())
    }
}

impl ConfigOptimizer for StateSyncConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        local_config_yaml: &Value,
        node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        // Optimize the driver and data streaming service configs
        let modified_driver_config =
            StateSyncDriverConfig::optimize(node_config, local_config_yaml, node_type, chain_id)?;
        let modified_data_streaming_config = DataStreamingServiceConfig::optimize(
            node_config,
            local_config_yaml,
            node_type,
            chain_id,
        )?;

        Ok(modified_driver_config || modified_data_streaming_config)
    }
}

impl ConfigOptimizer for StateSyncDriverConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        local_config_yaml: &Value,
        _node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        let state_sync_driver_config = &mut node_config.state_sync.state_sync_driver;
        let local_driver_config_yaml = &local_config_yaml["state_sync"]["state_sync_driver"];

        // Default to fast sync for all testnet and mainnet nodes
        // because pruning has kicked in, and nodes will struggle
        // to locate all the data since genesis.
        let mut modified_config = false;
        if let Some(chain_id) = chain_id {
            if (chain_id.is_testnet() || chain_id.is_mainnet())
                && local_driver_config_yaml["bootstrapping_mode"].is_null()
            {
                state_sync_driver_config.bootstrapping_mode =
                    BootstrappingMode::DownloadLatestStates;
                modified_config = true;
            }
        }

        Ok(modified_config)
    }
}

impl ConfigOptimizer for DataStreamingServiceConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        local_config_yaml: &Value,
        node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        let data_streaming_service_config = &mut node_config.state_sync.data_streaming_service;
        let local_stream_config_yaml = &local_config_yaml["state_sync"]["data_streaming_service"];

        // Double the aggression of the pre-fetcher for validators and VFNs
        let mut modified_config = false;
        if node_type.is_validator() || node_type.is_validator_fullnode() {
            // Double transaction prefetching
            if local_stream_config_yaml["max_concurrent_requests"].is_null() {
                data_streaming_service_config.max_concurrent_requests = MAX_CONCURRENT_REQUESTS * 2;
                modified_config = true;
            }

            // Double state-value prefetching
            if local_stream_config_yaml["max_concurrent_state_requests"].is_null() {
                data_streaming_service_config.max_concurrent_state_requests =
                    MAX_CONCURRENT_STATE_REQUESTS * 2;
                modified_config = true;
            }
        }

        Ok(modified_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimize_bootstrapping_mode_devnet_vfn() {
        // Create a node config with execution mode enabled
        let mut node_config = create_execution_mode_config();

        // Optimize the config and verify modifications are made
        let modified_config = StateSyncConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::ValidatorFullnode,
            Some(ChainId::new(40)), // Not mainnet or testnet
        )
        .unwrap();
        assert!(modified_config);

        // Verify that the bootstrapping mode is not changed
        assert_eq!(
            node_config.state_sync.state_sync_driver.bootstrapping_mode,
            BootstrappingMode::ExecuteTransactionsFromGenesis
        );
    }

    #[test]
    fn test_optimize_bootstrapping_mode_testnet_validator() {
        // Create a node config with execution mode enabled
        let mut node_config = create_execution_mode_config();

        // Optimize the config and verify modifications are made
        let modified_config = StateSyncConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::Validator,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that the bootstrapping mode is now set to fast sync
        let state_sync_driver_config = node_config.state_sync.state_sync_driver;
        assert!(state_sync_driver_config.bootstrapping_mode.is_fast_sync());
    }

    #[test]
    fn test_optimize_bootstrapping_mode_mainnet_vfn() {
        // Create a node config with execution mode enabled
        let mut node_config = create_execution_mode_config();

        // Optimize the config and verify modifications are made
        let modified_config = StateSyncConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::ValidatorFullnode,
            Some(ChainId::mainnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that the bootstrapping mode is now set to fast sync
        let state_sync_driver_config = node_config.state_sync.state_sync_driver;
        assert!(state_sync_driver_config.bootstrapping_mode.is_fast_sync());
    }

    #[test]
    fn test_optimize_bootstrapping_mode_no_override() {
        // Create a node config with execution mode enabled
        let mut node_config = create_execution_mode_config();

        // Create a local config YAML with the bootstrapping mode set to execution mode
        let local_config_yaml = serde_yaml::from_str(
            r#"
            state_sync:
                state_sync_driver:
                    bootstrapping_mode: ExecuteTransactionsFromGenesis
            "#,
        )
        .unwrap();

        // Optimize the config and verify modifications are made
        let modified_config = StateSyncConfig::optimize(
            &mut node_config,
            &local_config_yaml,
            NodeType::ValidatorFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that the bootstrapping mode is still set to execution mode
        assert_eq!(
            node_config.state_sync.state_sync_driver.bootstrapping_mode,
            BootstrappingMode::ExecuteTransactionsFromGenesis
        );
    }

    #[test]
    fn test_optimize_prefetcher_mainnet_validator() {
        // Create a default node config
        let mut node_config = NodeConfig::default();

        // Optimize the config and verify modifications are made
        let modified_config = StateSyncConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::Validator,
            Some(ChainId::mainnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that the prefetcher aggression has doubled
        let data_streaming_service_config = &node_config.state_sync.data_streaming_service;
        assert_eq!(
            data_streaming_service_config.max_concurrent_requests,
            MAX_CONCURRENT_REQUESTS * 2
        );
        assert_eq!(
            data_streaming_service_config.max_concurrent_state_requests,
            MAX_CONCURRENT_STATE_REQUESTS * 2
        );
    }

    #[test]
    fn test_optimize_prefetcher_testnet_pfn() {
        // Create a default node config
        let mut node_config = NodeConfig::default();

        // Optimize the config and verify modifications are made
        let modified_config = StateSyncConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::PublicFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that the prefetcher aggression has not changed
        let data_streaming_service_config = &node_config.state_sync.data_streaming_service;
        assert_eq!(
            data_streaming_service_config.max_concurrent_requests,
            MAX_CONCURRENT_REQUESTS
        );
        assert_eq!(
            data_streaming_service_config.max_concurrent_state_requests,
            MAX_CONCURRENT_STATE_REQUESTS
        );
    }

    #[test]
    fn test_optimize_prefetcher_vfn_no_override() {
        // Create a node config where the state prefetcher is set to 100
        let mut node_config = NodeConfig {
            state_sync: StateSyncConfig {
                data_streaming_service: DataStreamingServiceConfig {
                    max_concurrent_state_requests: 100,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Create a local config YAML where the state prefetcher is set to 100
        let local_config_yaml = serde_yaml::from_str(
            r#"
            state_sync:
                data_streaming_service:
                    max_concurrent_state_requests: 100
            "#,
        )
        .unwrap();

        // Optimize the config and verify modifications are made
        let modified_config = StateSyncConfig::optimize(
            &mut node_config,
            &local_config_yaml,
            NodeType::ValidatorFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that the prefetcher aggression has changed but not the state prefetcher
        let data_streaming_service_config = &node_config.state_sync.data_streaming_service;
        assert_eq!(
            data_streaming_service_config.max_concurrent_requests,
            MAX_CONCURRENT_REQUESTS * 2
        );
        assert_eq!(
            data_streaming_service_config.max_concurrent_state_requests,
            100
        );
    }

    #[test]
    fn test_sanitize_auto_bootstrapping_fast_sync() {
        // Create a node config with fast sync and
        // auto bootstrapping enabled.
        let node_config = NodeConfig {
            state_sync: StateSyncConfig {
                state_sync_driver: StateSyncDriverConfig {
                    bootstrapping_mode: BootstrappingMode::DownloadLatestStates,
                    enable_auto_bootstrapping: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Verify that sanitization fails
        let error =
            StateSyncConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::testnet()))
                .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    /// Creates and returns a node config with the syncing modes set to execution
    fn create_execution_mode_config() -> NodeConfig {
        NodeConfig {
            state_sync: StateSyncConfig {
                state_sync_driver: StateSyncDriverConfig {
                    bootstrapping_mode: BootstrappingMode::ExecuteTransactionsFromGenesis,
                    continuous_syncing_mode: ContinuousSyncingMode::ExecuteTransactions,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

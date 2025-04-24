// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::config::{
    config_optimizer::ConfigOptimizer, config_sanitizer::ConfigSanitizer,
    node_config_loader::NodeType, Error, NodeConfig, MAX_APPLICATION_MESSAGE_SIZE,
};
use aptos_global_constants::DEFAULT_BUCKETS;
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoadBalancingThresholdConfig {
    /// PFN load balances the traffic to multiple upstream FNs. The PFN calculates the average mempool traffic in TPS received since
    /// the last peer udpate. If the average received mempool traffic is greater than this threshold, then the below limits are used
    /// to decide the number of upstream peers to forward the mempool traffic.
    pub avg_mempool_traffic_threshold_in_tps: u64,
    /// Suppose the smallest ping latency amongst the connected upstream peers is `x`. If the average received mempool traffic is
    /// greater than `avg_mempool_traffic_threshold_in_tps`, then the PFN will forward mempool traffic to only those upstream peers
    /// with ping latency less than `x + latency_slack_between_top_upstream_peers`.
    pub latency_slack_between_top_upstream_peers: u64,
    /// If the average received mempool traffic is greater than avg_mempool_traffic_threshold_in_tps, then PFNs will forward to at most
    /// `max_number_of_upstream_peers` upstream FNs.
    pub max_number_of_upstream_peers: u8,
}

impl Default for LoadBalancingThresholdConfig {
    fn default() -> LoadBalancingThresholdConfig {
        LoadBalancingThresholdConfig {
            avg_mempool_traffic_threshold_in_tps: 0,
            latency_slack_between_top_upstream_peers: 50,
            max_number_of_upstream_peers: 1,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct MempoolConfig {
    /// Maximum number of transactions allowed in the Mempool
    pub capacity: usize,
    /// Maximum number of bytes allowed in the Mempool
    pub capacity_bytes: usize,
    /// Maximum number of sequence number based transactions allowed in the Mempool per user
    pub capacity_per_user: usize,
    /// Number of failover peers to broadcast to when the primary network is alive
    pub default_failovers: usize,
    /// Whether or not to enable intelligent peer prioritization
    pub enable_intelligent_peer_prioritization: bool,
    /// The maximum number of broadcasts sent to a single peer that are pending a response ACK at any point.
    pub max_broadcasts_per_peer: usize,
    /// Maximum number of inbound network messages to the Mempool application
    pub max_network_channel_size: usize,
    /// The interval to take a snapshot of the mempool to logs, only used when trace logging is enabled
    pub mempool_snapshot_interval_secs: u64,
    /// The maximum amount of time to wait for an ACK of Mempool submission to an upstream node.
    pub shared_mempool_ack_timeout_ms: u64,
    /// The amount of time to backoff between retries of Mempool submission to an upstream node.
    pub shared_mempool_backoff_interval_ms: u64,
    /// Maximum number of transactions to batch for a Mempool submission to an upstream node.
    pub shared_mempool_batch_size: usize,
    /// Maximum number of bytes to batch for a Mempool submission to an upstream node.
    pub shared_mempool_max_batch_bytes: u64,
    /// Maximum Mempool inbound message workers.  Controls concurrency of Mempool consumption.
    pub shared_mempool_max_concurrent_inbound_syncs: usize,
    /// Interval to broadcast to upstream nodes.
    pub shared_mempool_tick_interval_ms: u64,
    /// Interval to update peers in shared mempool.
    pub shared_mempool_peer_update_interval_ms: u64,
    /// Interval to update peer priorities in shared mempool (seconds).
    pub shared_mempool_priority_update_interval_secs: u64,
    /// The amount of time to wait after transaction insertion to broadcast to a failover peer.
    pub shared_mempool_failover_delay_ms: u64,
    /// Number of seconds until the transaction will be removed from the Mempool ignoring if the transaction has expired.
    ///
    /// This ensures that the Mempool isn't just full of non-expiring transactions that are way off into the future.
    pub system_transaction_timeout_secs: u64,
    /// Interval to garbage collect and remove transactions that have expired from the Mempool.
    pub system_transaction_gc_interval_ms: u64,
    /// Gas unit price buckets for broadcasting to upstream nodes.
    ///
    /// Overriding this won't make much of a difference if the upstream nodes don't match.
    pub broadcast_buckets: Vec<u64>,
    pub eager_expire_threshold_ms: Option<u64>,
    pub eager_expire_time_ms: u64,
    /// Uses the BroadcastTransactionsRequestWithReadyTime instead of BroadcastTransactionsRequest when sending
    /// mempool transactions to upstream nodes.
    pub include_ready_time_in_broadcast: bool,
    pub usecase_stats_num_blocks_to_track: usize,
    pub usecase_stats_num_top_to_track: usize,
    /// We divide the transactions into buckets based on hash of the sender address.
    /// This is the number of sender buckets we use.
    pub num_sender_buckets: u8,
    /// Load balancing configuration for the mempool. This is used only by PFNs.
    pub load_balancing_thresholds: Vec<LoadBalancingThresholdConfig>,
    /// When the load is low, PFNs send all the mempool traffic to only one upstream FN. When the load increases suddenly, PFNs will take
    /// up to 10 minutes (shared_mempool_priority_update_interval_secs) to enable the load balancing. If this flag is enabled,
    /// then the PFNs will always do load balancing irrespective of the load.
    pub enable_max_load_balancing_at_any_load: bool,
    /// Maximum number of orderless transactions allowed in the Mempool per user
    pub orderless_txn_capacity_per_user: usize,
}

impl Default for MempoolConfig {
    fn default() -> MempoolConfig {
        MempoolConfig {
            shared_mempool_tick_interval_ms: 10,
            shared_mempool_backoff_interval_ms: 30_000,
            shared_mempool_batch_size: 300,
            shared_mempool_max_batch_bytes: MAX_APPLICATION_MESSAGE_SIZE as u64,
            shared_mempool_ack_timeout_ms: 2_000,
            shared_mempool_max_concurrent_inbound_syncs: 4,
            max_broadcasts_per_peer: 20,
            max_network_channel_size: 1024,
            mempool_snapshot_interval_secs: 180,
            capacity: 2_000_000,
            capacity_bytes: 2 * 1024 * 1024 * 1024,
            capacity_per_user: 100,
            default_failovers: 1,
            enable_intelligent_peer_prioritization: true,
            shared_mempool_peer_update_interval_ms: 1_000,
            shared_mempool_priority_update_interval_secs: 600, // 10 minutes (frequent reprioritization is expensive)
            shared_mempool_failover_delay_ms: 500,
            system_transaction_timeout_secs: 600,
            system_transaction_gc_interval_ms: 60_000,
            broadcast_buckets: DEFAULT_BUCKETS.to_vec(),
            eager_expire_threshold_ms: Some(15_000),
            eager_expire_time_ms: 6_000,
            include_ready_time_in_broadcast: false,
            usecase_stats_num_blocks_to_track: 40,
            usecase_stats_num_top_to_track: 5,
            num_sender_buckets: 4,
            load_balancing_thresholds: vec![
                LoadBalancingThresholdConfig {
                    avg_mempool_traffic_threshold_in_tps: 500,
                    latency_slack_between_top_upstream_peers: 50,
                    max_number_of_upstream_peers: 2,
                },
                LoadBalancingThresholdConfig {
                    avg_mempool_traffic_threshold_in_tps: 1000,
                    latency_slack_between_top_upstream_peers: 50,
                    max_number_of_upstream_peers: 3,
                },
                LoadBalancingThresholdConfig {
                    avg_mempool_traffic_threshold_in_tps: 1500,
                    latency_slack_between_top_upstream_peers: 75,
                    max_number_of_upstream_peers: 4,
                },
                LoadBalancingThresholdConfig {
                    avg_mempool_traffic_threshold_in_tps: 2500,
                    latency_slack_between_top_upstream_peers: 100,
                    max_number_of_upstream_peers: 5,
                },
                LoadBalancingThresholdConfig {
                    avg_mempool_traffic_threshold_in_tps: 3500,
                    latency_slack_between_top_upstream_peers: 125,
                    max_number_of_upstream_peers: 6,
                },
                LoadBalancingThresholdConfig {
                    avg_mempool_traffic_threshold_in_tps: 4500,
                    latency_slack_between_top_upstream_peers: 150,
                    max_number_of_upstream_peers: 7,
                },
            ],
            enable_max_load_balancing_at_any_load: false,
            orderless_txn_capacity_per_user: 1000,
        }
    }
}

impl ConfigSanitizer for MempoolConfig {
    fn sanitize(
        _node_config: &NodeConfig,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        Ok(()) // TODO: add reasonable verifications
    }
}

impl ConfigOptimizer for MempoolConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        local_config_yaml: &Value,
        node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        let mempool_config = &mut node_config.mempool;
        let local_mempool_config_yaml = &local_config_yaml["mempool"];

        // Change the default configs for VFNs
        let mut modified_config = false;
        if node_type.is_validator() {
            // Set the max_broadcasts_per_peer to 2 (default is 20)
            if local_mempool_config_yaml["max_broadcasts_per_peer"].is_null() {
                mempool_config.max_broadcasts_per_peer = 2;
                modified_config = true;
            }
            // Set the batch size per broadcast to 200 (default is 300)
            if local_mempool_config_yaml["shared_mempool_batch_size"].is_null() {
                mempool_config.shared_mempool_batch_size = 200;
                modified_config = true;
            }
            // Set the number of sender buckets for load balancing to 1 (default is 4)
            if local_mempool_config_yaml["num_sender_buckets"].is_null() {
                mempool_config.num_sender_buckets = 1;
                modified_config = true;
            }
        }
        if node_type.is_validator_fullnode() {
            // Set the shared_mempool_max_concurrent_inbound_syncs to 16 (default is 4)
            if local_mempool_config_yaml["shared_mempool_max_concurrent_inbound_syncs"].is_null() {
                mempool_config.shared_mempool_max_concurrent_inbound_syncs = 16;
                modified_config = true;
            }

            // Set the default_failovers to 0 (default is 1)
            if local_mempool_config_yaml["default_failovers"].is_null() {
                mempool_config.default_failovers = 0;
                modified_config = true;
            }

            // Set the number of sender buckets for load balancing to 1 (default is 4)
            if local_mempool_config_yaml["num_sender_buckets"].is_null() {
                mempool_config.num_sender_buckets = 1;
                modified_config = true;
            }

            // Set the include_ready_time_in_broadcast to true (default is false)
            if local_mempool_config_yaml["include_ready_time_in_broadcast"].is_null() {
                mempool_config.include_ready_time_in_broadcast = true;
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
    fn test_optimize_vfn_configs() {
        // Create the default VFN config
        let mut node_config = NodeConfig::get_default_vfn_config();

        // Optimize the config and verify modifications are made
        let modified_config = MempoolConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::ValidatorFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that all relevant fields are modified
        let mempool_config = &node_config.mempool;
        assert_eq!(
            mempool_config.shared_mempool_max_concurrent_inbound_syncs,
            16
        );
        assert_eq!(mempool_config.max_broadcasts_per_peer, 20);
        assert_eq!(mempool_config.default_failovers, 0);
        assert_eq!(mempool_config.shared_mempool_batch_size, 300);
        assert_eq!(mempool_config.shared_mempool_tick_interval_ms, 10);
    }

    #[test]
    fn test_optimize_validator_config() {
        // Create the default validator config
        let mut node_config = NodeConfig::get_default_validator_config();

        // Optimize the config and verify no modifications are made
        let modified_config = MempoolConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::Validator,
            Some(ChainId::mainnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that all relevant fields are not modified
        let mempool_config = &node_config.mempool;
        let default_mempool_config = MempoolConfig::default();
        assert_eq!(
            mempool_config.shared_mempool_max_concurrent_inbound_syncs,
            default_mempool_config.shared_mempool_max_concurrent_inbound_syncs
        );
        assert_eq!(mempool_config.max_broadcasts_per_peer, 2);
        assert_eq!(
            mempool_config.default_failovers,
            default_mempool_config.default_failovers
        );
        assert_eq!(mempool_config.shared_mempool_batch_size, 200);
        assert_eq!(
            mempool_config.shared_mempool_tick_interval_ms,
            default_mempool_config.shared_mempool_tick_interval_ms
        );
    }

    #[test]
    fn test_optimize_vfn_config_no_overrides() {
        // Create the default validator config
        let local_shared_mempool_max_concurrent_inbound_syncs = 1;
        let local_max_broadcasts_per_peer = 1;
        let mut node_config = NodeConfig::get_default_vfn_config();
        node_config
            .mempool
            .shared_mempool_max_concurrent_inbound_syncs =
            local_shared_mempool_max_concurrent_inbound_syncs;
        node_config.mempool.max_broadcasts_per_peer = local_max_broadcasts_per_peer;

        // Create a local config YAML with some local overrides
        let local_config_yaml = serde_yaml::from_str(&format!(
            r#"
            mempool:
                shared_mempool_max_concurrent_inbound_syncs: {}
                max_broadcasts_per_peer: {}
            "#,
            local_shared_mempool_max_concurrent_inbound_syncs, local_max_broadcasts_per_peer
        ))
        .unwrap();

        // Optimize the config and verify modifications are made
        let modified_config = MempoolConfig::optimize(
            &mut node_config,
            &local_config_yaml,
            NodeType::ValidatorFullnode,
            Some(ChainId::mainnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that only the relevant fields are modified
        let mempool_config = &node_config.mempool;
        assert_eq!(
            mempool_config.shared_mempool_max_concurrent_inbound_syncs,
            local_shared_mempool_max_concurrent_inbound_syncs
        );
        assert_eq!(
            mempool_config.max_broadcasts_per_peer,
            local_max_broadcasts_per_peer
        );
    }
}

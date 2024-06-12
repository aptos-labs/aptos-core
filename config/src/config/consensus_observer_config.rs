// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::{
    config_optimizer::ConfigOptimizer, node_config_loader::NodeType, Error, NodeConfig,
};
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConsensusObserverConfig {
    /// Whether the consensus observer is enabled
    pub observer_enabled: bool,
    /// Whether the consensus observer publisher is enabled
    pub publisher_enabled: bool,

    /// Maximum number of pending network messages
    pub max_network_channel_size: u64,
    /// Maximum timeout (in milliseconds) for active subscriptions
    pub max_subscription_timeout_ms: u64,
    /// Maximum timeout (in milliseconds) we'll wait for the synced version to
    /// increase before terminating the active subscription.
    pub max_synced_version_timeout_ms: u64,
    /// Interval (in milliseconds) to check progress of the consensus observer
    pub progress_check_interval_ms: u64,
    /// Timeout (in milliseconds) for network RPC requests
    pub request_timeout_ms: u64,
}

impl Default for ConsensusObserverConfig {
    fn default() -> Self {
        Self {
            observer_enabled: false,
            publisher_enabled: false,
            max_network_channel_size: 1000,
            max_subscription_timeout_ms: 30_000,   // 30 seconds
            max_synced_version_timeout_ms: 60_000, // 60 seconds
            progress_check_interval_ms: 5_000,     // 5 seconds
            request_timeout_ms: 10_000,            // 10 seconds
        }
    }
}

impl ConsensusObserverConfig {
    /// Returns true iff the observer or publisher is enabled
    pub fn is_observer_or_publisher_enabled(&self) -> bool {
        self.observer_enabled || self.publisher_enabled
    }
}

impl ConfigOptimizer for ConsensusObserverConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        _local_config_yaml: &Value,
        node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        if node_type.is_validator() {
            node_config.consensus_observer.publisher_enabled = true;
            return Ok(true);
        } else if node_type.is_validator_fullnode() {
            node_config.consensus_observer.observer_enabled = true;
            return Ok(true);
        }

        Ok(false)
    }
}

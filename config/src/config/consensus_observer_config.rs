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
}

impl Default for ConsensusObserverConfig {
    fn default() -> Self {
        Self {
            observer_enabled: false,
            publisher_enabled: false,
            max_network_channel_size: 1000,
        }
    }
}

impl ConsensusObserverConfig {
    /// Returns true iff the observer or publisher is enabled
    pub fn is_enabled(&self) -> bool {
        self.observer_enabled || self.publisher_enabled
    }
}

impl ConfigOptimizer for ConsensusObserverConfig {
    fn optimize(
        _node_config: &mut NodeConfig,
        _local_config_yaml: &Value,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        // TODO: use me to enable consensus observer for
        // validators and VFNs in controlled environments.
        Ok(false)
    }
}

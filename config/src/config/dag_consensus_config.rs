// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    config_sanitizer::ConfigSanitizer, node_config_loader::NodeType, ChainHealthBackoffValues,
    Error, NodeConfig, PipelineBackpressureValues, QuorumStoreConfig,
};
use velor_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DagPayloadConfig {
    pub max_sending_txns_per_round: u64,
    pub max_sending_size_per_round_bytes: u64,
    pub max_receiving_txns_per_round: u64,
    pub max_receiving_size_per_round_bytes: u64,

    pub payload_pull_max_poll_time_ms: u64,
}

impl Default for DagPayloadConfig {
    fn default() -> Self {
        Self {
            max_sending_txns_per_round: 10000,
            max_sending_size_per_round_bytes: 10 * 1024 * 1024,
            max_receiving_txns_per_round: 11000,
            max_receiving_size_per_round_bytes: 20 * 1024 * 1024,

            payload_pull_max_poll_time_ms: 1000,
        }
    }
}

impl ConfigSanitizer for DagPayloadConfig {
    fn sanitize(
        node_config: &NodeConfig,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();
        let dag_node_payload_config = &node_config.dag_consensus.node_payload_config;

        // Sanitize the payload size limits
        Self::sanitize_payload_size_limits(&sanitizer_name, dag_node_payload_config)?;

        Ok(())
    }
}

impl DagPayloadConfig {
    fn sanitize_payload_size_limits(
        sanitizer_name: &str,
        config: &DagPayloadConfig,
    ) -> Result<(), Error> {
        let send_recv_pairs = [
            (
                config.max_sending_txns_per_round,
                config.max_receiving_txns_per_round,
                "txns",
            ),
            (
                config.max_sending_size_per_round_bytes,
                config.max_receiving_size_per_round_bytes,
                "bytes",
            ),
        ];
        for (send, recv, label) in &send_recv_pairs {
            if *send > *recv {
                return Err(Error::ConfigSanitizerFailed(
                    sanitizer_name.to_owned(),
                    format!("Failed {}: {} > {}", label, *send, *recv),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DagFetcherConfig {
    pub retry_interval_ms: u64,
    pub rpc_timeout_ms: u64,
    pub min_concurrent_responders: u32,
    pub max_concurrent_responders: u32,
    pub max_concurrent_fetches: usize,
}

impl Default for DagFetcherConfig {
    fn default() -> Self {
        Self {
            retry_interval_ms: 500,
            rpc_timeout_ms: 1000,
            min_concurrent_responders: 1,
            max_concurrent_responders: 4,
            max_concurrent_fetches: 4,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReliableBroadcastConfig {
    pub backoff_policy_base_ms: u64,
    pub backoff_policy_factor: u64,
    pub backoff_policy_max_delay_ms: u64,

    pub rpc_timeout_ms: u64,
}

impl Default for ReliableBroadcastConfig {
    fn default() -> Self {
        Self {
            // A backoff policy that starts at 100ms and doubles each iteration up to 3secs.
            backoff_policy_base_ms: 2,
            backoff_policy_factor: 50,
            backoff_policy_max_delay_ms: 3000,

            rpc_timeout_ms: 1000,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DagRoundStateConfig {
    pub adaptive_responsive_minimum_wait_time_ms: u64,
}

impl Default for DagRoundStateConfig {
    fn default() -> Self {
        Self {
            adaptive_responsive_minimum_wait_time_ms: 500,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DagHealthConfig {
    pub chain_backoff_config: Vec<ChainHealthBackoffValues>,
    pub voter_pipeline_latency_limit_ms: u64,
    pub pipeline_backpressure_config: Vec<PipelineBackpressureValues>,
}

impl Default for DagHealthConfig {
    fn default() -> Self {
        Self {
            chain_backoff_config: Vec::new(),
            voter_pipeline_latency_limit_ms: 30_000,
            pipeline_backpressure_config: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DagConsensusConfig {
    pub node_payload_config: DagPayloadConfig,
    pub rb_config: ReliableBroadcastConfig,
    pub fetcher_config: DagFetcherConfig,
    pub round_state_config: DagRoundStateConfig,
    pub health_config: DagHealthConfig,
    #[serde(default = "QuorumStoreConfig::default_for_dag")]
    pub quorum_store: QuorumStoreConfig,
}

impl ConfigSanitizer for DagConsensusConfig {
    fn sanitize(
        node_config: &NodeConfig,
        node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        DagPayloadConfig::sanitize(node_config, node_type, chain_id)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_txn_limits() {
        // Create a node config with invalid txn limits
        let node_config = NodeConfig {
            dag_consensus: DagConsensusConfig {
                node_payload_config: DagPayloadConfig {
                    max_sending_txns_per_round: 100,
                    max_receiving_txns_per_round: 99,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error =
            DagPayloadConfig::sanitize(&node_config, NodeType::Validator, None).unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_size_limits() {
        // Create a node config with invalid size limits
        let node_config = NodeConfig {
            dag_consensus: DagConsensusConfig {
                node_payload_config: DagPayloadConfig {
                    max_sending_size_per_round_bytes: 100,
                    max_receiving_size_per_round_bytes: 99,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error =
            DagPayloadConfig::sanitize(&node_config, NodeType::Validator, None).unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }
}

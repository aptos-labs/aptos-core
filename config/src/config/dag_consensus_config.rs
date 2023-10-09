use super::{config_sanitizer::ConfigSanitizer, node_config_loader::NodeType, Error, NodeConfig};
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DagNodePayloadConfig {
    pub max_sending_txns: u64,
    pub max_sending_size_bytes: u64,
    pub max_receiving_txns: u64,
    pub max_receiving_size_bytes: u64,

    pub payload_pull_max_poll_time_ms: u64,
}

impl Default for DagNodePayloadConfig {
    fn default() -> Self {
        Self {
            max_sending_txns: 1000,
            max_sending_size_bytes: 10 * 1024 * 1024,
            max_receiving_txns: 2000,
            max_receiving_size_bytes: 20 * 1024 * 1024,

            payload_pull_max_poll_time_ms: 1000,
        }
    }
}

impl ConfigSanitizer for DagNodePayloadConfig {
    fn sanitize(
        node_config: &NodeConfig,
        _node_type: NodeType,
        _chain_id: ChainId,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();
        let dag_node_payload_config = &node_config.dag_consensus.node_payload_config;

        Self::sanitize_payload_size_limits(&sanitizer_name, dag_node_payload_config)?;

        Ok(())
    }
}

impl DagNodePayloadConfig {
    fn sanitize_payload_size_limits(
        sanitizer_name: &str,
        config: &DagNodePayloadConfig,
    ) -> Result<(), Error> {
        let send_recv_pairs = [
            (config.max_sending_txns, config.max_receiving_txns, "txns"),
            (
                config.max_sending_size_bytes,
                config.max_receiving_size_bytes,
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
}

impl Default for DagFetcherConfig {
    fn default() -> Self {
        Self {
            retry_interval_ms: 500,
            rpc_timeout_ms: 1000,
            min_concurrent_responders: 1,
            max_concurrent_responders: 4,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReliableBroadcastConfig {
    pub backoff_policy_base: u64,
    pub backoff_policy_factor: u64,

    pub rpc_timeout_ms: u64,
}

impl Default for ReliableBroadcastConfig {
    fn default() -> Self {
        Self {
            // A backoff policy that starts at 100ms and doubles each iteration.
            backoff_policy_base: 2,
            backoff_policy_factor: 50,

            rpc_timeout_ms: 500,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DagRoundStateConfig {
    pub round_event_channel_size: usize,
    pub adaptive_responsive_minimum_wait_time_ms: u64,
}

impl Default for DagRoundStateConfig {
    fn default() -> Self {
        Self {
            round_event_channel_size: 1024,
            adaptive_responsive_minimum_wait_time_ms: 300,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DagConsensusConfig {
    pub node_payload_config: DagNodePayloadConfig,
    pub rb_config: ReliableBroadcastConfig,
    pub fetcher_config: DagFetcherConfig,
    pub round_state_config: DagRoundStateConfig,
}

impl Default for DagConsensusConfig {
    fn default() -> Self {
        Self {
            node_payload_config: DagNodePayloadConfig::default(),
            rb_config: ReliableBroadcastConfig::default(),
            fetcher_config: DagFetcherConfig::default(),
            round_state_config: DagRoundStateConfig::default(),
        }
    }
}

impl ConfigSanitizer for DagConsensusConfig {
    fn sanitize(
        node_config: &NodeConfig,
        node_type: NodeType,
        chain_id: ChainId,
    ) -> Result<(), Error> {
        DagNodePayloadConfig::sanitize(node_config, node_type, chain_id)?;

        Ok(())
    }
}

// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::config::{
    config_sanitizer::ConfigSanitizer, node_config_loader::NodeType, Error, NodeConfig,
    QuorumStoreConfig, ReliableBroadcastConfig, SafetyRulesConfig, BATCH_PADDING_BYTES,
};
use aptos_crypto::_once_cell::sync::Lazy;
use aptos_types::chain_id::ChainId;
use cfg_if::cfg_if;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// NOTE: when changing, make sure to update QuorumStoreBackPressureConfig::backlog_txn_limit_count as well.
const MAX_SENDING_BLOCK_UNIQUE_TXNS: u64 = 1900;
pub(crate) static MAX_RECEIVING_BLOCK_TXNS: Lazy<u64> =
    Lazy::new(|| 10000.max(2 * MAX_SENDING_BLOCK_UNIQUE_TXNS));
// The receiving validator can accept upto 2k more transactions in the block than the max sending limit.
// The extra cushion of 2k transactions is added just in case we need to increase the max sending limit in the future.
static MAX_SENDING_BLOCK_TXNS: Lazy<u64> =
    Lazy::new(|| MAX_SENDING_BLOCK_UNIQUE_TXNS.max(MAX_RECEIVING_BLOCK_TXNS.saturating_sub(2000)));

// stop reducing size at this point, so 1MB transactions can still go through
const MIN_BLOCK_BYTES_OVERRIDE: u64 = 1024 * 1024 + BATCH_PADDING_BYTES as u64;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConsensusConfig {
    // length of inbound queue of messages
    pub max_network_channel_size: usize,
    pub max_sending_block_txns: u64,
    pub max_sending_block_unique_txns: u64,
    pub max_sending_block_bytes: u64,
    pub max_sending_inline_txns: u64,
    pub max_sending_inline_bytes: u64,
    pub max_receiving_block_txns: u64,
    pub max_receiving_block_bytes: u64,
    pub max_pruned_blocks_in_mem: usize,
    // Timeout for consensus to get an ack from mempool for executed transactions (in milliseconds)
    pub mempool_executed_txn_timeout_ms: u64,
    // Timeout for consensus to pull transactions from mempool and get a response (in milliseconds)
    pub mempool_txn_pull_timeout_ms: u64,
    pub round_initial_timeout_ms: u64,
    pub round_timeout_backoff_exponent_base: f64,
    pub round_timeout_backoff_max_exponent: usize,
    pub safety_rules: SafetyRulesConfig,
    // Only sync committed transactions but not vote for any pending blocks. This is useful when
    // validators coordinate on the latest version to apply a manual transaction.
    pub sync_only: bool,
    pub channel_size: usize,
    pub quorum_store_pull_timeout_ms: u64,
    // Decides how long the leader waits before proposing empty block if there's no txns in mempool
    pub quorum_store_poll_time_ms: u64,
    // Whether to create partial blocks when few transactions exist, or empty blocks when there is
    // pending ordering, or to wait for quorum_store_poll_count * 30ms to collect transactions for a block
    //
    // It is more efficient to execute larger blocks, as it creates less overhead. On the other hand
    // waiting increases latency (unless we are under high load that added waiting latency
    // is compensated by faster execution time). So we want to balance the two, by waiting only
    // when we are saturating the execution pipeline:
    // - if there are more pending blocks then usual in the execution pipeline,
    //   block is going to wait there anyways, so we can wait to create a bigger/more efificent block
    // - in case our node is faster than others, and we don't have many pending blocks,
    //   but we still see very large recent (pending) blocks, we know that there is demand
    //   and others are creating large blocks, so we can wait as well.
    pub wait_for_full_blocks_above_pending_blocks: usize,
    pub wait_for_full_blocks_above_recent_fill_threshold: f32,
    pub intra_consensus_channel_buffer_size: usize,
    pub quorum_store: QuorumStoreConfig,
    pub vote_back_pressure_limit: u64,
    pub pipeline_backpressure: Vec<PipelineBackpressureValues>,
    // Used to decide if backoff is needed.
    // must match one of the CHAIN_HEALTH_WINDOW_SIZES values.
    pub window_for_chain_health: usize,
    pub chain_health_backoff: Vec<ChainHealthBackoffValues>,
    pub qc_aggregator_type: QcAggregatorType,
    // Max blocks allowed for block retrieval requests
    pub max_blocks_per_sending_request: u64,
    pub max_blocks_per_sending_request_quorum_store_override: u64,
    pub max_blocks_per_receiving_request: u64,
    pub max_blocks_per_receiving_request_quorum_store_override: u64,
    pub broadcast_vote: bool,
    pub proof_cache_capacity: u64,
    pub rand_rb_config: ReliableBroadcastConfig,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum QcAggregatorType {
    #[default]
    NoDelay,
    Delayed(DelayedQcAggregatorConfig),
}

impl QcAggregatorType {
    pub fn default_delayed() -> Self {
        // TODO: Enable the delayed aggregation by default once we have tested it more.
        Self::Delayed(DelayedQcAggregatorConfig::default())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DelayedQcAggregatorConfig {
    // Maximum Delay for a QC to be aggregated after round start (in milliseconds). This assumes that
    // we have enough voting power to form a QC. If we don't have enough voting power, we will wait
    // until we have enough voting power to form a QC.
    pub max_delay_after_round_start_ms: u64,
    // Percentage of aggregated voting power to wait for before aggregating a QC. For example, if this
    // is set to 95% then, a QC is formed as soon as we have 95% of the voting power aggregated without
    // any additional waiting.
    pub aggregated_voting_power_pct_to_wait: usize,
    // This knob control what is the % of the time (as compared to time between round start and time when we
    // have enough voting power to form a QC) we wait after we have enough voting power to form a QC. In a sense,
    // this knobs controls how much slower we are willing to make consensus to wait for more votes.
    pub pct_delay_after_qc_aggregated: usize,
    // In summary, let's denote the time we have enough voting power (2f + 1) to form a QC as T1 and
    // the time we have aggregated `aggregated_voting_power_pct_to_wait` as T2. Then, we wait for
    // min((T1 + `pct_delay_after_qc_aggregated` * T1 / 100), `max_delay_after_round_start_ms`, T2)
    // before forming a QC.
}

impl Default for DelayedQcAggregatorConfig {
    fn default() -> Self {
        Self {
            max_delay_after_round_start_ms: 700,
            aggregated_voting_power_pct_to_wait: 90,
            pct_delay_after_qc_aggregated: 30,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct PipelineBackpressureValues {
    pub back_pressure_pipeline_latency_limit_ms: u64,
    pub max_sending_block_txns_override: u64,
    pub max_sending_block_bytes_override: u64,
    // If there is backpressure, giving some more breathing room to go through the backlog,
    // and making sure rounds don't go extremely fast (even if they are smaller blocks)
    // Set to a small enough value, so it is unlikely to affect proposer being able to finish the round in time.
    // If we want to dynamically increase it beyond quorum_store_poll_time,
    // we need to adjust timeouts other nodes use for the backpressured round.
    pub backpressure_proposal_delay_ms: u64,
    pub max_txns_from_block_to_execute: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ChainHealthBackoffValues {
    pub backoff_if_below_participating_voting_power_percentage: usize,

    pub max_sending_block_txns_override: u64,
    pub max_sending_block_bytes_override: u64,

    pub backoff_proposal_delay_ms: u64,
    pub max_txns_from_block_to_execute: Option<usize>,
}

impl Default for ConsensusConfig {
    fn default() -> ConsensusConfig {
        ConsensusConfig {
            max_network_channel_size: 1024,
            max_sending_block_txns: *MAX_SENDING_BLOCK_TXNS,
            max_sending_block_unique_txns: MAX_SENDING_BLOCK_UNIQUE_TXNS,
            max_sending_block_bytes: 3 * 1024 * 1024, // 3MB
            max_receiving_block_txns: *MAX_RECEIVING_BLOCK_TXNS,
            max_sending_inline_txns: 100,
            max_sending_inline_bytes: 200 * 1024,       // 200 KB
            max_receiving_block_bytes: 6 * 1024 * 1024, // 6MB
            max_pruned_blocks_in_mem: 100,
            mempool_executed_txn_timeout_ms: 1000,
            mempool_txn_pull_timeout_ms: 1000,
            round_initial_timeout_ms: 1500,
            // 1.2^6 ~= 3
            // Timeout goes from initial_timeout to initial_timeout*3 in 6 steps
            round_timeout_backoff_exponent_base: 1.2,
            round_timeout_backoff_max_exponent: 6,
            safety_rules: SafetyRulesConfig::default(),
            sync_only: false,
            channel_size: 30, // hard-coded
            quorum_store_pull_timeout_ms: 400,
            quorum_store_poll_time_ms: 300,
            // disable wait_for_full until fully tested
            // We never go above 20-30 pending blocks, so this disables it
            wait_for_full_blocks_above_pending_blocks: 100,
            // Max is 1, so 1.1 disables it.
            wait_for_full_blocks_above_recent_fill_threshold: 1.1,
            intra_consensus_channel_buffer_size: 10,
            quorum_store: QuorumStoreConfig::default(),

            // Voting backpressure is only used as a backup, to make sure pending rounds don't
            // increase uncontrollably, and we know when to go to state sync.
            // Considering block gas limit and pipeline backpressure should keep number of blocks
            // in the pipline very low, we can keep this limit pretty low, too.
            vote_back_pressure_limit: 7,
            pipeline_backpressure: vec![
                PipelineBackpressureValues {
                    // pipeline_latency looks how long has the oldest block still in pipeline
                    // been in the pipeline.
                    // Block enters the pipeline after consensus orders it, and leaves the
                    // pipeline once quorum on execution result among validators has been reached
                    // (so-(badly)-called "commit certificate"), meaning 2f+1 validators have finished execution.
                    back_pressure_pipeline_latency_limit_ms: 800,
                    max_sending_block_txns_override: *MAX_SENDING_BLOCK_TXNS,
                    max_sending_block_bytes_override: 5 * 1024 * 1024,
                    backpressure_proposal_delay_ms: 100,
                    max_txns_from_block_to_execute: None,
                },
                PipelineBackpressureValues {
                    back_pressure_pipeline_latency_limit_ms: 1100,
                    max_sending_block_txns_override: *MAX_SENDING_BLOCK_TXNS,
                    max_sending_block_bytes_override: 5 * 1024 * 1024,
                    backpressure_proposal_delay_ms: 200,
                    max_txns_from_block_to_execute: None,
                },
                PipelineBackpressureValues {
                    back_pressure_pipeline_latency_limit_ms: 1400,
                    max_sending_block_txns_override: 2000,
                    max_sending_block_bytes_override: MIN_BLOCK_BYTES_OVERRIDE,
                    backpressure_proposal_delay_ms: 300,
                    max_txns_from_block_to_execute: None,
                },
                PipelineBackpressureValues {
                    back_pressure_pipeline_latency_limit_ms: 1700,
                    max_sending_block_txns_override: 1000,
                    max_sending_block_bytes_override: MIN_BLOCK_BYTES_OVERRIDE,
                    backpressure_proposal_delay_ms: 400,
                    max_txns_from_block_to_execute: None,
                },
                PipelineBackpressureValues {
                    back_pressure_pipeline_latency_limit_ms: 2000,
                    max_sending_block_txns_override: 1000,
                    max_sending_block_bytes_override: MIN_BLOCK_BYTES_OVERRIDE,
                    backpressure_proposal_delay_ms: 500,
                    max_txns_from_block_to_execute: Some(400),
                },
                PipelineBackpressureValues {
                    back_pressure_pipeline_latency_limit_ms: 2300,
                    max_sending_block_txns_override: 1000,
                    max_sending_block_bytes_override: MIN_BLOCK_BYTES_OVERRIDE,
                    backpressure_proposal_delay_ms: 500,
                    max_txns_from_block_to_execute: Some(150),
                },
                PipelineBackpressureValues {
                    back_pressure_pipeline_latency_limit_ms: 2700,
                    max_sending_block_txns_override: 1000,
                    max_sending_block_bytes_override: MIN_BLOCK_BYTES_OVERRIDE,
                    backpressure_proposal_delay_ms: 500,
                    max_txns_from_block_to_execute: Some(50),
                },
                PipelineBackpressureValues {
                    back_pressure_pipeline_latency_limit_ms: 3100,
                    max_sending_block_txns_override: 1000,
                    max_sending_block_bytes_override: MIN_BLOCK_BYTES_OVERRIDE,
                    backpressure_proposal_delay_ms: 500,
                    max_txns_from_block_to_execute: Some(20),
                },
                PipelineBackpressureValues {
                    back_pressure_pipeline_latency_limit_ms: 3500,
                    max_sending_block_txns_override: 1000,
                    max_sending_block_bytes_override: MIN_BLOCK_BYTES_OVERRIDE,
                    backpressure_proposal_delay_ms: 500,
                    // in practice, latencies and delay make it such that ~2 blocks/s is max,
                    // meaning that most aggressively we limit to ~10 TPS
                    // For transactions that are more expensive than that, we should
                    // instead rely on max gas per block to limit latency.
                    max_txns_from_block_to_execute: Some(5),
                },
            ],
            window_for_chain_health: 100,
            chain_health_backoff: vec![
                ChainHealthBackoffValues {
                    backoff_if_below_participating_voting_power_percentage: 80,
                    max_sending_block_txns_override: 10000,
                    max_sending_block_bytes_override: 5 * 1024 * 1024,
                    backoff_proposal_delay_ms: 150,
                    max_txns_from_block_to_execute: None,
                },
                ChainHealthBackoffValues {
                    backoff_if_below_participating_voting_power_percentage: 78,
                    max_sending_block_txns_override: 2000,
                    max_sending_block_bytes_override: MIN_BLOCK_BYTES_OVERRIDE,
                    backoff_proposal_delay_ms: 300,
                    max_txns_from_block_to_execute: None,
                },
                ChainHealthBackoffValues {
                    backoff_if_below_participating_voting_power_percentage: 76,
                    max_sending_block_txns_override: 500,
                    max_sending_block_bytes_override: MIN_BLOCK_BYTES_OVERRIDE,
                    backoff_proposal_delay_ms: 300,
                    max_txns_from_block_to_execute: None,
                },
                ChainHealthBackoffValues {
                    backoff_if_below_participating_voting_power_percentage: 74,
                    max_sending_block_txns_override: 500,
                    max_sending_block_bytes_override: MIN_BLOCK_BYTES_OVERRIDE,
                    backoff_proposal_delay_ms: 500,
                    max_txns_from_block_to_execute: Some(100),
                },
                ChainHealthBackoffValues {
                    backoff_if_below_participating_voting_power_percentage: 72,
                    max_sending_block_txns_override: 500,
                    max_sending_block_bytes_override: MIN_BLOCK_BYTES_OVERRIDE,
                    backoff_proposal_delay_ms: 500,
                    max_txns_from_block_to_execute: Some(25),
                },
                ChainHealthBackoffValues {
                    backoff_if_below_participating_voting_power_percentage: 70,
                    max_sending_block_txns_override: 500,
                    max_sending_block_bytes_override: MIN_BLOCK_BYTES_OVERRIDE,
                    backoff_proposal_delay_ms: 500,
                    // in practice, latencies and delay make it such that ~2 blocks/s is max,
                    // meaning that most aggressively we limit to ~10 TPS
                    // For transactions that are more expensive than that, we should
                    // instead rely on max gas per block to limit latency.
                    max_txns_from_block_to_execute: Some(5),
                },
            ],

            qc_aggregator_type: QcAggregatorType::default(),
            // This needs to fit into the network message size, so with quorum store it can be much bigger
            max_blocks_per_sending_request: 10,
            // TODO: this is for release compatibility, after release we can configure it to match the receiving max
            max_blocks_per_sending_request_quorum_store_override: 10,
            max_blocks_per_receiving_request: 10,
            max_blocks_per_receiving_request_quorum_store_override: 100,
            broadcast_vote: true,
            proof_cache_capacity: 10_000,
            rand_rb_config: ReliableBroadcastConfig {
                backoff_policy_base_ms: 2,
                backoff_policy_factor: 100,
                backoff_policy_max_delay_ms: 10000,
                rpc_timeout_ms: 10000,
            },
        }
    }
}

impl ConsensusConfig {
    pub fn set_data_dir(&mut self, data_dir: PathBuf) {
        self.safety_rules.set_data_dir(data_dir);
    }

    pub fn enable_broadcast_vote(&mut self, enable: bool) {
        self.broadcast_vote = enable;
    }

    pub fn max_blocks_per_sending_request(&self, quorum_store_enabled: bool) -> u64 {
        if quorum_store_enabled {
            self.max_blocks_per_sending_request_quorum_store_override
        } else {
            self.max_blocks_per_sending_request
        }
    }

    pub fn max_blocks_per_receiving_request(&self, quorum_store_enabled: bool) -> u64 {
        if quorum_store_enabled {
            self.max_blocks_per_receiving_request_quorum_store_override
        } else {
            self.max_blocks_per_receiving_request
        }
    }

    fn sanitize_send_recv_block_limits(
        sanitizer_name: &str,
        config: &ConsensusConfig,
    ) -> Result<(), Error> {
        let send_recv_pairs = [
            (
                config.max_sending_block_txns,
                config.max_receiving_block_txns,
                "txns",
            ),
            (
                config.max_sending_block_bytes,
                config.max_receiving_block_bytes,
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

    fn sanitize_batch_block_limits(
        sanitizer_name: &str,
        config: &ConsensusConfig,
    ) -> Result<(), Error> {
        // Note, we are strict here: receiver batch limits <= sender block limits
        let mut recv_batch_send_block_pairs = vec![
            (
                config.quorum_store.receiver_max_batch_txns as u64,
                config.max_sending_block_txns,
                "txns".to_string(),
            ),
            (
                config.quorum_store.receiver_max_batch_bytes as u64,
                config.max_sending_block_bytes,
                "bytes".to_string(),
            ),
        ];
        for backpressure_values in &config.pipeline_backpressure {
            recv_batch_send_block_pairs.push((
                config.quorum_store.receiver_max_batch_txns as u64,
                backpressure_values.max_sending_block_txns_override,
                format!(
                    "backpressure {} ms: txns",
                    backpressure_values.back_pressure_pipeline_latency_limit_ms,
                ),
            ));
            recv_batch_send_block_pairs.push((
                config.quorum_store.receiver_max_batch_bytes as u64,
                backpressure_values.max_sending_block_bytes_override,
                format!(
                    "backpressure {} ms: bytes",
                    backpressure_values.back_pressure_pipeline_latency_limit_ms,
                ),
            ));
        }
        for backoff_values in &config.chain_health_backoff {
            recv_batch_send_block_pairs.push((
                config.quorum_store.receiver_max_batch_txns as u64,
                backoff_values.max_sending_block_txns_override,
                format!(
                    "backoff {} %: txns",
                    backoff_values.backoff_if_below_participating_voting_power_percentage,
                ),
            ));
            recv_batch_send_block_pairs.push((
                config.quorum_store.receiver_max_batch_bytes as u64,
                backoff_values.max_sending_block_bytes_override,
                format!(
                    "backoff {} %: bytes",
                    backoff_values.backoff_if_below_participating_voting_power_percentage,
                ),
            ));
        }

        for (batch, block, label) in &recv_batch_send_block_pairs {
            if *batch > *block {
                return Err(Error::ConfigSanitizerFailed(
                    sanitizer_name.to_owned(),
                    format!("Failed {}: {} > {}", label, *batch, *block),
                ));
            }
        }
        Ok(())
    }
}

impl ConfigSanitizer for ConsensusConfig {
    fn sanitize(
        node_config: &NodeConfig,
        node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();

        // Verify that the safety rules and quorum store configs are valid
        SafetyRulesConfig::sanitize(node_config, node_type, chain_id)?;
        QuorumStoreConfig::sanitize(node_config, node_type, chain_id)?;

        // Verify that the consensus-only feature is not enabled in mainnet
        if let Some(chain_id) = chain_id {
            if chain_id.is_mainnet() && is_consensus_only_perf_test_enabled() {
                return Err(Error::ConfigSanitizerFailed(
                    sanitizer_name,
                    "consensus-only-perf-test should not be enabled in mainnet!".to_string(),
                ));
            }
        }

        // Sender block limits must be <= receiver block limits
        Self::sanitize_send_recv_block_limits(&sanitizer_name, &node_config.consensus)?;

        // Quorum store batches must be <= consensus blocks
        Self::sanitize_batch_block_limits(&sanitizer_name, &node_config.consensus)?;

        Ok(())
    }
}

/// Returns true iff consensus-only-perf-test is enabled
fn is_consensus_only_perf_test_enabled() -> bool {
    cfg_if! {
        if #[cfg(feature = "consensus-only-perf-test")] {
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = ConsensusConfig::default();
        let s = serde_yaml::to_string(&config).unwrap();

        serde_yaml::from_str::<ConsensusConfig>(&s).unwrap();
    }

    #[test]
    fn test_send_recv_block_txn_limits() {
        // Create a node config with invalid block txn limits
        let node_config = NodeConfig {
            consensus: ConsensusConfig {
                max_sending_block_txns: 100,
                max_receiving_block_txns: 50,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = ConsensusConfig::sanitize(
            &node_config,
            NodeType::ValidatorFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_send_recv_block_bytes_limits() {
        // Create a node config with invalid block byte limits
        let node_config = NodeConfig {
            consensus: ConsensusConfig {
                max_sending_block_bytes: 100,
                max_receiving_block_bytes: 50,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = ConsensusConfig::sanitize(
            &node_config,
            NodeType::ValidatorFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_send_recv_block_txn_override() {
        // Create a node config with invalid block txn limits
        let node_config = NodeConfig {
            consensus: ConsensusConfig {
                max_sending_block_txns: 100,
                max_receiving_block_txns: 50,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = ConsensusConfig::sanitize(
            &node_config,
            NodeType::ValidatorFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_send_recv_block_byte_override() {
        // Create a node config with invalid block byte limits
        let node_config = NodeConfig {
            consensus: ConsensusConfig {
                max_sending_block_bytes: 100,
                max_receiving_block_bytes: 50,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = ConsensusConfig::sanitize(
            &node_config,
            NodeType::ValidatorFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_invalid_batch_txn_limits() {
        // Create a node config with invalid batch txn limits
        let node_config = NodeConfig {
            consensus: ConsensusConfig {
                max_sending_block_txns: 100,
                quorum_store: QuorumStoreConfig {
                    receiver_max_batch_txns: 101,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error =
            ConsensusConfig::sanitize(&node_config, NodeType::ValidatorFullnode, None).unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_invalid_batch_byte_limits() {
        // Create a node config with invalid batch byte limits
        let node_config = NodeConfig {
            consensus: ConsensusConfig {
                max_sending_block_bytes: 100,
                quorum_store: QuorumStoreConfig {
                    receiver_max_batch_bytes: 101,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error =
            ConsensusConfig::sanitize(&node_config, NodeType::ValidatorFullnode, None).unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_invalid_pipeline_backpressure_txn_limits() {
        // Create a node config with invalid pipeline backpressure txn limits
        let node_config = NodeConfig {
            consensus: ConsensusConfig {
                pipeline_backpressure: vec![PipelineBackpressureValues {
                    back_pressure_pipeline_latency_limit_ms: 0,
                    max_sending_block_txns_override: 350,
                    max_sending_block_bytes_override: 0,
                    backpressure_proposal_delay_ms: 0,
                    max_txns_from_block_to_execute: None,
                }],
                quorum_store: QuorumStoreConfig {
                    receiver_max_batch_txns: 250,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error = ConsensusConfig::sanitize(
            &node_config,
            NodeType::ValidatorFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_invalid_pipeline_backpressure_byte_limits() {
        // Create a node config with invalid pipeline backpressure byte limits
        let node_config = NodeConfig {
            consensus: ConsensusConfig {
                pipeline_backpressure: vec![PipelineBackpressureValues {
                    back_pressure_pipeline_latency_limit_ms: 0,
                    max_sending_block_txns_override: 251,
                    max_sending_block_bytes_override: 100,
                    backpressure_proposal_delay_ms: 0,
                    max_txns_from_block_to_execute: None,
                }],
                quorum_store: QuorumStoreConfig {
                    receiver_max_batch_bytes: 2_000_000,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error =
            ConsensusConfig::sanitize(&node_config, NodeType::ValidatorFullnode, None).unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_invalid_chain_health_backoff_txn_limits() {
        // Create a node config with invalid chain health backoff txn limits
        let node_config = NodeConfig {
            consensus: ConsensusConfig {
                chain_health_backoff: vec![ChainHealthBackoffValues {
                    backoff_if_below_participating_voting_power_percentage: 0,
                    max_sending_block_txns_override: 100,
                    max_sending_block_bytes_override: 0,
                    backoff_proposal_delay_ms: 0,
                    max_txns_from_block_to_execute: None,
                }],
                quorum_store: QuorumStoreConfig {
                    receiver_max_batch_txns: 251,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error =
            ConsensusConfig::sanitize(&node_config, NodeType::ValidatorFullnode, None).unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_invalid_chain_health_backoff_byte_limits() {
        // Create a node config with invalid chain health backoff byte limits
        let node_config = NodeConfig {
            consensus: ConsensusConfig {
                chain_health_backoff: vec![ChainHealthBackoffValues {
                    backoff_if_below_participating_voting_power_percentage: 0,
                    max_sending_block_txns_override: 0,
                    max_sending_block_bytes_override: 100,
                    backoff_proposal_delay_ms: 0,
                    max_txns_from_block_to_execute: None,
                }],
                quorum_store: QuorumStoreConfig {
                    receiver_max_batch_bytes: 2_000_000,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error =
            ConsensusConfig::sanitize(&node_config, NodeType::ValidatorFullnode, None).unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }
}
